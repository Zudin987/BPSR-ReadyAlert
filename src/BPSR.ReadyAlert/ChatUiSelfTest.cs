using System.Drawing;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal static class ChatUiSelfTest
{
    internal static void Run()
    {
        TestTabEditorCreatesAndKeepsFooterVisible();
        TestSettingsShellCreatesAtMinimumSize();
        TestSupportDialogsCreateAtMinimumSize();
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
        PrepareAtMinimumSize(form);

        var save = FindButton(form, "Save tab") ?? throw new InvalidOperationException("Chat UI self-test failed: tab editor Save tab button missing");
        AssertInsideClient(form, save, "tab editor Save");
        Assert(save.Enabled, "tab editor valid filters keep Save enabled");
    }

    private static void TestSettingsShellCreatesAtMinimumSize()
    {
        var settings = new ChatOverlaySettings();
        settings.Normalize();
        using var form = new ChatGeneralSettingsForm(settings);
        PrepareAtMinimumSize(form);

        var save = FindButton(form, "Save changes") ?? throw new InvalidOperationException("Chat UI self-test failed: settings Save changes button missing");
        AssertInsideClient(form, save, "settings Save");
        Assert(FindButton(form, "Appearance") is not null, "settings Appearance navigation exists");
        Assert(FindButton(form, "Interaction") is not null, "settings Interaction navigation exists");
        Assert(FindButton(form, "Highlights & sounds") is not null, "settings alerts navigation exists");
        Assert(FindButton(form, "Advanced") is not null, "settings Advanced navigation exists");
    }

    private static void TestSupportDialogsCreateAtMinimumSize()
    {
        using (var colors = new ChannelColorsForm([]))
        {
            PrepareAtMinimumSize(colors);
            var save = FindButton(colors, "Save colors") ?? throw new InvalidOperationException("Chat UI self-test failed: channel colors Save button missing");
            AssertInsideClient(colors, save, "channel colors Save");
        }

        using (var blocked = new BlockedUsersForm([]))
        {
            PrepareAtMinimumSize(blocked);
            var done = FindButton(blocked, "Done") ?? throw new InvalidOperationException("Chat UI self-test failed: blocked users Done button missing");
            AssertInsideClient(blocked, done, "blocked users Done");
        }

        using (var status = new ChatDebugStatusForm())
        {
            PrepareAtMinimumSize(status);
            var done = FindButton(status, "Done") ?? throw new InvalidOperationException("Chat UI self-test failed: capture status Done button missing");
            AssertInsideClient(status, done, "capture status Done");
        }
    }

    private static void TestThemeControlsCreateHandles()
    {
        using var tab = new ChatTabButton { Text = "World", Selected = true };
        using var nav = new ChatNavButton { Text = "Appearance", Selected = true };
        _ = tab.Handle;
        _ = nav.Handle;
        Assert(tab.IsHandleCreated && nav.IsHandleCreated, "RC5 themed buttons create native handles");
    }

    private static void PrepareAtMinimumSize(Form form)
    {
        form.Size = form.MinimumSize;
        _ = form.Handle;
        form.CreateControl();
        PerformLayoutTree(form);
    }

    private static void PerformLayoutTree(Control parent)
    {
        parent.PerformLayout();
        foreach (Control child in parent.Controls)
            PerformLayoutTree(child);
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
        var rect = GetBoundsRelativeToForm(form, control);
        var client = form.ClientRectangle;
        Assert(rect.Left >= client.Left - 1, name + " left edge stays inside client area");
        Assert(rect.Top >= client.Top - 1, name + " top edge stays inside client area");
        Assert(rect.Right <= client.Right + 1, name + " right edge stays inside client area");
        Assert(rect.Bottom <= client.Bottom + 1, name + " bottom edge stays inside client area");
    }

    private static Rectangle GetBoundsRelativeToForm(Form form, Control control)
    {
        var rect = control.Bounds;
        var parent = control.Parent;
        while (parent is not null && !ReferenceEquals(parent, form))
        {
            rect.Offset(parent.Left, parent.Top);
            parent = parent.Parent;
        }
        if (!ReferenceEquals(parent, form))
            throw new InvalidOperationException("Chat UI self-test failed: control is not parented by expected form");
        return rect;
    }

    private static void Assert(bool condition, string name)
    {
        if (!condition) throw new InvalidOperationException("Chat UI self-test failed: " + name);
    }
}
