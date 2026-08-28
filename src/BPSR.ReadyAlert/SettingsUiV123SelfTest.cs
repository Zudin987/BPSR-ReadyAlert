using System.Drawing;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal static class SettingsUiV123SelfTest
{
    internal static void Run()
    {
        TestMainSettingsShell();
        TestCompactTabEditorStyling();
        TestSettingsStylingStaysScoped();
    }

    private static void TestMainSettingsShell()
    {
        var chat = new ChatOverlaySettings();
        chat.Normalize();
        var speech = new ChatSpeechTranslationSettings();
        speech.Normalize();

        using var form = new ChatGeneralSettingsForm(chat, speech);
        _ = form.Handle;
        form.CreateControl();
        PerformLayoutTree(form);

        Assert(form.Text == "Settings", "Settings window keeps the compact ZDPS-style title");

        var nav = FindTopNavigation(form);
        Assert(nav is not null, "top Settings tab bar exists");
        Assert(nav!.FlowDirection == FlowDirection.LeftToRight && !nav.WrapContents,
            "Settings tabs use one compact horizontal row");
        var navButtons = nav.Controls.OfType<ChatNavButton>().ToList();
        Assert(navButtons.Count == 5, "Settings exposes five compact top tabs");
        Assert(navButtons.Count(x => x.Selected) == 1, "exactly one top tab is selected");

        var compactSliders = FindControls<ChatCompactSlider>(form).ToList();
        Assert(compactSliders.Count >= 6,
            "opacity, chat-alert and TTS settings use compact slider surfaces");
        Assert(compactSliders.All(x => !string.IsNullOrWhiteSpace(x.AccessibleName)),
            "compact sliders retain meaningful accessibility names");

        foreach (var visual in compactSliders)
        {
            var backing = visual.Parent?.Controls.OfType<TrackBar>().FirstOrDefault();
            Assert(backing is not null, "every compact slider keeps a TrackBar backing value");
            Assert(!backing!.Visible, "backing TrackBar is hidden from the compact UI");

            var fromVisual = visual.Minimum < visual.Maximum
                ? Math.Min(visual.Maximum, visual.Minimum + Math.Max(1, (visual.Maximum - visual.Minimum) / 3))
                : visual.Minimum;
            visual.Value = fromVisual;
            Assert(backing.Value == fromVisual, "compact slider writes through to the existing settings value");

            var fromBacking = visual.Minimum < visual.Maximum
                ? Math.Max(visual.Minimum, visual.Maximum - Math.Max(1, (visual.Maximum - visual.Minimum) / 4))
                : visual.Maximum;
            backing.Value = fromBacking;
            Assert(visual.Value == fromBacking, "existing settings updates flow back to the compact slider");

            var originalEnabled = backing.Enabled;
            backing.Enabled = !originalEnabled;
            Assert(visual.Enabled == backing.Enabled, "compact slider mirrors the backing control enabled state");
            backing.Enabled = originalEnabled;
            Assert(visual.Enabled == originalEnabled, "compact slider restores the backing control enabled state");

            Assert(visual.TreatsKeyAsInputForSelfTest(Keys.Left) &&
                   visual.TreatsKeyAsInputForSelfTest(Keys.Right) &&
                   visual.TreatsKeyAsInputForSelfTest(Keys.PageUp) &&
                   visual.TreatsKeyAsInputForSelfTest(Keys.Home),
                "compact slider keeps navigation keys for slider input instead of dialog focus navigation");

            var sliderRow = visual.Parent?.Parent as TableLayoutPanel;
            var valueLabel = sliderRow?.GetControlFromPosition(2, 0) as Label;
            Assert(valueLabel is not null && valueLabel.Dock == DockStyle.Fill && valueLabel.Margin == Padding.Empty,
                "compact slider percentage stays constrained to its value cell");
        }

        var save = FindButton(form, "Save");
        var close = FindButton(form, "Close");
        Assert(save is not null && save.BackColor == ChatUiTheme.SettingsSave,
            "Settings Save uses the compact green primary action");
        Assert(close is not null && close.BackColor == ChatUiTheme.SettingsClose,
            "Settings Close uses the compact red closing action");

        var sections = FindControls<ChatSettingsSectionPanel>(form).ToList();
        Assert(sections.Count > 0, "Settings uses dedicated semantic section containers");
        Assert(sections.All(x => x.Padding == Padding.Empty && x.BackColor == ChatUiTheme.SettingsWindow),
            "Settings sections are flat rather than large bordered cards");
    }

    private static void TestCompactTabEditorStyling()
    {
        var tab = new ChatTabSettings
        {
            Name = "World",
            Channels = [(int)ChatChannel.World],
            MinLevel = 1
        };

        using var form = new ChatTabEditorForm(tab, isNew: true);
        _ = form.Handle;
        form.CreateControl();
        PerformLayoutTree(form);

        var save = FindButton(form, "Save tab");
        var cancel = FindButton(form, "Cancel");
        Assert(save is not null && save.BackColor == ChatUiTheme.SettingsSave,
            "Add/Edit Tab Save uses the same green Settings action");
        Assert(cancel is not null && cancel.BackColor == ChatUiTheme.SettingsClose,
            "Add/Edit Tab Cancel uses the same red close action");
        Assert(form.BackColor == ChatUiTheme.SettingsWindow,
            "Add/Edit Tab uses the same compact Settings surface");

        var sections = FindControls<ChatCardPanel>(form).ToList();
        Assert(sections.Count >= 3 && sections.All(x =>
                x.Padding == Padding.Empty && x.BackColor == ChatUiTheme.SettingsWindow),
            "Add/Edit Tab uses flat Basics, Channels and Filters sections");

        // Physical-size/clipping checks intentionally stay in SettingsUiV122SelfTest,
        // which lays the form out at its real minimum size after WinForms DPI scaling.
        var metrics = form.GetV122CompactMetricsForSelfTest();
        Assert(metrics.CancelText == "Cancel", "Add/Edit Tab keeps explicit discard semantics");
    }

    private static void TestSettingsStylingStaysScoped()
    {
        using var genericButton = new Button { Height = 10 };
        ChatUiTheme.StylePrimaryButton(genericButton);
        Assert(genericButton.Height >= 34,
            "generic overlay/support buttons keep the established touch target");

        using var settingsButton = new Button { Height = 10 };
        ChatUiTheme.StyleSettingsButton(settingsButton);
        Assert(settingsButton.Height >= 30 && settingsButton.Height < genericButton.Height,
            "Settings buttons can be compact without shrinking unrelated UI");

        using var genericCard = new ChatCardPanel();
        Assert(genericCard.Padding != Padding.Empty && genericCard.BackColor == ChatUiTheme.Surface,
            "generic cards keep their pre-v1.2.3 visual defaults");
        genericCard.Padding = Padding.Empty;
        Assert(genericCard.BackColor == ChatUiTheme.SettingsWindow,
            "legacy compact tab-editor cards explicitly opt into the flat Settings surface");
    }

    private static FlowLayoutPanel? FindTopNavigation(Control parent)
    {
        foreach (Control child in parent.Controls)
        {
            if (child is FlowLayoutPanel flow && flow.Controls.OfType<ChatNavButton>().Any())
                return flow;
            var nested = FindTopNavigation(child);
            if (nested is not null) return nested;
        }
        return null;
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

    private static IEnumerable<T> FindControls<T>(Control parent) where T : Control
    {
        foreach (Control child in parent.Controls)
        {
            if (child is T typed) yield return typed;
            foreach (var nested in FindControls<T>(child)) yield return nested;
        }
    }

    private static void PerformLayoutTree(Control parent)
    {
        parent.PerformLayout();
        foreach (Control child in parent.Controls)
            PerformLayoutTree(child);
    }

    private static void Assert(bool condition, string name)
    {
        if (!condition) throw new InvalidOperationException("v1.2.3 Settings UI self-test failed: " + name);
    }
}
