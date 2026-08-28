using System.ComponentModel;
using System.Runtime.InteropServices;
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
    private bool _darkScrollbarThemeRequested;

    [DllImport("uxtheme.dll", CharSet = CharSet.Unicode)]
    private static extern int SetWindowTheme(IntPtr hwnd, string? pszSubAppName, string? pszSubIdList);

    [Browsable(false)]
    [DesignerSerializationVisibility(DesignerSerializationVisibility.Hidden)]
    internal bool ActivePage
    {
        get => _activePage;
        set => _activePage = value;
    }

    protected override void OnHandleCreated(EventArgs e)
    {
        base.OnHandleCreated(e);
        ApplyDarkScrollbarTheme();
    }

    private void ApplyDarkScrollbarTheme()
    {
        if (!OperatingSystem.IsWindows() || !IsHandleCreated) return;
        _darkScrollbarThemeRequested = true;
        try
        {
            _ = SetWindowTheme(Handle, "DarkMode_Explorer", null);
        }
        catch (DllNotFoundException) { }
        catch (EntryPointNotFoundException) { }
    }

    internal bool UsesV125DarkScrollbarThemeForSelfTest() => _darkScrollbarThemeRequested;
}
