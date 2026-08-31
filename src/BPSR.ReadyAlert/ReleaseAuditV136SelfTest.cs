namespace BPSR.ReadyAlert;

/// <summary>
/// Final release-gate regressions for startup retention seeding and cleanup cadence
/// around manual/system wall-clock changes.
/// </summary>
internal static class ReleaseAuditV136SelfTest
{
    internal static void Run()
    {
        TestConfiguredRetentionUsedByFirstStartupCleanup();
        TestPeriodicCleanupSurvivesClockRollback();
    }

    private static void TestConfiguredRetentionUsedByFirstStartupCleanup()
    {
        var root = Path.Combine(
            Path.GetTempPath(),
            "BPSR-ReadyAlert-v136-startup-retention-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(root);

        try
        {
            var old = new ChatMessageEvent(
                1,
                "Old24HourUser",
                80,
                ChatChannel.World,
                DateTime.Now.AddHours(-25),
                ChatMessageKind.Text,
                "must be removed by first 24-hour startup cleanup");
            var path = Path.Combine(root, "configured-startup.txt");
            File.WriteAllText(
                path,
                ChatLocalLogWriter.FormatLineForSelfTest(old) + Environment.NewLine);

            using var writer = new ChatLocalLogWriter(
                root,
                startWorker: true,
                initialRetentionHours: ChatLocalLogRetention.OneDayHours,
                initialEnabled: false);

            Assert(writer.RetentionHoursForSelfTest == ChatLocalLogRetention.OneDayHours,
                "writer is seeded with persisted retention before its worker starts");
            Assert(writer.WaitForStartupCleanupForSelfTest(TimeSpan.FromSeconds(5)),
                "configured startup cleanup completes");
            Assert(!File.Exists(path),
                "first startup cleanup honors configured 24-hour retention instead of defaulting to seven days");
        }
        finally
        {
            try { Directory.Delete(root, recursive: true); } catch { }
        }
    }

    private static void TestPeriodicCleanupSurvivesClockRollback()
    {
        var lastCleanup = new DateTimeOffset(2026, 8, 31, 12, 0, 0, TimeSpan.Zero);
        var nextCleanup = lastCleanup.AddMinutes(5);

        Assert(!ChatLocalLogWriter.IsPeriodicCleanupDueForSelfTest(lastCleanup.AddMinutes(2), nextCleanup),
            "normal time before the five-minute boundary does not trigger early cleanup");
        Assert(ChatLocalLogWriter.IsPeriodicCleanupDueForSelfTest(nextCleanup, nextCleanup),
            "normal five-minute boundary triggers cleanup");
        Assert(ChatLocalLogWriter.IsPeriodicCleanupDueForSelfTest(lastCleanup.AddHours(-1), nextCleanup),
            "large wall-clock rollback triggers immediate cleanup and cadence re-anchoring");
    }

    private static void Assert(bool condition, string message)
    {
        if (!condition)
            throw new InvalidOperationException("v1.3.6 release-audit self-test failed: " + message);
    }
}
