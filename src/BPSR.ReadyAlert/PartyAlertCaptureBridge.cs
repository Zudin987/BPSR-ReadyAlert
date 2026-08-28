using System.Collections.Concurrent;

namespace BPSR.ReadyAlert;

/// <summary>
/// Core ReadyAlert consumer for incoming BPSR party invitations and join requests.
/// This runs on CaptureEngine's already decoded Notify stream and is intentionally
/// independent from the optional Chat Overlay / translation / TTS features.
/// </summary>
internal static class PartyAlertCaptureBridge
{
    // Verified against BPSR-ZDPS GrpcTeamNtf protocol metadata.
    private const ulong GrpcTeamNtfService = 966_773_353UL;
    private const uint NotifyApplyJoin = 0x05;
    private const uint NotifyInvitation = 0x06;
    private static readonly TimeSpan DuplicateWindow = TimeSpan.FromSeconds(5);

    private static readonly object Gate = new();
    private static ConcurrentQueue<AlertEvent>? _events;
    private static ulong _lastInviteFingerprint;
    private static ulong _lastRequestFingerprint;
    private static DateTime _lastInviteUtc = DateTime.MinValue;
    private static DateTime _lastRequestUtc = DateTime.MinValue;

    internal static void Configure(ConcurrentQueue<AlertEvent> events)
    {
        ArgumentNullException.ThrowIfNull(events);
        Volatile.Write(ref _events, events);
    }

    /// <summary>
    /// Returns true only for the two incoming GrpcTeamNtf notifications owned by
    /// this feature. Exact service/method matching avoids treating ordinary party
    /// state updates, matchmaking packets, or outgoing actions as social alerts.
    /// </summary>
    internal static bool TryHandle(ulong service, uint method, byte[] payload)
    {
        if (service != GrpcTeamNtfService)
            return false;

        var isInvite = method == NotifyInvitation;
        var isRequest = method == NotifyApplyJoin;
        if (!isInvite && !isRequest)
            return false;

        payload ??= [];
        var fingerprint = Fingerprint(payload);
        var now = DateTime.UtcNow;

        lock (Gate)
        {
            ref var lastFingerprint = ref (isInvite ? ref _lastInviteFingerprint : ref _lastRequestFingerprint);
            ref var lastUtc = ref (isInvite ? ref _lastInviteUtc : ref _lastRequestUtc);

            if (lastFingerprint == fingerprint && now - lastUtc < DuplicateWindow)
            {
                AppLog.Write($"alert: duplicate suppressed kind={(isInvite ? "party-invite" : "party-request")} method=0x{method:X}");
                return true;
            }

            lastFingerprint = fingerprint;
            lastUtc = now;
        }

        var events = Volatile.Read(ref _events);
        if (events is null)
        {
            AppLog.Write($"alert: party social notify detected before event queue was configured method=0x{method:X}");
            return true;
        }

        if (isInvite)
        {
            events.Enqueue(new AlertEvent(
                "party-invite",
                "BPSR Party Invite",
                "You received a party invitation."));
            AppLog.Write("alert: enqueued kind=party-invite source=GrpcTeamNtf.NotifyInvitation");
        }
        else
        {
            events.Enqueue(new AlertEvent(
                "party-request",
                "BPSR Party Join Request",
                "Someone requested to join your party."));
            AppLog.Write("alert: enqueued kind=party-request source=GrpcTeamNtf.NotifyApplyJoin");
        }

        return true;
    }

    // FNV-1a is enough for short-lived duplicate suppression and keeps this hot path
    // allocation-free. The method is part of the identity via separate per-kind slots.
    private static ulong Fingerprint(ReadOnlySpan<byte> payload)
    {
        const ulong offset = 14_695_981_039_346_656_037UL;
        const ulong prime = 1_099_511_628_211UL;
        var hash = offset;
        foreach (var value in payload)
        {
            hash ^= value;
            hash *= prime;
        }
        return hash;
    }

    internal static ulong ServiceIdForSelfTest => GrpcTeamNtfService;
    internal static uint InvitationMethodForSelfTest => NotifyInvitation;
    internal static uint ApplyJoinMethodForSelfTest => NotifyApplyJoin;

    internal static void ResetForSelfTest()
    {
        lock (Gate)
        {
            _lastInviteFingerprint = 0;
            _lastRequestFingerprint = 0;
            _lastInviteUtc = DateTime.MinValue;
            _lastRequestUtc = DateTime.MinValue;
        }
    }
}
