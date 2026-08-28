using NAudio.Wave;
using NAudio.Wave.SampleProviders;

namespace BPSR.ReadyAlert;

/// <summary>
/// Reliable Windows MP3 playback for Google Translate TTS responses.
/// Media Foundation performs the MP3 decode. TTS gain is applied to the
/// decoded sample stream before WaveOut so the TTS slider never changes the
/// process-wide Windows audio-session volume used by Ready / Queue alerts.
/// </summary>
internal static class ChatTtsAudioPlayer
{
    private static readonly SemaphoreSlim PlaybackGate = new(1, 1);
    private static readonly TimeSpan PlaybackWatchdog = TimeSpan.FromSeconds(45);
    private static int _fileCounter;

    internal static async Task PlayAsync(byte[] mp3Bytes, int volume, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(mp3Bytes);
        if (mp3Bytes.Length < 200) throw new InvalidDataException("Google TTS audio is unexpectedly small.");
        if (volume <= 0) return;

        await PlaybackGate.WaitAsync(cancellationToken).ConfigureAwait(false);
        string? path = null;
        try
        {
            // Keep every operation after the gate acquisition inside this try/finally.
            // Even an unwritable/missing temp directory must not strand the process-wide
            // playback semaphore and deadlock all future speech attempts.
            var folder = Path.Combine(Path.GetTempPath(), "BPSR-ReadyAlert", "tts");
            Directory.CreateDirectory(folder);
            path = Path.Combine(folder,
                $"speech-{Environment.ProcessId}-{Interlocked.Increment(ref _fileCounter)}.mp3");

            await File.WriteAllBytesAsync(path, mp3Bytes, cancellationToken).ConfigureAwait(false);

            using var reader = new MediaFoundationReader(path);
            using var output = new WaveOutEvent
            {
                DesiredLatency = 90,
                NumberOfBuffers = 3
            };

            // IMPORTANT: do not assign WaveOutEvent.Volume here. On modern Windows,
            // waveOut volume can map to the application's shared audio session, which
            // would also scale Ready / Queue and chat-alert playback from this process.
            // Apply TTS-only gain to the decoded samples instead.
            var ttsSamples = CreateVolumeProvider(reader.ToSampleProvider(), volume);

            var stopped = new TaskCompletionSource<Exception?>(TaskCreationOptions.RunContinuationsAsynchronously);
            void PlaybackStopped(object? sender, StoppedEventArgs args) => stopped.TrySetResult(args.Exception);
            output.PlaybackStopped += PlaybackStopped;

            try
            {
                // Convert back to 16-bit PCM for broad WaveOut compatibility after
                // applying gain in the floating-point sample domain.
                output.Init(ttsSamples, convertTo16Bit: true);
                cancellationToken.ThrowIfCancellationRequested();

                // Start first, then register cancellation. Cancellation that races
                // between these two statements causes Register() to invoke Stop()
                // immediately, while avoiding the old Stop-before-Play race.
                AppLog.Write($"tts: playback start backend=NAudio/MediaFoundation streamVolume={volume}% bytes={mp3Bytes.Length}");
                output.Play();
                using var registration = cancellationToken.Register(() =>
                {
                    try { output.Stop(); } catch { }
                });

                Exception? error;
                try
                {
                    // A WaveOut/driver failure must not be able to hold PlaybackGate
                    // forever if PlaybackStopped is never delivered. 45 seconds is far
                    // above the expected duration of one <=200-character Google chunk.
                    error = await stopped.Task.WaitAsync(PlaybackWatchdog, cancellationToken).ConfigureAwait(false);
                }
                catch (TimeoutException)
                {
                    try { output.Stop(); } catch { }
                    throw new TimeoutException("TTS playback did not stop within the 45-second safety limit.");
                }

                cancellationToken.ThrowIfCancellationRequested();
                if (error is not null) throw new InvalidOperationException("TTS playback failed.", error);
                AppLog.Write("tts: playback completed backend=NAudio/MediaFoundation");
            }
            finally
            {
                output.PlaybackStopped -= PlaybackStopped;
            }
        }
        finally
        {
            if (!string.IsNullOrWhiteSpace(path))
            {
                try { File.Delete(path); } catch { }
            }
            PlaybackGate.Release();
        }
    }

    private static ISampleProvider CreateVolumeProvider(ISampleProvider source, int volume)
    {
        ArgumentNullException.ThrowIfNull(source);
        return new VolumeSampleProvider(source)
        {
            Volume = Math.Clamp(volume, 0, 100) / 100f
        };
    }

    internal static ISampleProvider CreateVolumeProviderForSelfTest(ISampleProvider source, int volume) =>
        CreateVolumeProvider(source, volume);
}
