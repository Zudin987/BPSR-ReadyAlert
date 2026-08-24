using System.Collections.Concurrent;
using System.Net;
using ZstdSharp;

namespace BPSR.ReadyAlert;

internal readonly record struct AlertEvent(string Kind, string Title, string Message);

internal sealed class CaptureEngine : IDisposable
{
    private const ulong WorldNtfService = 1_664_308_034UL;
    private const ulong MatchNtfService = 822_849_903UL;
    private const ulong GrpcTeamNtfService = 966_773_353UL;

    private const uint NotifyAllMemberReady = 0x46;
    private const uint NotifyCaptainReady = 0x47;
    private const uint EnterMatchResult = 0x04;

    private const int MaxInitialFrame = 512 * 1024;
    private const int MaxGameFrame = 2 * 1024 * 1024;
    private const int MaxPending = 2 * 1024 * 1024;
    private const int MaxFlows = 2048;

    private readonly ConcurrentQueue<AlertEvent> _events;
    private readonly NpcapCapturePlan _plan;
    private readonly Dictionary<string, FlowState> _flows = new();
    private readonly Dictionary<string, CaptureStats> _stats = new(StringComparer.OrdinalIgnoreCase);
    private readonly HashSet<(string Device, ulong Service, uint Method)> _seenNotifyKeys = new();
    private readonly HashSet<(string Device, int Datalink)> _unsupportedDatalinks = new();
    private readonly Decompressor _zstd = new();
    private Thread? _thread;
    private volatile bool _stopping;
    private DateTime _lastReadyAlertUtc = DateTime.MinValue;
    private DateTime _lastQueueAlertUtc = DateTime.MinValue;
    private DateTime _lastCaptureErrorNoticeUtc = DateTime.MinValue;
    private DateTime _lastCleanupUtc = DateTime.MinValue;
    private DateTime _lastStatsUtc = DateTime.MinValue;

    internal CaptureEngine(ConcurrentQueue<AlertEvent> events, NpcapCapturePlan plan)
    {
        _events = events;
        _plan = plan;
        foreach (var candidate in plan.Candidates)
            _stats[candidate.DeviceName] = new CaptureStats(candidate);
    }

    internal void Start()
    {
        _thread = new Thread(Run)
        {
            IsBackground = true,
            Name = "BPSR-ReadyAlert-Capture"
        };
        _thread.Start();
    }

    private void Run()
    {
        while (!_stopping)
        {
            var captures = new List<OpenedCapture>();
            try
            {
                foreach (var candidate in _plan.Candidates)
                {
                    if (_stopping) break;
                    try
                    {
                        AppLog.Write($"capture: opening Npcap device={candidate.DeviceName} source={candidate.Source}");
                        var capture = new NpcapCapture(candidate.DeviceName);
                        captures.Add(new OpenedCapture(candidate, capture));
                        AppLog.Write($"capture: opened backend=Npcap datalink={capture.DataLink} device={candidate.DeviceName} description={candidate.Description} source={candidate.Source}");
                    }
                    catch (Exception ex)
                    {
                        AppLog.Write($"capture: adapter open failed device={candidate.DeviceName} description={candidate.Description}: {ex.Message}");
                    }
                }

                if (captures.Count == 0)
                {
                    const string error = "Npcap could not open any capture adapter.";
                    AppLog.Write("capture: " + error);
                    NotifyCaptureErrorThrottled(error);
                    SleepWhileRunning(1500);
                    continue;
                }

                AppLog.Write($"capture: started backend=Npcap adapters={captures.Count}");
                _lastStatsUtc = DateTime.UtcNow;

                while (!_stopping && captures.Count > 0)
                {
                    var sawPacket = false;

                    for (var i = captures.Count - 1; i >= 0; i--)
                    {
                        var opened = captures[i];
                        try
                        {
                            if (!opened.Capture.TryRead(out var packet) || packet is null)
                                continue;

                            sawPacket = true;
                            ProcessCapturedPacket(packet, opened.Capture.DataLink, opened.Candidate);
                        }
                        catch (Exception ex)
                        {
                            AppLog.Write($"capture: adapter read failed device={opened.Candidate.DeviceName}: {ex.Message}");
                            opened.Dispose();
                            captures.RemoveAt(i);
                        }
                    }

                    if ((DateTime.UtcNow - _lastStatsUtc).TotalSeconds >= 15)
                    {
                        _lastStatsUtc = DateTime.UtcNow;
                        LogCaptureStats();
                    }

                    if (!sawPacket)
                        Thread.Sleep(1);
                }

                if (!_stopping && captures.Count == 0)
                {
                    NotifyCaptureErrorThrottled("All Npcap capture adapters stopped.");
                    SleepWhileRunning(1000);
                }
            }
            catch (Exception ex)
            {
                if (_stopping) break;
                AppLog.Write("capture: Npcap fatal " + ex);
                NotifyCaptureErrorThrottled(ex.Message);
                SleepWhileRunning(1500);
            }
            finally
            {
                foreach (var opened in captures)
                    opened.Dispose();
                _flows.Clear();
            }
        }

        AppLog.Write("capture: stopped");
    }

    private void LogCaptureStats()
    {
        foreach (var candidate in _plan.Candidates)
        {
            if (!_stats.TryGetValue(candidate.DeviceName, out var stats)) continue;
            AppLog.Write(
                $"capture: stats device={candidate.Description} source={candidate.Source} " +
                $"packets={stats.Packets} tcpPayload={stats.TcpPayloadPackets} " +
                $"gameFrames={stats.GameFrames} notifyFrames={stats.NotifyFrames}");
        }
    }

    private void NotifyCaptureErrorThrottled(string error)
    {
        var now = DateTime.UtcNow;
        if ((now - _lastCaptureErrorNoticeUtc).TotalSeconds < 30) return;
        _lastCaptureErrorNoticeUtc = now;
        _events.Enqueue(new AlertEvent(
            "error",
            "BPSR Ready Alert",
            "Npcap capture is unavailable: " + error));
    }

    private void ProcessCapturedPacket(byte[] packet, int datalink, NpcapCaptureCandidate candidate)
    {
        if (!_stats.TryGetValue(candidate.DeviceName, out var stats))
        {
            stats = new CaptureStats(candidate);
            _stats[candidate.DeviceName] = stats;
        }
        stats.Packets++;

        var offset = datalink switch
        {
            NpcapCapture.DltRaw => 0,
            NpcapCapture.DltIpv4 => 0,
            NpcapCapture.DltIpv6 => 0,
            NpcapCapture.DltNull => 4,
            NpcapCapture.DltLoop => 4,
            NpcapCapture.DltEthernet => TryGetEthernetPayloadOffset(packet, out var ethernetOffset) ? ethernetOffset : -1,
            _ => -1
        };

        if (offset < 0)
        {
            if (_unsupportedDatalinks.Add((candidate.DeviceName, datalink)))
                AppLog.Write($"capture: unsupported Npcap datalink={datalink} device={candidate.Description}");
            return;
        }

        if (offset >= packet.Length) return;
        if (offset == 0)
        {
            ProcessIpPacket(packet, packet.Length, candidate, stats);
            return;
        }

        var ipPacket = new byte[packet.Length - offset];
        Buffer.BlockCopy(packet, offset, ipPacket, 0, ipPacket.Length);
        ProcessIpPacket(ipPacket, ipPacket.Length, candidate, stats);
    }

    private static bool TryGetEthernetPayloadOffset(byte[] packet, out int offset)
    {
        offset = -1;
        if (packet.Length < 14) return false;

        var etherType = ReadU16BE(packet, 12);
        var cursor = 14;
        var vlanDepth = 0;
        while (etherType is 0x8100 or 0x88A8 or 0x9100)
        {
            if (++vlanDepth > 2 || cursor + 4 > packet.Length) return false;
            etherType = ReadU16BE(packet, cursor + 2);
            cursor += 4;
        }

        if (etherType is not (0x0800 or 0x86DD)) return false;
        offset = cursor;
        return true;
    }

    private void ProcessIpPacket(
        byte[] packet,
        int length,
        NpcapCaptureCandidate candidate,
        CaptureStats stats)
    {
        if (!TryLocateTcp(packet, length, out var tcp, out var packetEnd, out var flowPrefix)) return;
        if (packetEnd < tcp + 20) return;

        var tcpHeader = ((packet[tcp + 12] >> 4) & 0x0F) * 4;
        if (tcpHeader < 20 || packetEnd < tcp + tcpHeader) return;

        var payloadOffset = tcp + tcpHeader;
        var payloadLen = packetEnd - payloadOffset;
        var flags = packet[tcp + 13];
        var seq = ReadU32BE(packet, tcp + 4);
        var sourcePort = ReadU16BE(packet, tcp);
        var destPort = ReadU16BE(packet, tcp + 2);
        var key = $"{candidate.DeviceName}|{flowPrefix}:{sourcePort}>{destPort}";

        if (payloadLen > 0) stats.TcpPayloadPackets++;

        if (!_flows.TryGetValue(key, out var flow))
        {
            if (_flows.Count >= MaxFlows) CleanupFlows(aggressive: true);
            flow = new FlowState();
            _flows[key] = flow;
        }

        flow.LastSeenUtc = DateTime.UtcNow;
        if ((flags & 0x02) != 0) flow.Reset(seq + 1);
        if (payloadLen > 0) InsertSegment(flow, seq, packet, payloadOffset, payloadLen, candidate, stats);
        if ((flags & 0x05) != 0) flow.Reset(null);

        if ((DateTime.UtcNow - _lastCleanupUtc).TotalSeconds >= 30)
        {
            _lastCleanupUtc = DateTime.UtcNow;
            CleanupFlows(aggressive: false);
        }
    }

    private static bool TryLocateTcp(byte[] packet, int length, out int tcp, out int packetEnd, out string flowPrefix)
    {
        tcp = 0;
        packetEnd = length;
        flowPrefix = string.Empty;
        if (length < 1) return false;

        var version = packet[0] >> 4;
        if (version == 4)
        {
            if (length < 40) return false;
            var ipHeader = (packet[0] & 0x0F) * 4;
            if (ipHeader < 20 || length < ipHeader + 20 || packet[9] != 6) return false;

            var total = ReadU16BE(packet, 2);
            if (total >= ipHeader && total < packetEnd) packetEnd = total;
            tcp = ipHeader;
            flowPrefix = $"v4:{packet[12]}.{packet[13]}.{packet[14]}.{packet[15]}>" +
                         $"{packet[16]}.{packet[17]}.{packet[18]}.{packet[19]}";
            return true;
        }

        if (version != 6 || length < 60) return false;

        var payloadLength = ReadU16BE(packet, 4);
        if (payloadLength != 0)
        {
            var total = 40 + payloadLength;
            if (total < packetEnd) packetEnd = total;
        }

        byte next = packet[6];
        var cursor = 40;
        while (next != 6)
        {
            if (cursor + 2 > packetEnd) return false;
            switch (next)
            {
                case 0:
                case 43:
                case 60:
                {
                    next = packet[cursor];
                    var extLength = (packet[cursor + 1] + 1) * 8;
                    if (extLength < 8 || cursor + extLength > packetEnd) return false;
                    cursor += extLength;
                    break;
                }
                case 44:
                {
                    if (cursor + 8 > packetEnd) return false;
                    next = packet[cursor];
                    var fragmentOffset = (ushort)(ReadU16BE(packet, cursor + 2) & 0xFFF8);
                    if (fragmentOffset != 0) return false;
                    cursor += 8;
                    break;
                }
                case 51:
                {
                    next = packet[cursor];
                    var extLength = (packet[cursor + 1] + 2) * 4;
                    if (extLength < 8 || cursor + extLength > packetEnd) return false;
                    cursor += extLength;
                    break;
                }
                default:
                    return false;
            }
        }

        if (cursor + 20 > packetEnd) return false;
        tcp = cursor;
        var source = new IPAddress(packet.AsSpan(8, 16)).ToString();
        var dest = new IPAddress(packet.AsSpan(24, 16)).ToString();
        flowPrefix = $"v6:[{source}]>[{dest}]";
        return true;
    }

    private void CleanupFlows(bool aggressive)
    {
        var cutoff = DateTime.UtcNow.AddSeconds(aggressive ? -10 : -90);
        var dead = _flows.Where(kv => kv.Value.LastSeenUtc < cutoff).Select(kv => kv.Key).ToArray();
        foreach (var key in dead) _flows.Remove(key);

        if (aggressive && _flows.Count >= MaxFlows)
        {
            var oldest = _flows.OrderBy(kv => kv.Value.LastSeenUtc).FirstOrDefault();
            if (!string.IsNullOrEmpty(oldest.Key)) _flows.Remove(oldest.Key);
        }
    }

    private static bool SeqBefore(uint a, uint b) => unchecked((int)(a - b)) < 0;

    private void InsertSegment(
        FlowState flow,
        uint seq,
        byte[] packet,
        int offset,
        int len,
        NpcapCaptureCandidate candidate,
        CaptureStats stats)
    {
        if (!flow.HasNext)
        {
            flow.HasNext = true;
            flow.NextSeq = seq;
        }

        if (SeqBefore(seq, flow.NextSeq))
        {
            var overlap = flow.NextSeq - seq;
            if (overlap >= (uint)len) return;
            offset += (int)overlap;
            len -= (int)overlap;
            seq = flow.NextSeq;
        }

        if (seq == flow.NextSeq)
        {
            AppendStream(flow, packet, offset, len, candidate, stats);
            flow.NextSeq += (uint)len;
            DrainPending(flow, candidate, stats);
            return;
        }

        if (!flow.Pending.ContainsKey(seq))
        {
            var copy = new byte[len];
            Buffer.BlockCopy(packet, offset, copy, 0, len);
            flow.Pending[seq] = copy;
            flow.PendingBytes += len;
        }

        if (flow.PendingBytes <= MaxPending) return;
        flow.Reset(null);
    }

    private void DrainPending(FlowState flow, NpcapCaptureCandidate candidate, CaptureStats stats)
    {
        while (true)
        {
            if (flow.Pending.Remove(flow.NextSeq, out var exact))
            {
                flow.PendingBytes -= exact.Length;
                AppendStream(flow, exact, 0, exact.Length, candidate, stats);
                flow.NextSeq += (uint)exact.Length;
                continue;
            }

            if (flow.Pending.Count == 0) return;
            var first = flow.Pending.First();
            if (!SeqBefore(first.Key, flow.NextSeq)) return;

            var overlap = flow.NextSeq - first.Key;
            flow.Pending.Remove(first.Key);
            flow.PendingBytes -= first.Value.Length;
            if (overlap >= (uint)first.Value.Length) continue;

            var trim = (int)overlap;
            AppendStream(flow, first.Value, trim, first.Value.Length - trim, candidate, stats);
            flow.NextSeq += (uint)(first.Value.Length - trim);
        }
    }

    private void AppendStream(
        FlowState flow,
        byte[] data,
        int offset,
        int len,
        NpcapCaptureCandidate candidate,
        CaptureStats stats)
    {
        for (var i = 0; i < len; i++) flow.Stream.Add(data[offset + i]);
        ProcessFrames(flow, candidate, stats);

        var cap = flow.LooksLikeGame ? MaxGameFrame * 2 : MaxInitialFrame * 2;
        if (flow.Stream.Count <= cap) return;
        flow.Stream.Clear();
        flow.LooksLikeGame = false;
    }

    private void ProcessFrames(FlowState flow, NpcapCaptureCandidate candidate, CaptureStats stats)
    {
        while (flow.Stream.Count >= 6)
        {
            var size = ((uint)flow.Stream[0] << 24) |
                       ((uint)flow.Stream[1] << 16) |
                       ((uint)flow.Stream[2] << 8) |
                       flow.Stream[3];
            var packetType = (ushort)(((uint)flow.Stream[4] << 8) | flow.Stream[5]);
            var fragment = packetType & 0x7FFF;
            var max = flow.LooksLikeGame ? MaxGameFrame : MaxInitialFrame;

            if (size < 6 || size > max || fragment is not (1 or 2 or 5 or 6))
            {
                flow.Stream.RemoveAt(0);
                continue;
            }

            if (flow.Stream.Count < (int)size) return;
            var frame = flow.Stream.GetRange(0, (int)size).ToArray();
            flow.Stream.RemoveRange(0, (int)size);
            flow.LooksLikeGame = true;
            stats.GameFrames++;
            ProcessFragment(frame, 0, frame.Length, depth: 0, candidate, stats);
        }
    }

    private void ProcessFragment(
        byte[] frame,
        int start,
        int end,
        int depth,
        NpcapCaptureCandidate candidate,
        CaptureStats stats)
    {
        if (depth > 3 || end - start < 6) return;

        var cursor = start;
        while (cursor + 6 <= end)
        {
            var packetSize = ReadU32BE(frame, cursor);
            if (packetSize < 6 || cursor + packetSize > end) return;

            var typeRaw = ReadU16BE(frame, cursor + 4);
            var compressed = (typeRaw & 0x8000) != 0;
            var fragment = typeRaw & 0x7FFF;
            var payloadStart = cursor + 6;
            var payloadEnd = cursor + checked((int)packetSize);

            if (fragment == 2)
            {
                ProcessNotify(frame, payloadStart, payloadEnd, compressed, candidate, stats);
            }
            else if (fragment is 5 or 6 && payloadEnd - payloadStart >= 4)
            {
                var nestedStart = payloadStart + 4;
                if (compressed)
                {
                    try
                    {
                        var zipped = frame.AsSpan(nestedStart, payloadEnd - nestedStart).ToArray();
                        var nested = _zstd.Unwrap(zipped).ToArray();
                        ProcessFragment(nested, 0, nested.Length, depth + 1, candidate, stats);
                    }
                    catch (Exception ex)
                    {
                        AppLog.Write("packet: nested zstd decompression failed " + ex.Message);
                    }
                }
                else
                {
                    ProcessFragment(frame, nestedStart, payloadEnd, depth + 1, candidate, stats);
                }
            }

            cursor = payloadEnd;
        }
    }

    private void ProcessNotify(
        byte[] frame,
        int start,
        int end,
        bool compressed,
        NpcapCaptureCandidate candidate,
        CaptureStats stats)
    {
        if (end - start < 16) return;
        stats.NotifyFrames++;

        var service = ReadU64BE(frame, start);
        var method = ReadU32BE(frame, start + 12);
        var protoStart = start + 16;
        var protoLength = end - protoStart;

        if (_seenNotifyKeys.Add((candidate.DeviceName, service, method)))
            AppLog.Write($"probe: notify device={candidate.Description} service={service} method=0x{method:X} compressed={compressed} protoLen={protoLength}");

        if (service == WorldNtfService && method is NotifyAllMemberReady or NotifyCaptainReady)
        {
            AppLog.Write($"event: ready-check notify device={candidate.Description} method=0x{method:X} compressed={compressed}");
            var now = DateTime.UtcNow;
            if ((now - _lastReadyAlertUtc).TotalSeconds >= 3)
            {
                _lastReadyAlertUtc = now;
                _events.Enqueue(new AlertEvent("ready", "BPSR Ready Check", "Party Ready Check started."));
                AppLog.Write("alert: enqueued kind=ready");
            }
            return;
        }

        if (service == GrpcTeamNtfService && method is 0x12 or 0x13 or 0x14 or 0x1F)
            AppLog.Write($"probe: GrpcTeamNtf matchmaking-related device={candidate.Description} method=0x{method:X} compressed={compressed} protoLen={protoLength}");

        if (service != MatchNtfService) return;

        AppLog.Write($"probe: MatchNtf device={candidate.Description} method=0x{method:X} compressed={compressed} protoLen={protoLength}");
        if (method != EnterMatchResult) return;

        byte[] payload;
        try
        {
            var raw = frame.AsSpan(protoStart, protoLength).ToArray();
            payload = compressed ? _zstd.Unwrap(raw).ToArray() : raw;
        }
        catch (Exception ex)
        {
            AppLog.Write("event: match EnterMatchResult decompression failed " + ex.Message);
            return;
        }

        if (!TryParseMatchStatus(payload, 0, payload.Length, out var status))
        {
            AppLog.Write($"event: match EnterMatchResult payload could not be parsed len={payload.Length}");
            return;
        }

        AppLog.Write($"event: match EnterMatchResult device={candidate.Description} status={status} compressed={compressed}");
        if (status != 2) return;

        var alertNow = DateTime.UtcNow;
        if ((alertNow - _lastQueueAlertUtc).TotalSeconds < 5) return;
        _lastQueueAlertUtc = alertNow;
        _events.Enqueue(new AlertEvent("queue", "BPSR Match Found", "Matchmaking is waiting for acceptance."));
        AppLog.Write("alert: enqueued kind=queue");
    }

    private static bool TryParseMatchStatus(byte[] data, int offset, int length, out int status)
    {
        status = -1;
        if (!TryGetLengthField(data, offset, length, 1, out var requestOffset, out var requestLength)) return false;
        if (!TryGetLengthField(data, requestOffset, requestLength, 2, out var infoOffset, out var infoLength)) return false;
        if (!TryGetVarintField(data, infoOffset, infoLength, 2, out var value)) return false;
        status = checked((int)value);
        return true;
    }

    private static bool TryGetLengthField(byte[] data, int offset, int length, int wantedField, out int valueOffset, out int valueLength)
    {
        valueOffset = 0;
        valueLength = 0;
        var p = offset;
        var limit = offset + length;

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

    private static uint ReadU32BE(byte[] data, int offset) =>
        ((uint)data[offset] << 24) |
        ((uint)data[offset + 1] << 16) |
        ((uint)data[offset + 2] << 8) |
        data[offset + 3];

    private static ushort ReadU16BE(byte[] data, int offset) =>
        (ushort)(((uint)data[offset] << 8) | data[offset + 1]);

    private static ulong ReadU64BE(byte[] data, int offset) =>
        ((ulong)data[offset] << 56) |
        ((ulong)data[offset + 1] << 48) |
        ((ulong)data[offset + 2] << 40) |
        ((ulong)data[offset + 3] << 32) |
        ((ulong)data[offset + 4] << 24) |
        ((ulong)data[offset + 5] << 16) |
        ((ulong)data[offset + 6] << 8) |
        data[offset + 7];

    private void SleepWhileRunning(int milliseconds)
    {
        for (var elapsed = 0; elapsed < milliseconds && !_stopping; elapsed += 100)
            Thread.Sleep(Math.Min(100, milliseconds - elapsed));
    }

    public void Dispose()
    {
        _stopping = true;
        if (_thread is { IsAlive: true }) _thread.Join(3000);
        _zstd.Dispose();
    }

    private sealed class OpenedCapture : IDisposable
    {
        internal NpcapCaptureCandidate Candidate { get; }
        internal NpcapCapture Capture { get; }

        internal OpenedCapture(NpcapCaptureCandidate candidate, NpcapCapture capture)
        {
            Candidate = candidate;
            Capture = capture;
        }

        public void Dispose() => Capture.Dispose();
    }

    private sealed class CaptureStats
    {
        internal NpcapCaptureCandidate Candidate { get; }
        internal long Packets;
        internal long TcpPayloadPackets;
        internal long GameFrames;
        internal long NotifyFrames;

        internal CaptureStats(NpcapCaptureCandidate candidate) => Candidate = candidate;
    }

    private sealed class FlowState
    {
        internal bool HasNext;
        internal uint NextSeq;
        internal SortedDictionary<uint, byte[]> Pending { get; } = new();
        internal int PendingBytes;
        internal List<byte> Stream { get; } = new(8192);
        internal bool LooksLikeGame;
        internal DateTime LastSeenUtc = DateTime.UtcNow;

        internal void Reset(uint? next)
        {
            Pending.Clear();
            PendingBytes = 0;
            Stream.Clear();
            LooksLikeGame = false;
            if (next.HasValue)
            {
                HasNext = true;
                NextSeq = next.Value;
            }
            else
            {
                HasNext = false;
                NextSeq = 0;
            }
        }
    }
}
