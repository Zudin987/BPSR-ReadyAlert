namespace BPSR.ReadyAlert;

internal static class CaptureRecoveryPolicy
{
    internal const int NetworkChangeSettleMs = 500;
    internal const int SilentCaptureMs = 45_000;
    internal const int ProtocolStallMs = 20_000;
    internal const int RecentPacketMs = 5_000;
    internal const int TcpGapRecoveryMs = 1_500;
    internal const int TcpGapPendingBytes = 256 * 1024;
    internal const int TcpGapPendingSegments = 64;

    internal static int RetryDelayMs(int consecutiveFailures) => consecutiveFailures switch
    {
        <= 1 => 1_000,
        2 => 2_000,
        3 => 5_000,
        _ => 10_000
    };

    internal static bool IsManualPlan(NpcapCapturePlan plan) =>
        plan.Candidates.Any(candidate =>
            string.Equals(candidate.Source, "User selected", StringComparison.OrdinalIgnoreCase));

    internal static bool ShouldRestartSilentCapture(
        bool gameRunning,
        DateTime nowUtc,
        DateTime captureOpenedUtc,
        DateTime lastBpsrPacketUtc)
    {
        if (!gameRunning || captureOpenedUtc == DateTime.MinValue) return false;
        var anchor = lastBpsrPacketUtc == DateTime.MinValue ? captureOpenedUtc : lastBpsrPacketUtc;
        return (nowUtc - anchor).TotalMilliseconds >= SilentCaptureMs;
    }

    internal static bool ShouldResetProtocolFlows(
        bool gameRunning,
        DateTime nowUtc,
        DateTime lastBpsrPacketUtc,
        DateTime lastValidFrameUtc)
    {
        if (!gameRunning || lastBpsrPacketUtc == DateTime.MinValue) return false;
        if ((nowUtc - lastBpsrPacketUtc).TotalMilliseconds > RecentPacketMs) return false;
        if (lastValidFrameUtc == DateTime.MinValue) return false;
        return (nowUtc - lastValidFrameUtc).TotalMilliseconds >= ProtocolStallMs;
    }

    internal static bool ShouldRecoverTcpGap(
        DateTime nowUtc,
        DateTime? gapStartedUtc,
        int pendingBytes,
        int pendingSegments)
    {
        if (pendingSegments <= 0) return false;
        if (pendingBytes >= TcpGapPendingBytes || pendingSegments >= TcpGapPendingSegments)
            return true;
        return gapStartedUtc.HasValue &&
               (nowUtc - gapStartedUtc.Value).TotalMilliseconds >= TcpGapRecoveryMs;
    }
}
