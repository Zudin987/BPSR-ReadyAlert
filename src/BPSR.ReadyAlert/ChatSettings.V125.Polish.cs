using System.Drawing;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed partial class ChatGeneralSettingsForm
{
    /// <summary>
    /// v1.2.3 intentionally capped normal text inputs at 300 px. That works for
    /// short general fields, but it makes the three one-line rule editors in Alerts
    /// look misaligned beside their full-width sound-file rows. Keep the compact
    /// single-line height while allowing only these rule inputs to use the available
    /// column width.
    /// </summary>
    private void ApplyV125SettingsPolish()
    {
        MakeV125AlertInputFluid(_highlight);
        for (var i = 0; i < V124SoundRuleCount; i++)
            MakeV125AlertInputFluid(_soundRuleMatch[i]);
    }

    private static void MakeV125AlertInputFluid(TextBox box)
    {
        box.Multiline = false;
        box.ScrollBars = ScrollBars.None;
        box.MaximumSize = Size.Empty;
        box.MinimumSize = Size.Empty;
        box.AutoSize = false;
        box.Height = 24;
        box.Dock = DockStyle.Top;
        box.Anchor = AnchorStyles.Left | AnchorStyles.Right | AnchorStyles.Top;
    }

    internal (bool HighlightFluid, bool Rule1Fluid, bool Rule2Fluid, bool SingleLine)
        GetV125AlertInputMetricsForSelfTest()
    {
        static bool Fluid(TextBox box) => box.MaximumSize.Width == 0 &&
                                           box.Dock == DockStyle.Top &&
                                           (box.Anchor & AnchorStyles.Right) != 0;
        return (
            Fluid(_highlight),
            Fluid(_soundRuleMatch[0]),
            Fluid(_soundRuleMatch[1]),
            !_highlight.Multiline && !_soundRuleMatch[0].Multiline && !_soundRuleMatch[1].Multiline);
    }

    internal (int Highlight, int Rule1, int Rule2, int AlertsPageWidth)
        GetV125AlertInputWidthsForSelfTest()
    {
        _pages.TryGetValue("Alerts", out var alerts);
        return (_highlight.Width, _soundRuleMatch[0].Width, _soundRuleMatch[1].Width, alerts.Page?.ClientSize.Width ?? 0);
    }

    internal bool AreV125SettingsScrollbarsDarkThemedForSelfTest() =>
        _pages.Values.All(x => x.Page.UsesV125DarkScrollbarThemeForSelfTest());

    internal string GetV125CleanupLabelForSelfTest() => _hideRichNoise.Text;
}
