using System.Media;

namespace BPSR.ReadyAlert;

internal readonly record struct WaveMetadata(long Bytes, double DurationSeconds, int SampleRate, int Channels);

internal sealed class AlertAudioPlayer : IDisposable
{
    private readonly object _sync = new();
    private readonly byte[] _sourceWav;
    private SoundPlayer? _player;
    private MemoryStream? _stream;
    private int _volume;

    internal AlertAudioPlayer(string path, int volume)
    {
        _sourceWav = File.ReadAllBytes(path);
        _ = ValidatePcm16Wave(_sourceWav);
        Volume = volume;
    }

    internal int Volume
    {
        get { lock (_sync) return _volume; }
        set
        {
            lock (_sync)
            {
                var clamped = Math.Clamp(value, 0, 100);
                if (_player is not null && _volume == clamped) return;
                _volume = clamped;
                RebuildPlayer();
                AppLog.Write($"audio: prepared SoundPlayer volume={_volume}%");
            }
        }
    }

    internal void Play(string reason)
    {
        lock (_sync)
        {
            try
            {
                AppLog.Write($"audio: play requested reason={reason} volume={_volume}% backend=SoundPlayer");
                if (_volume <= 0)
                {
                    AppLog.Write($"audio: muted reason={reason}");
                    return;
                }
                _player ??= CreatePlayer(_sourceWav);
                _player.Stop();
                _player.Play();
                AppLog.Write($"audio: play submitted reason={reason} backend=SoundPlayer");
            }
            catch (Exception ex)
            {
                AppLog.Write("audio: SoundPlayer failed " + ex.Message);
                SystemSounds.Exclamation.Play();
            }
        }
    }

    internal static WaveMetadata ProbePcm16Wave(string path)
    {
        var wav = File.ReadAllBytes(path);
        return ValidatePcm16Wave(wav);
    }

    private void RebuildPlayer()
    {
        try { _player?.Stop(); } catch { }
        _player?.Dispose();
        _stream?.Dispose();
        _player = null;
        _stream = null;
        if (_volume <= 0) return;

        var wav = _volume >= 100 ? _sourceWav : ScalePcm16Wave(_sourceWav, _volume / 100.0);
        _stream = new MemoryStream(wav, writable: false);
        _player = new SoundPlayer(_stream);
        _player.Load();
    }

    private SoundPlayer CreatePlayer(byte[] wav)
    {
        _stream?.Dispose();
        _stream = new MemoryStream(wav, writable: false);
        var player = new SoundPlayer(_stream);
        player.Load();
        return player;
    }

    private static byte[] ScalePcm16Wave(byte[] source, double gain)
    {
        var output = (byte[])source.Clone();
        var data = FindChunk(output, "data");
        for (var i = data.Offset; i + 1 < data.Offset + data.Length; i += 2)
        {
            var sample = (short)(output[i] | (output[i + 1] << 8));
            var scaled = Math.Clamp((int)Math.Round(sample * gain), short.MinValue, short.MaxValue);
            output[i] = (byte)(scaled & 0xFF);
            output[i + 1] = (byte)((scaled >> 8) & 0xFF);
        }
        return output;
    }

    private static WaveMetadata ValidatePcm16Wave(byte[] wav)
    {
        if (wav.Length < 44 || ReadAscii(wav, 0, 4) != "RIFF" || ReadAscii(wav, 8, 4) != "WAVE")
            throw new InvalidDataException("Alert sound is not a valid RIFF/WAVE file.");

        var fmt = FindChunk(wav, "fmt ");
        if (fmt.Length < 16) throw new InvalidDataException("Alert sound has an invalid fmt chunk.");
        var format = ReadUInt16(wav, fmt.Offset);
        var channels = ReadUInt16(wav, fmt.Offset + 2);
        var sampleRate = ReadUInt32(wav, fmt.Offset + 4);
        var byteRate = ReadUInt32(wav, fmt.Offset + 8);
        var bits = ReadUInt16(wav, fmt.Offset + 14);
        if (format != 1 || bits != 16)
            throw new InvalidDataException($"Alert sound must be 16-bit PCM WAV (format={format}, bits={bits}).");
        if (channels == 0 || sampleRate == 0 || byteRate == 0)
            throw new InvalidDataException("Alert sound has invalid PCM rate/channel metadata.");

        var data = FindChunk(wav, "data");
        var duration = data.Length / (double)byteRate;
        return new WaveMetadata(wav.LongLength, duration, checked((int)sampleRate), channels);
    }

    private static (int Offset, int Length) FindChunk(byte[] wav, string id)
    {
        var cursor = 12;
        while (cursor + 8 <= wav.Length)
        {
            var chunkId = ReadAscii(wav, cursor, 4);
            var length = checked((int)ReadUInt32(wav, cursor + 4));
            var dataOffset = cursor + 8;
            if (dataOffset + length > wav.Length)
                throw new InvalidDataException($"WAV chunk '{chunkId}' is truncated.");
            if (chunkId == id) return (dataOffset, length);
            cursor = dataOffset + length + (length & 1);
        }
        throw new InvalidDataException($"WAV chunk '{id}' was not found.");
    }

    private static ushort ReadUInt16(byte[] data, int offset) => (ushort)(data[offset] | (data[offset + 1] << 8));
    private static uint ReadUInt32(byte[] data, int offset) => (uint)(data[offset] | (data[offset + 1] << 8) | (data[offset + 2] << 16) | (data[offset + 3] << 24));
    private static string ReadAscii(byte[] data, int offset, int count) => System.Text.Encoding.ASCII.GetString(data, offset, count);

    public void Dispose()
    {
        lock (_sync)
        {
            try { _player?.Stop(); } catch { }
            _player?.Dispose();
            _stream?.Dispose();
            _player = null;
            _stream = null;
        }
    }
}
