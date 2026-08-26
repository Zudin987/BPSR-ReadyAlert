using System.Collections.Concurrent;

namespace BPSR.ReadyAlert;

internal readonly record struct ChatNotificationStatus(
    bool Enabled,
    int QueueCount,
    long Enqueued,
    long Processed,
    long Matched,
    long Played,
    long Failed,
    long Dropped,
    string LastReason,
    string LastMatchedText,
    DateTime? LastAttemptUtc,
    DateTime? LastSuccessUtc,
    string LastError);

/// <summary>
/// Notification-only chat consumer. It is deliberately independent of the WinForms
/// overlay so keyword/private sounds continue while the overlay is collapsed, hidden,
/// repainting, resizing, or temporarily busy. CaptureEngine only enqueues a tiny value
/// object; matching/audio runs on one ThreadPool worker at a time and never creates a
/// second Npcap handle or TCP parser.
/// </summary>
internal static class ChatNotificationEngine
{
    private const int MaxQueuedNotifications = 256;
    private static readonly ConcurrentQueue<ChatMessageEvent> Queue = new();
    private static readonly object StatusLock = new();
    private static Snapshot _snapshot = Snapshot.Empty;
    private static volatile bool _enabled;
    private static int _workerScheduled;
    private static long _enqueued;
    private static long _processed;
    private static long _matched;
    private static long _played;
    private static long _failed;
    private static long _dropped;
    private static string _lastReason = string.Empty;
    private static string _lastMatchedText = string.Empty;
    private static long _lastAttemptTicks;
    private static long _lastSuccessTicks;
    private static string _lastError = string.Empty;

    private sealed record SoundRuleSnapshot(string Match, string SoundPath);

    private sealed class Snapshot
    {
        internal static readonly Snapshot Empty = new();
        internal HashSet<long> BlockedIds { get; init; } = [];
        internal bool HideStickers { get; init; }
        internal bool PrivateSoundEnabled { get; init; }
        internal string PrivateSoundPath { get; init; } = string.Empty;
        internal int Volume { get; init; } = 100;
        internal string DefaultSoundPath { get; init; } = string.Empty;
        internal SoundRuleSnapshot[] Rules { get; init; } = [];
    }

    internal static bool Enabled
    {
        get => _enabled;
        set
        {
            _enabled = value;
            if (!value)
            {
                while (Queue.TryDequeue(out _)) { }
            }
        }
    }

    internal static void Configure(ChatOverlaySettings settings, string defaultSoundPath)
    {
        ArgumentNullException.ThrowIfNull(settings);
        settings.Normalize();

        var snapshot = new Snapshot
        {
            BlockedIds = settings.BlockedUsers
                .Where(x => x is not null && x.Id != 0)
                .Select(x => x.Id)
                .ToHashSet(),
            HideStickers = settings.HideStickers,
            PrivateSoundEnabled = settings.PrivateSoundEnabled,
            PrivateSoundPath = settings.PrivateSoundPath ?? string.Empty,
            Volume = Math.Clamp(settings.ChatSoundVolume, 0, 100),
            DefaultSoundPath = defaultSoundPath ?? string.Empty,
            Rules = settings.HighlightSoundRules
                .Where(x => x is not null && x.Enabled && !string.IsNullOrWhiteSpace(x.Match))
                .Take(3)
                .Select(x => new SoundRuleSnapshot(x.Match.Trim(), x.SoundPath ?? string.Empty))
                .ToArray()
        };

        Volatile.Write(ref _snapshot, snapshot);
    }

    internal static void Enqueue(ChatMessageEvent message)
    {
        if (!Enabled) return;

        while (Queue.Count >= MaxQueuedNotifications && Queue.TryDequeue(out _))
            Interlocked.Increment(ref _dropped);

        Queue.Enqueue(message);
        Interlocked.Increment(ref _enqueued);
        ScheduleWorker();
    }

    private static void ScheduleWorker()
    {
        if (Interlocked.CompareExchange(ref _workerScheduled, 1, 0) != 0) return;
        ThreadPool.UnsafeQueueUserWorkItem(static _ => DrainWorker(), null);
    }

    private static void DrainWorker()
    {
        try
        {
            while (Enabled && Queue.TryDequeue(out var message))
            {
                Interlocked.Increment(ref _processed);
                ProcessMessage(message, Volatile.Read(ref _snapshot), playAudio: true);
            }
        }
        catch (Exception ex)
        {
            RecordFailure("worker", string.Empty, ex.Message);
            AppLog.Write("chat-notify: worker failed " + ex);
        }
        finally
        {
            Interlocked.Exchange(ref _workerScheduled, 0);
            if (Enabled && !Queue.IsEmpty) ScheduleWorker();
        }
    }

    private static bool ProcessMessage(ChatMessageEvent message, Snapshot snapshot, bool playAudio)
    {
        if (snapshot.BlockedIds.Contains(message.SenderId) && message.SenderId != 0) return false;
        if (snapshot.HideStickers && message.Kind == ChatMessageKind.Sticker) return false;

        if (message.Channel == ChatChannel.Private && snapshot.PrivateSoundEnabled)
        {
            MatchAndPlay(message, snapshot, snapshot.PrivateSoundPath, "private", playAudio);
            return true;
        }

        var searchable = message.Text ?? string.Empty;
        for (var i = 0; i < snapshot.Rules.Length; i++)
        {
            var rule = snapshot.Rules[i];
            if (!ChatFilterExpression.IsMatch(searchable, rule.Match)) continue;
            MatchAndPlay(message, snapshot, rule.SoundPath, $"rule-{i + 1}", playAudio);
            return true;
        }

        return false;
    }

    private static void MatchAndPlay(
        ChatMessageEvent message,
        Snapshot snapshot,
        string configuredPath,
        string reason,
        bool playAudio)
    {
        Interlocked.Increment(ref _matched);
        var text = message.Text ?? string.Empty;
        lock (StatusLock)
        {
            _lastReason = reason;
            _lastMatchedText = text.Length <= 160 ? text : text[..160];
            _lastAttemptTicks = DateTime.UtcNow.Ticks;
            _lastError = string.Empty;
        }

        if (!playAudio) return;

        var preferred = !string.IsNullOrWhiteSpace(configuredPath) && File.Exists(configuredPath)
            ? configuredPath
            : snapshot.DefaultSoundPath;

        if (ChatSoundVolumePlayer.TryPlay(preferred, snapshot.DefaultSoundPath, snapshot.Volume, reason, out var error))
        {
            Interlocked.Increment(ref _played);
            lock (StatusLock)
            {
                _lastSuccessTicks = DateTime.UtcNow.Ticks;
                _lastError = string.Empty;
            }
            AppLog.Write($"chat-notify: played reason={reason} seq={message.SequenceId}");
        }
        else
        {
            RecordFailure(reason, text, error);
        }
    }

    private static void RecordFailure(string reason, string text, string error)
    {
        Interlocked.Increment(ref _failed);
        lock (StatusLock)
        {
            _lastReason = reason;
            if (!string.IsNullOrWhiteSpace(text))
                _lastMatchedText = text.Length <= 160 ? text : text[..160];
            _lastAttemptTicks = DateTime.UtcNow.Ticks;
            _lastError = error ?? string.Empty;
        }
        AppLog.Write($"chat-notify: failed reason={reason}: {error}");
    }

    internal static ChatNotificationStatus GetStatus()
    {
        lock (StatusLock)
        {
            var attempt = Interlocked.Read(ref _lastAttemptTicks);
            var success = Interlocked.Read(ref _lastSuccessTicks);
            return new ChatNotificationStatus(
                Enabled,
                Queue.Count,
                Interlocked.Read(ref _enqueued),
                Interlocked.Read(ref _processed),
                Interlocked.Read(ref _matched),
                Interlocked.Read(ref _played),
                Interlocked.Read(ref _failed),
                Interlocked.Read(ref _dropped),
                _lastReason,
                _lastMatchedText,
                attempt > 0 ? new DateTime(attempt, DateTimeKind.Utc) : null,
                success > 0 ? new DateTime(success, DateTimeKind.Utc) : null,
                _lastError);
        }
    }

    internal static bool EvaluateForSelfTest(ChatOverlaySettings settings, ChatMessageEvent message, string fallbackPath = "")
    {
        var old = Volatile.Read(ref _snapshot);
        try
        {
            Configure(settings, fallbackPath);
            return ProcessMessage(message, Volatile.Read(ref _snapshot), playAudio: false);
        }
        finally
        {
            Volatile.Write(ref _snapshot, old);
        }
    }
}
