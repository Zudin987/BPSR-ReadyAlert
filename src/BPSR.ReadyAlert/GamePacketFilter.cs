using System.Diagnostics;
using System.Net;
using System.Runtime.InteropServices;

namespace BPSR.ReadyAlert;

internal static class GamePacketFilter
{
    private static readonly string[] GameProcessNames =
    [
        "BPSR", "BPSR_STEAM", "BPSR_EPIC",
        "StarSEA", "StarASIA", "StarSEA_STEAM", "StarASIA_STEAM", "Star"
    ];

    private const int ForcedOwnerRefreshMinIntervalMs = 50;

    private static readonly object Sync = new();
    private static HashSet<int> _gamePids = new();
    private static HashSet<EndpointKey> _localEndpoints = new();
    private static DateTime _lastPidRefreshUtc = DateTime.MinValue;
    private static DateTime _lastEndpointRefreshUtc = DateTime.MinValue;
    private static DateTime _lastForcedOwnerRefreshUtc = DateTime.MinValue;
    private static string _lastSummary = string.Empty;

    internal static bool IsBpsrPacket(byte[] packet, int datalink) =>
        IsBpsrPacket(packet, packet.Length, datalink);

    internal static bool IsBpsrPacket(byte[] packet, int packetLength, int datalink)
    {
        if (!TryGetTcpEndpoints(packet, packetLength, datalink, out var source, out var destination, out var flags))
            return false;

        RefreshSnapshotsIfNeeded(force: false);
        if (IsKnownEndpoint(source, destination))
            return true;

        // The Android relay can create a new StarSEA-owned outbound connection and
        // send its SYN/early payload before the normal 100 ms owner-table snapshot is
        // refreshed. Missing those first bytes makes passive capture join mid-stream.
        // On an unmatched SYN, perform one rate-limited immediate refresh and recheck
        // ownership. We still require an actual Windows TCP-owner match, so unrelated
        // browser/Discord high-port traffic is never admitted merely because it is SYN.
        if ((flags & 0x02) == 0 || !ReserveForcedOwnerRefresh(DateTime.UtcNow))
            return false;

        RefreshSnapshotsIfNeeded(force: true);
        return IsKnownEndpoint(source, destination);
    }

    private static bool IsKnownEndpoint(EndpointKey source, EndpointKey destination)
    {
        lock (Sync)
        {
            return _localEndpoints.Contains(source) || _localEndpoints.Contains(destination);
        }
    }

    private static bool ReserveForcedOwnerRefresh(DateTime nowUtc)
    {
        lock (Sync)
        {
            if ((nowUtc - _lastForcedOwnerRefreshUtc).TotalMilliseconds < ForcedOwnerRefreshMinIntervalMs)
                return false;
            _lastForcedOwnerRefreshUtc = nowUtc;
            return true;
        }
    }

    private static void RefreshSnapshotsIfNeeded(bool force)
    {
        var now = DateTime.UtcNow;

        lock (Sync)
        {
            now = DateTime.UtcNow;

            // Process IDs are stable for a running game, so do the relatively
            // expensive process enumeration infrequently. While no game is found,
            // retry quickly so Ready Alert can be started before BPSR. A forced SYN
            // refresh also closes the race where StarSEA itself was just launched.
            var pidIntervalMs = _gamePids.Count == 0 ? 100 : 2000;
            if (force || (now - _lastPidRefreshUtc).TotalMilliseconds >= pidIntervalMs)
            {
                _lastPidRefreshUtc = now;
                _gamePids = FindGamePids();
            }

            // Connections can change during login, matchmaking and scene changes.
            // Refresh this much faster than the process list so a brand-new BPSR
            // TCP connection is not allowed to send several packets before we know
            // that its local endpoint belongs to the game.
            if (!force && (now - _lastEndpointRefreshUtc).TotalMilliseconds < 100)
                return;

            _lastEndpointRefreshUtc = now;
            var endpoints = new HashSet<EndpointKey>();
            if (_gamePids.Count > 0)
            {
                try
                {
                    foreach (var row in TcpOwnerTable.GetIpv4Rows())
                    {
                        if (!_gamePids.Contains(row.OwningPid) || row.LocalPort <= 0) continue;
                        endpoints.Add(row.Endpoint);
                    }
                }
                catch (Exception ex)
                {
                    AppLog.Write("game-filter: IPv4 TCP owner table failed " + ex.Message);
                }

                try
                {
                    foreach (var row in TcpOwnerTable.GetIpv6Rows())
                    {
                        if (!_gamePids.Contains(row.OwningPid) || row.LocalPort <= 0) continue;
                        endpoints.Add(row.Endpoint);
                    }
                }
                catch (Exception ex)
                {
                    // IPv4 remains usable if an unusual Windows/Npcap environment
                    // cannot expose the IPv6 owner table.
                    AppLog.Write("game-filter: IPv6 TCP owner table failed " + ex.Message);
                }
            }

            _localEndpoints = endpoints;
            LogSnapshotIfChanged();
        }
    }

    private static HashSet<int> FindGamePids()
    {
        var pids = new HashSet<int>();
        try
        {
            foreach (var process in Process.GetProcesses())
            {
                try
                {
                    if (GameProcessNames.Contains(process.ProcessName, StringComparer.OrdinalIgnoreCase))
                        pids.Add(process.Id);
                }
                catch { }
                finally { process.Dispose(); }
            }
        }
        catch (Exception ex)
        {
            AppLog.Write("game-filter: process enumeration failed " + ex.Message);
        }

        return pids;
    }

    private static void LogSnapshotIfChanged()
    {
        var summary = $"pids={string.Join(',', _gamePids.Order())} endpoints={_localEndpoints.Count}";
        if (string.Equals(summary, _lastSummary, StringComparison.Ordinal)) return;

        _lastSummary = summary;
        AppLog.Write("game-filter: " + summary);
        foreach (var endpoint in _localEndpoints
                     .OrderBy(x => x.IpVersion)
                     .ThenBy(x => x.AddressHigh)
                     .ThenBy(x => x.AddressLow)
                     .ThenBy(x => x.Port))
            AppLog.Write($"game-filter: local={EndpointAddressToString(endpoint)}:{endpoint.Port}");
    }

    private static bool TryGetTcpEndpoints(
        byte[] packet,
        int packetLength,
        int datalink,
        out EndpointKey source,
        out EndpointKey destination,
        out byte flags)
    {
        source = default;
        destination = default;
        flags = 0;
        if (packetLength <= 0 || packetLength > packet.Length) return false;

        var offset = 0;
        int forcedVersion = 0;
        switch (datalink)
        {
            case NpcapCapture.DltRaw:
                offset = 0;
                break;
            case NpcapCapture.DltIpv4:
                offset = 0;
                forcedVersion = 4;
                break;
            case NpcapCapture.DltIpv6:
                offset = 0;
                forcedVersion = 6;
                break;
            case NpcapCapture.DltNull:
            case NpcapCapture.DltLoop:
                offset = 4;
                break;
            case NpcapCapture.DltEthernet:
                if (!TryGetEthernetNetworkOffset(packet, packetLength, out offset, out var etherType)) return false;
                forcedVersion = etherType == 0x0800 ? 4 : etherType == 0x86DD ? 6 : 0;
                if (forcedVersion == 0) return false;
                break;
            default:
                return false;
        }

        if (offset < 0 || offset >= packetLength) return false;
        var version = forcedVersion != 0 ? forcedVersion : packet[offset] >> 4;
        return version switch
        {
            4 => TryGetIpv4TcpEndpoints(packet, packetLength, offset, out source, out destination, out flags),
            6 => TryGetIpv6TcpEndpoints(packet, packetLength, offset, out source, out destination, out flags),
            _ => false
        };
    }

    private static bool TryGetIpv4TcpEndpoints(
        byte[] packet,
        int packetLength,
        int offset,
        out EndpointKey source,
        out EndpointKey destination,
        out byte flags)
    {
        source = default;
        destination = default;
        flags = 0;
        if (offset + 20 > packetLength || (packet[offset] >> 4) != 4) return false;

        var ipHeader = (packet[offset] & 0x0F) * 4;
        if (ipHeader < 20 || offset + ipHeader + 20 > packetLength) return false;
        if (packet[offset + 9] != 6) return false;

        var tcp = offset + ipHeader;
        var tcpHeader = ((packet[tcp + 12] >> 4) & 0x0F) * 4;
        if (tcpHeader < 20 || tcp + tcpHeader > packetLength) return false;
        var srcPort = ReadU16BE(packet, tcp);
        var dstPort = ReadU16BE(packet, tcp + 2);
        if (srcPort <= 1000 || dstPort <= 1000) return false;

        flags = packet[tcp + 13];
        source = new EndpointKey(4, 0, ReadU32BE(packet, offset + 12), srcPort);
        destination = new EndpointKey(4, 0, ReadU32BE(packet, offset + 16), dstPort);
        return true;
    }

    private static bool TryGetIpv6TcpEndpoints(
        byte[] packet,
        int packetLength,
        int offset,
        out EndpointKey source,
        out EndpointKey destination,
        out byte flags)
    {
        source = default;
        destination = default;
        flags = 0;
        if (offset + 40 > packetLength || (packet[offset] >> 4) != 6) return false;

        var nextHeader = packet[offset + 6];
        var cursor = offset + 40;
        for (var depth = 0; depth < 8 && nextHeader != 6; depth++)
        {
            if (cursor >= packetLength) return false;
            switch (nextHeader)
            {
                case 0:
                case 43:
                case 60:
                    if (cursor + 2 > packetLength) return false;
                    nextHeader = packet[cursor];
                    var extLength = (packet[cursor + 1] + 1) * 8;
                    if (extLength < 8 || cursor + extLength > packetLength) return false;
                    cursor += extLength;
                    break;
                case 44:
                    if (cursor + 8 > packetLength) return false;
                    nextHeader = packet[cursor];
                    var fragmentField = ReadU16BE(packet, cursor + 2);
                    if ((fragmentField & 0xFFF8) != 0) return false;
                    cursor += 8;
                    break;
                case 51:
                    if (cursor + 2 > packetLength) return false;
                    nextHeader = packet[cursor];
                    var ahLength = (packet[cursor + 1] + 2) * 4;
                    if (ahLength < 8 || cursor + ahLength > packetLength) return false;
                    cursor += ahLength;
                    break;
                default:
                    return false;
            }
        }

        if (nextHeader != 6 || cursor + 20 > packetLength) return false;
        var tcpHeader = ((packet[cursor + 12] >> 4) & 0x0F) * 4;
        if (tcpHeader < 20 || cursor + tcpHeader > packetLength) return false;
        var srcPort = ReadU16BE(packet, cursor);
        var dstPort = ReadU16BE(packet, cursor + 2);
        if (srcPort <= 1000 || dstPort <= 1000) return false;

        flags = packet[cursor + 13];
        source = new EndpointKey(
            6,
            ReadU64BE(packet, offset + 8),
            ReadU64BE(packet, offset + 16),
            srcPort);
        destination = new EndpointKey(
            6,
            ReadU64BE(packet, offset + 24),
            ReadU64BE(packet, offset + 32),
            dstPort);
        return true;
    }

    private static bool TryGetEthernetNetworkOffset(
        byte[] packet,
        int packetLength,
        out int offset,
        out ushort etherType)
    {
        offset = -1;
        etherType = 0;
        if (packetLength < 14) return false;
        etherType = ReadU16BE(packet, 12);
        var cursor = 14;
        var vlanDepth = 0;
        while (etherType is 0x8100 or 0x88A8 or 0x9100)
        {
            if (++vlanDepth > 2 || cursor + 4 > packetLength) return false;
            etherType = ReadU16BE(packet, cursor + 2);
            cursor += 4;
        }
        offset = cursor;
        return true;
    }

    private static ushort ReadU16BE(byte[] data, int offset) =>
        (ushort)(((uint)data[offset] << 8) | data[offset + 1]);

    private static uint ReadU32BE(byte[] data, int offset) =>
        ((uint)data[offset] << 24) |
        ((uint)data[offset + 1] << 16) |
        ((uint)data[offset + 2] << 8) |
        data[offset + 3];

    private static ulong ReadU64BE(byte[] data, int offset) =>
        ((ulong)data[offset] << 56) |
        ((ulong)data[offset + 1] << 48) |
        ((ulong)data[offset + 2] << 40) |
        ((ulong)data[offset + 3] << 32) |
        ((ulong)data[offset + 4] << 24) |
        ((ulong)data[offset + 5] << 16) |
        ((ulong)data[offset + 6] << 8) |
        data[offset + 7];

    private static EndpointKey EndpointFromAddressBytes(byte[] address, int port)
    {
        if (address.Length == 4)
            return new EndpointKey(4, 0, ReadU32BE(address, 0), port);
        if (address.Length == 16)
            return new EndpointKey(6, ReadU64BE(address, 0), ReadU64BE(address, 8), port);
        return default;
    }

    private static string EndpointAddressToString(EndpointKey endpoint)
    {
        if (endpoint.IpVersion == 4)
        {
            var bytes = new byte[4];
            WriteU32BE(bytes, 0, checked((uint)endpoint.AddressLow));
            return new IPAddress(bytes).ToString();
        }

        if (endpoint.IpVersion == 6)
        {
            var bytes = new byte[16];
            WriteU64BE(bytes, 0, endpoint.AddressHigh);
            WriteU64BE(bytes, 8, endpoint.AddressLow);
            return new IPAddress(bytes).ToString();
        }

        return string.Empty;
    }

    private static void WriteU32BE(byte[] data, int offset, uint value)
    {
        data[offset] = (byte)(value >> 24);
        data[offset + 1] = (byte)(value >> 16);
        data[offset + 2] = (byte)(value >> 8);
        data[offset + 3] = (byte)value;
    }

    private static void WriteU64BE(byte[] data, int offset, ulong value)
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

    private readonly record struct EndpointKey(byte IpVersion, ulong AddressHigh, ulong AddressLow, int Port);

    private static class TcpOwnerTable
    {
        private const int AfInet = 2;
        private const int AfInet6 = 23;
        private const int TcpTableOwnerPidAll = 5;
        private const uint ErrorInsufficientBuffer = 122;

        [DllImport("iphlpapi.dll", SetLastError = true)]
        private static extern uint GetExtendedTcpTable(
            IntPtr table,
            ref int size,
            bool order,
            int ipVersion,
            int tableClass,
            uint reserved);

        [StructLayout(LayoutKind.Sequential)]
        private struct NativeTcpRow
        {
            internal uint State;
            internal uint LocalAddr;
            internal uint LocalPort;
            internal uint RemoteAddr;
            internal uint RemotePort;
            internal int OwningPid;
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct NativeTcp6Row
        {
            [MarshalAs(UnmanagedType.ByValArray, SizeConst = 16)]
            internal byte[] LocalAddr;
            internal uint LocalScopeId;
            internal uint LocalPort;
            [MarshalAs(UnmanagedType.ByValArray, SizeConst = 16)]
            internal byte[] RemoteAddr;
            internal uint RemoteScopeId;
            internal uint RemotePort;
            internal uint State;
            internal int OwningPid;
        }

        internal readonly record struct TcpRow(EndpointKey Endpoint, int LocalPort, int OwningPid);

        internal static IReadOnlyList<TcpRow> GetIpv4Rows()
        {
            return ReadTable<NativeTcpRow>(AfInet, row =>
            {
                var bytes = new IPAddress(row.LocalAddr).GetAddressBytes();
                var port = DecodePort(row.LocalPort);
                return new TcpRow(EndpointFromAddressBytes(bytes, port), port, row.OwningPid);
            });
        }

        internal static IReadOnlyList<TcpRow> GetIpv6Rows()
        {
            return ReadTable<NativeTcp6Row>(AfInet6, row =>
            {
                var bytes = row.LocalAddr ?? new byte[16];
                var port = DecodePort(row.LocalPort);
                return new TcpRow(EndpointFromAddressBytes(bytes, port), port, row.OwningPid);
            });
        }

        private static IReadOnlyList<TcpRow> ReadTable<T>(int family, Func<T, TcpRow> projector) where T : struct
        {
            var size = 0;
            var first = GetExtendedTcpTable(IntPtr.Zero, ref size, false, family, TcpTableOwnerPidAll, 0);
            if (first != ErrorInsufficientBuffer && first != 0)
                throw new InvalidOperationException("GetExtendedTcpTable(size) failed: " + first);
            if (size <= sizeof(uint)) return Array.Empty<TcpRow>();

            var buffer = Marshal.AllocHGlobal(size);
            try
            {
                var result = GetExtendedTcpTable(buffer, ref size, false, family, TcpTableOwnerPidAll, 0);
                if (result != 0)
                    throw new InvalidOperationException("GetExtendedTcpTable(data) failed: " + result);

                var count = Marshal.ReadInt32(buffer);
                var rowSize = Marshal.SizeOf<T>();
                var cursor = IntPtr.Add(buffer, sizeof(uint));
                var rows = new List<TcpRow>(Math.Max(0, count));
                for (var i = 0; i < count; i++)
                {
                    var row = Marshal.PtrToStructure<T>(cursor);
                    cursor = IntPtr.Add(cursor, rowSize);
                    rows.Add(projector(row));
                }
                return rows;
            }
            finally
            {
                Marshal.FreeHGlobal(buffer);
            }
        }

        private static int DecodePort(uint nativePort) =>
            (ushort)IPAddress.NetworkToHostOrder((short)(nativePort & 0xFFFF));
    }

    internal static bool TryParsePacketForSelfTest(
        byte[] packet,
        int datalink,
        out string srcAddress,
        out int srcPort,
        out string dstAddress,
        out int dstPort)
    {
        srcAddress = string.Empty;
        dstAddress = string.Empty;
        srcPort = dstPort = 0;
        if (!TryGetTcpEndpoints(packet, packet.Length, datalink, out var source, out var destination, out _))
            return false;

        srcAddress = EndpointAddressToString(source);
        srcPort = source.Port;
        dstAddress = EndpointAddressToString(destination);
        dstPort = destination.Port;
        return true;
    }

    internal static (bool Success, int SourcePort, int DestinationPort) ProbePacketForSelfTest(
        byte[] packet,
        int packetLength,
        int datalink)
    {
        var success = TryGetTcpEndpoints(packet, packetLength, datalink, out var source, out var destination, out _);
        return (success, source.Port, destination.Port);
    }

    internal static (bool Success, int SourcePort, int DestinationPort, byte Flags) ProbePacketWithFlagsForSelfTest(
        byte[] packet,
        int packetLength,
        int datalink)
    {
        var success = TryGetTcpEndpoints(packet, packetLength, datalink, out var source, out var destination, out var flags);
        return (success, source.Port, destination.Port, flags);
    }

    internal static bool ShouldForceOwnerRefreshForSelfTest(byte flags, int elapsedMs) =>
        (flags & 0x02) != 0 && elapsedMs >= ForcedOwnerRefreshMinIntervalMs;
}
