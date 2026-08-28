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
