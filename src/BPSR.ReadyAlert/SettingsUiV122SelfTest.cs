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
        var initial = form.GetV122DpiSafeMetricsForSelfTest();
        Check(91, initial.BufferedHost, "Settings content host is double-buffered");
        Check(92, initial.SidebarWidth == 0, "Settings no longer reserves a left sidebar");
        Check(96, initial.SelectedPages == 1 && initial.ActiveKey == "Appearance",
            "Settings starts with exactly one selected page");

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

        form.ShowV122PageForSelfTest("Appearance");
        var repeated = form.GetV122DpiSafeMetricsForSelfTest();
        Check(99, repeated.SelectedPages == 1 && repeated.ActiveKey == "Appearance",
            "reselecting the active Settings page is a stable no-op");

        var save = RequireButton(form, "Save", 100);
        var close = RequireButton(form, "Close", 101);
        var reset = RequireButton(form, "Reset", 102);
        AssertInsideClient(form, save, "Settings Save button", 103);
        AssertInsideClient(form, close, "Settings Close button", 104);
        AssertInsideClient(form, reset, "Settings Reset button", 105);
        AssertButtonTextFits(save, "Settings Save button", 106);
        AssertButtonTextFits(close, "Settings Close button", 107);
        AssertButtonTextFits(reset, "Settings Reset button", 108);

        foreach (var text in new[] { "Appearance", "Interaction", "Alerts", "Speech", "Advanced" })
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
        var initial = form.GetV122CompactMetricsForSelfTest();
        Check(111, initial.DefaultClient.Width > 0 && initial.DefaultClient.Height > 0,
            "Add Chat Tab has a valid default client size");
        Check(112, initial.MinimumWindow.Width > 0 && initial.MinimumWindow.Height > 0,
            "Add Chat Tab has a valid resizable minimum");
        Check(113, initial.ChannelsHeight > 0, "channel picker has usable height");
        Check(114, initial.ShowHeight > 0 && initial.HideHeight > 0, "filter boxes have usable height");
        Check(115, initial.NameWidth > 0, "tab-name input has usable width");
        Check(116, initial.FooterHeight > 0, "tab-editor footer is present");
        Check(117, initial.CancelText == "Cancel", "tab editor clearly labels the discard action as Cancel");

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
        var result = FindButton(parent, text);
        if (result is not null) return result;
        Fail(code, $"'{text}' button missing");
        throw new InvalidOperationException();
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
        if (!condition) Fail(code, name);
    }

    private static void Fail(int code, string name)
    {
        Environment.ExitCode = code;
        throw new InvalidOperationException("Settings UI self-test failed: " + name);
    }
}
