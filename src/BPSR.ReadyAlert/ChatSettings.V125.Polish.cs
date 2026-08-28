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

        // Dock=Top already stretches the control horizontally inside the one-column
        // field block. Do not also assign Anchor: WinForms treats Dock and Anchor as
        // mutually exclusive layout modes and the later Anchor assignment can undo
        // the full-width dock, leaving the editor at its old narrow width.
        box.Dock = DockStyle.Top;
    }

    internal (bool HighlightFluid, bool Rule1Fluid, bool Rule2Fluid, bool SingleLine)
        GetV125AlertInputMetricsForSelfTest()
    {
        static bool Fluid(TextBox box) => box.MaximumSize.Width == 0 &&
                                           box.Dock == DockStyle.Top;
        return (
            Fluid(_highlight),
            Fluid(_soundRuleMatch[0]),
            Fluid(_soundRuleMatch[1]),
            !_highlight.Multiline && !_soundRuleMatch[0].Multiline && !_soundRuleMatch[1].Multiline);
    }

    internal (int Highlight, int Rule1, int Rule2, int AlertsPageWidth)
        GetV125AlertInputWidthsForSelfTest()
    {
        var pageWidth = _pages.TryGetValue("Alerts", out var alerts)
            ? alerts.Page.ClientSize.Width
            : 0;
        return (_highlight.Width, _soundRuleMatch[0].Width, _soundRuleMatch[1].Width, pageWidth);
    }

    internal bool AreV125SettingsScrollbarsDarkThemedForSelfTest() =>
        _pages.Values.All(x => x.Page.UsesV125DarkScrollbarThemeForSelfTest());

    internal string GetV125CleanupLabelForSelfTest() => _hideRichNoise.Text;
}
