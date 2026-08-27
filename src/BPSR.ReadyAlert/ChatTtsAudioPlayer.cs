using NAudio.Wave;

namespace BPSR.ReadyAlert;

/// <summary>
/// Reliable Windows MP3 playback for Google Translate TTS responses.
/// Media Foundation performs the MP3 decode and WaveOutEvent owns the application
/// volume, avoiding the legacy MCI backend used by the first v1.2 RC.
/// </summary>
internal static class ChatTtsAudioPlayer
{
    private static readonly SemaphoreSlim PlaybackGate = new(1, 1);
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
                NumberOfBuffers = 3,
                Volume = Math.Clamp(volume, 0, 100) / 100f
            };

            var stopped = new TaskCompletionSource<Exception?>(TaskCreationOptions.RunContinuationsAsynchronously);
            void PlaybackStopped(object? sender, StoppedEventArgs args) => stopped.TrySetResult(args.Exception);
            output.PlaybackStopped += PlaybackStopped;

            try
            {
                output.Init(reader);
                cancellationToken.ThrowIfCancellationRequested();

                // Start first, then register cancellation. Cancellation that races
                // between these two statements causes Register() to invoke Stop()
                // immediately, while avoiding the old Stop-before-Play race.
                AppLog.Write($"tts: playback start backend=NAudio/MediaFoundation volume={volume}% bytes={mp3Bytes.Length}");
                output.Play();
                using var registration = cancellationToken.Register(() =>
                {
                    try { output.Stop(); } catch { }
                });

                var error = await stopped.Task.ConfigureAwait(false);
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
}
