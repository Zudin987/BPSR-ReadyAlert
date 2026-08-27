using System.Collections.Concurrent;

namespace BPSR.ReadyAlert;

internal readonly record struct ChatCaptureStatus(
    bool Enabled,
    long MatchingNotifies,
    long ParsedMessages,
    long ParseFailures,
    long DroppedQueuedMessages,
    int QueueCount,
    int LastPayloadLength,
    DateTime? LastMessageUtc);

/// <summary>
/// Cheap opt-in chat consumer for CaptureEngine's existing decoded Notify stream.
/// It deliberately owns no Npcap handle, TCP flow state, decompressor, or capture thread.
/// </summary>
internal static class ChatCaptureBridge
{
    private const int MaxQueuedMessages = 1_000;
    private static ConcurrentQueue<ChatMessageEvent>? _events;
    private static volatile bool _enabled;
    private static int _loggedParseFailure;
    private static long _matchingNotifies;
    private static long _parsedMessages;
    private static long _parseFailures;
    private static long _droppedQueuedMessages;
    private static long _lastMessageUtcTicks;
    private static int _lastPayloadLength;
    private static long _sequenceId;

    internal static bool Enabled
    {
        get => _enabled;
        set
        {
            _enabled = value;
            ChatNotificationEngine.Enabled = value;
            ChatSpeechTranslationEngine.Enabled = value;
        }
    }

    internal static void Configure(ConcurrentQueue<ChatMessageEvent> events)
    {
        ArgumentNullException.ThrowIfNull(events);
        Volatile.Write(ref _events, events);
    }

    internal static ChatCaptureStatus GetStatus()
    {
        var events = Volatile.Read(ref _events);
        var ticks = Interlocked.Read(ref _lastMessageUtcTicks);
        return new ChatCaptureStatus(
            Enabled,
            Interlocked.Read(ref _matchingNotifies),
            Interlocked.Read(ref _parsedMessages),
            Interlocked.Read(ref _parseFailures),
            Interlocked.Read(ref _droppedQueuedMessages),
            events?.Count ?? 0,
            Volatile.Read(ref _lastPayloadLength),
            ticks > 0 ? new DateTime(ticks, DateTimeKind.Utc) : null);
    }

    /// <summary>
    /// Returns true when this Notify belongs to the chat service/method, even when
    /// chat is disabled. That lets CaptureEngine stop dispatching this known packet
    /// without doing any protobuf work while the feature is off.
    /// </summary>
    internal static bool TryHandle(ulong service, uint method, byte[] payload)
    {
        if (service != ChatProtocol.ServiceId || method != ChatProtocol.NotifyNewestChitChatMsgs)
            return false;

        if (!_enabled) return true;

        Interlocked.Increment(ref _matchingNotifies);
        Volatile.Write(ref _lastPayloadLength, payload.Length);

        var events = Volatile.Read(ref _events);
        if (events is null) return true;

        if (!ChatProtocol.TryParseNotify(payload, out var message))
        {
            Interlocked.Increment(ref _parseFailures);
            if (Interlocked.Exchange(ref _loggedParseFailure, 1) == 0)
                AppLog.Write($"chat: first ChitChatNtf parse failure protoLen={payload.Length}");
            return true;
        }

        message = message with { SequenceId = Interlocked.Increment(ref _sequenceId) };
        Interlocked.Increment(ref _parsedMessages);
        Interlocked.Exchange(ref _lastMessageUtcTicks, DateTime.UtcNow.Ticks);

        var blocked = ChatNotificationEngine.IsSenderBlocked(message.SenderId);
        if (!blocked)
        {
            ChatNotificationEngine.Enqueue(message);
            if (ShouldRouteToSpeech(message))
                ChatSpeechTranslationEngine.Enqueue(message);
        }

        // Keep bounded history routing unchanged so unblocking can reveal any still-
        // retained rows. While the block is active the overlay hides them, and work
        // skipped here (keyword/private sound, translation and TTS) is never replayed.
        while (events.Count >= MaxQueuedMessages && events.TryDequeue(out _))
            Interlocked.Increment(ref _droppedQueuedMessages);

        events.Enqueue(message);
        return true;
    }

    private static bool ShouldRouteToSpeech(ChatMessageEvent message) =>
        !ChatNotificationEngine.IsSenderBlocked(message.SenderId) &&
        !ChatContentVisibility.ShouldSkipSpeech(message);

    internal static bool ShouldRouteToSpeechForSelfTest(ChatMessageEvent message) =>
        ShouldRouteToSpeech(message);
}
