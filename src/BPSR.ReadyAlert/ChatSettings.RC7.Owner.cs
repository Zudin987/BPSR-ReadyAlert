using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed partial class ChatGeneralSettingsForm
{
    protected override void OnShown(EventArgs e)
    {
        base.OnShown(e);

        // v1.2.4: always use the native Standard/AutoCheck interaction path for
        // Settings checkboxes. Do this after every page (including Speech) exists.
        ApplyV124InteractiveCheckboxes();
        ApplyV125SettingsPolish();
        InstallV121UsabilityTracking();

        // WinForms has already applied monitor DPI scaling by this point. Refresh
        // only compact wording/visibility here; never reassign logical geometry
        // after handle creation or high-DPI displays can get undersized controls.
        RefreshV122CompactCopyOnly();

        // A modal child should never disappear behind the overlay that opened it.
        // When the overlay is TopMost, promote the settings window to the same
        // topmost band while preserving the normal owner relationship.
        if (Owner is ChatOverlayForm overlay && overlay.TopMost)
        {
            TopMost = true;
            BringToFront();
            Activate();
        }
    }
}
