using System.Collections.Concurrent;

namespace BPSR.ReadyAlert;

/// <summary>
/// Cheap opt-in chat consumer for CaptureEngine's existing decoded Notify stream.
/// It deliberately owns no Npcap handle, TCP flow state, decompressor, or thread.
/// </summary>
internal static class ChatCaptureBridge
{
    private const int MaxQueuedMessages = 1_000;
    private static ConcurrentQueue<ChatMessageEvent>? _events;
    private static volatile bool _enabled;
    private static int _loggedParseFailure;

    internal static bool Enabled
    {
        get => _enabled;
        set => _enabled = value;
    }

    internal static void Configure(ConcurrentQueue<ChatMessageEvent> events)
    {
        ArgumentNullException.ThrowIfNull(events);
        Volatile.Write(ref _events, events);
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

        var events = Volatile.Read(ref _events);
        if (events is null) return true;

        if (!ChatProtocol.TryParseNotify(payload, out var message))
        {
            if (Interlocked.Exchange(ref _loggedParseFailure, 1) == 0)
                AppLog.Write($"chat: first ChitChatNtf parse failure protoLen={payload.Length}");
            return true;
        }

        // The UI normally drains every 25 ms. Keep a hard emergency ceiling so a
        // blocked UI thread or malformed packet flood cannot grow memory forever.
        while (events.Count >= MaxQueuedMessages && events.TryDequeue(out _)) { }
        events.Enqueue(message);
        return true;
    }
}
