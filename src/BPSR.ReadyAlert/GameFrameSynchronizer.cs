namespace BPSR.ReadyAlert;

internal readonly record struct GameFrameSyncMatch(int Offset, int Size, ushort TypeRaw)
{
    internal bool Found => Offset >= 0;
    internal int MessageType => TypeRaw & 0x7FFF;
}

/// <summary>
/// Finds trustworthy BPSR packet boundaries after passive capture joins an existing
/// TCP stream mid-frame. This is deliberately stricter than the normal synchronized
/// parser: arbitrary relay payload bytes must never be allowed to pin the parser behind
/// a plausible-but-incomplete fake frame header for the global watchdog interval.
/// </summary>
internal static class GameFrameSynchronizer
{
    internal const int HeaderBytes = 6;
    internal const int RpcHeaderBytes = 16;
    internal const int MaxUnsynchronizedBytes = 512 * 1024;
    internal const int UnsynchronizedTailBytes = 128 * 1024;

    private const ulong WorldNtfService = 1_664_308_034UL;
    private const ulong MatchNtfService = 822_849_903UL;
    private const ulong GrpcTeamNtfService = 966_773_353UL;

    internal static GameFrameSyncMatch FindStrongFrame(IReadOnlyList<byte> data, int maxFrame)
    {
        if (data.Count < HeaderBytes)
            return new GameFrameSyncMatch(-1, 0, 0);

        // First preference: a complete Notify for a service ReadyAlert actually knows.
        // Matching a 64-bit service ID makes accidental synchronization inside arbitrary
        // relay bytes vanishingly unlikely, while chat/ready/team/match traffic gives us
        // frequent anchors in a real BPSR stream.
        for (var offset = 0; offset <= data.Count - HeaderBytes; offset++)
        {
            if (!TryReadCompleteHeader(data, offset, maxFrame, out var size, out var typeRaw))
                continue;
            if ((typeRaw & 0x7FFF) != 2 || size < HeaderBytes + RpcHeaderBytes)
                continue;

            var service = ReadU64BE(data, offset + HeaderBytes);
            if (IsKnownNotifyService(service))
                return new GameFrameSyncMatch(offset, size, typeRaw);
        }

        // A compressed FrameDown has a four-byte sequence field followed by a standard
        // zstd frame. Size/type + zstd magic is a strong standalone signature, allowing
        // a quiet relay connection to synchronize immediately without waiting for a
        // second outer BPSR frame just to prove alignment.
        for (var offset = 0; offset <= data.Count - HeaderBytes; offset++)
        {
            if (!TryReadCompleteHeader(data, offset, maxFrame, out var size, out var typeRaw))
                continue;
            if ((typeRaw & 0x7FFF) != 6 || (typeRaw & 0x8000) == 0 || size < 14)
                continue;

            var zstd = offset + 10; // 6-byte game header + 4-byte FrameDown sequence.
            if (data[zstd] == 0x28 && data[zstd + 1] == 0xB5 &&
                data[zstd + 2] == 0x2F && data[zstd + 3] == 0xFD)
                return new GameFrameSyncMatch(offset, size, typeRaw);
        }

        // Final fallback: two complete consecutive protocol frames, with at least one
        // carrying server data (Notify or FrameDown). This also covers non-standard
        // compressed containers where a future zstd framing variant is encountered.
        for (var offset = 0; offset <= data.Count - HeaderBytes; offset++)
        {
            if (!TryReadCompleteHeader(data, offset, maxFrame, out var firstSize, out var firstType))
                continue;

            var secondOffset = offset + firstSize;
            if (secondOffset > data.Count - HeaderBytes)
                continue;
            if (!TryReadCompleteHeader(data, secondOffset, maxFrame, out _, out var secondType))
                continue;

            var firstMessageType = firstType & 0x7FFF;
            var secondMessageType = secondType & 0x7FFF;
            if (firstMessageType is not (2 or 6) && secondMessageType is not (2 or 6))
                continue;

            return new GameFrameSyncMatch(offset, firstSize, firstType);
        }

        return new GameFrameSyncMatch(-1, 0, 0);
    }

    internal static bool TryReadCompleteHeader(
        IReadOnlyList<byte> data,
        int offset,
        int maxFrame,
        out int size,
        out ushort typeRaw)
    {
        size = 0;
        typeRaw = 0;
        if (offset < 0 || offset > data.Count - HeaderBytes)
            return false;

        var rawSize = ReadU32BE(data, offset);
        typeRaw = ReadU16BE(data, offset + 4);
        var messageType = typeRaw & 0x7FFF;
        if (rawSize < HeaderBytes || rawSize > maxFrame || messageType > 8)
            return false;
        if (rawSize > int.MaxValue)
            return false;

        size = (int)rawSize;
        return size <= data.Count - offset;
    }

    internal static bool IsPlausibleHeader(
        IReadOnlyList<byte> data,
        int offset,
        int maxFrame,
        out int size,
        out ushort typeRaw)
    {
        size = 0;
        typeRaw = 0;
        if (offset < 0 || offset > data.Count - HeaderBytes)
            return false;

        var rawSize = ReadU32BE(data, offset);
        typeRaw = ReadU16BE(data, offset + 4);
        var messageType = typeRaw & 0x7FFF;
        if (rawSize < HeaderBytes || rawSize > maxFrame || rawSize > int.MaxValue || messageType > 8)
            return false;

        size = (int)rawSize;
        return true;
    }

    internal static int BytesToTrimWhenUnsynchronized(int count)
    {
        if (count <= MaxUnsynchronizedBytes) return 0;
        return Math.Max(0, count - UnsynchronizedTailBytes);
    }

    private static bool IsKnownNotifyService(ulong service) =>
        service == ChatProtocol.ServiceId ||
        service == WorldNtfService ||
        service == MatchNtfService ||
        service == GrpcTeamNtfService;

    private static uint ReadU32BE(IReadOnlyList<byte> data, int offset) =>
        ((uint)data[offset] << 24) |
        ((uint)data[offset + 1] << 16) |
        ((uint)data[offset + 2] << 8) |
        data[offset + 3];

    private static ushort ReadU16BE(IReadOnlyList<byte> data, int offset) =>
        (ushort)(((uint)data[offset] << 8) | data[offset + 1]);

    private static ulong ReadU64BE(IReadOnlyList<byte> data, int offset) =>
        ((ulong)data[offset] << 56) |
        ((ulong)data[offset + 1] << 48) |
        ((ulong)data[offset + 2] << 40) |
        ((ulong)data[offset + 3] << 32) |
        ((ulong)data[offset + 4] << 24) |
        ((ulong)data[offset + 5] << 16) |
        ((ulong)data[offset + 6] << 8) |
        data[offset + 7];
}
