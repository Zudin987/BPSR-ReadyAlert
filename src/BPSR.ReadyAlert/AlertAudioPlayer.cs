using NAudio.Wave;

namespace BPSR.ReadyAlert;

internal sealed class AlertAudioPlayer : IDisposable
{
    private readonly object _sync = new();
    private readonly AudioFileReader _reader;
    private readonly WaveOutEvent _output;
    private int _volume;

    internal AlertAudioPlayer(string path, int volume)
    {
        _reader = new AudioFileReader(path);
        _output = new WaveOutEvent
        {
            DesiredLatency = 50,
            NumberOfBuffers = 2
        };
        _output.Init(_reader);
        Volume = volume;
    }

    internal int Volume
    {
        get => _volume;
        set
        {
            lock (_sync)
            {
                _volume = Math.Clamp(value, 0, 100);
                _output.Volume = _volume / 100f;
            }
        }
    }

    internal void Play(string reason)
    {
        lock (_sync)
        {
            AppLog.Write($"audio: play requested reason={reason} volume={_volume}%");
            _output.Stop();
            _reader.Position = 0;
            _output.Volume = _volume / 100f;
            _output.Play();
            AppLog.Write($"audio: play submitted reason={reason}");
        }
    }

    public void Dispose()
    {
        lock (_sync)
        {
            try { _output.Stop(); } catch { }
            _output.Dispose();
            _reader.Dispose();
        }
    }
}
