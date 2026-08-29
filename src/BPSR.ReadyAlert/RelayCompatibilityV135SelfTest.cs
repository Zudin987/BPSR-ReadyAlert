namespace BPSR.ReadyAlert;

internal static class RelayCompatibilityV135SelfTest
{
    internal static void Run()
    {
        TestFalseIncompleteHeaderCannotHideChat();
        TestSplitStrongFrameWaitsOnlyForCompletion();
        TestCompressedFrameDownIsStandaloneAnchor();
        TestConsecutiveFramesProvideFallbackAnchor();
        TestUnknownSingleFrameIsNotTrusted();
        TestUnsynchronizedBufferIsBounded();
        TestSynOwnerRefreshPolicyAndFlagParsing();
    }

    private static void TestFalseIncompleteHeaderCannotHideChat()
    {
        const int prefixLength = 37;
        var stream = new List<byte>(prefixLength + 22);
        stream.AddRange(BuildHeaderOnly(400 * 1024, 2));
        while (stream.Count < prefixLength) stream.Add((byte)(0xA0 + stream.Count % 31));
        stream.AddRange(BuildKnownNotify(ChatProtocol.ServiceId, ChatProtocol.NotifyNewestChitChatMsgs));

        Check(231,
            GameFrameSynchronizer.IsPlausibleHeader(stream, 0, 2 * 1024 * 1024, out var fakeSize, out _) &&
            fakeSize == 400 * 1024,
            "relay fixture no longer contains the plausible incomplete false header");
        Check(232,
            !GameFrameSynchronizer.TryReadCompleteHeader(stream, 0, 2 * 1024 * 1024, out _, out _),
            "false 400 KiB frame unexpectedly became complete");

        var match = GameFrameSynchronizer.FindStrongFrame(stream, 2 * 1024 * 1024);
        Check(233, match.Found && match.Offset == prefixLength && match.MessageType == 2,
            $"strong chat frame was not found behind false header offset={match.Offset}");
    }

    private static void TestSplitStrongFrameWaitsOnlyForCompletion()
    {
        var full = BuildKnownNotify(ChatProtocol.ServiceId, ChatProtocol.NotifyNewestChitChatMsgs);
        var partial = full.Take(13).ToList();
        var before = GameFrameSynchronizer.FindStrongFrame(partial, 2 * 1024 * 1024);
        Check(234, !before.Found, "truncated chat frame was accepted as complete");

        partial.AddRange(full.Skip(13));
        var after = GameFrameSynchronizer.FindStrongFrame(partial, 2 * 1024 * 1024);
        Check(235, after.Found && after.Offset == 0 && after.Size == full.Length,
            "split chat frame did not synchronize after its remaining bytes arrived");
    }

    private static void TestCompressedFrameDownIsStandaloneAnchor()
    {
        var stream = new List<byte> { 0x91, 0x92, 0x93, 0x94 };
        var frame = new byte[18];
        Array.Copy(BuildHeaderOnly(frame.Length, 0x8006), frame, 6);
        // FrameDown sequence occupies bytes 6..9. Standard zstd magic starts at 10.
        frame[10] = 0x28;
        frame[11] = 0xB5;
        frame[12] = 0x2F;
        frame[13] = 0xFD;
        stream.AddRange(frame);

        var match = GameFrameSynchronizer.FindStrongFrame(stream, 2 * 1024 * 1024);
        Check(242, match.Found && match.Offset == 4 && match.MessageType == 6,
            "standalone compressed FrameDown with zstd signature was not a strong sync anchor");
    }

    private static void TestConsecutiveFramesProvideFallbackAnchor()
    {
        var stream = new List<byte> { 0xEE, 0xEE, 0xEE };
        var frameDown = new byte[10];
        Array.Copy(BuildHeaderOnly(10, 6), frameDown, 6);
        stream.AddRange(frameDown);
        stream.AddRange(BuildHeaderOnly(6, 4));

        var match = GameFrameSynchronizer.FindStrongFrame(stream, 2 * 1024 * 1024);
        Check(236, match.Found && match.Offset == 3 && match.MessageType == 6,
            "FrameDown plus a consecutive complete frame did not provide a fallback sync anchor");
    }

    private static void TestUnknownSingleFrameIsNotTrusted()
    {
        var frame = new byte[22];
        Array.Copy(BuildHeaderOnly(22, 2), frame, 6);
        WriteU64BE(frame, 6, 0x1122334455667788UL);

        var match = GameFrameSynchronizer.FindStrongFrame(frame, 2 * 1024 * 1024);
        Check(237, !match.Found,
            "one arbitrary complete Notify was trusted without a known service or consecutive frame");
    }

    private static void TestUnsynchronizedBufferIsBounded()
    {
        Check(238,
            GameFrameSynchronizer.BytesToTrimWhenUnsynchronized(GameFrameSynchronizer.MaxUnsynchronizedBytes) == 0,
            "unsynchronized buffer trims before its configured bound");

        var oversized = GameFrameSynchronizer.MaxUnsynchronizedBytes + 12345;
        var trim = GameFrameSynchronizer.BytesToTrimWhenUnsynchronized(oversized);
        Check(239,
            trim > 0 && oversized - trim == GameFrameSynchronizer.UnsynchronizedTailBytes,
            "oversized unsynchronized stream is not reduced to the bounded tail");
    }

    private static void TestSynOwnerRefreshPolicyAndFlagParsing()
    {
        var packet = BuildEthernetIpv4TcpPacket(flags: 0x02);
        var parsed = GamePacketFilter.ProbePacketWithFlagsForSelfTest(
            packet,
            packet.Length,
            NpcapCapture.DltEthernet);
        Check(240,
            parsed.Success && parsed.SourcePort == 50_000 && parsed.DestinationPort == 51_000 && parsed.Flags == 0x02,
            "relay SYN fixture was not parsed with its TCP flags intact");
        Check(241,
            GamePacketFilter.ShouldForceOwnerRefreshForSelfTest(0x02, 50) &&
            !GamePacketFilter.ShouldForceOwnerRefreshForSelfTest(0x02, 49) &&
            !GamePacketFilter.ShouldForceOwnerRefreshForSelfTest(0x10, 500),
            "forced owner refresh is not limited to rate-safe unmatched SYN packets");
    }

    private static byte[] BuildKnownNotify(ulong service, uint method)
    {
        const int frameSize = 22;
        var frame = new byte[frameSize];
        Array.Copy(BuildHeaderOnly(frameSize, 2), frame, 6);
        WriteU64BE(frame, 6, service);
        WriteU32BE(frame, 18, method);
        return frame;
    }

    private static byte[] BuildHeaderOnly(int declaredSize, ushort typeRaw)
    {
        var bytes = new byte[6];
        WriteU32BE(bytes, 0, checked((uint)declaredSize));
        bytes[4] = (byte)(typeRaw >> 8);
        bytes[5] = (byte)typeRaw;
        return bytes;
    }

    private static byte[] BuildEthernetIpv4TcpPacket(byte flags)
    {
        const int ip = 14;
        const int tcp = ip + 20;
        var packet = new byte[54];
        packet[12] = 0x08;
        packet[13] = 0x00;
        packet[ip] = 0x45;
        packet[ip + 2] = 0;
        packet[ip + 3] = 40;
        packet[ip + 9] = 6;
        packet[ip + 12] = 10;
        packet[ip + 13] = 1;
        packet[ip + 14] = 2;
        packet[ip + 15] = 3;
        packet[ip + 16] = 10;
        packet[ip + 17] = 9;
        packet[ip + 18] = 8;
        packet[ip + 19] = 7;
        packet[tcp] = 0xC3;
        packet[tcp + 1] = 0x50;
        packet[tcp + 2] = 0xC7;
        packet[tcp + 3] = 0x38;
        packet[tcp + 12] = 0x50;
        packet[tcp + 13] = flags;
        return packet;
    }

    private static void WriteU32BE(IList<byte> data, int offset, uint value)
    {
        data[offset] = (byte)(value >> 24);
        data[offset + 1] = (byte)(value >> 16);
        data[offset + 2] = (byte)(value >> 8);
        data[offset + 3] = (byte)value;
    }

    private static void WriteU64BE(IList<byte> data, int offset, ulong value)
    {
        data[offset] = (byte)(value >> 56);
        data[offset + 1] = (byte)(value >> 48);
        data[offset + 2] = (byte)(value >> 40);
        data[offset + 3] = (byte)(value >> 32);
        data[offset + 4] = (byte)(value >> 24);
        data[offset + 5] = (byte)(value >> 16);
        data[offset + 6] = (byte)(value >> 8);
        data[offset + 7] = (byte)value;
    }

    private static void Check(int code, bool condition, string message)
    {
        if (condition) return;
        Environment.ExitCode = code;
        throw new InvalidOperationException("v1.3.5 relay compatibility self-test failed: " + message);
    }
}
