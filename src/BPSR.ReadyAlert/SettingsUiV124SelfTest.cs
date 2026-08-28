using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal static class SettingsUiV124SelfTest
{
    internal static void Run()
    {
        var chat = new ChatOverlaySettings();
        chat.Normalize();
        var speech = new ChatSpeechTranslationSettings();
        speech.Normalize();

        using var form = new ChatGeneralSettingsForm(chat, speech);
        _ = form.Handle;
        form.CreateControl();
        form.PerformLayout();

        var before = form.GetV124PageSwitchStateForSelfTest();
        Assert(before.PageCount == 5, "all five Settings pages are registered");
        Assert(before.VisiblePages == 5, "all Settings pages stay realized instead of hide/show switching");
        Assert(before.ActivePages == 1 && before.ActiveKey == "Appearance",
            "exactly one realized Settings page is active initially");
        Assert(before.FrontKey == "Appearance", "active Settings page starts at the front of z-order");

        var visibleChanged = 0;
        form.SubscribeV124VisibleChangedForSelfTest(() => visibleChanged++);

        foreach (var key in new[]
                 {
                     "Interaction", "Alerts", "Speech", "Advanced", "Appearance",
                     "Speech", "Alerts", "Interaction", "Advanced", "Appearance"
                 })
        {
            form.ShowV122PageForSelfTest(key);
            var state = form.GetV124PageSwitchStateForSelfTest();
            Assert(state.VisiblePages == state.PageCount,
                "tab switching never hides or re-shows a Settings page");
            Assert(state.ActivePages == 1 && state.ActiveKey == key,
                "tab switching keeps exactly one logical active page");
            Assert(state.FrontKey == key,
                "tab switching changes only the front page/z-order");
        }

        Assert(visibleChanged == 0,
            "repeated Settings tab switching does not trigger VisibleChanged layout cascades");
    }

    private static void Assert(bool condition, string name)
    {
        if (!condition)
            throw new InvalidOperationException("v1.2.4 Settings performance self-test failed: " + name);
    }
}
