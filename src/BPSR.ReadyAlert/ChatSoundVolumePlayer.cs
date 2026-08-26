using System.Media;

namespace BPSR.ReadyAlert;

/// <summary>
/// Keeps one preloaded PCM WAV player for chat notifications. It reuses
/// ReadyAlert's existing sample-scaling audio path, so chat volume needs no new
/// media framework or long-lived background audio engine.
/// </summary>
internal static class ChatSoundVolumePlayer
{
    private static readonly object Sync = new();
    private static AlertAudioPlayer? _player;
    private static string _loadedPath = string.Empty;
    private static int _loadedVolume = -1;

    internal static void Play(string preferredPath, string fallbackPath, int volume, string reason)
    {
        volume = Math.Clamp(volume, 0, 100);
        if (volume <= 0)
        {
            AppLog.Write($"chat: sound muted reason={reason}");
            return;
        }

        lock (Sync)
        {
            if (TryPlay(preferredPath, volume, reason)) return;
            if (!string.Equals(preferredPath, fallbackPath, StringComparison.OrdinalIgnoreCase) &&
                TryPlay(fallbackPath, volume, reason + " fallback")) return;

            try { SystemSounds.Asterisk.Play(); } catch { }
        }
    }

    internal static bool IsSupportedWave(string path, out string error)
    {
        try
        {
            using var probe = new AlertAudioPlayer(path, 100);
            error = string.Empty;
            return true;
        }
        catch (Exception ex)
        {
            error = ex.Message;
            return false;
        }
    }

    private static bool TryPlay(string path, int volume, string reason)
    {
        if (string.IsNullOrWhiteSpace(path) || !File.Exists(path)) return false;
        try
        {
            if (_player is null || _loadedVolume != volume || !string.Equals(_loadedPath, path, StringComparison.OrdinalIgnoreCase))
            {
                _player?.Dispose();
                _player = new AlertAudioPlayer(path, volume);
                _loadedPath = path;
                _loadedVolume = volume;
            }
            _player.Play("chat-" + reason);
            return true;
        }
        catch (Exception ex)
        {
            AppLog.Write($"chat: volume-controlled sound failed path='{path}' reason={reason}: {ex.Message}");
            _player?.Dispose();
            _player = null;
            _loadedPath = string.Empty;
            _loadedVolume = -1;
            return false;
        }
    }
}
