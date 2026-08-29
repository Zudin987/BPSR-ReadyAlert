using System.Collections.Concurrent;

namespace BPSR.ReadyAlert;

/// <summary>
/// Owns the two server notification paths that can open the matchmaking / party
/// activity acceptance prompt. Both are one logical Queue Pop event and therefore
/// share one duplicate window and one queue sound/toggle.
/// </summary>
internal static class QueueAlertCaptureBridge
{
    private const ulong MatchNtfService = 822_849_903UL;
    private const ulong GrpcTeamNtfService = 966_773_353UL;
    private const uint EnterMatchResult = 0x04;
    private const uint NotifyTeamActivityState = 0x0E;
    private const int TeamActivityVoting = 3;
    private const int MatchStatusWaitReady = 2;
    private static readonly TimeSpan DuplicateWindow = TimeSpan.FromSeconds(5);

    private static readonly object Gate = new();
    private static ConcurrentQueue<AlertEvent>? _events;
    private static DateTime _lastQueueUtc = DateTime.MinValue;

    internal static void Configure(ConcurrentQueue<AlertEvent> events)
    {
        ArgumentNullException.ThrowIfNull(events);
        Volatile.Write(ref _events, events);
    }

    /// <summary>
    /// Returns true only when this packet is a confirmed queue/acceptance prompt.
    /// Non-voting team activity states and non-wait-ready match results stay unclaimed
    /// so CaptureEngine can continue its normal protocol handling/logging.
    /// </summary>
    internal static bool TryHandle(ulong service, uint method, byte[] payload)
    {
        payload ??= [];

        string? source = null;
        string? title = null;
        string? message = null;

        if (service == GrpcTeamNtfService && method == NotifyTeamActivityState)
        {
            if (!TryParseTeamActivityState(payload, 0, payload.Length, out var state) ||
                state != TeamActivityVoting)
                return false;

            source = "GrpcTeamNtf.NotifyTeamActivityState(voting)";
            title = "BPSR Party Ready Vote";
            message = "A party activity is waiting for your vote.";
        }
        else if (service == MatchNtfService && method == EnterMatchResult)
        {
            if (!TryParseMatchStatus(payload, 0, payload.Length, out var status) ||
                status != MatchStatusWaitReady)
                return false;

            source = "MatchNtf.EnterMatchResult(wait-ready)";
            title = "BPSR Match Found";
            message = "Matchmaking is waiting for acceptance.";
        }
        else
        {
            return false;
        }

        var now = DateTime.UtcNow;
        lock (Gate)
        {
            if (now - _lastQueueUtc < DuplicateWindow)
            {
                AppLog.Write($"alert: duplicate suppressed kind=queue source={source}");
                return true;
            }
            _lastQueueUtc = now;
        }

        var events = Volatile.Read(ref _events);
        if (events is null)
        {
            AppLog.Write($"alert: queue signal detected before event queue was configured source={source}");
            return true;
        }

        events.Enqueue(new AlertEvent("queue", title, message));
        AppLog.Write($"alert: enqueued kind=queue source={source}");
        return true;
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

    internal static ulong MatchServiceIdForSelfTest => MatchNtfService;
    internal static ulong TeamServiceIdForSelfTest => GrpcTeamNtfService;
    internal static uint EnterMatchResultMethodForSelfTest => EnterMatchResult;
    internal static uint TeamActivityMethodForSelfTest => NotifyTeamActivityState;

    internal static void ResetForSelfTest()
    {
        lock (Gate)
            _lastQueueUtc = DateTime.MinValue;
    }
}
