using System.ComponentModel;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

/// <summary>
/// Settings page that can stay visible/realized behind the active page without
/// allowing keyboard focus into the background page. Keeping pages realized avoids
/// WinForms recursively re-running Visible/AutoSize layout on every tab click.
/// </summary>
internal sealed class ChatSettingsPagePanel : Panel
{
    private bool _activePage;

    [Browsable(false)]
    [DesignerSerializationVisibility(DesignerSerializationVisibility.Hidden)]
    internal bool ActivePage
    {
        get => _activePage;
        set => _activePage = value;
    }

    protected override bool CanSelectCore => _activePage && base.CanSelectCore;
}
