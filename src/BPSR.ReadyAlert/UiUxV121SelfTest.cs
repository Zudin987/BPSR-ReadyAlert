using System.Drawing;

namespace BPSR.ReadyAlert;

internal static class UiUxV121SelfTest
{
    internal static void Run()
    {
        TestAudioVolumesStayIndependent();
        TestExplicitReadyQueueVolumeLabel();
        TestVolumeControlsStaySeparatedInUi();
        TestHiddenGarbageCannotTriggerChatSounds();
        TestBlockedUsersSuppressAllChatOutputs();
        TestMutedTtsToolbarState();
        TestSettingsDirtyState();
        TestSettingsSaveResultIsTruthful();
        TestChatUiDrainIsBounded();
        TestSupportDialogMicrocopy();
    }

    private static void TestAudioVolumesStayIndependent()
    {
        var settings = new AppSettings
        {
            AlertVolume = 20,
            Chat = new ChatOverlaySettings { ChatSoundVolume = 40 },
            SpeechTranslation = new ChatSpeechTranslationSettings { TtsVolume = 60 }
        };
        settings.Chat.Normalize();
        settings.SpeechTranslation.Normalize();

        settings.AlertVolume = 10;
        Assert(settings.Chat.ChatSoundVolume == 40 && settings.SpeechTranslation.TtsVolume == 60,
            "changing Ready / Queue volume does not alter Chat alert or TTS volume");

        settings.Chat.ChatSoundVolume = 30;
        settings.Chat.Normalize();
        Assert(settings.AlertVolume == 10 && settings.SpeechTranslation.TtsVolume == 60,
            "changing Chat alert volume does not alter Ready / Queue or TTS volume");

        settings.SpeechTranslation.TtsVolume = 50;
        settings.SpeechTranslation.Normalize();
        Assert(settings.AlertVolume == 10 && settings.Chat.ChatSoundVolume == 30,
            "changing TTS volume does not alter Ready / Queue or Chat alert volume");
    }

    private static void TestExplicitReadyQueueVolumeLabel()
    {
        var label = TrayApplicationContext.ReadyQueueVolumeMenuText(35);
        Assert(label.Contains("Ready", StringComparison.Ordinal) &&
               label.Contains("Queue", StringComparison.Ordinal) &&
               label.Contains("35%", StringComparison.Ordinal),
            "tray volume label explicitly identifies Ready / Queue sounds");
        Assert(!label.Equals("Alert Volume: 35%", StringComparison.Ordinal),
            "tray no longer uses the ambiguous generic Alert Volume label");
    }

    private static void TestVolumeControlsStaySeparatedInUi()
    {
        var chat = new ChatOverlaySettings();
        chat.Normalize();
        var speech = new ChatSpeechTranslationSettings();
        speech.Normalize();

        using var form = new ChatGeneralSettingsForm(chat, speech);
        var layout = form.GetV121VolumeSeparationForSelfTest();
        Assert(layout.DistinctControls,
            "Chat alert and TTS volumes are physically different slider controls");
        Assert(layout.ChatOnAlertsPage && layout.TtsOnSpeechPage,
            "Chat alert and TTS volume sliders remain on separate Settings pages and cannot overlap");
        Assert(layout.ChatName.Contains("Chat alert", StringComparison.OrdinalIgnoreCase) &&
               layout.TtsName.Contains("TTS", StringComparison.Ordinal),
            "both independent volume sliders have unambiguous accessibility names");
    }

    private static void TestHiddenGarbageCannotTriggerChatSounds()
    {
        var settings = new ChatOverlaySettings
        {
            PrivateSoundEnabled = true,
            HighlightSoundRules =
            [
                new ChatSoundRule { Enabled = true, Match = "sprite | Hypertext" }
            ]
        };
        settings.Normalize();

        var sprite = new ChatMessageEvent(
            1, "Tester", 80, ChatChannel.Private, DateTime.Now,
            ChatMessageKind.Text, "<sprite=31>", 1);
        var hypertext = sprite with
        {
            SenderId = 2,
            Kind = ChatMessageKind.Hypertext,
            Text = "[Hypertext 1050001] Item",
            SequenceId = 2
        };
        var normalPrivate = sprite with
        {
            SenderId = 3,
            Text = "hello party",
            SequenceId = 3
        };

        ChatContentVisibility.Configure(hideEmoji: true, hideLinkedItems: true);
        try
        {
            Assert(!ChatNotificationEngine.EvaluateForSelfTest(settings, sprite),
                "emoji-only rows hidden by cleanup cannot still trigger chat sounds");
            Assert(!ChatNotificationEngine.EvaluateForSelfTest(settings, hypertext),
                "hidden Hypertext rows cannot still trigger chat sounds");
            Assert(ChatNotificationEngine.EvaluateForSelfTest(settings, normalPrivate),
                "normal Private chat remains eligible for its configured sound");
        }
        finally
        {
            ChatContentVisibility.Configure(hideEmoji: false, hideLinkedItems: false);
        }
    }

    private static void TestBlockedUsersSuppressAllChatOutputs()
    {
        var settings = new ChatOverlaySettings
        {
            PrivateSoundEnabled = true,
            BlockedUsers = [new ChatBlockedUser { Id = 77, Name = "Blocked" }]
        };
        settings.Normalize();
        ChatNotificationEngine.Configure(settings, string.Empty);

        try
        {
            var blocked = new ChatMessageEvent(
                77, "Blocked", 80, ChatChannel.Union, DateTime.Now,
                ChatMessageKind.Text, "hello guild", 701);
            var normal = blocked with
            {
                SenderId = 78,
                SenderName = "Allowed",
                SequenceId = 702
            };

            Assert(ChatNotificationEngine.IsSenderBlocked(blocked.SenderId),
                "blocked player ID is exposed consistently to shared chat routing");
            Assert(!ChatCaptureBridge.ShouldRouteToSpeechForSelfTest(blocked),
                "blocked player messages never reach translation or TTS routing");
            Assert(ChatCaptureBridge.ShouldRouteToSpeechForSelfTest(normal),
                "unblocked normal chat remains eligible for translation/TTS routing");
            Assert(!ChatNotificationEngine.EvaluateForSelfTest(settings, blocked),
                "blocked player messages cannot trigger keyword/private chat sounds");
        }
        finally
        {
            var reset = new ChatOverlaySettings();
            reset.Normalize();
            ChatNotificationEngine.Configure(reset, string.Empty);
        }
    }

    private static void TestMutedTtsToolbarState()
    {
        var tempPath = Path.Combine(Path.GetTempPath(), $"BPSR-ReadyAlert-v121-toolbar-{Guid.NewGuid():N}.json");
        try
        {
            var settings = new AppSettings { ChatOverlayEnabled = true };
            settings.Chat.Normalize();
            settings.SpeechTranslation.TtsEnabled = true;
            settings.SpeechTranslation.TtsGuild = true;
            settings.SpeechTranslation.TtsPartyTeam = false;
            settings.SpeechTranslation.TtsVolume = 0;
            settings.SpeechTranslation.Normalize();

            using var form = new ChatOverlayForm(settings, new SettingsStore(tempPath), string.Empty, string.Empty);
            var muted = form.GetV120TtsToolbarStateForSelfTest();
            Assert(muted.Enabled && !muted.Strikeout,
                "muted TTS remains an enabled master switch rather than pretending to be OFF");
            Assert(muted.Background.R > muted.Background.G && muted.Background.G > muted.Background.B,
                "enabled-but-muted TTS uses the amber warning state rather than green active state");

            settings.SpeechTranslation.TtsVolume = 70;
            settings.SpeechTranslation.TtsGuild = false;
            settings.SpeechTranslation.TtsPartyTeam = false;
            form.ApplySettingsFromOpenDialog();
            var noChannels = form.GetV120TtsToolbarStateForSelfTest();
            Assert(noChannels.Enabled && !noChannels.Strikeout &&
                   noChannels.Background.R > noChannels.Background.G &&
                   noChannels.Background.G > noChannels.Background.B,
                "TTS with no selected speech channels also uses the amber inactive state");
        }
        finally
        {
            TryDelete(tempPath);
            TryDelete(tempPath + ".bak");
            TryDelete(tempPath + ".new");
        }
    }

    private static void TestSettingsDirtyState()
    {
        var chat = new ChatOverlaySettings();
        chat.Normalize();
        var speech = new ChatSpeechTranslationSettings();
        speech.Normalize();

        using var form = new ChatGeneralSettingsForm(chat, speech);
        Assert(form.GetV121CancelButtonTextForSelfTest() == "Cancel",
            "settings footer uses Cancel rather than ambiguous Close for unapplied edits");
        Assert(string.IsNullOrEmpty(form.GetV121SaveStateForSelfTest()),
            "newly opened settings do not falsely claim there are unsaved edits");

        var changedVolume = speech.TtsVolume == 65 ? 60 : 65;
        form.SetV121TtsVolumeForSelfTest(changedVolume);
        Assert(form.GetV121SaveStateForSelfTest() == "Unsaved",
            "editing TTS volume immediately clears stale Saved state and marks the editor dirty");
    }

    private static void TestSettingsSaveResultIsTruthful()
    {
        var goodPath = Path.Combine(Path.GetTempPath(), $"BPSR-ReadyAlert-v121-save-{Guid.NewGuid():N}.json");
        var blocker = Path.Combine(Path.GetTempPath(), $"BPSR-ReadyAlert-v121-blocker-{Guid.NewGuid():N}");

        try
        {
            var goodStore = new SettingsStore(goodPath);
            Assert(goodStore.Save(new AppSettings()),
                "settings store reports true only after a normal durable save succeeds");

            File.WriteAllText(blocker, "not a directory");
            var badStore = new SettingsStore(Path.Combine(blocker, "settings.json"));
            Assert(!badStore.Save(new AppSettings()),
                "settings store reports false when Windows cannot persist the file");
        }
        finally
        {
            TryDelete(goodPath);
            TryDelete(goodPath + ".bak");
            TryDelete(goodPath + ".new");
            TryDelete(blocker);
        }
    }

    private static void TestChatUiDrainIsBounded()
    {
        Assert(TrayApplicationContext.ChatUiDrainLimitForSelfTest is > 0 and <= 250,
            "chat UI work per WinForms timer tick is explicitly bounded to protect responsiveness during bursts");
    }

    private static void TestSupportDialogMicrocopy()
    {
        Assert(BlockedUsersForm.ScopeText.Contains("translation", StringComparison.OrdinalIgnoreCase) &&
               BlockedUsersForm.ScopeText.Contains("TTS", StringComparison.Ordinal) &&
               BlockedUsersForm.ScopeText.Contains("Ready / Queue", StringComparison.Ordinal),
            "blocked-user dialog accurately explains every ReadyAlert chat output it suppresses");
        Assert(ChatDebugStatusForm.LiveUpdateHint.Contains("pause", StringComparison.OrdinalIgnoreCase),
            "diagnostics explains that live refresh pauses during selection/scroll interaction");
    }

    private static void TryDelete(string path)
    {
        try
        {
            if (File.Exists(path)) File.Delete(path);
        }
        catch { }
    }

    private static void Assert(bool condition, string name)
    {
        if (!condition) throw new InvalidOperationException("v1.2.1 UI/UX self-test failed: " + name);
    }
}
