namespace BPSR.ReadyAlert;

internal static class CaptureRecoveryV134SelfTest
{
    internal static void Run()
    {
        TestRetryBackoff();
        TestSilentCaptureWatchdog();
        TestProtocolStallWatchdog();
        TestTcpGapPolicy();
        TestAdapterRecoverySelection();
        TestWaitingPlans();
    }

    private static void TestRetryBackoff()
    {
        var actual = Enumerable.Range(1, 6)
            .Select(CaptureRecoveryPolicy.RetryDelayMs)
            .ToArray();
        Assert(actual.SequenceEqual(new[] { 1_000, 2_000, 5_000, 10_000, 10_000, 10_000 }),
            "capture retry uses bounded 1s/2s/5s/10s backoff");
    }

    private static void TestSilentCaptureWatchdog()
    {
        var opened = new DateTime(2026, 8, 29, 0, 0, 0, DateTimeKind.Utc);
        Assert(!CaptureRecoveryPolicy.ShouldRestartSilentCapture(
                false, opened.AddMinutes(5), opened, DateTime.MinValue),
            "silent watchdog stays idle when BPSR is not running");
        Assert(!CaptureRecoveryPolicy.ShouldRestartSilentCapture(
                true, opened.AddSeconds(44), opened, DateTime.MinValue),
            "silent watchdog allows normal quiet window");
        Assert(CaptureRecoveryPolicy.ShouldRestartSilentCapture(
                true, opened.AddSeconds(46), opened, DateTime.MinValue),
            "silent watchdog rebuilds a capture that never sees BPSR packets");

        var packet = opened.AddSeconds(40);
        Assert(!CaptureRecoveryPolicy.ShouldRestartSilentCapture(
                true, packet.AddSeconds(40), opened, packet),
            "recent BPSR traffic refreshes the silent watchdog");
        Assert(CaptureRecoveryPolicy.ShouldRestartSilentCapture(
                true, packet.AddSeconds(46), opened, packet),
            "silent watchdog rebuilds after BPSR traffic stops");
    }

    private static void TestProtocolStallWatchdog()
    {
        var now = new DateTime(2026, 8, 29, 0, 1, 0, DateTimeKind.Utc);
        Assert(CaptureRecoveryPolicy.ShouldResetProtocolFlows(
                true, now, now.AddSeconds(-1), now.AddSeconds(-21)),
            "recent packets plus stale valid frames reset TCP reassembly");
        Assert(!CaptureRecoveryPolicy.ShouldResetProtocolFlows(
                true, now, now.AddSeconds(-8), now.AddSeconds(-30)),
            "old packet traffic does not trigger protocol-only reset");
        Assert(!CaptureRecoveryPolicy.ShouldResetProtocolFlows(
                false, now, now.AddSeconds(-1), now.AddSeconds(-30)),
            "protocol watchdog stays idle while game is closed");
    }

    private static void TestTcpGapPolicy()
    {
        var now = new DateTime(2026, 8, 29, 0, 2, 0, DateTimeKind.Utc);
        Assert(!CaptureRecoveryPolicy.ShouldRecoverTcpGap(
                now, now.AddMilliseconds(-500), 8_192, 2),
            "short small TCP reordering remains buffered");
        Assert(CaptureRecoveryPolicy.ShouldRecoverTcpGap(
                now, now.AddMilliseconds(-1_600), 8_192, 2),
            "persistent missing capture segment resynchronizes after timeout");
        Assert(CaptureRecoveryPolicy.ShouldRecoverTcpGap(
                now, now.AddMilliseconds(-100), CaptureRecoveryPolicy.TcpGapPendingBytes, 2),
            "large TCP gap buffer resynchronizes before hard memory limit");
        Assert(CaptureRecoveryPolicy.ShouldRecoverTcpGap(
                now, now.AddMilliseconds(-100), 8_192, CaptureRecoveryPolicy.TcpGapPendingSegments),
            "many pending TCP segments resynchronize without waiting for timeout");
    }

    private static void TestAdapterRecoverySelection()
    {
        var deviceA = new NpcapDevice(@"\\Device\\NPF_{AAAA}", "Wi-Fi Adapter");
        var deviceB = new NpcapDevice(@"\\Device\\NPF_{BBBB}", "Ethernet Adapter");
        var devices = new[] { deviceA, deviceB };

        var automatic = new NpcapCapturePlan(
            [new NpcapCaptureCandidate(deviceA.Name, deviceA.Description, "Auto-selected")],
            devices,
            null);
        var switched = CaptureRecoveryPlanner.SelectDeviceFromSnapshot(
            automatic,
            devices,
            [deviceB.Name, deviceA.Name],
            manual: false);
        Assert(switched?.Name == deviceB.Name,
            "automatic recovery follows the newly preferred active adapter");

        var manual = new NpcapCapturePlan(
            [new NpcapCaptureCandidate(deviceA.Name, deviceA.Description, "User selected")],
            devices,
            null);
        var keptManual = CaptureRecoveryPlanner.SelectDeviceFromSnapshot(
            manual,
            devices,
            [deviceB.Name],
            manual: true);
        Assert(keptManual?.Name == deviceA.Name,
            "manual adapter is never silently replaced by an automatic adapter");

        var missingManual = CaptureRecoveryPlanner.SelectDeviceFromSnapshot(
            manual,
            [deviceB],
            [deviceB.Name],
            manual: true);
        Assert(missingManual is null,
            "missing manual adapter waits for that exact adapter instead of switching");
    }

    private static void TestWaitingPlans()
    {
        var automatic = CaptureRecoveryPlanner.CreateWaitingPlan(null);
        Assert(!CaptureRecoveryPolicy.IsManualPlan(automatic) &&
               string.IsNullOrEmpty(automatic.Primary.DeviceName),
            "automatic startup can remain alive while no adapter exists");

        var manual = CaptureRecoveryPlanner.CreateWaitingPlan(@"\\Device\\NPF_{MANUAL}");
        Assert(CaptureRecoveryPolicy.IsManualPlan(manual) &&
               manual.Primary.DeviceName == @"\\Device\\NPF_{MANUAL}",
            "manual startup preserves unavailable adapter preference for recovery");
    }

    private static void Assert(bool condition, string name)
    {
        if (condition) return;
        Environment.ExitCode = 230;
        throw new InvalidOperationException("v1.3.4 capture recovery self-test failed: " + name);
    }
}
