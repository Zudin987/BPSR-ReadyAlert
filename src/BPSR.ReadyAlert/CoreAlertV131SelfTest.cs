using System.Collections.Concurrent;
using System.Reflection;

namespace BPSR.ReadyAlert;

internal static class CoreAlertV131SelfTest
{
    internal static void Run()
    {
        TestDefaultsAndIndependentToggles();
        TestV132DefaultProfile();
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

    private static void TestV132DefaultProfile()
    {
        var settings = new AppSettings();
        settings.Chat.Normalize();
        settings.SpeechTranslation.Normalize();

        Assert(settings.QueuePopAlert && settings.ReadyCheckAlert &&
               settings.PartyInviteAlert && settings.PartyRequestAlert,
            "v1.3.2 core alerts default enabled");
        Assert(settings.DesktopNotification,
            "v1.3.2 desktop notifications default enabled");
        Assert(settings.ChatOverlayEnabled,
            "v1.3.2 chat overlay defaults enabled");

        var chat = settings.Chat;
        Assert(chat.TopMost && !chat.CompactMode &&
               chat.ShowTime && chat.ShowTimeAsAgo && chat.HideStickers &&
               chat.BackgroundOpacity == 82 && chat.ToolbarOpacity == 92 &&
               chat.TextOpacity == 100 && chat.WindowOpacity == 100 &&
               string.Equals(chat.FontFamily, "Segoe UI", StringComparison.Ordinal) &&
               Math.Abs(chat.FontSize - 12F) < 0.01F &&
               !chat.BoldMessageText && chat.TextShadow &&
               chat.ShowSeparators && chat.ShowZebraStripes && chat.ShowColorBand &&
               !chat.ClickThrough && string.Equals(chat.CollapseSide, "Left", StringComparison.Ordinal),
            "v1.3.2 chat appearance defaults match the requested profile");

        var expectedTabs = new (long Id, string Name, int[] Channels, int MinLevel)[]
        {
            (639233255393111833L, "All", [1, 2, 3, 4, 5, 6, 9], 50),
            (639233255393111900L, "Guild&Team", [3, 4, 5, 6], 1),
            (639233255393111918L, "Guild", [4], 1),
            (639235625391474596L, "Team", [3, 6], 1)
        };
        Assert(chat.Tabs.Count == expectedTabs.Length,
            "v1.3.2 default profile has exactly four tabs");
        for (var i = 0; i < expectedTabs.Length; i++)
        {
            var actual = chat.Tabs[i];
            var expected = expectedTabs[i];
            Assert(actual.Id == expected.Id &&
                   string.Equals(actual.Name, expected.Name, StringComparison.Ordinal) &&
                   actual.Channels.SequenceEqual(expected.Channels) &&
                   actual.MinLevel == expected.MinLevel &&
                   actual.ShowIfMatches == string.Empty &&
                   actual.HideIfMatches == string.Empty,
                $"v1.3.2 default tab {i + 1} matches requested ID/name/channels/level");
        }
        Assert(chat.LastSelectedTabId == expectedTabs[0].Id,
            "v1.3.2 default selected tab is All");

        var speech = settings.SpeechTranslation;
        Assert(speech.TranslationEnabled && speech.TranslationWorld &&
               speech.TranslationGuild && speech.TranslationPartyTeam &&
               speech.ShowTranslationInOverlay && speech.TtsEnabled &&
               !speech.TtsGuild && speech.TtsPartyTeam && speech.ReadSenderName &&
               speech.TtsVolume == 100 && speech.HideEmojiMessages &&
               speech.HideLinkedItemMessages,
            "v1.3.2 translation and TTS defaults match the requested profile");
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
        var path = Path.Combine(Path.GetTempPath(), $"BPSR-ReadyAlert-v132-{Guid.NewGuid():N}.json");
        try
        {
            var store = new SettingsStore(path);
            var settings = new AppSettings
            {
                PartyInviteAlert = false,
                PartyRequestAlert = true,
                DesktopNotification = false,
                ChatOverlayEnabled = false
            };
            settings.Chat.TopMost = false;
            settings.Chat.CompactMode = true;
            settings.SpeechTranslation.TranslationEnabled = false;
            settings.SpeechTranslation.TtsEnabled = false;

            Assert(store.Save(settings), "explicit user preferences persist successfully");
            var loaded = store.Load();
            Assert(!loaded.PartyInviteAlert && loaded.PartyRequestAlert,
                "party alert preferences round-trip independently");
            Assert(!loaded.DesktopNotification && !loaded.ChatOverlayEnabled &&
                   !loaded.Chat.TopMost && loaded.Chat.CompactMode &&
                   !loaded.SpeechTranslation.TranslationEnabled &&
                   !loaded.SpeechTranslation.TtsEnabled,
                "new v1.3.2 defaults do not overwrite explicit saved preferences");

            Delete(path);
            Delete(path + ".bak");
            Delete(path + ".new");

            // Simulate an older sparse settings file with no newer alert/default fields.
            File.WriteAllText(path, "{\"queuePopAlert\":true,\"readyCheckAlert\":true}");
            var upgraded = new SettingsStore(path).Load();
            Assert(upgraded.PartyInviteAlert && upgraded.PartyRequestAlert,
                "older settings without party fields upgrade with both alerts enabled");
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
