using System.Drawing;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal static class ChatUiSelfTest
{
    internal static void Run()
    {
        TestTabEditorCreatesAndKeepsFooterVisible();
        TestSettingsShellCreatesAtMinimumSize();
        TestThemeControlsCreateHandles();
    }

    private static void TestTabEditorCreatesAndKeepsFooterVisible()
    {
        var tab = new ChatTabSettings
        {
            Name = "World",
            Channels = [(int)ChatChannel.World],
            MinLevel = 50,
            ShowIfMatches = "serum | food | raid",
            HideIfMatches = "spam"
        };
        using var form = new ChatTabEditorForm(tab, isNew: false);
        form.Size = form.MinimumSize;
        _ = form.Handle;
        form.CreateControl();
        form.PerformLayout();

        var save = FindButton(form, "Save tab") ?? throw new InvalidOperationException("Chat UI self-test failed: tab editor Save tab button missing");
        AssertInsideClient(form, save, "tab editor Save");
        Assert(save.Enabled, "tab editor valid filters keep Save enabled");
    }

    private static void TestSettingsShellCreatesAtMinimumSize()
    {
        var settings = new ChatOverlaySettings();
        settings.Normalize();
        using var form = new ChatGeneralSettingsForm(settings);
        form.Size = form.MinimumSize;
        _ = form.Handle;
        form.CreateControl();
        form.PerformLayout();

        var save = FindButton(form, "Save changes") ?? throw new InvalidOperationException("Chat UI self-test failed: settings Save changes button missing");
        AssertInsideClient(form, save, "settings Save");
        Assert(FindButton(form, "Appearance") is not null, "settings Appearance navigation exists");
        Assert(FindButton(form, "Interaction") is not null, "settings Interaction navigation exists");
        Assert(FindButton(form, "Highlights & sounds") is not null, "settings alerts navigation exists");
        Assert(FindButton(form, "Advanced") is not null, "settings Advanced navigation exists");
    }

    private static void TestThemeControlsCreateHandles()
    {
        using var tab = new ChatTabButton { Text = "World", Selected = true };
        using var nav = new ChatNavButton { Text = "Appearance", Selected = true };
        _ = tab.Handle;
        _ = nav.Handle;
        Assert(tab.IsHandleCreated && nav.IsHandleCreated, "RC5 themed buttons create native handles");
    }

    private static Button? FindButton(Control parent, string text)
    {
        foreach (Control child in parent.Controls)
        {
            if (child is Button button && string.Equals(button.Text, text, StringComparison.Ordinal))
                return button;
            var nested = FindButton(child, text);
            if (nested is not null) return nested;
        }
        return null;
    }

    private static void AssertInsideClient(Form form, Control control, string name)
    {
        var screen = control.RectangleToScreen(control.ClientRectangle);
        var client = form.RectangleToScreen(form.ClientRectangle);
        Assert(screen.Left >= client.Left - 1, name + " left edge stays inside client area");
        Assert(screen.Top >= client.Top - 1, name + " top edge stays inside client area");
        Assert(screen.Right <= client.Right + 1, name + " right edge stays inside client area");
        Assert(screen.Bottom <= client.Bottom + 1, name + " bottom edge stays inside client area");
    }

    private static void Assert(bool condition, string name)
    {
        if (!condition) throw new InvalidOperationException("Chat UI self-test failed: " + name);
    }
}
