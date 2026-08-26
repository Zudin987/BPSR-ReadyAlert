using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal static class ChatRc8SelfTest
{
    internal static void Run()
    {
        TestChatSoundVolumeNormalization();
        TestSettingsApplyStaysOpen();
        TestTabApplyStaysOpen();
    }

    private static void TestChatSoundVolumeNormalization()
    {
        var settings = new ChatOverlaySettings { ChatSoundVolume = -20 };
        settings.Normalize();
        Assert(settings.ChatSoundVolume == 0, "chat sound volume lower clamp");

        settings.ChatSoundVolume = 500;
        settings.Normalize();
        Assert(settings.ChatSoundVolume == 100, "chat sound volume upper clamp");
    }

    private static void TestSettingsApplyStaysOpen()
    {
        var settings = new ChatOverlaySettings();
        settings.Normalize();
        using var form = new ChatGeneralSettingsForm(settings);
        _ = form.Handle;
        form.CreateControl();
        PerformLayoutTree(form);

        var save = FindButton(form, "Save changes") ?? throw new InvalidOperationException("RC8 self-test: Save changes button missing");
        var reset = FindButton(form, "Reset to defaults") ?? throw new InvalidOperationException("RC8 self-test: Reset to defaults button missing");
        var close = FindButton(form, "Close") ?? throw new InvalidOperationException("RC8 self-test: Settings Close button missing");
        Assert(save.DialogResult == DialogResult.None, "settings Save does not carry a closing DialogResult");
        Assert(reset.DialogResult == DialogResult.None, "settings Reset does not carry a closing DialogResult");
        Assert(close.DialogResult == DialogResult.Cancel, "settings Close is the explicit closing action");

        save.PerformClick();
        Assert(!form.IsDisposed && form.DialogResult == DialogResult.None, "settings Save applies without closing the dialog");
    }

    private static void TestTabApplyStaysOpen()
    {
        var tab = new ChatTabSettings
        {
            Name = "World",
            Channels = [(int)ChatChannel.World],
            MinLevel = 50,
            ShowIfMatches = "PA"
        };
        using var form = new ChatTabEditorForm(tab, isNew: false);
        _ = form.Handle;
        form.CreateControl();
        PerformLayoutTree(form);

        var save = FindButton(form, "Save tab") ?? throw new InvalidOperationException("RC8 self-test: Save tab button missing");
        var close = FindButton(form, "Close") ?? throw new InvalidOperationException("RC8 self-test: tab editor Close button missing");
        Assert(save.DialogResult == DialogResult.None, "tab Save does not carry a closing DialogResult");
        Assert(close.DialogResult == DialogResult.Cancel, "tab Close is the explicit closing action");

        save.PerformClick();
        Assert(!form.IsDisposed && form.DialogResult == DialogResult.None, "tab Save applies without closing the editor");
        Assert(tab.ShowIfMatches == "PA", "tab Apply preserves short filter content");
    }

    private static Button? FindButton(Control parent, string text)
    {
        foreach (Control child in parent.Controls)
        {
            if (child is Button button && string.Equals(button.Text, text, StringComparison.Ordinal)) return button;
            var nested = FindButton(child, text);
            if (nested is not null) return nested;
        }
        return null;
    }

    private static void PerformLayoutTree(Control parent)
    {
        parent.PerformLayout();
        foreach (Control child in parent.Controls) PerformLayoutTree(child);
    }

    private static void Assert(bool condition, string name)
    {
        if (!condition) throw new InvalidOperationException("RC8 self-test failed: " + name);
    }
}
