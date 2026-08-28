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
        _ = form.Handle;
        form.CreateControl();
        form.Size = form.MinimumSize;
        PerformLayoutTree(form);

        var metrics = form.GetV122DpiSafeMetricsForSelfTest();
        Check(91, metrics.BufferedHost, "Settings content host is double-buffered");
        Check(92, metrics.SidebarWidth > 0 && metrics.SidebarWidth <= ScaleLogical(form, 180),
            "Settings sidebar stays compact");
        Check(93, metrics.FooterHeight > 0 && metrics.FooterHeight <= ScaleLogical(form, 62),
            "Settings footer stays compact");
        Check(94, metrics.MaxRuleHeight <= ScaleLogical(form, 70), "Settings multiline rule inputs are not oversized");
        Check(95, metrics.MaxNavHeight <= ScaleLogical(form, 36), "Settings navigation uses compact rows");
        Check(96, metrics.SelectedPages == 1 && metrics.ActiveKey == "Appearance",
            "Settings starts with exactly one selected page");
        Check(124, form.GetV122MaxBoundedInputWidthForSelfTest() <= ScaleLogical(form, 340),
            "Settings single-line editor fields stay bounded instead of filling the whole page");

        foreach (var key in new[] { "Interaction", "Alerts", "Speech", "Advanced", "Appearance" })
        {
            form.ShowV122PageForSelfTest(key);
            PerformLayoutTree(form);
            metrics = form.GetV122DpiSafeMetricsForSelfTest();
            Check(97, metrics.SelectedPages == 1, "page switching keeps exactly one selected navigation item");
            Check(98, metrics.ActiveKey == key, "page switching activates only the requested page");
        }

        // Clicking the already-active page must be a no-op instead of forcing another
        // large WinForms visibility/layout cycle.
        form.ShowV122PageForSelfTest("Appearance");
        var repeated = form.GetV122DpiSafeMetricsForSelfTest();
        Check(99, repeated.SelectedPages == 1 && repeated.ActiveKey == "Appearance",
            "reselecting the active Settings page is a stable no-op");

        var save = RequireButton(form, "Save changes", 100);
        var cancel = RequireButton(form, "Cancel", 101);
        var reset = RequireButton(form, "Reset defaults", 102);
        AssertInsideClient(form, save, "Settings Save button", 103);
        AssertInsideClient(form, cancel, "Settings Cancel button", 104);
        AssertInsideClient(form, reset, "Settings Reset button", 105);
        AssertButtonTextFits(save, "Settings Save button", 106);
        AssertButtonTextFits(cancel, "Settings Cancel button", 107);
        AssertButtonTextFits(reset, "Settings Reset button", 108);

        foreach (var text in new[] { "Appearance", "Interaction", "Highlights & sounds", "Speech", "Advanced" })
        {
            var nav = RequireButton(form, text, 109);
            AssertButtonTextFits(nav, $"Settings navigation '{text}'", 110);
        }
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
        _ = form.Handle;
        form.CreateControl();
        PerformLayoutTree(form);

        var metrics = form.GetV122CompactMetricsForSelfTest();
        Check(111, metrics.DefaultClient.Width <= ScaleLogical(form, 740) &&
                   metrics.DefaultClient.Height <= ScaleLogical(form, 660),
            "Add Chat Tab opens at a compact default size");
        Check(112, metrics.MinimumWindow.Width <= ScaleLogical(form, 640) &&
                   metrics.MinimumWindow.Height <= ScaleLogical(form, 520),
            "Add Chat Tab keeps a compact resizable minimum");
        Check(113, metrics.ChannelsHeight <= ScaleLogical(form, 160), "channel picker is compact");
        Check(114, metrics.ShowHeight <= ScaleLogical(form, 70) && metrics.HideHeight <= ScaleLogical(form, 70),
            "filter boxes are compact");
        Check(115, metrics.NameWidth <= ScaleLogical(form, 320),
            "tab-name input is bounded instead of stretching across the dialog");
        Check(116, metrics.FooterHeight <= ScaleLogical(form, 60), "tab-editor footer is compact");
        Check(117, metrics.CancelText == "Cancel", "tab editor clearly labels the discard action as Cancel");

        form.Size = form.MinimumSize;
        PerformLayoutTree(form);
        var save = RequireButton(form, "Save tab", 118);
        var cancel = RequireButton(form, "Cancel", 119);
        AssertInsideClient(form, save, "tab-editor Save button", 120);
        AssertInsideClient(form, cancel, "tab-editor Cancel button", 121);
        AssertButtonTextFits(save, "tab-editor Save button", 122);
        AssertButtonTextFits(cancel, "tab-editor Cancel button", 123);
    }

    private static int ScaleLogical(Form form, int logicalPixels)
    {
        var dpi = Math.Max(96, form.DeviceDpi);
        return (int)Math.Ceiling(logicalPixels * dpi / 96d);
    }

    private static void PerformLayoutTree(Control parent)
    {
        parent.PerformLayout();
        foreach (Control child in parent.Controls)
            PerformLayoutTree(child);
    }

    private static Button RequireButton(Control parent, string text, int code)
    {
        Environment.ExitCode = code;
        return FindButton(parent, text) ??
               throw new InvalidOperationException($"v1.2.2 Settings UI self-test failed: '{text}' button missing");
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

    private static void AssertButtonTextFits(Button button, string name, int code)
    {
        var textWidth = TextRenderer.MeasureText(
            button.Text,
            button.Font,
            Size.Empty,
            TextFormatFlags.SingleLine | TextFormatFlags.NoPadding).Width;
        var available = Math.Max(0, button.ClientSize.Width - button.Padding.Horizontal - 4);
        Check(code, textWidth <= available, name + " text fits without clipping");
    }

    private static void Check(int code, bool condition, string name)
    {
        Environment.ExitCode = code;
        if (!condition) throw new InvalidOperationException("v1.2.2 Settings UI self-test failed: " + name);
    }
}
