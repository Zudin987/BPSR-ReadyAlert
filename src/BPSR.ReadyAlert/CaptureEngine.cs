using System.Collections.Concurrent;
using System.Net.NetworkInformation;
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
    private const uint NotifyTeamActivityState = 0x0E;

    private const int TeamActivityVoting = 3;
    private const int MatchStatusWaitReady = 2;

    private const int MaxInitialFrame = 512 * 1024;
    private const int MaxGameFrame = 2 * 1024 * 1024;
    private const int MaxPending = 2 * 1024 * 1024;
    private const int MaxFlows = 2048;
    private const int MaxFrameDepth = 4;

    private readonly ConcurrentQueue<AlertEvent> _events;
    private NpcapCapturePlan _plan;
    private readonly Dictionary<FlowKey, FlowState> _flows = new();
    private readonly Dictionary<string, CaptureStats> _stats = new(StringComparer.OrdinalIgnoreCase);
    private readonly HashSet<(string Device, ulong Service, uint Method)> _seenNotifyKeys = new();
    private readonly HashSet<(string Device, int Datalink)> _unsupportedDatalinks = new();
    private readonly Decompressor _zstd = new();
    private readonly AutoResetEvent _wake = new(false);

    private Thread? _thread;
    private volatile bool _stopping;
    private int _networkChangePending;
    private bool _gameWasRunning;
    private int _recoverySequence;
    private DateTime _lastReadyAlertUtc = DateTime.MinValue;
    private DateTime _lastQueueAlertUtc = DateTime.MinValue;
    private DateTime _lastCaptureErrorNoticeUtc = DateTime.MinValue;
    private DateTime _lastCleanupUtc = DateTime.MinValue;
    private DateTime _lastStatsUtc = DateTime.MinValue;
    private DateTime _lastWatchdogUtc = DateTime.MinValue;
    private DateTime _captureOpenedUtc = DateTime.MinValue;
    private DateTime _lastBpsrPacketUtc = DateTime.MinValue;
    private DateTime _lastValidFrameUtc = DateTime.MinValue;

    internal CaptureEngine(ConcurrentQueue<AlertEvent> events, NpcapCapturePlan plan)
    {
        _events = events;
        _plan = plan;
        EnsureStatsForPlan(plan);
        NetworkChange.NetworkAddressChanged += OnNetworkAddressChanged;
        NetworkChange.NetworkAvailabilityChanged += OnNetworkAvailabilityChanged;
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
        var consecutiveFailures = 0;
        var refreshPlan = string.IsNullOrWhiteSpace(_plan.Primary.DeviceName);
        var refreshReason = refreshPlan ? "startup-waiting-plan" : string.Empty;

        while (!_stopping)
        {
            if (refreshPlan)
            {
                if (!TryRefreshPlan(refreshReason))
                {
                    consecutiveFailures++;
                    NotifyCaptureErrorThrottled("Waiting for a usable Npcap adapter.");
                    WaitForRetry(CaptureRecoveryPolicy.RetryDelayMs(consecutiveFailures));
                    continue;
                }
                refreshPlan = false;
                refreshReason = string.Empty;
            }

            var captures = new List<OpenedCapture>();
            var rebuildReason = string.Empty;
            try
            {
                foreach (var candidate in _plan.Candidates)
                {
                    if (_stopping) break;
                    if (string.IsNullOrWhiteSpace(candidate.DeviceName)) continue;
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
                    const string error = "Npcap could not open the selected capture adapter.";
                    AppLog.Write("capture: " + error);
                    NotifyCaptureErrorThrottled(error);
                    consecutiveFailures++;
                    refreshPlan = true;
                    refreshReason = "open-failure";
                    WaitForRetry(CaptureRecoveryPolicy.RetryDelayMs(consecutiveFailures));
                    continue;
                }

                consecutiveFailures = 0;
                _captureOpenedUtc = DateTime.UtcNow;
                _lastBpsrPacketUtc = DateTime.MinValue;
                _lastValidFrameUtc = DateTime.MinValue;
                _lastWatchdogUtc = DateTime.MinValue;
                _lastStatsUtc = DateTime.UtcNow;
                AppLog.Write($"capture: started backend=Npcap adapters={captures.Count} parser=ZDPS-compatible recovery=self-healing");

                while (!_stopping && captures.Count > 0)
                {
                    if (ConsumeNetworkChange())
                    {
                        rebuildReason = "network-change";
                        refreshPlan = true;
                        refreshReason = rebuildReason;
                        AppLog.Write("capture-recovery: Windows network change detected; rebuilding capture");
                        break;
                    }

                    var sawPacket = false;

                    for (var i = captures.Count - 1; i >= 0; i--)
                    {
                        var opened = captures[i];
                        try
                        {
                            if (!opened.Capture.TryRead(out var packet) || packet is null)
                                continue;

                            sawPacket = true;
                            _lastBpsrPacketUtc = DateTime.UtcNow;
                            ProcessCapturedPacket(packet, opened.Capture.DataLink, opened.Candidate);
                        }
                        catch (Exception ex)
                        {
                            AppLog.Write($"capture: adapter read failed device={opened.Candidate.DeviceName}: {ex.Message}");
                            opened.Dispose();
                            captures.RemoveAt(i);
                        }
                    }

                    var now = DateTime.UtcNow;
                    if ((now - _lastWatchdogUtc).TotalSeconds >= 1)
                    {
                        _lastWatchdogUtc = now;
                        var gameRunning = BpsrProcessProbe.IsRunning();
                        if (_gameWasRunning && !gameRunning)
                        {
                            _flows.Clear();
                            _lastBpsrPacketUtc = DateTime.MinValue;
                            _lastValidFrameUtc = DateTime.MinValue;
                            PlayerIdentityCaptureBridge.ClearCurrent();
                            AppLog.Write("capture-recovery: BPSR process exited; cleared transient session state");
                        }
                        _gameWasRunning = gameRunning;

                        if (CaptureRecoveryPolicy.ShouldRestartSilentCapture(
                                gameRunning,
                                now,
                                _captureOpenedUtc,
                                _lastBpsrPacketUtc))
                        {
                            rebuildReason = "silent-watchdog";
                            refreshPlan = true;
                            refreshReason = rebuildReason;
                            AppLog.Write("capture-recovery: BPSR is running but no captured game packets arrived within watchdog window; rebuilding capture");
                            break;
                        }

                        if (CaptureRecoveryPolicy.ShouldResetProtocolFlows(
                                gameRunning,
                                now,
                                _lastBpsrPacketUtc,
                                _lastValidFrameUtc))
                        {
                            _flows.Clear();
                            _lastValidFrameUtc = now;
                            AppLog.Write("capture-recovery: BPSR packets are arriving but protocol frames stalled; reset TCP reassembly state");
                        }
                    }

                    if ((now - _lastStatsUtc).TotalSeconds >= 15)
                    {
                        _lastStatsUtc = now;
                        LogCaptureStats();
                    }

                    if (!sawPacket)
                        _wake.WaitOne(1);
                }

                if (!_stopping && captures.Count == 0)
                {
                    rebuildReason = "read-failure";
                    refreshPlan = true;
                    refreshReason = rebuildReason;
                    NotifyCaptureErrorThrottled("All Npcap capture handles stopped. ReadyAlert is recovering automatically.");
                }
            }
            catch (Exception ex)
            {
                if (_stopping) break;
                rebuildReason = "capture-fatal";
                refreshPlan = true;
                refreshReason = rebuildReason;
                AppLog.Write("capture: Npcap fatal " + ex);
                NotifyCaptureErrorThrottled(ex.Message);
            }
            finally
            {
                foreach (var opened in captures)
                    opened.Dispose();
                _flows.Clear();
            }

            if (_stopping) break;

            if (string.Equals(rebuildReason, "network-change", StringComparison.Ordinal))
            {
                WaitForRetry(CaptureRecoveryPolicy.NetworkChangeSettleMs);
                continue;
            }

            if (!string.IsNullOrEmpty(rebuildReason))
            {
                consecutiveFailures++;
                WaitForRetry(CaptureRecoveryPolicy.RetryDelayMs(consecutiveFailures));
            }
        }

        AppLog.Write("capture: stopped");
    }

    private bool TryRefreshPlan(string reason)
    {
        try
        {
            var previous = _plan;
            var refreshed = CaptureRecoveryPlanner.Refresh(previous);
            var changed = !string.Equals(
                previous.Primary.DeviceName,
                refreshed.Primary.DeviceName,
                StringComparison.OrdinalIgnoreCase);

            _plan = refreshed;
            EnsureStatsForPlan(refreshed);
            var sequence = Interlocked.Increment(ref _recoverySequence);
            AppLog.Write(
                $"capture-recovery: plan refreshed seq={sequence} reason={reason} " +
                $"device={refreshed.Primary.DeviceName} description={refreshed.Primary.Description} " +
                $"source={refreshed.Primary.Source} changed={changed}");

            if (changed && !string.IsNullOrWhiteSpace(previous.Primary.DeviceName))
            {
                _events.Enqueue(new AlertEvent(
                    "capture-recovered",
                    "BPSR Ready Alert",
                    "Network changed. ReadyAlert moved capture to " + refreshed.Primary.Description + "."));
            }
            return true;
        }
        catch (Exception ex)
        {
            AppLog.Write($"capture-recovery: plan refresh failed reason={reason}: {ex.Message}");
            return false;
        }
    }

    private void EnsureStatsForPlan(NpcapCapturePlan plan)
    {
        foreach (var candidate in plan.Candidates)
        {
            if (string.IsNullOrWhiteSpace(candidate.DeviceName) || _stats.ContainsKey(candidate.DeviceName))
                continue;
            _stats[candidate.DeviceName] = new CaptureStats(candidate);
        }
    }

    private void OnNetworkAddressChanged(object? sender, EventArgs e) => SignalNetworkChange();

    private void OnNetworkAvailabilityChanged(object? sender, NetworkAvailabilityEventArgs e) => SignalNetworkChange();

    private void SignalNetworkChange()
    {
        if (_stopping) return;
        Interlocked.Exchange(ref _networkChangePending, 1);
        _wake.Set();
    }

    private bool ConsumeNetworkChange() =>
        Interlocked.Exchange(ref _networkChangePending, 0) != 0;

    private void LogCaptureStats()
    {
        foreach (var candidate in _plan.Candidates)
        {
            if (!_stats.TryGetValue(candidate.DeviceName, out var stats)) continue;
            AppLog.Write(
                $"capture: stats device={candidate.Description} source={candidate.Source} " +
                $"packets={stats.Packets} tcpPayload={stats.TcpPayloadPackets} " +
                $"gameFrames={stats.GameFrames} protocolMessages={stats.ProtocolMessages} " +
                $"notifyFrames={stats.NotifyFrames} tcpGapRecoveries={stats.TcpGapRecoveries}");
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
            "Npcap capture is temporarily unavailable: " + error + " No app restart is required; ReadyAlert will keep retrying."));
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

        // Parse directly inside the Npcap packet buffer. The old Ethernet/loopback
        // path copied packet[offset..] into a new byte[] for every captured IP packet,
        // creating continuous allocation/GC pressure while the game was running.
        ProcessIpPacket(packet, offset, packet.Length, candidate, stats);
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
        int ipStart,
        int packetLimit,
        NpcapCaptureCandidate candidate,
        CaptureStats stats)
    {
        if (!TryLocateTcp(packet, ipStart, packetLimit, out var tcp, out var packetEnd, out var address)) return;
        if (packetEnd < tcp + 20) return;

        var tcpHeader = ((packet[tcp + 12] >> 4) & 0x0F) * 4;
        if (tcpHeader < 20 || packetEnd < tcp + tcpHeader) return;

        var payloadOffset = tcp + tcpHeader;
        var payloadLen = packetEnd - payloadOffset;
        var flags = packet[tcp + 13];
        var seq = ReadU32BE(packet, tcp + 4);
        var sourcePort = ReadU16BE(packet, tcp);
        var destPort = ReadU16BE(packet, tcp + 2);
        var key = new FlowKey(
            candidate.DeviceName,
            address.IpVersion,
            address.SourceHigh,
            address.SourceLow,
            address.DestinationHigh,
            address.DestinationLow,
            sourcePort,
            destPort);

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
        if ((flags & 0x05) != 0)
            _flows.Remove(key);

        if ((DateTime.UtcNow - _lastCleanupUtc).TotalSeconds >= 30)
        {
            _lastCleanupUtc = DateTime.UtcNow;
            CleanupFlows(aggressive: false);
        }
    }

    private static bool TryLocateTcp(
        byte[] packet,
        int ipStart,
        int packetLimit,
        out int tcp,
        out int packetEnd,
        out NetworkFlowAddress address)
    {
        tcp = ipStart;
        packetEnd = packetLimit;
        address = default;
        if (ipStart < 0 || packetLimit > packet.Length || ipStart >= packetLimit) return false;

        var length = packetLimit - ipStart;
        var version = packet[ipStart] >> 4;
        if (version == 4)
        {
            if (length < 40) return false;
            var ipHeader = (packet[ipStart] & 0x0F) * 4;
            if (ipHeader < 20 || length < ipHeader + 20 || packet[ipStart + 9] != 6) return false;

            var total = ReadU16BE(packet, ipStart + 2);
            if (total >= ipHeader)
            {
                var declaredEnd = ipStart + total;
                if (declaredEnd < packetEnd) packetEnd = declaredEnd;
            }

            tcp = ipStart + ipHeader;
            address = new NetworkFlowAddress(
                4,
                0,
                ReadU32BE(packet, ipStart + 12),
                0,
                ReadU32BE(packet, ipStart + 16));
            return true;
        }

        if (version != 6 || length < 60) return false;

        var payloadLength = ReadU16BE(packet, ipStart + 4);
        if (payloadLength != 0)
        {
            var declaredEnd = ipStart + 40 + payloadLength;
            if (declaredEnd < packetEnd) packetEnd = declaredEnd;
        }

        byte next = packet[ipStart + 6];
        var cursor = ipStart + 40;
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
        address = new NetworkFlowAddress(
            6,
            ReadU64BE(packet, ipStart + 8),
            ReadU64BE(packet, ipStart + 16),
            ReadU64BE(packet, ipStart + 24),
            ReadU64BE(packet, ipStart + 32));
        return true;
    }

    internal static (bool Success, int TcpOffset, int PacketEnd, byte IpVersion) ProbeIpPacketForSelfTest(
        byte[] packet,
        int ipStart)
    {
        var success = TryLocateTcp(packet, ipStart, packet.Length, out var tcp, out var end, out var address);
        return (success, tcp, end, address.IpVersion);
    }

    private void CleanupFlows(bool aggressive)
    {
        var cutoff = DateTime.UtcNow.AddSeconds(aggressive ? -10 : -90);
        var dead = _flows.Where(kv => kv.Value.LastSeenUtc < cutoff).Select(kv => kv.Key).ToArray();
        foreach (var key in dead) _flows.Remove(key);

        if (aggressive && _flows.Count >= MaxFlows)
        {
            var oldest = _flows.OrderBy(kv => kv.Value.LastSeenUtc).FirstOrDefault();
            if (!oldest.Equals(default(KeyValuePair<FlowKey, FlowState>))) _flows.Remove(oldest.Key);
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
            if (flow.Pending.Count == 0) flow.GapStartedUtc = null;
            return;
        }

        if (!flow.Pending.ContainsKey(seq))
        {
            var copy = new byte[len];
            Buffer.BlockCopy(packet, offset, copy, 0, len);
            flow.Pending[seq] = copy;
            flow.PendingBytes += len;
        }

        flow.GapStartedUtc ??= DateTime.UtcNow;
        if (CaptureRecoveryPolicy.ShouldRecoverTcpGap(
                DateTime.UtcNow,
                flow.GapStartedUtc,
                flow.PendingBytes,
                flow.Pending.Count))
        {
            RecoverTcpGap(flow, candidate, stats);
            return;
        }

        if (flow.PendingBytes <= MaxPending) return;
        AppLog.Write($"capture-recovery: TCP pending data exceeded hard limit device={candidate.Description}; resetting flow");
        flow.Reset(null);
    }

    private void RecoverTcpGap(FlowState flow, NpcapCaptureCandidate candidate, CaptureStats stats)
    {
        if (flow.Pending.Count == 0) return;
        var first = flow.Pending.First();
        stats.TcpGapRecoveries++;
        AppLog.Write(
            $"capture-recovery: TCP capture gap resync device={candidate.Description} " +
            $"expectedSeq={flow.NextSeq} resumeSeq={first.Key} pendingSegments={flow.Pending.Count} pendingBytes={flow.PendingBytes}");

        // A packet missed by Npcap must not poison this flow indefinitely. Drop the
        // incomplete frame prefix and resume from the earliest captured segment. The
        // game frame parser already byte-scans invalid prefixes, so it can find the
        // next complete protocol-frame boundary without restarting ReadyAlert.
        flow.Stream.Clear();
        flow.LooksLikeGame = false;
        flow.HasNext = true;
        flow.NextSeq = first.Key;
        flow.GapStartedUtc = null;
        DrainPending(flow, candidate, stats);
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

            if (flow.Pending.Count == 0)
            {
                flow.GapStartedUtc = null;
                return;
            }

            var first = flow.Pending.First();
            if (!SeqBefore(first.Key, flow.NextSeq))
            {
                flow.GapStartedUtc ??= DateTime.UtcNow;
                return;
            }

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
            var typeRaw = (ushort)(((uint)flow.Stream[4] << 8) | flow.Stream[5]);
            var messageType = typeRaw & 0x7FFF;
            var max = flow.LooksLikeGame ? MaxGameFrame : MaxInitialFrame;

            // ZDPS MsgTypeId is 0..8. Consuming Echo/UNK/Return/None frames is
            // important even though ReadyAlert ignores their contents; treating them
            // as invalid corrupts stream alignment and can hide the next Notify.
            if (size < 6 || size > max || messageType > 8)
            {
                flow.Stream.RemoveAt(0);
                continue;
            }

            if (flow.Stream.Count < (int)size) return;

            var frame = flow.Stream.GetRange(0, checked((int)size)).ToArray();
            flow.Stream.RemoveRange(0, checked((int)size));
            flow.LooksLikeGame = true;
            stats.GameFrames++;
            _lastValidFrameUtc = DateTime.UtcNow;
            ProcessGameMessages(frame, 0, frame.Length, depth: 0, candidate, stats);
        }
    }

    private void ProcessGameMessages(
        byte[] data,
        int start,
        int end,
        int depth,
        NpcapCaptureCandidate candidate,
        CaptureStats stats)
    {
        if (depth > MaxFrameDepth || end - start < 6) return;

        var cursor = start;
        while (cursor + 6 <= end)
        {
            var packetSize = ReadU32BE(data, cursor);
            if (packetSize < 6 || packetSize > MaxGameFrame || cursor + packetSize > end)
            {
                AppLog.Write($"packet: invalid nested frame device={candidate.Description} depth={depth} size={packetSize} remaining={end - cursor}");
                return;
            }

            var typeRaw = ReadU16BE(data, cursor + 4);
            var compressed = (typeRaw & 0x8000) != 0;
            var messageType = typeRaw & 0x7FFF;
            if (messageType > 8)
            {
                AppLog.Write($"packet: unknown message type device={candidate.Description} type={messageType} depth={depth}");
                return;
            }

            var payloadStart = cursor + 6;
            var payloadEnd = cursor + checked((int)packetSize);
            stats.ProtocolMessages++;

            switch (messageType)
            {
                case 2: // Notify
                    ProcessNotify(data, payloadStart, payloadEnd, compressed, candidate, stats);
                    break;

                case 6: // FrameDown: uint32 sequence + nested packet stream
                    ProcessFrameDown(data, payloadStart, payloadEnd, compressed, depth, candidate, stats);
                    break;

                // 0=None, 1=Call, 3=Return, 4=Echo, 5=FrameUp, 7/8=UNK.
                // ZDPS consumes all of these but only FrameDown contains server->client
                // nested Notify messages. FrameUp has a different embedded proxy layout
                // and must NOT be parsed as FrameDown.
                default:
                    break;
            }

            cursor = payloadEnd;
        }
    }

    private void ProcessFrameDown(
        byte[] data,
        int payloadStart,
        int payloadEnd,
        bool compressed,
        int depth,
        NpcapCaptureCandidate candidate,
        CaptureStats stats)
    {
        if (payloadEnd - payloadStart < 4) return;
        var nestedStart = payloadStart + 4; // skip FrameDown sequence number

        if (!compressed)
        {
            ProcessGameMessages(data, nestedStart, payloadEnd, depth + 1, candidate, stats);
            return;
        }

        try
        {
            var zipped = data.AsSpan(nestedStart, payloadEnd - nestedStart).ToArray();
            var nested = _zstd.Unwrap(zipped).ToArray();
            ProcessGameMessages(nested, 0, nested.Length, depth + 1, candidate, stats);
        }
        catch (Exception ex)
        {
            AppLog.Write($"packet: FrameDown zstd decompression failed device={candidate.Description}: {ex.Message}");
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

        byte[] payload;
        try
        {
            var raw = frame.AsSpan(protoStart, protoLength).ToArray();
            payload = compressed ? _zstd.Unwrap(raw).ToArray() : raw;
        }
        catch (Exception ex)
        {
            AppLog.Write($"packet: notify zstd decompression failed device={candidate.Description} service={service} method=0x{method:X}: {ex.Message}");
            return;
        }

        if (_seenNotifyKeys.Add((candidate.DeviceName, service, method)))
            AppLog.Write($"probe: notify device={candidate.Description} service={service} method=0x{method:X} compressed={compressed} protoLen={payload.Length}");

        // Chat is another consumer of this already-filtered/reassembled/decompressed
        // Notify stream. Its dispatcher also routes independent queue/party and local-
        // player identity consumers before optional chat handling.
        if (ChatCaptureBridge.TryHandle(service, method, payload))
            return;

        // Exact ZDPS Ready Check trigger: NotifyAllMemberReady opens the Ready Check UI.
        // NotifyCaptainReady is a response/update and is used by ZDPS to stop its loop,
        // not to start the alert.
        if (service == WorldNtfService)
        {
            if (method == NotifyAllMemberReady)
            {
                AppLog.Write($"event: ready-check open device={candidate.Description} method=0x{method:X}");
                EnqueueReadyAlert("world-ready-check", "BPSR Ready Check", "Party Ready Check started.");
                return;
            }

            if (method == NotifyCaptainReady)
            {
                AppLog.Write($"event: ready-check response device={candidate.Description} method=0x{method:X}");
                return;
            }
        }

        // Legacy fallback only. QueueAlertCaptureBridge normally consumes Voting before
        // this point and emits kind=queue; keep this parser for diagnostics/compatibility.
        if (service == GrpcTeamNtfService && method == NotifyTeamActivityState)
        {
            if (TryParseTeamActivityState(payload, 0, payload.Length, out var state))
            {
                AppLog.Write($"event: team-activity state device={candidate.Description} state={state}");
                if (state == TeamActivityVoting)
                    EnqueueReadyAlert("team-activity-vote", "BPSR Party Ready Vote", "A party activity is waiting for your vote.");
            }
            else
            {
                AppLog.Write($"event: team-activity payload could not be parsed device={candidate.Description} len={payload.Length}");
            }
            return;
        }

        if (service == GrpcTeamNtfService && method is 0x12 or 0x13 or 0x14 or 0x1F)
            AppLog.Write($"probe: GrpcTeamNtf matchmaking-related device={candidate.Description} method=0x{method:X} protoLen={payload.Length}");

        if (service != MatchNtfService) return;

        AppLog.Write($"probe: MatchNtf device={candidate.Description} method=0x{method:X} protoLen={payload.Length}");
        if (method != EnterMatchResult) return;

        if (!TryParseMatchStatus(payload, 0, payload.Length, out var status))
        {
            AppLog.Write($"event: match EnterMatchResult payload could not be parsed device={candidate.Description} len={payload.Length}");
            return;
        }

        AppLog.Write($"event: match EnterMatchResult device={candidate.Description} status={status}");
        if (status != MatchStatusWaitReady) return;

        var now = DateTime.UtcNow;
        if ((now - _lastQueueAlertUtc).TotalSeconds < 5) return;
        _lastQueueAlertUtc = now;
        _events.Enqueue(new AlertEvent("queue", "BPSR Match Found", "Matchmaking is waiting for acceptance."));
        AppLog.Write("alert: enqueued kind=queue source=match-wait-ready");
    }

    private void EnqueueReadyAlert(string source, string title, string message)
    {
        var now = DateTime.UtcNow;
        if ((now - _lastReadyAlertUtc).TotalSeconds < 3)
        {
            AppLog.Write($"alert: duplicate suppressed kind=ready source={source}");
            return;
        }

        _lastReadyAlertUtc = now;
        _events.Enqueue(new AlertEvent("ready", title, message));
        AppLog.Write($"alert: enqueued kind=ready source={source}");
    }

    private static bool TryParseMatchStatus(byte[] data, int offset, int length, out int status)
    {
        status = -1;

        // MatchNtf.EnterMatchResultNtf.vRequest = field 1
        if (!TryGetLengthField(data, offset, length, 1, out var requestOffset, out var requestLength)) return false;
        // EnterMatchResultNtfRequest.matchInfo = field 2
        if (!TryGetLengthField(data, requestOffset, requestLength, 2, out var infoOffset, out var infoLength)) return false;
        // MatchInfo.matchStatus = field 2
        if (!TryGetVarintField(data, infoOffset, infoLength, 2, out var value)) return false;

        status = checked((int)value);
        return true;
    }

    private static bool TryParseTeamActivityState(byte[] data, int offset, int length, out int state)
    {
        state = -1;

        // GrpcTeamNtf.NotifyTeamActivityState.vRequest = field 1
        if (!TryGetLengthField(data, offset, length, 1, out var requestOffset, out var requestLength)) return false;
        // NotifyTeamActivityStateRequest.state (TeamActivity) = field 1
        if (!TryGetLengthField(data, requestOffset, requestLength, 1, out var activityOffset, out var activityLength)) return false;
        // TeamActivity.state = field 2
        if (!TryGetVarintField(data, activityOffset, activityLength, 2, out var value)) return false;

        state = checked((int)value);
        return true;
    }

    private static bool TryGetLengthField(
        byte[] data,
        int offset,
        int length,
        int wantedField,
        out int valueOffset,
        out int valueLength)
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

    private void WaitForRetry(int milliseconds)
    {
        for (var elapsed = 0; elapsed < milliseconds && !_stopping;)
        {
            var slice = Math.Min(250, milliseconds - elapsed);
            _wake.WaitOne(slice);
            if (_stopping || Volatile.Read(ref _networkChangePending) != 0) return;
            elapsed += slice;
        }
    }

    public void Dispose()
    {
        _stopping = true;
        NetworkChange.NetworkAddressChanged -= OnNetworkAddressChanged;
        NetworkChange.NetworkAvailabilityChanged -= OnNetworkAvailabilityChanged;
        _wake.Set();
        if (_thread is { IsAlive: true }) _thread.Join(3000);
        _zstd.Dispose();
        _wake.Dispose();
    }

    private readonly record struct NetworkFlowAddress(
        byte IpVersion,
        ulong SourceHigh,
        ulong SourceLow,
        ulong DestinationHigh,
        ulong DestinationLow);

    private readonly record struct FlowKey(
        string Device,
        byte IpVersion,
        ulong SourceHigh,
        ulong SourceLow,
        ulong DestinationHigh,
        ulong DestinationLow,
        ushort SourcePort,
        ushort DestinationPort);

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
        internal long ProtocolMessages;
        internal long NotifyFrames;
        internal long TcpGapRecoveries;

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
        internal DateTime? GapStartedUtc;

        internal void Reset(uint? next)
        {
            Pending.Clear();
            PendingBytes = 0;
            Stream.Clear();
            LooksLikeGame = false;
            GapStartedUtc = null;
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
