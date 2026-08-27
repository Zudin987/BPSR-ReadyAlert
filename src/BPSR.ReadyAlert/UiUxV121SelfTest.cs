using System.Drawing;

namespace BPSR.ReadyAlert;

internal static class UiUxV121SelfTest
{
    internal static void Run()
    {
        TestAudioVolumesStayIndependent();
        TestExplicitReadyQueueVolumeLabel();
        TestHiddenGarbageCannotTriggerChatSounds();
        TestMutedTtsToolbarState();
        TestSettingsDirtyState();
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
