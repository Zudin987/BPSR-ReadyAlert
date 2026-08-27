using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed partial class ChatGeneralSettingsForm
{
    protected override void OnShown(EventArgs e)
    {
        base.OnShown(e);

        InstallV121UsabilityTracking();

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
