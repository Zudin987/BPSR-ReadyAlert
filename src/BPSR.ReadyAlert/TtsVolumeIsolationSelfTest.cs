using NAudio.Wave;

namespace BPSR.ReadyAlert;

internal static class TtsVolumeIsolationSelfTest
{
    internal static void Run()
    {
        var source = new FixedSampleProvider([1.0f, -0.5f, 0.25f]);
        var provider = ChatTtsAudioPlayer.CreateVolumeProviderForSelfTest(source, 5);
        var buffer = new float[3];

        var read = provider.Read(buffer, 0, buffer.Length);
        Assert(read == 3, "TTS stream gain preserves sample count");
        Assert(Near(buffer[0], 0.05f) && Near(buffer[1], -0.025f) && Near(buffer[2], 0.0125f),
            "5% TTS volume is applied to TTS samples only");

        var full = ChatTtsAudioPlayer.CreateVolumeProviderForSelfTest(
            new FixedSampleProvider([0.4f]), 100);
        var fullBuffer = new float[1];
        _ = full.Read(fullBuffer, 0, 1);
        Assert(Near(fullBuffer[0], 0.4f), "100% TTS volume keeps samples at unity gain");
    }

    private sealed class FixedSampleProvider(float[] samples) : ISampleProvider
    {
        private int _position;

        public WaveFormat WaveFormat { get; } = WaveFormat.CreateIeeeFloatWaveFormat(48_000, 1);

        public int Read(float[] buffer, int offset, int count)
        {
            var available = Math.Min(count, samples.Length - _position);
            if (available <= 0) return 0;
            Array.Copy(samples, _position, buffer, offset, available);
            _position += available;
            return available;
        }
    }

    private static bool Near(float actual, float expected) => Math.Abs(actual - expected) < 0.00001f;

    private static void Assert(bool condition, string name)
    {
        if (!condition) throw new InvalidOperationException("TTS volume isolation self-test failed: " + name);
    }
}
