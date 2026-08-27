namespace BPSR.ReadyAlert;

/// <summary>
/// Keeps a tiny cache of preloaded PCM WAV players for chat notifications.
/// Custom notification sounds are intentionally bounded because AlertAudioPlayer
/// keeps the source WAV plus a volume-adjusted copy in memory.
/// </summary>
internal static class ChatSoundVolumePlayer
{
    internal const long MaxNotificationWaveBytes = 4L * 1024 * 1024;
    internal const double MaxNotificationWaveSeconds = 15.0;
    private const int MaxCachedPlayers = 5;

    private static readonly object Sync = new();
    private static readonly Dictionary<string, CacheEntry> Players = new(StringComparer.OrdinalIgnoreCase);
    private static long _stamp;

    private sealed class CacheEntry(AlertAudioPlayer player, int volume, long stamp)
    {
        internal AlertAudioPlayer Player { get; } = player;
        internal int Volume { get; } = volume;
        internal long Stamp { get; set; } = stamp;
    }

    internal static void Play(string preferredPath, string fallbackPath, int volume, string reason)
    {
        _ = TryPlay(preferredPath, fallbackPath, volume, reason, out _);
    }

    internal static bool TryPlay(string preferredPath, string fallbackPath, int volume, string reason, out string error)
    {
        error = string.Empty;
        volume = Math.Clamp(volume, 0, 100);
        if (volume <= 0)
        {
            AppLog.Write($"chat: sound muted reason={reason}");
            return true;
        }

        lock (Sync)
        {
            if (TryPlayPath(preferredPath, volume, reason, out var preferredError)) return true;
            if (!string.Equals(preferredPath, fallbackPath, StringComparison.OrdinalIgnoreCase) &&
                TryPlayPath(fallbackPath, volume, reason + " fallback", out var fallbackError)) return true;

            error = !string.IsNullOrWhiteSpace(preferredError)
                ? preferredError
                : "No usable chat notification WAV was available.";

            // Do not substitute a Windows SystemSound here. System sounds bypass
            // Chat alert volume, so a broken/missing WAV could otherwise produce a
            // much louder sound than the user selected. Fail softly and report it
            // through diagnostics/logging instead.
            return false;
        }
    }

    internal static bool IsSupportedWave(string path, out string error)
    {
        try
        {
            if (string.IsNullOrWhiteSpace(path) || !File.Exists(path))
            {
                error = "The WAV file does not exist.";
                return false;
            }

            var info = new FileInfo(path);
            if (info.Length > MaxNotificationWaveBytes)
            {
                error = $"Notification WAV is too large ({info.Length / 1024d / 1024d:F1} MiB). Keep it at or below {MaxNotificationWaveBytes / 1024 / 1024} MiB.";
                return false;
            }

            var metadata = AlertAudioPlayer.ProbePcm16Wave(path);
            if (metadata.DurationSeconds > MaxNotificationWaveSeconds)
            {
                error = $"Notification WAV is too long ({metadata.DurationSeconds:F1}s). Keep alert sounds at or below {MaxNotificationWaveSeconds:F0} seconds.";
                return false;
            }

            error = string.Empty;
            return true;
        }
        catch (Exception ex)
        {
            error = ex.Message;
            return false;
        }
    }

    private static bool TryPlayPath(string path, int volume, string reason, out string error)
    {
        error = string.Empty;
        if (string.IsNullOrWhiteSpace(path) || !File.Exists(path)) return false;
        try
        {
            // User-selected chat sounds are bounded before they enter the cache.
            // The bundled fallback remains trusted and is not rejected by this guard.
            if (!Players.ContainsKey(path) && !IsSupportedWave(path, out var validationError))
            {
                error = validationError;
                AppLog.Write($"chat: sound rejected path='{path}' reason={reason}: {validationError}");
                return false;
            }

            if (!Players.TryGetValue(path, out var entry) || entry.Volume != volume)
            {
                if (entry is not null)
                {
                    entry.Player.Dispose();
                    Players.Remove(path);
                }

                TrimCacheForNewPath(path);
                entry = new CacheEntry(new AlertAudioPlayer(path, volume), volume, ++_stamp);
                Players[path] = entry;
            }
            else
            {
                entry.Stamp = ++_stamp;
            }

            entry.Player.Play("chat-" + reason);
            return true;
        }
        catch (Exception ex)
        {
            error = ex.Message;
            AppLog.Write($"chat: volume-controlled sound failed path='{path}' reason={reason}: {ex.Message}");
            if (Players.Remove(path, out var failed)) failed.Player.Dispose();
            return false;
        }
    }

    private static void TrimCacheForNewPath(string incomingPath)
    {
        if (Players.ContainsKey(incomingPath)) return;
        while (Players.Count >= MaxCachedPlayers)
        {
            var oldest = Players.OrderBy(x => x.Value.Stamp).First();
            if (Players.Remove(oldest.Key, out var entry))
                entry.Player.Dispose();
        }
    }

    internal static void ClearCacheForSelfTest()
    {
        lock (Sync)
        {
            foreach (var entry in Players.Values) entry.Player.Dispose();
            Players.Clear();
            _stamp = 0;
        }
    }

    internal static int CachedPlayerCountForSelfTest
    {
        get { lock (Sync) return Players.Count; }
    }
}
