using System.Collections.Concurrent;
using System.Drawing;
using System.Net;
using System.Text;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal static class ChatRc2SelfTest
{
    internal static void Run()
    {
        TestNotificationEngineIsIndependentFromUiQueue();
        TestSoundRulesRemainMessageOnly();
        TestWheelMathHonorsWindowsSettings();
        TestSmoothScrollCancelsProgrammaticTarget();
        TestCustomDarkScrollbarCreates();
        TestMessageSequenceIdentity();
        TestSenderColorDistribution();
        TestRegexCompilationCache();
        TestIpv6PacketParsing();
        TestRichChatParsing();
        TestNotificationWaveLimits();
    }

    private static void TestNotificationEngineIsIndependentFromUiQueue()
    {
        var settings = new ChatOverlaySettings
        {
            ChatSoundVolume = 0,
            HighlightSoundRules =
            [
                new ChatSoundRule { Enabled = true, Match = @"\bserum\b" }
            ]
        };
        settings.Normalize();

        var uiQueue = new ConcurrentQueue<ChatMessageEvent>();
        ChatNotificationEngine.Configure(settings, string.Empty);
        ChatCaptureBridge.Configure(uiQueue);
        ChatCaptureBridge.Enabled = true;

        try
        {
            var before = ChatNotificationEngine.GetStatus().Processed;
            var payload = BuildNotify(ChatMessageKind.Text, directText: "need SERUM now");
            Assert(ChatCaptureBridge.TryHandle(ChatProtocol.ServiceId, ChatProtocol.NotifyNewestChitChatMsgs, payload),
                "chat notify payload is claimed by the shared capture bridge");

            Assert(SpinWait.SpinUntil(
                    () => ChatNotificationEngine.GetStatus().Processed > before,
                    TimeSpan.FromSeconds(2)),
                "notification worker processes a chat event without waiting for the UI queue");
            Assert(uiQueue.Count == 1,
                "UI queue can remain undrained while the independent notification worker already processed the message");
        }
        finally
        {
            ChatCaptureBridge.Enabled = false;
            while (uiQueue.TryDequeue(out _)) { }
        }
    }

    private static void TestSoundRulesRemainMessageOnly()
    {
        var settings = new ChatOverlaySettings
        {
            HighlightSoundRules =
            [
                new ChatSoundRule { Enabled = true, Match = @"\bserum\b" }
            ]
        };
        settings.Normalize();

        var namedSerum = new ChatMessageEvent(
            1, "Serum", 80, ChatChannel.World, DateTime.Now, ChatMessageKind.Text, "hello everyone", 1);
        var actualSerumMessage = namedSerum with
        {
            SenderId = 2,
            SenderName = "Artemis",
            Text = "need SERUM for boss",
            SequenceId = 2
        };

        Assert(!ChatNotificationEngine.EvaluateForSelfTest(settings, namedSerum),
            "a sender named Serum does not trigger a serum sound rule");
        Assert(ChatNotificationEngine.EvaluateForSelfTest(settings, actualSerumMessage),
            "message content still triggers the serum sound rule case-insensitively");

        settings.BlockedUsers = [new ChatBlockedUser { Id = 2, Name = "Artemis" }];
        settings.Normalize();
        Assert(!ChatNotificationEngine.EvaluateForSelfTest(settings, actualSerumMessage),
            "blocked senders do not trigger notification sounds");

        settings.BlockedUsers.Clear();
        settings.HideStickers = true;
        settings.Normalize();
        Assert(!ChatNotificationEngine.EvaluateForSelfTest(
                settings,
                actualSerumMessage with { Kind = ChatMessageKind.Sticker, Text = "serum" }),
            "hidden stickers do not trigger notification sounds");
    }

    private static void TestWheelMathHonorsWindowsSettings()
    {
        double remainder = 0;
        Assert(ChatWheelMath.AccumulateRows(120, 0, 10, ref remainder) == 0,
            "Windows no-wheel-scroll setting is respected");

        remainder = 0;
        Assert(ChatWheelMath.AccumulateRows(120, -1, 10, ref remainder) == 9,
            "Windows page-at-a-time setting uses the visible page size");

        remainder = 0;
        var total = 0;
        for (var i = 0; i < 4; i++)
            total += ChatWheelMath.AccumulateRows(30, 3, 10, ref remainder);
        Assert(total == 3,
            "four high-resolution quarter-detents produce the same three-row motion as one normal detent");
    }

    private static void TestSmoothScrollCancelsProgrammaticTarget()
    {
        using var list = new ListBox
        {
            Size = new Size(320, 120),
            IntegralHeight = false,
            ItemHeight = 20
        };
        for (var i = 0; i < 100; i++) list.Items.Add("row " + i);
        _ = list.Handle;
        list.TopIndex = 20;

        using var controller = new ChatListBoxUxController(list, () => { });
        controller.HandleWheelDelta(-120);

        ChatListScrollMath.ScrollToBottom(list);
        var expected = list.TopIndex;
        controller.CancelAndSyncToCurrent();
        var state = controller.GetStateForSelfTest();
        Assert(!state.TimerEnabled && state.TargetTopIndex == expected,
            "programmatic scroll cancels and synchronizes any stale smooth-scroll target");

        controller.AdvanceForSelfTest();
        Assert(list.TopIndex == expected && ChatListScrollMath.IsAtBottom(list),
            "an old animation cannot pull the viewport away after Go to latest");
    }

    private static void TestCustomDarkScrollbarCreates()
    {
        using var list = new ListBox
        {
            Size = new Size(320, 120),
            IntegralHeight = false,
            ItemHeight = 20
        };
        for (var i = 0; i < 40; i++) list.Items.Add("row " + i);
        _ = list.Handle;

        using var bar = new ChatDarkScrollBar(list, _ => { }, () => { })
        {
            Size = new Size(Math.Max(12, SystemInformation.VerticalScrollBarWidth), 120)
        };
        _ = bar.Handle;
        bar.SyncFromList();
        Assert(bar.IsHandleCreated && bar.BackColor.R < 80 && bar.BackColor.G < 80 && bar.BackColor.B < 80,
            "custom chat scrollbar creates as a dark owner-drawn control");
    }

    private static void TestMessageSequenceIdentity()
    {
        var timestamp = DateTime.Now;
        var a = new ChatMessageEvent(7, "Same", 80, ChatChannel.World, timestamp, ChatMessageKind.Text, "same", 101);
        var b = a with { SequenceId = 102 };
        Assert(a != b,
            "otherwise-identical same-second messages remain uniquely identifiable by local sequence ID");
    }

    private static void TestSenderColorDistribution()
    {
        var sample = new ChatMessageEvent(1, "User", 80, ChatChannel.World, DateTime.Now, ChatMessageKind.Text, "hi");
        var colors = Enumerable.Range(1, 48)
            .Select(i => ChatSenderColor.ForMessage(sample with { SenderId = i }))
            .Distinct()
            .Count();
        Assert(colors >= 24,
            "deterministic sender coloring provides substantially more identity separation than the old 16-color palette");
    }

    private static void TestRegexCompilationCache()
    {
        ChatFilterExpression.ClearCacheForSelfTest();
        const string expression = @"\b(?:tina|tr|towering)\b";
        for (var i = 0; i < 100; i++)
            Assert(ChatFilterExpression.IsMatch("need TR now", expression), "cached regex continues matching correctly");
        Assert(ChatFilterExpression.CachedExpressionCountForSelfTest == 1,
            "repeated chat matching reuses one compiled expression instead of rebuilding regex every message");
    }

    private static void TestIpv6PacketParsing()
    {
        var packet = BuildIpv6TcpPacket(withDestinationOptions: false);
        Assert(GamePacketFilter.TryParsePacketForSelfTest(
                packet,
                NpcapCapture.DltIpv6,
                out var source,
                out var sourcePort,
                out var destination,
                out var destinationPort),
            "raw IPv6 TCP packet is recognized by the BPSR packet filter");
        Assert(IPAddress.Parse(source).Equals(IPAddress.Parse("2001:db8::1")) && sourcePort == 50000,
            "IPv6 source endpoint is decoded correctly");
        Assert(IPAddress.Parse(destination).Equals(IPAddress.Parse("2001:db8::2")) && destinationPort == 50001,
            "IPv6 destination endpoint is decoded correctly");

        var extended = BuildIpv6TcpPacket(withDestinationOptions: true);
        Assert(GamePacketFilter.TryParsePacketForSelfTest(
                extended,
                NpcapCapture.DltIpv6,
                out _, out _, out _, out _),
            "IPv6 destination-options extension header is safely walked to TCP");
    }

    private static void TestRichChatParsing()
    {
        var voice = Message(StringField(2, "need serum"));
        Assert(ChatProtocol.TryParseNotify(
                BuildNotify(ChatMessageKind.Voice, LengthField(6, voice)),
                out var voiceMessage) &&
               voiceMessage.Text.Contains("serum", StringComparison.OrdinalIgnoreCase),
            "voice transcript text is exposed to chat filters");

        var notice = Message(
            VarintField(1, 42),
            StringField(2, "towering"),
            StringField(2, "tr"));
        Assert(ChatProtocol.TryParseNotify(
                BuildNotify(ChatMessageKind.MultiLanguageNotice, LengthField(4, notice)),
                out var noticeMessage) &&
               noticeMessage.Text.Contains("towering", StringComparison.OrdinalIgnoreCase) &&
               noticeMessage.Text.Contains("tr", StringComparison.OrdinalIgnoreCase),
            "multi-language notice arguments are exposed to chat filters");

        var placeholder = Message(
            VarintField(1, 7),
            LengthField(2, Encoding.UTF8.GetBytes("tina")));
        var hypertext = Message(
            VarintField(1, 99),
            LengthField(2, placeholder));
        Assert(ChatProtocol.TryParseNotify(
                BuildNotify(ChatMessageKind.Hypertext, LengthField(7, hypertext)),
                out var hyperMessage) &&
               hyperMessage.Text.Contains("tina", StringComparison.OrdinalIgnoreCase),
            "hypertext string placeholders are exposed to chat filters");
    }

    private static void TestNotificationWaveLimits()
    {
        var root = Path.Combine(Path.GetTempPath(), "BPSR-ReadyAlert-rc2-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(root);
        var shortPath = Path.Combine(root, "short.wav");
        var longPath = Path.Combine(root, "long.wav");
        var hugePath = Path.Combine(root, "huge.wav");

        try
        {
            WritePcm16Wave(shortPath, seconds: 1);
            Assert(ChatSoundVolumePlayer.IsSupportedWave(shortPath, out _),
                "short standard PCM WAV is accepted for chat notification audio");

            WritePcm16Wave(longPath, seconds: 16);
            Assert(!ChatSoundVolumePlayer.IsSupportedWave(longPath, out var longError) &&
                   longError.Contains("too long", StringComparison.OrdinalIgnoreCase),
                "notification WAV duration is capped to protect memory");

            using (var stream = new FileStream(hugePath, FileMode.Create, FileAccess.Write, FileShare.None))
                stream.SetLength(ChatSoundVolumePlayer.MaxNotificationWaveBytes + 1);
            Assert(!ChatSoundVolumePlayer.IsSupportedWave(hugePath, out var sizeError) &&
                   sizeError.Contains("too large", StringComparison.OrdinalIgnoreCase),
                "notification WAV file size is capped before loading bytes into memory");
        }
        finally
        {
            ChatSoundVolumePlayer.ClearCacheForSelfTest();
            try { Directory.Delete(root, recursive: true); } catch { }
        }
    }

    private static byte[] BuildNotify(ChatMessageKind kind, byte[]? richField = null, string directText = "")
    {
        var sender = Message(
            VarintField(1, 1234),
            StringField(2, "Tester"),
            VarintField(5, 80));

        var infoParts = new List<byte[]> { VarintField(1, (ulong)kind) };
        if (!string.IsNullOrEmpty(directText)) infoParts.Add(StringField(3, directText));
        if (richField is { Length: > 0 }) infoParts.Add(richField);

        var chat = Message(
            LengthField(2, sender),
            VarintField(3, 1_750_000_000),
            LengthField(4, Message(infoParts.ToArray())));
        var request = Message(
            VarintField(1, (ulong)ChatChannel.World),
            LengthField(2, chat));
        return Message(LengthField(1, request));
    }

    private static byte[] BuildIpv6TcpPacket(bool withDestinationOptions)
    {
        var tcpOffset = withDestinationOptions ? 48 : 40;
        var packet = new byte[tcpOffset + 20];
        packet[0] = 0x60;
        var payloadLength = packet.Length - 40;
        packet[4] = (byte)(payloadLength >> 8);
        packet[5] = (byte)payloadLength;
        packet[6] = withDestinationOptions ? (byte)60 : (byte)6;
        packet[7] = 64;
        IPAddress.Parse("2001:db8::1").GetAddressBytes().CopyTo(packet, 8);
        IPAddress.Parse("2001:db8::2").GetAddressBytes().CopyTo(packet, 24);

        if (withDestinationOptions)
        {
            packet[40] = 6;
            packet[41] = 0;
        }

        WriteU16Be(packet, tcpOffset, 50000);
        WriteU16Be(packet, tcpOffset + 2, 50001);
        packet[tcpOffset + 12] = 0x50;
        return packet;
    }

    private static void WritePcm16Wave(string path, int seconds)
    {
        const int sampleRate = 8000;
        const short channels = 1;
        const short bits = 16;
        var bytesPerSample = channels * bits / 8;
        var byteRate = sampleRate * bytesPerSample;
        var dataLength = checked(byteRate * seconds);

        using var stream = new FileStream(path, FileMode.Create, FileAccess.Write, FileShare.None);
        using var writer = new BinaryWriter(stream, Encoding.ASCII, leaveOpen: false);
        writer.Write(Encoding.ASCII.GetBytes("RIFF"));
        writer.Write(36 + dataLength);
        writer.Write(Encoding.ASCII.GetBytes("WAVE"));
        writer.Write(Encoding.ASCII.GetBytes("fmt "));
        writer.Write(16);
        writer.Write((short)1);
        writer.Write(channels);
        writer.Write(sampleRate);
        writer.Write(byteRate);
        writer.Write((short)bytesPerSample);
        writer.Write(bits);
        writer.Write(Encoding.ASCII.GetBytes("data"));
        writer.Write(dataLength);
        writer.Write(new byte[dataLength]);
    }

    private static byte[] Message(params byte[][] parts)
    {
        var length = parts.Sum(x => x.Length);
        var result = new byte[length];
        var offset = 0;
        foreach (var part in parts)
        {
            Buffer.BlockCopy(part, 0, result, offset, part.Length);
            offset += part.Length;
        }
        return result;
    }

    private static byte[] StringField(int field, string value) =>
        LengthField(field, Encoding.UTF8.GetBytes(value));

    private static byte[] LengthField(int field, byte[] value) =>
        Message(Varint((ulong)((field << 3) | 2)), Varint((ulong)value.Length), value);

    private static byte[] VarintField(int field, ulong value) =>
        Message(Varint((ulong)(field << 3)), Varint(value));

    private static byte[] Varint(ulong value)
    {
        var bytes = new List<byte>(10);
        do
        {
            var next = (byte)(value & 0x7F);
            value >>= 7;
            if (value != 0) next |= 0x80;
            bytes.Add(next);
        } while (value != 0);
        return bytes.ToArray();
    }

    private static void WriteU16Be(byte[] data, int offset, int value)
    {
        data[offset] = (byte)(value >> 8);
        data[offset + 1] = (byte)value;
    }

    private static void Assert(bool condition, string name)
    {
        if (!condition) throw new InvalidOperationException("Chat RC2 self-test failed: " + name);
    }
}
