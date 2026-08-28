using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed partial class ChatGeneralSettingsForm
{
    /// <summary>
    /// Text/visibility-only refresh safe to run after WinForms has already applied
    /// DPI scaling. Geometry is deliberately excluded: reassigning logical pixel
    /// sizes after handle creation can overwrite DPI-scaled dimensions.
    /// </summary>
    private void RefreshV122CompactCopyOnly()
    {
        if (_pages.TryGetValue("Speech", out var speech))
            speech.Button.Text = "Speech";
        HideV122CardByTitle("Translation used by TTS");
        ReplaceV122Copy(this);
    }

    internal (int SelectedPages, bool BufferedHost, int SidebarWidth, int FooterHeight, int MaxRuleHeight, int MaxNavHeight, string ActiveKey)
        GetV122DpiSafeMetricsForSelfTest()
    {
        RefreshV122CompactCopyOnly();
        var selected = _pages.Values.Count(x => x.Button.Selected);
        var sidebar = FindV122Sidebar();
        var footer = FindV122Footer();
        var maxRuleHeight = new[] { _highlight.Height }.Concat(_soundRuleMatch.Select(x => x.Height)).Max();
        var maxNav = _pages.Values.Select(x => x.Button.Height).DefaultIfEmpty(0).Max();
        return (
            selected,
            _contentHost is ChatBufferedPanel,
            sidebar?.Width ?? 0,
            footer?.Height ?? 0,
            maxRuleHeight,
            maxNav,
            _activePageKey);
    }
}
