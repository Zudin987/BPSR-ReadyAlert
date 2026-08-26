using System.Collections.Concurrent;
using System.Net;
using ZstdSharp;

namespace BPSR.ReadyAlert;

/// <summary>
/// Optional chat-only capture pipeline. It intentionally uses its own Npcap handle only while
/// Chat Overlay is enabled so the stable Ready/Queue capture engine remains untouched.
/// Packets are still filtered by NpcapCapture/GamePacketFilter to BPSR-owned TCP endpoints.
/// </summary>
internal sealed class ChatCaptureEngine : IDisposable
{
    private const int MaxInitialFrame = 512 * 1024;
    private const int MaxGameFrame = 2 * 1024 * 1024;
    private const int MaxPending = 2 * 1024 * 1024;
    private const int MaxFlows = 2048;
    private const int MaxFrameDepth = 4;

    private readonly ConcurrentQueue<ChatMessageEvent> _events;
    private readonly NpcapCaptureCandidate _candidate;
    private readonly Dictionary<string, FlowState> _flows = new();
    private readonly Decompressor _zstd = new();
    private Thread? _thread;
    private volatile bool _stopping;
    private DateTime _lastCleanupUtc = DateTime.MinValue;
    private bool _loggedParseFailure;

    internal ChatCaptureEngine(ConcurrentQueue<ChatMessageEvent> events, NpcapCapturePlan plan)
    {
        _events = events;
        _candidate = plan.Primary;
    }

    internal void Start()
    {
        if (_thread is { IsAlive: true }) return;
        _thread = new Thread(Run)
        {
            IsBackground = true,
            Name = "BPSR-ReadyAlert-ChatCapture"
        };
        _thread.Start();
    }

    private void Run()
    {
        while (!_stopping)
        {
            try
            {
                AppLog.Write($"chat-capture: opening Npcap device={_candidate.DeviceName} source={_candidate.Source}");
                using var capture = new NpcapCapture(_candidate.DeviceName);
                AppLog.Write($"chat-capture: started datalink={capture.DataLink} device={_candidate.Description}");

                while (!_stopping)
                {
                    if (!capture.TryRead(out var packet) || packet is null)
                    {
                        Thread.Sleep(1);
                        continue;
                    }

                    ProcessCapturedPacket(packet, capture.DataLink);
                }
            }
            catch (Exception ex)
            {
                if (_stopping) break;
                AppLog.Write("chat-capture: failure " + ex.Message);
                SleepWhileRunning(1200);
            }
            finally
            {
                _flows.Clear();
            }
        }

        AppLog.Write("chat-capture: stopped");
    }

    private void ProcessCapturedPacket(byte[] packet, int datalink)
    {
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

        if (offset < 0 || offset >= packet.Length) return;
        if (offset == 0)
        {
            ProcessIpPacket(packet, packet.Length);
            return;
        }

        var ipPacket = new byte[packet.Length - offset];
        Buffer.BlockCopy(packet, offset, ipPacket, 0, ipPacket.Length);
        ProcessIpPacket(ipPacket, ipPacket.Length);
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

    private void ProcessIpPacket(byte[] packet, int length)
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
        var key = $"{flowPrefix}:{sourcePort}>{destPort}";

        if (!_flows.TryGetValue(key, out var flow))
        {
            if (_flows.Count >= MaxFlows) CleanupFlows(aggressive: true);
            flow = new FlowState();
            _flows[key] = flow;
        }

        flow.LastSeenUtc = DateTime.UtcNow;
        if ((flags & 0x02) != 0) flow.Reset(seq + 1);
        if (payloadLen > 0) InsertSegment(flow, seq, packet, payloadOffset, payloadLen);
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

    private void InsertSegment(FlowState flow, uint seq, byte[] packet, int offset, int len)
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
            AppendStream(flow, packet, offset, len);
            flow.NextSeq += (uint)len;
            DrainPending(flow);
            return;
        }

        if (!flow.Pending.ContainsKey(seq))
        {
            var copy = new byte[len];
            Buffer.BlockCopy(packet, offset, copy, 0, len);
            flow.Pending[seq] = copy;
            flow.PendingBytes += len;
        }

        if (flow.PendingBytes > MaxPending)
            flow.Reset(null);
    }

    private void DrainPending(FlowState flow)
    {
        while (true)
        {
            if (flow.Pending.Remove(flow.NextSeq, out var exact))
            {
                flow.PendingBytes -= exact.Length;
                AppendStream(flow, exact, 0, exact.Length);
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
            AppendStream(flow, first.Value, trim, first.Value.Length - trim);
            flow.NextSeq += (uint)(first.Value.Length - trim);
        }
    }

    private void AppendStream(FlowState flow, byte[] data, int offset, int len)
    {
        for (var i = 0; i < len; i++) flow.Stream.Add(data[offset + i]);
        ProcessFrames(flow);

        var cap = flow.LooksLikeGame ? MaxGameFrame * 2 : MaxInitialFrame * 2;
        if (flow.Stream.Count <= cap) return;
        flow.Stream.Clear();
        flow.LooksLikeGame = false;
    }

    private void ProcessFrames(FlowState flow)
    {
        while (flow.Stream.Count >= 6)
        {
            var size = ((uint)flow.Stream[0] << 24) |
                       ((uint)flow.Stream[1] << 16) |
                       ((uint)flow.Stream[2] << 8) |
                       flow.Stream[3];
            var typeRaw = (ushort)(((uint)flow.Stream[4] << 8) | flow.Stream[5]);
            var messageType = typeRaw & 0x7FFF;
            var max = flow.LooksLikeGame ? MaxGameFrame : MaxInitialFrame;

            if (size < 6 || size > max || messageType > 8)
            {
                flow.Stream.RemoveAt(0);
                continue;
            }

            if (flow.Stream.Count < (int)size) return;

            var frame = flow.Stream.GetRange(0, checked((int)size)).ToArray();
            flow.Stream.RemoveRange(0, checked((int)size));
            flow.LooksLikeGame = true;
            ProcessGameMessages(frame, 0, frame.Length, 0);
        }
    }

    private void ProcessGameMessages(byte[] data, int start, int end, int depth)
    {
        if (depth > MaxFrameDepth || end - start < 6) return;

        var cursor = start;
        while (cursor + 6 <= end)
        {
            var packetSize = ReadU32BE(data, cursor);
            if (packetSize < 6 || packetSize > MaxGameFrame || cursor + packetSize > end)
                return;

            var typeRaw = ReadU16BE(data, cursor + 4);
            var compressed = (typeRaw & 0x8000) != 0;
            var messageType = typeRaw & 0x7FFF;
            if (messageType > 8) return;

            var payloadStart = cursor + 6;
            var payloadEnd = cursor + checked((int)packetSize);

            switch (messageType)
            {
                case 2:
                    ProcessNotify(data, payloadStart, payloadEnd, compressed);
                    break;
                case 6:
                    ProcessFrameDown(data, payloadStart, payloadEnd, compressed, depth);
                    break;
            }

            cursor = payloadEnd;
        }
    }

    private void ProcessFrameDown(byte[] data, int payloadStart, int payloadEnd, bool compressed, int depth)
    {
        if (payloadEnd - payloadStart < 4) return;
        var nestedStart = payloadStart + 4;

        if (!compressed)
        {
            ProcessGameMessages(data, nestedStart, payloadEnd, depth + 1);
            return;
        }

        try
        {
            var zipped = data.AsSpan(nestedStart, payloadEnd - nestedStart).ToArray();
            var nested = _zstd.Unwrap(zipped).ToArray();
            ProcessGameMessages(nested, 0, nested.Length, depth + 1);
        }
        catch (Exception ex)
        {
            AppLog.Write("chat-capture: FrameDown zstd failed " + ex.Message);
        }
    }

    private void ProcessNotify(byte[] frame, int start, int end, bool compressed)
    {
        if (end - start < 16) return;

        var service = ReadU64BE(frame, start);
        var method = ReadU32BE(frame, start + 12);
        if (service != ChatProtocol.ServiceId || method != ChatProtocol.NotifyNewestChitChatMsgs)
            return;

        var protoStart = start + 16;
        var protoLength = end - protoStart;
        byte[] payload;
        try
        {
            var raw = frame.AsSpan(protoStart, protoLength).ToArray();
            payload = compressed ? _zstd.Unwrap(raw).ToArray() : raw;
        }
        catch (Exception ex)
        {
            AppLog.Write("chat-capture: notify zstd failed " + ex.Message);
            return;
        }

        if (ChatProtocol.TryParseNotify(payload, out var message))
        {
            _events.Enqueue(message);
            return;
        }

        if (_loggedParseFailure) return;
        _loggedParseFailure = true;
        AppLog.Write($"chat-capture: first ChitChatNtf parse failure protoLen={payload.Length}");
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
