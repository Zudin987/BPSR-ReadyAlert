using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal static class SettingsUiV124SelfTest
{
    internal static void Run()
    {
        TestHiddenPrewarmAndReuse();
        TestRealizedTabSwitching();
        TestScreenshotDrivenSimplification();
        TestSavedStateClosesWithoutWarning();
        TestCompactChannelEditor();
    }

    private static void TestHiddenPrewarmAndReuse()
    {
        var chat = new ChatOverlaySettings();
        chat.Normalize();
        var speech = new ChatSpeechTranslationSettings();
        speech.Normalize();
        var sourceTtsVolume = speech.TtsVolume;

        using var owner = new Form { ShowInTaskbar = false, Opacity = 0d };
        owner.Show();
        Application.DoEvents();

        using var form = new ChatGeneralSettingsForm(chat, speech)
        {
            ShowInTaskbar = false,
            Opacity = 0d
        };

        Check(140, form.AreV124InstalledFontsDeferredForSelfTest(),
            "constructing Settings does not enumerate the full installed-font collection");

        form.PrewarmV124ForOwner(owner);
        var prewarmed = form.GetV124ReuseStateForSelfTest();
        Check(141, prewarmed.HandleReady && !prewarmed.Visible,
            "Settings can realize its handles and layout while remaining hidden");
        Check(142, form.AreV124InstalledFontsDeferredForSelfTest(),
            "hidden prewarm still defers the installed-font scan until the dropdown is used");

        form.Show();
        Application.DoEvents();
        form.Hide();
        Application.DoEvents();

        var dirtyVolume = sourceTtsVolume >= 5 ? sourceTtsVolume - 5 : sourceTtsVolume + 5;
        form.SetV121TtsVolumeForSelfTest(dirtyVolume);
        Check(143, form.GetV121SaveStateForSelfTest() == "Unsaved",
            "cached Settings still detects an unapplied edit");

        form.DialogResult = DialogResult.Cancel;
        form.PrepareV124ForOpen(owner);
        var prepared = form.GetV124ReuseStateForSelfTest();
        Check(144, prepared.HandleReady && !prepared.Visible,
            "preparing a cached Settings dialog does not recreate or show it");
        Check(145, prepared.Result == DialogResult.None && prepared.TtsVolume == sourceTtsVolume,
            "reopening cached Settings discards prior unapplied controls and resets DialogResult");
        Check(146, form.GetV121SaveStateForSelfTest() != "Unsaved",
            "reopening cached Settings refreshes the dirty-state baseline");

        form.PrepareV124ForOpen(owner);
        var preparedAgain = form.GetV124ReuseStateForSelfTest();
        Check(147, preparedAgain.HandleReady && preparedAgain.TtsVolume == sourceTtsVolume,
            "repeated cached Settings preparation is stable");
    }

    private static void TestRealizedTabSwitching()
    {
        var chat = new ChatOverlaySettings();
        chat.Normalize();
        var speech = new ChatSpeechTranslationSettings();
        speech.Normalize();

        using var form = new ChatGeneralSettingsForm(chat, speech)
        {
            ShowInTaskbar = false,
            Opacity = 0d
        };
        form.Show();
        Application.DoEvents();
        form.PerformLayout();
        Application.DoEvents();

        var before = form.GetV124PageSwitchStateForSelfTest();
        Check(131, before.PageCount == 5, "all five Settings pages are registered");
        Check(132, before.VisiblePages == 5, "all Settings pages stay realized instead of hide/show switching");
        Check(133, before.ActivePages == 1 && before.ActiveKey == "Appearance",
            "exactly one realized Settings page is active initially");
        Check(134, before.FrontKey == "Appearance", "active Settings page starts at the front of z-order");

        var visibleChanged = 0;
        var layoutEvents = 0;
        form.SubscribeV124VisibleChangedForSelfTest(() => visibleChanged++);
        form.SubscribeV124LayoutForSelfTest(() => layoutEvents++);

        foreach (var key in new[]
                 {
                     "Interaction", "Alerts", "Speech", "Advanced", "Appearance",
                     "Speech", "Alerts", "Interaction", "Advanced", "Appearance"
                 })
        {
            form.ShowV122PageForSelfTest(key);
            var state = form.GetV124PageSwitchStateForSelfTest();
            Check(135, state.VisiblePages == state.PageCount,
                "tab switching never hides or re-shows a Settings page");
            Check(136, state.ActivePages == 1 && state.ActiveKey == key,
                "tab switching keeps exactly one logical active page");
            Check(137, state.FrontKey == key,
                "tab switching changes only the front page/z-order");
        }

        Check(138, visibleChanged == 0,
            "repeated Settings tab switching does not trigger VisibleChanged cascades");
        Check(139, layoutEvents == 0,
            "repeated Settings tab switching does not relayout the content host or page trees");

        form.Hide();
    }

    private static void TestScreenshotDrivenSimplification()
    {
        var chat = new ChatOverlaySettings();
        chat.Normalize();
        var speech = new ChatSpeechTranslationSettings();
        speech.Normalize();

        using var form = new ChatGeneralSettingsForm(chat, speech)
        {
            ShowInTaskbar = false,
            Opacity = 0d
        };
        form.Show();
        Application.DoEvents();

        var ui = form.GetV124SimplifiedUiForSelfTest();
        Check(148, ui.OnlyWindowOpacity,
            "Appearance exposes only Window opacity and removes background/toolbar/text opacity controls");
        Check(149, ui.CombinedCleanup,
            "Interaction exposes one combined emoji/link cleanup checkbox");
        Check(150, ui.ClickThroughCheckboxRemoved,
            "Interaction no longer exposes a Click-through state checkbox");
        Check(151, ui.CollapseHotkeyRemoved,
            "Interaction no longer exposes the collapse/expand hotkey editor");
        Check(152, ui.HighlightSingleLine,
            "visual highlight expression is a one-line input");
        Check(153, ui.TwoSoundRulesOnly,
            "Alerts exposes only sound rules 1 and 2");
        Check(154, ui.SoundRulesSingleLine,
            "both sound rule match expressions are one-line inputs");

        Check(155, form.ToggleV124CheckboxForSelfTest("Always on top"),
            "active Settings checkboxes are enabled and toggle state reliably");
        Check(156, form.ToggleV124CheckboxForSelfTest("Hide emoji-only + linked items / Hypertext"),
            "combined cleanup checkbox is enabled and toggles state reliably");

        form.Hide();
    }

    private static void TestSavedStateClosesWithoutWarning()
    {
        var chat = new ChatOverlaySettings();
        chat.Normalize();
        var speech = new ChatSpeechTranslationSettings();
        speech.Normalize();

        using var form = new ChatGeneralSettingsForm(chat, speech)
        {
            ShowInTaskbar = false,
            Opacity = 0d
        };
        form.Show();
        Application.DoEvents();

        var changedVolume = speech.TtsVolume == 72 ? 71 : 72;
        form.SetV121TtsVolumeForSelfTest(changedVolume);
        Check(157, form.GetV121CloseWarningKindForSelfTest() == "dirty",
            "an actual unsaved change still requires a close warning");

        // Model the successful Save baseline directly: after saving, closing must be
        // silent unless another edit occurs or persistence itself failed.
        speech.TtsVolume = changedVolume;
        speech.Normalize();
        form.MarkV121SavedForSelfTest();
        Check(158, form.GetV121CloseWarningKindForSelfTest().Length == 0,
            "a successfully saved Settings state closes without an unapplied-changes warning");
        Check(159, form.GetV121SaveStateForSelfTest() == "Saved ✓",
            "successful Save remains visibly marked as saved rather than flipping back to Unsaved");

        form.Hide();
    }

    private static void TestCompactChannelEditor()
    {
        var tab = new ChatTabSettings
        {
            Name = "Raid",
            Channels = [(int)ChatChannel.World, (int)ChatChannel.Newbie, (int)ChatChannel.Union],
            ShowIfMatches = "raid | boss",
            HideIfMatches = "spam"
        };
        using var form = new ChatTabEditorForm(tab, isNew: false);
        var contract = form.GetV124ChannelEditorForSelfTest();

        Check(160, contract.Labels.SequenceEqual(new[]
        {
            "World + Newbie", "Guild", "Team + Group", "Private", "Local"
        }), "Chat Tab editor exposes exactly the requested five grouped channel choices");
        Check(161, contract.SingleLineShow && contract.SingleLineHide,
            "Chat Tab Show/Hide filters are one-line inputs");
        Check(162, !contract.HasScrollableChannelList,
            "Chat Tab editor removes the old scrollable channel checklist");
    }

    private static void Check(int code, bool condition, string name)
    {
        if (condition) return;
        Environment.ExitCode = code;
        throw new InvalidOperationException("v1.2.4 Settings performance self-test failed: " + name);
    }
}
