using System.Diagnostics;
using System.Net;
using System.Net.NetworkInformation;
using System.Net.Sockets;
using System.Runtime.InteropServices;

namespace BPSR.ReadyAlert;

internal static class GamePacketFilter
{
    private static readonly string[] GameProcessNames =
    [
        "BPSR", "BPSR_STEAM", "BPSR_EPIC",
        "StarSEA", "StarASIA", "StarSEA_STEAM", "StarASIA_STEAM", "Star"
    ];

    private static readonly object Sync = new();
    private static HashSet<int> _gamePids = new();
    private static HashSet<EndpointKey> _localEndpoints = new();
    private static DateTime _lastPidRefreshUtc = DateTime.MinValue;
    private static DateTime _lastEndpointRefreshUtc = DateTime.MinValue;
    private static string _lastSummary = string.Empty;

    internal static bool IsBpsrPacket(byte[] packet, int datalink)
    {
        if (!TryGetTcpEndpoints(packet, datalink, out var srcAddress, out var srcPort, out var dstAddress, out var dstPort))
            return false;

        RefreshSnapshotsIfNeeded();

        lock (Sync)
        {
            return _localEndpoints.Contains(new EndpointKey(srcAddress, srcPort)) ||
                   _localEndpoints.Contains(new EndpointKey(dstAddress, dstPort));
        }
    }

    private static void RefreshSnapshotsIfNeeded()
    {
        var now = DateTime.UtcNow;

        lock (Sync)
        {
            now = DateTime.UtcNow;

            // Process IDs are stable for a running game, so do the relatively
            // expensive process enumeration infrequently. While no game is found,
            // retry quickly so Ready Alert can be started before BPSR.
            var pidIntervalMs = _gamePids.Count == 0 ? 100 : 2000;
            if ((now - _lastPidRefreshUtc).TotalMilliseconds >= pidIntervalMs)
            {
                _lastPidRefreshUtc = now;
                _gamePids = FindGamePids();
            }

            // Connections can change during login, matchmaking and scene changes.
            // Refresh this much faster than the process list so a brand-new BPSR
            // TCP connection is not allowed to send several packets before we know
            // that its local endpoint belongs to the game.
            if ((now - _lastEndpointRefreshUtc).TotalMilliseconds < 100)
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
                        endpoints.Add(new EndpointKey(row.LocalAddress, row.LocalPort));
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
                        endpoints.Add(new EndpointKey(row.LocalAddress, row.LocalPort));
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
        foreach (var endpoint in _localEndpoints.OrderBy(x => x.Address, StringComparer.Ordinal).ThenBy(x => x.Port))
            AppLog.Write($"game-filter: local={endpoint.Address}:{endpoint.Port}");
    }

    private static bool TryGetTcpEndpoints(
        byte[] packet,
        int datalink,
        out string srcAddress,
        out int srcPort,
        out string dstAddress,
        out int dstPort)
    {
        srcAddress = string.Empty;
        dstAddress = string.Empty;
        srcPort = 0;
        dstPort = 0;

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
                if (!TryGetEthernetNetworkOffset(packet, out offset, out var etherType)) return false;
                forcedVersion = etherType == 0x0800 ? 4 : etherType == 0x86DD ? 6 : 0;
                if (forcedVersion == 0) return false;
                break;
            default:
                return false;
        }

        if (offset < 0 || offset >= packet.Length) return false;
        var version = forcedVersion != 0 ? forcedVersion : packet[offset] >> 4;
        return version switch
        {
            4 => TryGetIpv4TcpEndpoints(packet, offset, out srcAddress, out srcPort, out dstAddress, out dstPort),
            6 => TryGetIpv6TcpEndpoints(packet, offset, out srcAddress, out srcPort, out dstAddress, out dstPort),
            _ => false
        };
    }

    private static bool TryGetIpv4TcpEndpoints(
        byte[] packet,
        int offset,
        out string srcAddress,
        out int srcPort,
        out string dstAddress,
        out int dstPort)
    {
        srcAddress = string.Empty;
        dstAddress = string.Empty;
        srcPort = dstPort = 0;
        if (offset + 20 > packet.Length || (packet[offset] >> 4) != 4) return false;

        var ipHeader = (packet[offset] & 0x0F) * 4;
        if (ipHeader < 20 || offset + ipHeader + 20 > packet.Length) return false;
        if (packet[offset + 9] != 6) return false;

        var tcp = offset + ipHeader;
        srcPort = ReadU16BE(packet, tcp);
        dstPort = ReadU16BE(packet, tcp + 2);
        if (srcPort <= 1000 || dstPort <= 1000) return false;

        srcAddress = new IPAddress(packet.AsSpan(offset + 12, 4)).ToString();
        dstAddress = new IPAddress(packet.AsSpan(offset + 16, 4)).ToString();
        return true;
    }

    private static bool TryGetIpv6TcpEndpoints(
        byte[] packet,
        int offset,
        out string srcAddress,
        out int srcPort,
        out string dstAddress,
        out int dstPort)
    {
        srcAddress = string.Empty;
        dstAddress = string.Empty;
        srcPort = dstPort = 0;
        if (offset + 40 > packet.Length || (packet[offset] >> 4) != 6) return false;

        var nextHeader = packet[offset + 6];
        var cursor = offset + 40;
        for (var depth = 0; depth < 8 && nextHeader != 6; depth++)
        {
            if (cursor >= packet.Length) return false;
            switch (nextHeader)
            {
                // Hop-by-Hop, Routing, Destination Options.
                case 0:
                case 43:
                case 60:
                    if (cursor + 2 > packet.Length) return false;
                    nextHeader = packet[cursor];
                    var extLength = (packet[cursor + 1] + 1) * 8;
                    if (extLength < 8 || cursor + extLength > packet.Length) return false;
                    cursor += extLength;
                    break;

                // Fragment header. Only the first fragment can contain the TCP header.
                case 44:
                    if (cursor + 8 > packet.Length) return false;
                    nextHeader = packet[cursor];
                    var fragmentField = ReadU16BE(packet, cursor + 2);
                    if ((fragmentField & 0xFFF8) != 0) return false;
                    cursor += 8;
                    break;

                // Authentication Header.
                case 51:
                    if (cursor + 2 > packet.Length) return false;
                    nextHeader = packet[cursor];
                    var ahLength = (packet[cursor + 1] + 2) * 4;
                    if (ahLength < 8 || cursor + ahLength > packet.Length) return false;
                    cursor += ahLength;
                    break;

                // ESP or an unknown extension cannot be inspected safely here.
                default:
                    return false;
            }
        }

        if (nextHeader != 6 || cursor + 20 > packet.Length) return false;
        srcPort = ReadU16BE(packet, cursor);
        dstPort = ReadU16BE(packet, cursor + 2);
        if (srcPort <= 1000 || dstPort <= 1000) return false;

        srcAddress = new IPAddress(packet.AsSpan(offset + 8, 16)).ToString();
        dstAddress = new IPAddress(packet.AsSpan(offset + 24, 16)).ToString();
        return true;
    }

    private static bool TryGetEthernetNetworkOffset(byte[] packet, out int offset, out ushort etherType)
    {
        offset = -1;
        etherType = 0;
        if (packet.Length < 14) return false;
        etherType = ReadU16BE(packet, 12);
        var cursor = 14;
        var vlanDepth = 0;
        while (etherType is 0x8100 or 0x88A8 or 0x9100)
        {
            if (++vlanDepth > 2 || cursor + 4 > packet.Length) return false;
            etherType = ReadU16BE(packet, cursor + 2);
            cursor += 4;
        }
        offset = cursor;
        return true;
    }

    private static ushort ReadU16BE(byte[] data, int offset) =>
        (ushort)(((uint)data[offset] << 8) | data[offset + 1]);

    private readonly record struct EndpointKey(string Address, int Port);

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

        internal readonly record struct TcpRow(string LocalAddress, int LocalPort, int OwningPid);

        internal static IReadOnlyList<TcpRow> GetIpv4Rows()
        {
            return ReadTable<NativeTcpRow>(AfInet, row =>
            {
                var address = new IPAddress(row.LocalAddr).ToString();
                var port = DecodePort(row.LocalPort);
                return new TcpRow(address, port, row.OwningPid);
            });
        }

        internal static IReadOnlyList<TcpRow> GetIpv6Rows()
        {
            return ReadTable<NativeTcp6Row>(AfInet6, row =>
            {
                var address = new IPAddress(row.LocalAddr ?? new byte[16]).ToString();
                var port = DecodePort(row.LocalPort);
                return new TcpRow(address, port, row.OwningPid);
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
        out int dstPort) =>
        TryGetTcpEndpoints(packet, datalink, out srcAddress, out srcPort, out dstAddress, out dstPort);
}
