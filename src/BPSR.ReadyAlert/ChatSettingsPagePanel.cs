using System.ComponentModel;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

/// <summary>
/// Settings page that stays visible/realized behind the active page. Keeping page
/// trees realized avoids WinForms recursively re-running Visible/AutoSize layout on
/// every tab click. The form owns focus routing when the logical active page changes.
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
}
