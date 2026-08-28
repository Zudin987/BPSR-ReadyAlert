using System.Drawing;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal static class SettingsUiV123SelfTest
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
        PerformLayoutTree(form);

        var nav = FindTopNavigation(form);
        Assert(nav is not null, "top Settings tab bar exists");
        Assert(nav!.FlowDirection == FlowDirection.LeftToRight && !nav.WrapContents,
            "Settings tabs use one compact horizontal row");
        var navButtons = nav.Controls.OfType<ChatNavButton>().ToList();
        Assert(navButtons.Count == 5, "Settings exposes five compact top tabs");
        Assert(navButtons.Count(x => x.Selected) == 1, "exactly one top tab is selected");
        Assert(navButtons.All(x => x.Height <= 30), "top tabs stay compact");

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
        }

        var save = FindButton(form, "Save");
        var close = FindButton(form, "Close");
        Assert(save is not null && save.BackColor == ChatUiTheme.SettingsSave,
            "Settings Save uses the compact green primary action");
        Assert(close is not null && close.BackColor == ChatUiTheme.SettingsClose,
            "Settings Close uses the compact red closing action");

        var sections = FindControls<ChatCardPanel>(form).ToList();
        Assert(sections.Count > 0, "Settings still has semantic section containers");
        Assert(sections.All(x => x.Padding == Padding.Empty && x.BackColor == ChatUiTheme.SettingsWindow),
            "Settings sections are flat rather than large bordered cards");
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
