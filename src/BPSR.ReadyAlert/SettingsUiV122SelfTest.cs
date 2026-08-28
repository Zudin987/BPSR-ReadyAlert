using System.Drawing;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal static class SettingsUiV122SelfTest
{
    internal static void Run()
    {
        TestSettingsNavigationAndDensity();
        TestCompactTabEditor();
    }

    private static void TestSettingsNavigationAndDensity()
    {
        var chat = new ChatOverlaySettings();
        chat.Normalize();
        var speech = new ChatSpeechTranslationSettings();
        speech.Normalize();

        using var form = new ChatGeneralSettingsForm(chat, speech);
        form.Size = form.MinimumSize;
        _ = form.Handle;
        form.CreateControl();
        PerformLayoutTree(form);

        var metrics = form.GetV122CompactMetricsForSelfTest();
        Check(91, metrics.BufferedHost, "Settings content host is double-buffered");
        Check(92, metrics.SidebarWidth is > 0 and <= 180, "Settings sidebar stays compact");
        Check(93, metrics.FooterHeight is > 0 and <= 62, "Settings footer stays compact");
        Check(94, metrics.MaxRuleHeight <= 70, "Settings multiline rule inputs are not oversized");
        Check(95, metrics.MaxNavHeight <= 36, "Settings navigation uses compact rows");
        Check(96, metrics.SelectedPages == 1 && metrics.ActiveKey == "Appearance",
            "Settings starts with exactly one selected page");

        foreach (var key in new[] { "Interaction", "Alerts", "Speech", "Advanced", "Appearance" })
        {
            form.ShowV122PageForSelfTest(key);
            PerformLayoutTree(form);
            metrics = form.GetV122CompactMetricsForSelfTest();
            Check(97, metrics.SelectedPages == 1, "page switching keeps exactly one selected navigation item");
            Check(98, metrics.ActiveKey == key, "page switching activates only the requested page");
        }

        // Clicking the already-active page must be a no-op instead of forcing another
        // large WinForms visibility/layout cycle.
        form.ShowV122PageForSelfTest("Appearance");
        var repeated = form.GetV122CompactMetricsForSelfTest();
        Check(99, repeated.SelectedPages == 1 && repeated.ActiveKey == "Appearance",
            "reselecting the active Settings page is a stable no-op");

        Environment.ExitCode = 100;
        var save = FindButton(form, "Save changes") ??
            throw new InvalidOperationException("v1.2.2 Settings UI self-test failed: Save changes button missing");
        AssertInsideClient(form, save, "Settings Save button", 100);
    }

    private static void TestCompactTabEditor()
    {
        var tab = new ChatTabSettings
        {
            Name = "Raid",
            Channels = [(int)ChatChannel.World, (int)ChatChannel.Union],
            MinLevel = 1,
            ShowIfMatches = "raid | boss",
            HideIfMatches = "spam"
        };

        using var form = new ChatTabEditorForm(tab, isNew: true);
        var metrics = form.GetV122CompactMetricsForSelfTest();
        Check(101, metrics.DefaultClient.Width <= 740 && metrics.DefaultClient.Height <= 660,
            "Add Chat Tab opens at a compact default size");
        Check(102, metrics.MinimumWindow.Width <= 640 && metrics.MinimumWindow.Height <= 520,
            "Add Chat Tab keeps a compact resizable minimum");
        Check(103, metrics.ChannelsHeight <= 160, "channel picker is compact");
        Check(104, metrics.ShowHeight <= 70 && metrics.HideHeight <= 70, "filter boxes are compact");
        Check(105, metrics.NameWidth <= 320, "tab-name input is bounded instead of stretching across the dialog");
        Check(106, metrics.FooterHeight <= 60, "tab-editor footer is compact");
        Check(107, metrics.CancelText == "Cancel", "tab editor clearly labels the discard action as Cancel");

        form.Size = form.MinimumSize;
        _ = form.Handle;
        form.CreateControl();
        PerformLayoutTree(form);
        Environment.ExitCode = 108;
        var save = FindButton(form, "Save tab") ??
            throw new InvalidOperationException("v1.2.2 Settings UI self-test failed: Save tab button missing");
        Environment.ExitCode = 109;
        var cancel = FindButton(form, "Cancel") ??
            throw new InvalidOperationException("v1.2.2 Settings UI self-test failed: Cancel button missing");
        AssertInsideClient(form, save, "tab-editor Save button", 110);
        AssertInsideClient(form, cancel, "tab-editor Cancel button", 111);
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

    private static void AssertInsideClient(Form form, Control control, string name, int code)
    {
        var rect = control.Bounds;
        for (Control? parent = control.Parent; parent is not null && !ReferenceEquals(parent, form); parent = parent.Parent)
            rect.Offset(parent.Left, parent.Top);

        var client = form.ClientRectangle;
        Check(code,
            rect.Left >= client.Left - 1 && rect.Top >= client.Top - 1 &&
            rect.Right <= client.Right + 1 && rect.Bottom <= client.Bottom + 1,
            name + " stays inside the client area at minimum size");
    }

    private static void Check(int code, bool condition, string name)
    {
        Environment.ExitCode = code;
        if (!condition) throw new InvalidOperationException("v1.2.2 Settings UI self-test failed: " + name);
    }
}
