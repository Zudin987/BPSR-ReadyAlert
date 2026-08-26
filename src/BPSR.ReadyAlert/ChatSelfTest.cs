using System.Text;

namespace BPSR.ReadyAlert;

/// <summary>
/// Pure, deterministic checks executed by the published EXE's CI smoke-test mode.
/// These cover the chat code paths that do not require a live BPSR/Npcap session.
/// </summary>
internal static class ChatSelfTest
{
    internal static void Run()
    {
        TestFilters();
        TestHotkeys();
        TestUnicodeTextNotify();
        TestStickerNotify();
        TestSettingsNormalization();
    }

    private static void TestFilters()
    {
        Assert(ChatFilterExpression.IsMatch("SERUM", "serum"), "case-insensitive filter");
        Assert(ChatFilterExpression.IsMatch("SERUM", "serum | food | raid"), "friendly spaced-pipe OR");
        Assert(ChatFilterExpression.IsMatch("Need FOOD now", "serum\nfood\nraid"), "newline OR");
        Assert(ChatFilterExpression.IsMatch("hard world boss", "boss AND hard"), "AND filter");
        Assert(!ChatFilterExpression.IsMatch("easy world boss", "boss AND hard"), "AND negative filter");
        Assert(ChatFilterExpression.IsMatch("dungeon 12", "(raid|dungeon)\\s+\\d+"), "advanced regex");
        Assert(ChatFilterExpression.TryValidate("serum | food | raid", out _), "valid friendly filter");
        Assert(!ChatFilterExpression.TryValidate("(", out var error) && error.Length > 0, "invalid regex validation");
        Assert(!ChatFilterExpression.IsMatch("anything", "("), "invalid regex runtime safety");
    }

    private static void TestHotkeys()
    {
        Assert(ChatHotkey.TryParse("Ctrl+Shift+F10", out var click, out _), "click-through hotkey parse");
        Assert(click.Ctrl && click.Shift && !click.Alt && click.Key == System.Windows.Forms.Keys.F10, "click-through hotkey fields");
        Assert(click.DisplayText == "Ctrl+Shift+F10", "click-through hotkey canonical text");
        Assert(ChatHotkey.TryParse("alt+f9", out var collapse, out _), "case-insensitive hotkey parse");
        Assert(collapse.DisplayText == "Alt+F9", "hotkey canonical casing");
        Assert(!ChatHotkey.TryParse("Ctrl+Shift", out _, out var error) && error.Length > 0, "modifier-only hotkey rejected");
    }

    private static void TestUnicodeTextNotify()
    {
        const long senderId = 9_876_543_210;
        const string senderName = "Aiman玩家";
        const string text = "SeRuM makanan café 食物 — raid";
        const long timestamp = 1_700_000_000;

        var sender = ProtoMessage(
            VarintField(1, unchecked((ulong)senderId)),
            StringField(2, senderName),
            VarintField(5, 88));

        var info = ProtoMessage(
            VarintField(1, (ulong)ChatMessageKind.Text),
            StringField(3, text));

        var chatMsg = ProtoMessage(
            BytesField(2, sender),
            VarintField(3, timestamp),
            BytesField(4, info));

        var request = ProtoMessage(
            VarintField(1, (ulong)ChatChannel.World),
            BytesField(2, chatMsg));

        var notify = ProtoMessage(BytesField(1, request));
        Assert(ChatProtocol.TryParseNotify(notify, out var parsed), "text notify parse");
        Assert(parsed.SenderId == senderId, "sender id");
        Assert(parsed.SenderName == senderName, "UTF-8 sender name");
        Assert(parsed.SenderLevel == 88, "sender level");
        Assert(parsed.Channel == ChatChannel.World, "world channel");
        Assert(parsed.Kind == ChatMessageKind.Text, "text kind");
        Assert(parsed.Text == text, "UTF-8 Malay/Chinese message");
        Assert(new DateTimeOffset(parsed.Timestamp).ToUnixTimeSeconds() == timestamp, "Unix timestamp");
    }

    private static void TestStickerNotify()
    {
        const ulong stickerId = 2001;
        var sender = ProtoMessage(
            VarintField(1, 123),
            StringField(2, "StickerUser"),
            VarintField(5, 50));

        var sticker = ProtoMessage(VarintField(1, stickerId));
        var info = ProtoMessage(
            VarintField(1, (ulong)ChatMessageKind.Sticker),
            BytesField(5, sticker));
        var chatMsg = ProtoMessage(
            BytesField(2, sender),
            VarintField(3, 1_700_000_000),
            BytesField(4, info));
        var request = ProtoMessage(
            VarintField(1, (ulong)ChatChannel.Team),
            BytesField(2, chatMsg));
        var notify = ProtoMessage(BytesField(1, request));

        Assert(ChatProtocol.TryParseNotify(notify, out var parsed), "sticker notify parse");
        Assert(parsed.Channel == ChatChannel.Team, "team channel");
        Assert(parsed.Kind == ChatMessageKind.Sticker, "sticker kind");
        Assert(parsed.Text == "[Image(2001)]", "sticker config id");
    }

    private static void TestSettingsNormalization()
    {
        var settings = new ChatOverlaySettings
        {
            Tabs = [],
            BlockedUsers = [],
            ChannelColors = new Dictionary<int, string>
            {
                [(int)ChatChannel.World] = "not-a-color",
                [12345] = "#FFFFFF"
            },
            BackgroundOpacity = -10,
            ToolbarOpacity = 500,
            TextOpacity = 0,
            WindowOpacity = 500,
            FontSize = 99,
            CollapseSide = "Diagonal",
            MaxHistory = 9
        };
        settings.Normalize();

        Assert(settings.Tabs.Count == 3, "default tabs");
        Assert(settings.Tabs.Any(x => x.Name == "World"), "World tab");
        Assert(settings.Tabs.Any(x => x.Name == "Guild / Team"), "Guild / Team tab");
        var all = settings.Tabs.Single(x => x.Name == "All");
        Assert(all.Channels.Contains((int)ChatChannel.Null), "All includes Null");
        Assert(all.Channels.Contains((int)ChatChannel.Newbie), "All includes Newbie");
        Assert(all.Channels.Contains((int)ChatChannel.Play), "All includes Play");
        Assert(settings.BackgroundOpacity == 10, "background opacity clamp");
        Assert(settings.ToolbarOpacity == 100, "toolbar opacity clamp");
        Assert(settings.TextOpacity == 40, "text opacity clamp");
        Assert(settings.WindowOpacity == 100, "window opacity clamp");
        Assert(Math.Abs(settings.FontSize - 24F) < 0.01F, "font size clamp");
        Assert(settings.CollapseSide == "Right", "collapse side normalization");
        Assert(settings.MaxHistory == 10, "history clamp");
        Assert(!settings.ChannelColors.ContainsKey(12345), "unknown channel color removed");
        Assert(settings.ChannelColors[(int)ChatChannel.World] == "#63C7FF", "invalid channel color repaired");
        Assert(settings.ChannelColors.Count == Enum.GetValues<ChatChannel>().Length, "all channel colors present");
    }

    private static byte[] ProtoMessage(params byte[][] fields)
    {
        var length = fields.Sum(x => x.Length);
        var result = new byte[length];
        var offset = 0;
        foreach (var field in fields)
        {
            Buffer.BlockCopy(field, 0, result, offset, field.Length);
            offset += field.Length;
        }
        return result;
    }

    private static byte[] StringField(int fieldNumber, string value) =>
        BytesField(fieldNumber, Encoding.UTF8.GetBytes(value));

    private static byte[] BytesField(int fieldNumber, byte[] value)
    {
        using var stream = new MemoryStream();
        WriteVarint(stream, ((ulong)fieldNumber << 3) | 2UL);
        WriteVarint(stream, (ulong)value.Length);
        stream.Write(value, 0, value.Length);
        return stream.ToArray();
    }

    private static byte[] VarintField(int fieldNumber, long value) =>
        VarintField(fieldNumber, unchecked((ulong)value));

    private static byte[] VarintField(int fieldNumber, ulong value)
    {
        using var stream = new MemoryStream();
        WriteVarint(stream, (ulong)fieldNumber << 3);
        WriteVarint(stream, value);
        return stream.ToArray();
    }

    private static void WriteVarint(Stream stream, ulong value)
    {
        while (value >= 0x80)
        {
            stream.WriteByte((byte)(value | 0x80));
            value >>= 7;
        }
        stream.WriteByte((byte)value);
    }

    private static void Assert(bool condition, string name)
    {
        if (!condition)
            throw new InvalidOperationException("Chat self-test failed: " + name);
    }
}
