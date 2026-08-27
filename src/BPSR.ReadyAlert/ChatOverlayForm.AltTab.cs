using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed partial class ChatOverlayForm
{
    // Windows excludes tool windows from Alt+Tab. Keep APPWINDOW cleared so the
    // overlay behaves like a desktop overlay rather than a normal application window.
    private const int WsExToolWindow = 0x00000080;
    private const int WsExAppWindow = 0x00040000;

    protected override CreateParams CreateParams
    {
        get
        {
            var parameters = base.CreateParams;
            parameters.ExStyle |= WsExToolWindow;
            parameters.ExStyle &= ~WsExAppWindow;
            return parameters;
        }
    }

    internal (bool ToolWindow, bool AppWindow) GetAltTabWindowStylesForSelfTest()
    {
        var exStyle = CreateParams.ExStyle;
        return ((exStyle & WsExToolWindow) != 0, (exStyle & WsExAppWindow) != 0);
    }
}
