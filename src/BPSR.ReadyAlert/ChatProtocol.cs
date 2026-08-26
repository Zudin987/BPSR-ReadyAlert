using System.Text;

namespace BPSR.ReadyAlert;

internal static class ChatProtocol
{
    internal const ulong ServiceId = 164_931_432UL;
    internal const uint NotifyNewestChitChatMsgs = 0x01;
    private const int MaxDecodedStringBytes = 64 * 1024;
    private static readonly UTF8Encoding StrictUtf8 = new(false, true);

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

        switch (kind)
        {
            case ChatMessageKind.Sticker:
                if (TryGetLengthField(payload, infoOffset, infoLength, 5, out var stickerOffset, out var stickerLength) &&
                    TryGetVarintField(payload, stickerOffset, stickerLength, 1, out var configId))
                    text = $"[Image({configId})]";
                else if (string.IsNullOrWhiteSpace(text))
                    text = "[Sticker]";
                break;

            case ChatMessageKind.MultiLanguageNotice:
                if (TryGetLengthField(payload, infoOffset, infoLength, 4, out var noticeOffset, out var noticeLength))
                    text = CombineText(text, DecodeMultiLanguageNotice(payload, noticeOffset, noticeLength));
                if (string.IsNullOrWhiteSpace(text)) text = "[Multi-language notice]";
                break;

            case ChatMessageKind.Voice:
                if (TryGetLengthField(payload, infoOffset, infoLength, 6, out var voiceOffset, out var voiceLength) &&
                    TryGetStringField(payload, voiceOffset, voiceLength, 2, out var voiceText) &&
                    !string.IsNullOrWhiteSpace(voiceText))
                    text = CombineText(text, voiceText);
                if (string.IsNullOrWhiteSpace(text)) text = "[Voice]";
                break;

            case ChatMessageKind.Hypertext:
                if (TryGetLengthField(payload, infoOffset, infoLength, 7, out var hyperOffset, out var hyperLength))
                    text = CombineText(text, DecodeHypertext(payload, hyperOffset, hyperLength));
                if (string.IsNullOrWhiteSpace(text)) text = "[Hypertext]";
                break;

            case ChatMessageKind.TextNotice:
                if (string.IsNullOrWhiteSpace(text)) text = "[Notice]";
                break;

            case ChatMessageKind.Picture:
                if (string.IsNullOrWhiteSpace(text)) text = "[Picture]";
                break;
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

    private static string DecodeMultiLanguageNotice(byte[] data, int offset, int length)
    {
        var parts = new List<string>();
        if (TryGetVarintField(data, offset, length, 1, out var configId) && configId != 0)
            parts.Add($"[Notice {configId}]");

        foreach (var field in GetLengthFields(data, offset, length, 2))
        {
            if (TryDecodePrintableUtf8(data, field.Offset, field.Length, out var arg))
                AddUnique(parts, arg);
        }
        return string.Join(" ", parts);
    }

    private static string DecodeHypertext(byte[] data, int offset, int length)
    {
        var parts = new List<string>();
        if (TryGetVarintField(data, offset, length, 1, out var configId) && configId != 0)
            parts.Add($"[Hypertext {configId}]");

        foreach (var holder in GetLengthFields(data, offset, length, 2))
        {
            _ = TryGetVarintField(data, holder.Offset, holder.Length, 1, out var type);
            if (!TryGetLengthField(data, holder.Offset, holder.Length, 2, out var bytesOffset, out var bytesLength))
                continue;

            // PlaceHolderTypeString = 7 and stores its value directly in bytes_content.
            if (type == 7 && TryDecodePrintableUtf8(data, bytesOffset, bytesLength, out var direct))
            {
                AddUnique(parts, direct);
                continue;
            }

            // Player/item/union/etc. placeholders often contain a tiny protobuf with
            // one or more human-readable strings. Extract only strict printable UTF-8
            // leaves; binary IDs and coordinates are ignored. This makes filters useful
            // for richer BPSR cards without embedding version-specific game tables.
            CollectPrintableProtoStrings(data, bytesOffset, bytesLength, 0, parts);
        }

        return string.Join(" ", parts);
    }

    private static void CollectPrintableProtoStrings(
        byte[] data,
        int offset,
        int length,
        int depth,
        List<string> output)
    {
        if (depth > 2 || length <= 0 || length > MaxDecodedStringBytes) return;
        var p = offset;
        var limit = offset + length;
        if (offset < 0 || limit < offset || limit > data.Length) return;

        while (p < limit)
        {
            if (!ReadVarint(data, ref p, limit, out var key)) return;
            var wire = (int)(key & 7);
            if (wire == 2)
            {
                if (!ReadVarint(data, ref p, limit, out var len) || len > (ulong)(limit - p)) return;
                var childLength = checked((int)len);
                if (TryDecodePrintableUtf8(data, p, childLength, out var printable))
                    AddUnique(output, printable);
                else
                    CollectPrintableProtoStrings(data, p, childLength, depth + 1, output);
                p += childLength;
            }
            else if (!SkipField(data, ref p, limit, wire))
            {
                return;
            }
        }
    }

    private static bool TryDecodePrintableUtf8(byte[] data, int offset, int length, out string value)
    {
        value = string.Empty;
        if (length <= 0 || length > 2048 || offset < 0 || offset + length > data.Length) return false;
        try
        {
            var decoded = StrictUtf8.GetString(data, offset, length).Trim();
            if (decoded.Length == 0 || decoded.Length > 512) return false;

            var useful = false;
            foreach (var c in decoded)
            {
                if (char.IsControl(c) && !char.IsWhiteSpace(c)) return false;
                if (char.IsLetterOrDigit(c)) useful = true;
            }
            if (!useful) return false;
            value = decoded;
            return true;
        }
        catch (DecoderFallbackException)
        {
            return false;
        }
    }

    private static string CombineText(string first, string second)
    {
        first = first?.Trim() ?? string.Empty;
        second = second?.Trim() ?? string.Empty;
        if (first.Length == 0) return second;
        if (second.Length == 0 || first.Contains(second, StringComparison.OrdinalIgnoreCase)) return first;
        return first + " " + second;
    }

    private static void AddUnique(List<string> values, string value)
    {
        value = value.Trim();
        if (value.Length == 0) return;
        if (!values.Any(x => string.Equals(x, value, StringComparison.OrdinalIgnoreCase)))
            values.Add(value);
    }

    private static List<(int Offset, int Length)> GetLengthFields(
        byte[] data,
        int offset,
        int length,
        int wantedField)
    {
        var result = new List<(int, int)>();
        var p = offset;
        var limit = offset + length;
        if (offset < 0 || length < 0 || limit < offset || limit > data.Length) return result;

        while (p < limit)
        {
            if (!ReadVarint(data, ref p, limit, out var key)) break;
            var field = (int)(key >> 3);
            var wire = (int)(key & 7);
            if (wire == 2)
            {
                if (!ReadVarint(data, ref p, limit, out var len) || len > (ulong)(limit - p)) break;
                var size = checked((int)len);
                if (field == wantedField) result.Add((p, size));
                p += size;
            }
            else if (!SkipField(data, ref p, limit, wire))
            {
                break;
            }
        }
        return result;
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
            // Chinese, emoji, and mixed-language BPSR chat. Keep the normal text
            // decoder tolerant so one malformed sequence cannot crash capture.
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
