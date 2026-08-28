using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed partial class ChatGeneralSettingsForm
{
    protected override void OnShown(EventArgs e)
    {
        base.OnShown(e);

        InstallV121UsabilityTracking();
        // The Speech page is registered after the base Settings constructor. Run the
        // compact pass once more immediately before display so every page uses the
        // same density and the v1.2.1 usability copy cannot re-expand the layout.
        InstallV122CompactUi();

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
