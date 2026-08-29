using System.Collections.Concurrent;
using System.Reflection;

namespace BPSR.ReadyAlert;

internal static class CoreAlertV131SelfTest
{
    internal static void Run()
    {
        TestDefaultsAndIndependentToggles();
        TestDistinctSoundRouting();
        TestLiveQueueProtocolRouting();
        TestTrayMenuContract();
        TestSettingsPersistenceAndUpgradeDefaults();
        TestBundledWaveResources();
    }

    private static void TestDefaultsAndIndependentToggles()
    {
        var settings = new AppSettings();
        Assert(settings.PartyInviteAlert && settings.PartyRequestAlert,
            "new party alert toggles default to enabled for upgrades");

        settings.PartyInviteAlert = false;
        Assert(!TrayApplicationContext.IsAlertEnabledForSelfTest(settings, "party-invite"),
            "party invite toggle independently disables invite alerts");
        Assert(TrayApplicationContext.IsAlertEnabledForSelfTest(settings, "party-request"),
            "disabling party invite does not disable party requests");

        settings.PartyInviteAlert = true;
        settings.PartyRequestAlert = false;
        Assert(TrayApplicationContext.IsAlertEnabledForSelfTest(settings, "party-invite"),
            "party invite can remain enabled independently");
        Assert(!TrayApplicationContext.IsAlertEnabledForSelfTest(settings, "party-request"),
            "party request toggle independently disables request alerts");
    }

    private static void TestDistinctSoundRouting()
    {
        var kinds = new[] { "queue", "ready", "party-invite", "party-request" };
        var keys = kinds.Select(CoreAlertAudioPlayer.SoundKeyForSelfTest).ToArray();
        Assert(keys.All(k => !string.IsNullOrWhiteSpace(k)),
            "all four core event kinds have a sound mapping");
        Assert(keys.Distinct(StringComparer.Ordinal).Count() == 4,
            "queue, ready, party invite and party request use four distinct sounds");
    }

    private static void TestLiveQueueProtocolRouting()
    {
        PartyAlertCaptureBridge.ResetForSelfTest();
        var events = new ConcurrentQueue<AlertEvent>();
        PartyAlertCaptureBridge.Configure(events);

        // GrpcTeamNtf.NotifyTeamActivityState -> vRequest.state.state = Voting (3).
        // This is the live queue/party-activity acceptance path that v1.3.1 wrongly
        // allowed CaptureEngine to classify as kind=ready.
        var teamVoting = new byte[] { 0x0A, 0x04, 0x0A, 0x02, 0x10, 0x03 };
        Assert(PartyAlertCaptureBridge.TryHandle(
                QueueAlertCaptureBridge.TeamServiceIdForSelfTest,
                QueueAlertCaptureBridge.TeamActivityMethodForSelfTest,
                teamVoting),
            "team-activity voting queue signal is owned by core queue routing");
        Assert(events.TryDequeue(out var teamEvent) && teamEvent.Kind == "queue",
            "team-activity voting emits queue kind instead of ready");
        Assert(CoreAlertAudioPlayer.SoundKeyForSelfTest(teamEvent.Kind) == "Queue",
            "team-activity voting selects Queue.wav");

        // MatchNtf.EnterMatchResult -> vRequest.matchInfo.matchStatus = WaitReady (2).
        // A server can expose both paths for the same acceptance prompt, so the second
        // signal must be consumed but not play a second Queue sound.
        var matchWaitReady = new byte[] { 0x0A, 0x04, 0x12, 0x02, 0x10, 0x02 };
        Assert(PartyAlertCaptureBridge.TryHandle(
                QueueAlertCaptureBridge.MatchServiceIdForSelfTest,
                QueueAlertCaptureBridge.EnterMatchResultMethodForSelfTest,
                matchWaitReady),
            "match wait-ready signal is owned by the same queue router");
        Assert(events.IsEmpty,
            "paired team-voting and match-wait-ready signals are de-duplicated");

        PartyAlertCaptureBridge.ResetForSelfTest();
        var teamNotVoting = new byte[] { 0x0A, 0x04, 0x0A, 0x02, 0x10, 0x02 };
        Assert(!PartyAlertCaptureBridge.TryHandle(
                QueueAlertCaptureBridge.TeamServiceIdForSelfTest,
                QueueAlertCaptureBridge.TeamActivityMethodForSelfTest,
                teamNotVoting),
            "non-voting team activity remains unclaimed by queue routing");
    }

    private static void TestTrayMenuContract()
    {
        var order = TrayApplicationContext.CoreAlertMenuOrderForSelfTest;
        Assert(order.SequenceEqual(new[]
        {
            "Queue Pop Alert",
            "Ready Check Alert",
            "Party Invite Alert",
            "Party Request Alert"
        }, StringComparer.Ordinal),
            "party toggles appear directly below Ready Check in the tray menu");
        Assert(!TrayApplicationContext.LegacyShowHideChatItemsForSelfTest,
            "legacy Show Chat / Hide Chat tray items are removed");

        var volume = TrayApplicationContext.ReadyQueueVolumeMenuText(35);
        Assert(volume.Contains("Ready", StringComparison.Ordinal) &&
               volume.Contains("Queue", StringComparison.Ordinal) &&
               volume.Contains("Party", StringComparison.Ordinal) &&
               volume.Contains("35%", StringComparison.Ordinal),
            "shared core volume label covers Ready, Queue and Party sounds");
    }

    private static void TestSettingsPersistenceAndUpgradeDefaults()
    {
        var path = Path.Combine(Path.GetTempPath(), $"BPSR-ReadyAlert-v131-{Guid.NewGuid():N}.json");
        try
        {
            var store = new SettingsStore(path);
            var settings = new AppSettings
            {
                PartyInviteAlert = false,
                PartyRequestAlert = true
            };
            Assert(store.Save(settings), "party alert preferences persist successfully");
            var loaded = store.Load();
            Assert(!loaded.PartyInviteAlert && loaded.PartyRequestAlert,
                "party alert preferences round-trip independently");

            Delete(path);
            Delete(path + ".bak");
            Delete(path + ".new");

            // Simulate an existing v1.3.0 settings file with no new party fields.
            File.WriteAllText(path, "{\"queuePopAlert\":true,\"readyCheckAlert\":true}");
            var upgraded = new SettingsStore(path).Load();
            Assert(upgraded.PartyInviteAlert && upgraded.PartyRequestAlert,
                "v1.3.0 settings without party fields upgrade with both alerts enabled");
        }
        finally
        {
            Delete(path);
            Delete(path + ".bak");
            Delete(path + ".new");
        }
    }

    private static void TestBundledWaveResources()
    {
        var expected = new (string Resource, double MinSeconds, double MaxSeconds)[]
        {
            ("BPSR.ReadyAlert.Assets.Queue.wav", 1.47, 1.50),
            ("BPSR.ReadyAlert.Assets.ReadyCheck.wav", 1.28, 1.32),
            ("BPSR.ReadyAlert.Assets.PartyInvite.wav", 1.79, 1.83),
            ("BPSR.ReadyAlert.Assets.PartyRequest.wav", 1.88, 1.93)
        };

        var assembly = Assembly.GetExecutingAssembly();
        foreach (var item in expected)
        {
            using var resource = assembly.GetManifestResourceStream(item.Resource)
                ?? throw new InvalidOperationException("v1.3.x missing embedded audio resource: " + item.Resource);
            var temp = Path.Combine(Path.GetTempPath(), $"BPSR-ReadyAlert-v13x-audio-{Guid.NewGuid():N}.wav");
            try
            {
                using (var output = File.Create(temp)) resource.CopyTo(output);
                var meta = AlertAudioPlayer.ProbePcm16Wave(temp);
                Assert(meta.SampleRate == 44_100 && meta.Channels == 1,
                    item.Resource + " is 44.1 kHz mono PCM16");
                Assert(meta.DurationSeconds >= item.MinSeconds && meta.DurationSeconds <= item.MaxSeconds,
                    item.Resource + " duration matches the supplied sound");
            }
            finally
            {
                Delete(temp);
            }
        }
    }

    private static void Delete(string path)
    {
        try { if (File.Exists(path)) File.Delete(path); } catch { }
    }

    private static void Assert(bool condition, string name)
    {
        if (!condition)
            throw new InvalidOperationException("v1.3.x core alert self-test failed: " + name);
    }
}
