using System.Media;

namespace BPSR.ReadyAlert;

/// <summary>
/// Keeps a tiny cache of preloaded PCM WAV players for chat notifications. RC9
/// supports three keyword sounds plus Private/Talk and the built-in fallback, so
/// switching between rules does not reopen/rescale a WAV on every chat message.
/// </summary>
internal static class ChatSoundVolumePlayer
{
    private const int MaxCachedPlayers = 5;
    private static readonly object Sync = new();
    private static readonly Dictionary<string, CacheEntry> Players = new(StringComparer.OrdinalIgnoreCase);
    private static readonly Queue<string> LoadOrder = new();

    private sealed class CacheEntry(AlertAudioPlayer player, int volume)
    {
        internal AlertAudioPlayer Player { get; } = player;
        internal int Volume { get; } = volume;
    }

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
            if (!Players.TryGetValue(path, out var entry) || entry.Volume != volume)
            {
                if (entry is not null)
                {
                    entry.Player.Dispose();
                    Players.Remove(path);
                }

                TrimCacheForNewPath(path);
                entry = new CacheEntry(new AlertAudioPlayer(path, volume), volume);
                Players[path] = entry;
                LoadOrder.Enqueue(path);
            }

            entry.Player.Play("chat-" + reason);
            return true;
        }
        catch (Exception ex)
        {
            AppLog.Write($"chat: volume-controlled sound failed path='{path}' reason={reason}: {ex.Message}");
            if (Players.Remove(path, out var failed)) failed.Player.Dispose();
            return false;
        }
    }

    private static void TrimCacheForNewPath(string incomingPath)
    {
        if (Players.ContainsKey(incomingPath)) return;
        while (Players.Count >= MaxCachedPlayers && LoadOrder.Count > 0)
        {
            var oldest = LoadOrder.Dequeue();
            if (!Players.Remove(oldest, out var entry)) continue;
            entry.Player.Dispose();
        }
    }
}
