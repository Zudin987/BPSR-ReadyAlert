using System.Windows.Forms;

namespace BPSR.ReadyAlert;

/// <summary>
/// Small flicker-resistant WinForms surface used for settings page swaps.
/// Keeping this isolated avoids turning on expensive repaint behavior globally.
/// </summary>
internal sealed class ChatBufferedPanel : Panel
{
    internal ChatBufferedPanel()
    {
        DoubleBuffered = true;
        ResizeRedraw = true;
        SetStyle(
            ControlStyles.OptimizedDoubleBuffer |
            ControlStyles.AllPaintingInWmPaint |
            ControlStyles.UserPaint,
            true);
    }
}
