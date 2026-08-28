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

        // WinForms can apply DPI autoscaling during construction, before Handle is
        // explicitly requested. Validate density as a proportion of the actual
        // client size instead of assuming raw 96-DPI pixel values. The second phase
        // below then verifies the scaled controls remain usable at minimum size.
        var logical = form.GetV122DpiSafeMetricsForSelfTest();
        Check(91, logical.BufferedHost, "Settings content host is double-buffered");
        Check(92, logical.SidebarWidth > 0 && logical.SidebarWidth <= form.ClientSize.Width * 0.22,
            "Settings sidebar stays compact relative to the window");
        Check(93, logical.FooterHeight > 0 && logical.FooterHeight <= form.ClientSize.Height * 0.12,
            "Settings footer stays compact relative to the window");
        Check(94, logical.MaxRuleHeight <= form.ClientSize.Height * 0.13,
            "Settings multiline rule inputs are not oversized");
        Check(95, logical.MaxNavHeight <= form.ClientSize.Height * 0.08,
            "Settings navigation uses compact rows");
        Check(96, logical.SelectedPages == 1 && logical.ActiveKey == "Appearance",
            "Settings starts with exactly one selected page");
        Check(124, form.GetV122MaxBoundedInputWidthForSelfTest() <= form.ClientSize.Width * 0.40,
            "Settings single-line editor fields stay bounded instead of filling the whole page");

        _ = form.Handle;
        form.CreateControl();
        form.Size = form.MinimumSize;
        PerformLayoutTree(form);

        foreach (var key in new[] { "Interaction", "Alerts", "Speech", "Advanced", "Appearance" })
        {
            form.ShowV122PageForSelfTest(key);
            PerformLayoutTree(form);
            var metrics = form.GetV122DpiSafeMetricsForSelfTest();
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
        var logical = form.GetV122CompactMetricsForSelfTest();
        Check(111, logical.DefaultClient.Width > 0 && logical.DefaultClient.Height > 0,
            "Add Chat Tab has a valid compact default client size");
        Check(112, logical.MinimumWindow.Width <= logical.DefaultClient.Width &&
                   logical.MinimumWindow.Height <= logical.DefaultClient.Height,
            "Add Chat Tab minimum stays below its default size");
        Check(113, logical.ChannelsHeight <= logical.DefaultClient.Height * 0.30,
            "channel picker stays compact relative to the dialog");
        Check(114, logical.ShowHeight <= logical.DefaultClient.Height * 0.14 &&
                   logical.HideHeight <= logical.DefaultClient.Height * 0.14,
            "filter boxes stay compact relative to the dialog");
        Check(115, logical.NameWidth <= logical.DefaultClient.Width * 0.50,
            "tab-name input is bounded instead of stretching across the dialog");
        Check(116, logical.FooterHeight <= logical.DefaultClient.Height * 0.12,
            "tab-editor footer stays compact relative to the dialog");
        Check(117, logical.CancelText == "Cancel", "tab editor clearly labels the discard action as Cancel");

        _ = form.Handle;
        form.CreateControl();
        form.Size = form.MinimumSize;
        PerformLayoutTree(form);
        var save = RequireButton(form, "Save tab", 118);
        var cancel = RequireButton(form, "Cancel", 119);
        AssertInsideClient(form, save, "tab-editor Save button", 120);
        AssertInsideClient(form, cancel, "tab-editor Cancel button", 121);
        AssertButtonTextFits(save, "tab-editor Save button", 122);
        AssertButtonTextFits(cancel, "tab-editor Cancel button", 123);
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
