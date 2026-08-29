using System.Runtime.CompilerServices;

namespace BPSR.ReadyAlert;

internal readonly record struct GameFrameSyncMatch(int Offset, int Size, ushort TypeRaw)
{
    internal bool Found => Offset >= 0;
    internal int MessageType => TypeRaw & 0x7FFF;
}

/// <summary>
/// Finds trustworthy BPSR packet boundaries after passive capture joins an existing
/// TCP stream mid-frame. A strong signature may be identified before the full declared
/// frame arrives, allowing the caller to lock onto the real header and then wait only
/// for that frame instead of repeatedly rescanning an ever-growing relay buffer.
/// </summary>
internal static class GameFrameSynchronizer
{
    internal const int HeaderBytes = 6;
    internal const int RpcHeaderBytes = 16;
    internal const int MaxUnsynchronizedBytes = 512 * 1024;
    internal const int UnsynchronizedTailBytes = 128 * 1024;
    internal const int SignatureLookBehindBytes = 40;

    private const ulong WorldNtfService = 1_664_308_034UL;
    private const ulong MatchNtfService = 822_849_903UL;
    private const ulong GrpcTeamNtfService = 966_773_353UL;

    // Flow streams are stable List<byte> instances. Remember how far each unsynchronized
    // stream has already been inspected, revisiting only a small overlap after append.
    // ConditionalWeakTable prevents this cache from extending a flow's lifetime.
    private static readonly ConditionalWeakTable<object, ScanState> ScanStates = new();
    private static long _selfTestExaminedOffsets;

    internal static GameFrameSyncMatch FindStrongFrame(
        IReadOnlyList<byte> data,
        int maxFrame,
        int startOffset = 0)
    {
        if (data.Count < HeaderBytes)
            return new GameFrameSyncMatch(-1, 0, 0);

        var cacheKey = (object)data;
        var state = ScanStates.GetOrCreateValue(cacheKey);
        if (data.Count < state.LastCount)
            state.NextOffset = 0;

        var requestedStart = Math.Clamp(startOffset, 0, Math.Max(0, data.Count - HeaderBytes));
        var scanStart = Math.Max(requestedStart, state.NextOffset);
        if (scanStart > data.Count - HeaderBytes)
            scanStart = Math.Max(requestedStart, data.Count - HeaderBytes);

        for (var offset = scanStart; offset <= data.Count - HeaderBytes; offset++)
        {
            Interlocked.Increment(ref _selfTestExaminedOffsets);
            if (!IsPlausibleHeader(data, offset, maxFrame, out var size, out var typeRaw))
                continue;

            var messageType = typeRaw & 0x7FFF;

            // A Notify exposes its service/method routing header before the protobuf
            // body. A known 64-bit service is strong enough to establish alignment even
            // if the full Notify body is still split across later TCP segments.
            if (messageType == 2 &&
                size >= HeaderBytes + RpcHeaderBytes &&
                offset + HeaderBytes + RpcHeaderBytes <= data.Count)
            {
                var service = ReadU64BE(data, offset + HeaderBytes);
                if (IsKnownNotifyService(service))
                {
                    ScanStates.Remove(cacheKey);
                    return new GameFrameSyncMatch(offset, size, typeRaw);
                }
            }

            if (messageType != 6 || size < 10)
                continue;

            var compressed = (typeRaw & 0x8000) != 0;
            if (compressed)
            {
                // FrameDown = 6-byte game header + 4-byte sequence + zstd frame.
                // Standard zstd magic gives a strong signature without requiring a
                // second outer frame or the complete compressed payload.
                var zstd = offset + 10;
                if (size >= 14 && zstd + 4 <= data.Count &&
                    data[zstd] == 0x28 && data[zstd + 1] == 0xB5 &&
                    data[zstd + 2] == 0x2F && data[zstd + 3] == 0xFD)
                {
                    ScanStates.Remove(cacheKey);
                    return new GameFrameSyncMatch(offset, size, typeRaw);
                }
                continue;
            }

            // Uncompressed FrameDown exposes its nested packet stream. Recognize a
            // nested known Notify as another relay-safe single-frame anchor.
            var nested = offset + 10;
            if (nested + HeaderBytes + RpcHeaderBytes > data.Count || size < 10 + HeaderBytes + RpcHeaderBytes)
                continue;
            if (!IsPlausibleHeader(data, nested, maxFrame, out _, out var nestedType) ||
                (nestedType & 0x7FFF) != 2)
                continue;

            var nestedService = ReadU64BE(data, nested + HeaderBytes);
            if (IsKnownNotifyService(nestedService))
            {
                ScanStates.Remove(cacheKey);
                return new GameFrameSyncMatch(offset, size, typeRaw);
            }
        }

        state.LastCount = data.Count;
        state.NextOffset = Math.Max(requestedStart, NextIncrementalScanOffset(data.Count));
        return new GameFrameSyncMatch(-1, 0, 0);
    }

    internal static bool TryReadCompleteHeader(
        IReadOnlyList<byte> data,
        int offset,
        int maxFrame,
        out int size,
        out ushort typeRaw)
    {
        if (!IsPlausibleHeader(data, offset, maxFrame, out size, out typeRaw))
            return false;
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

    internal static int NextIncrementalScanOffset(int previousCount) =>
        Math.Max(0, previousCount - SignatureLookBehindBytes);

    internal static int BytesToTrimWhenUnsynchronized(int count)
    {
        if (count <= MaxUnsynchronizedBytes) return 0;
        return Math.Max(0, count - UnsynchronizedTailBytes);
    }

    internal static void ResetScanMetricsForSelfTest() =>
        Interlocked.Exchange(ref _selfTestExaminedOffsets, 0);

    internal static long ExaminedOffsetsForSelfTest() =>
        Interlocked.Read(ref _selfTestExaminedOffsets);

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

    private sealed class ScanState
    {
        internal int LastCount;
        internal int NextOffset;
    }
}
