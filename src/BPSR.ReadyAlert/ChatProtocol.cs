using System.Text;

namespace BPSR.ReadyAlert;

internal static class ChatProtocol
{
    internal const ulong ServiceId = 164_931_432UL;
    internal const uint NotifyNewestChitChatMsgs = 0x01;
    private const int MaxDecodedStringBytes = 64 * 1024;

    internal static bool TryParseNotify(byte[] payload, out ChatMessageEvent message)
    {
        message = default;

        // ChitChatNtf.NotifyNewestChitChatMsgs.vRequest = field 1
        if (!TryGetLengthField(payload, 0, payload.Length, 1, out var requestOffset, out var requestLength))
            return false;

        if (!TryGetVarintField(payload, requestOffset, requestLength, 1, out var channelRaw))
            channelRaw = 0;

        // NotifyNewestChitChatMsgsRequest.chatMsg = field 2
        if (!TryGetLengthField(payload, requestOffset, requestLength, 2, out var chatOffset, out var chatLength))
            return false;

        long senderId = 0;
        string senderName = string.Empty;
        int senderLevel = 0;
        long timestamp = 0;
        var kind = ChatMessageKind.Text;
        string text = string.Empty;

        // ChitChatMsg.sendCharInfo = field 2 (BasicShowInfo)
        if (TryGetLengthField(payload, chatOffset, chatLength, 2, out var senderOffset, out var senderLength))
        {
            if (TryGetVarintField(payload, senderOffset, senderLength, 1, out var idRaw))
                senderId = unchecked((long)idRaw);
            _ = TryGetStringField(payload, senderOffset, senderLength, 2, out senderName);
            if (TryGetVarintField(payload, senderOffset, senderLength, 5, out var levelRaw))
                senderLevel = checked((int)Math.Min(levelRaw, int.MaxValue));
        }

        // ChitChatMsg.timestamp = field 3 (Unix seconds), matching ZDPS.
        if (TryGetVarintField(payload, chatOffset, chatLength, 3, out var timestampRaw))
            timestamp = unchecked((long)timestampRaw);

        // ChitChatMsg.msgInfo = field 4
        if (!TryGetLengthField(payload, chatOffset, chatLength, 4, out var infoOffset, out var infoLength))
            return false;

        if (TryGetVarintField(payload, infoOffset, infoLength, 1, out var kindRaw) && kindRaw <= 6)
            kind = (ChatMessageKind)(int)kindRaw;

        _ = TryGetStringField(payload, infoOffset, infoLength, 3, out text);

        if (kind == ChatMessageKind.Sticker)
        {
            if (TryGetLengthField(payload, infoOffset, infoLength, 5, out var stickerOffset, out var stickerLength) &&
                TryGetVarintField(payload, stickerOffset, stickerLength, 1, out var configId))
                text = $"[Image({configId})]";
            else
                text = "[Sticker]";
        }
        else if (string.IsNullOrWhiteSpace(text))
        {
            text = kind switch
            {
                ChatMessageKind.TextNotice => "[Notice]",
                ChatMessageKind.MultiLanguageNotice => "[Multi-language notice]",
                ChatMessageKind.Picture => "[Picture]",
                ChatMessageKind.Voice => "[Voice]",
                ChatMessageKind.Hypertext => "[Hypertext]",
                _ => string.Empty
            };
        }

        DateTime localTime;
        try
        {
            localTime = timestamp > 0
                ? DateTimeOffset.FromUnixTimeSeconds(timestamp).LocalDateTime
                : DateTime.Now;
        }
        catch (ArgumentOutOfRangeException)
        {
            localTime = DateTime.Now;
        }

        var channel = channelRaw <= int.MaxValue && Enum.IsDefined(typeof(ChatChannel), (int)channelRaw)
            ? (ChatChannel)(int)channelRaw
            : ChatChannel.Null;

        message = new ChatMessageEvent(
            senderId,
            senderName,
            senderLevel,
            channel,
            localTime,
            kind,
            text ?? string.Empty);
        return true;
    }

    private static bool TryGetStringField(
        byte[] data,
        int offset,
        int length,
        int wantedField,
        out string value)
    {
        value = string.Empty;
        if (!TryGetLengthField(data, offset, length, wantedField, out var valueOffset, out var valueLength))
            return false;
        if (valueLength > MaxDecodedStringBytes)
            return false;

        try
        {
            // UTF-8 is the protobuf string encoding and correctly preserves Malay,
            // Chinese, emoji, and mixed-language BPSR chat. The framework decoder is
            // intentionally tolerant of a malformed byte sequence instead of crashing.
            value = Encoding.UTF8.GetString(data, valueOffset, valueLength);
            return true;
        }
        catch
        {
            return false;
        }
    }

    private static bool TryGetLengthField(
        byte[] data,
        int offset,
        int length,
        int wantedField,
        out int valueOffset,
        out int valueLength)
    {
        valueOffset = 0;
        valueLength = 0;
        var p = offset;
        var limit = offset + length;
        if (offset < 0 || length < 0 || limit < offset || limit > data.Length) return false;

        while (p < limit)
        {
            if (!ReadVarint(data, ref p, limit, out var key)) return false;
            var field = (int)(key >> 3);
            var wire = (int)(key & 7);

            if (wire == 2)
            {
                if (!ReadVarint(data, ref p, limit, out var len) || len > (ulong)(limit - p)) return false;
                if (field == wantedField)
                {
                    valueOffset = p;
                    valueLength = checked((int)len);
                    return true;
                }
                p += checked((int)len);
            }
            else if (!SkipField(data, ref p, limit, wire))
            {
                return false;
            }
        }

        return false;
    }

    private static bool TryGetVarintField(byte[] data, int offset, int length, int wantedField, out ulong value)
    {
        value = 0;
        var p = offset;
        var limit = offset + length;
        if (offset < 0 || length < 0 || limit < offset || limit > data.Length) return false;

        while (p < limit)
        {
            if (!ReadVarint(data, ref p, limit, out var key)) return false;
            var field = (int)(key >> 3);
            var wire = (int)(key & 7);

            if (wire == 0)
            {
                if (!ReadVarint(data, ref p, limit, out var v)) return false;
                if (field == wantedField)
                {
                    value = v;
                    return true;
                }
            }
            else if (!SkipField(data, ref p, limit, wire))
            {
                return false;
            }
        }

        return false;
    }

    private static bool ReadVarint(byte[] data, ref int p, int limit, out ulong value)
    {
        value = 0;
        var shift = 0;
        while (p < limit && shift < 64)
        {
            var b = data[p++];
            value |= ((ulong)(b & 0x7F)) << shift;
            if ((b & 0x80) == 0) return true;
            shift += 7;
        }
        return false;
    }

    private static bool SkipField(byte[] data, ref int p, int limit, int wire)
    {
        switch (wire)
        {
            case 0:
                return ReadVarint(data, ref p, limit, out _);
            case 1:
                if (p + 8 > limit) return false;
                p += 8;
                return true;
            case 2:
                if (!ReadVarint(data, ref p, limit, out var len) || len > (ulong)(limit - p)) return false;
                p += checked((int)len);
                return true;
            case 5:
                if (p + 4 > limit) return false;
                p += 4;
                return true;
            default:
                return false;
        }
    }
}
