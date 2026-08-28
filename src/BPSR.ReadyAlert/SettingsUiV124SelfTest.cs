using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal static class SettingsUiV124SelfTest
{
    internal static void Run()
    {
        TestHiddenPrewarmAndReuse();
        TestRealizedTabSwitching();
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

        // First display installs the existing dirty-state tracker. Hide rather than
        // dispose to model the cached overlay-owned dialog.
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

        // A second prepare must be idempotent because every later gear click uses it.
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

    private static void Check(int code, bool condition, string name)
    {
        if (condition) return;
        Environment.ExitCode = code;
        throw new InvalidOperationException("v1.2.4 Settings performance self-test failed: " + name);
    }
}
