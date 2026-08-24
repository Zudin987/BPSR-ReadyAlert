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

    private static readonly object Sync = new();
    private static HashSet<EndpointKey> _localEndpoints = new();
    private static DateTime _lastRefreshUtc = DateTime.MinValue;
    private static string _lastSummary = string.Empty;

    internal static bool IsBpsrPacket(byte[] packet, int datalink)
    {
        if (!TryGetIpv4TcpEndpoints(packet, datalink, out var srcAddress, out var srcPort, out var dstAddress, out var dstPort))
            return false;

        RefreshIfNeeded();

        lock (Sync)
        {
            return _localEndpoints.Contains(new EndpointKey(srcAddress, srcPort)) ||
                   _localEndpoints.Contains(new EndpointKey(dstAddress, dstPort));
        }
    }

    private static void RefreshIfNeeded()
    {
        var now = DateTime.UtcNow;
        if ((now - _lastRefreshUtc).TotalMilliseconds < 750) return;

        lock (Sync)
        {
            now = DateTime.UtcNow;
            if ((now - _lastRefreshUtc).TotalMilliseconds < 750) return;
            _lastRefreshUtc = now;

            var pids = new HashSet<int>();
            var foundNames = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
            try
            {
                foreach (var process in Process.GetProcesses())
                {
                    try
                    {
                        if (!GameProcessNames.Contains(process.ProcessName, StringComparer.OrdinalIgnoreCase))
                            continue;
                        pids.Add(process.Id);
                        foundNames.Add(process.ProcessName);
                    }
                    catch { }
                    finally { process.Dispose(); }
                }
            }
            catch (Exception ex)
            {
                AppLog.Write("game-filter: process enumeration failed " + ex.Message);
            }

            var endpoints = new HashSet<EndpointKey>();
            if (pids.Count > 0)
            {
                try
                {
                    foreach (var row in TcpOwnerTable.GetIpv4Rows())
                    {
                        if (!pids.Contains(row.OwningPid)) continue;
                        if (row.LocalPort <= 0) continue;
                        endpoints.Add(new EndpointKey(row.LocalAddress, row.LocalPort));
                    }
                }
                catch (Exception ex)
                {
                    AppLog.Write("game-filter: TCP owner table failed " + ex.Message);
                }
            }

            _localEndpoints = endpoints;
            var summary = $"pids={string.Join(',', pids.OrderBy(x => x))} names={string.Join(',', foundNames.OrderBy(x => x, StringComparer.OrdinalIgnoreCase))} endpoints={endpoints.Count}";
            if (!string.Equals(summary, _lastSummary, StringComparison.Ordinal))
            {
                _lastSummary = summary;
                AppLog.Write("game-filter: " + summary);
                foreach (var endpoint in endpoints.OrderBy(x => x.Address, StringComparer.Ordinal).ThenBy(x => x.Port))
                    AppLog.Write($"game-filter: local={endpoint.Address}:{endpoint.Port}");
            }
        }
    }

    private static bool TryGetIpv4TcpEndpoints(
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

        var offset = datalink switch
        {
            NpcapCapture.DltRaw => 0,
            NpcapCapture.DltIpv4 => 0,
            NpcapCapture.DltNull => 4,
            NpcapCapture.DltLoop => 4,
            NpcapCapture.DltEthernet => GetEthernetIpv4Offset(packet),
            _ => -1
        };

        if (offset < 0 || offset + 20 > packet.Length) return false;
        if ((packet[offset] >> 4) != 4) return false;

        var ipHeader = (packet[offset] & 0x0F) * 4;
        if (ipHeader < 20 || offset + ipHeader + 20 > packet.Length) return false;
        if (packet[offset + 9] != 6) return false;

        var tcp = offset + ipHeader;
        srcPort = ReadU16BE(packet, tcp);
        dstPort = ReadU16BE(packet, tcp + 2);
        if (srcPort <= 1000 || dstPort <= 1000) return false;

        srcAddress = $"{packet[offset + 12]}.{packet[offset + 13]}.{packet[offset + 14]}.{packet[offset + 15]}";
        dstAddress = $"{packet[offset + 16]}.{packet[offset + 17]}.{packet[offset + 18]}.{packet[offset + 19]}";
        return true;
    }

    private static int GetEthernetIpv4Offset(byte[] packet)
    {
        if (packet.Length < 14) return -1;
        var etherType = ReadU16BE(packet, 12);
        var cursor = 14;
        var vlanDepth = 0;
        while (etherType is 0x8100 or 0x88A8 or 0x9100)
        {
            if (++vlanDepth > 2 || cursor + 4 > packet.Length) return -1;
            etherType = ReadU16BE(packet, cursor + 2);
            cursor += 4;
        }
        return etherType == 0x0800 ? cursor : -1;
    }

    private static ushort ReadU16BE(byte[] data, int offset) =>
        (ushort)(((uint)data[offset] << 8) | data[offset + 1]);

    private readonly record struct EndpointKey(string Address, int Port);

    private static class TcpOwnerTable
    {
        private const int AfInet = 2;
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

        internal readonly record struct TcpRow(string LocalAddress, int LocalPort, int OwningPid);

        internal static IReadOnlyList<TcpRow> GetIpv4Rows()
        {
            var size = 0;
            var first = GetExtendedTcpTable(IntPtr.Zero, ref size, false, AfInet, TcpTableOwnerPidAll, 0);
            if (first != ErrorInsufficientBuffer && first != 0)
                throw new InvalidOperationException("GetExtendedTcpTable(size) failed: " + first);

            var buffer = Marshal.AllocHGlobal(size);
            try
            {
                var result = GetExtendedTcpTable(buffer, ref size, false, AfInet, TcpTableOwnerPidAll, 0);
                if (result != 0)
                    throw new InvalidOperationException("GetExtendedTcpTable(data) failed: " + result);

                var count = Marshal.ReadInt32(buffer);
                var rowSize = Marshal.SizeOf<NativeTcpRow>();
                var cursor = IntPtr.Add(buffer, sizeof(uint));
                var rows = new List<TcpRow>(Math.Max(0, count));
                for (var i = 0; i < count; i++)
                {
                    var row = Marshal.PtrToStructure<NativeTcpRow>(cursor);
                    cursor = IntPtr.Add(cursor, rowSize);

                    var address = new IPAddress(row.LocalAddr).ToString();
                    var port = (ushort)IPAddress.NetworkToHostOrder((short)(row.LocalPort & 0xFFFF));
                    rows.Add(new TcpRow(address, port, row.OwningPid));
                }
                return rows;
            }
            finally
            {
                Marshal.FreeHGlobal(buffer);
            }
        }
    }
}
