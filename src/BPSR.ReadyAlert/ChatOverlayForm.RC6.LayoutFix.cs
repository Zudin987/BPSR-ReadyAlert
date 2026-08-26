using System.Drawing;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed partial class ChatOverlayForm
{
    private bool _bodyLayoutInitialized;

    /// <summary>
    /// The toolbar and message list are intentionally siblings so the collapsed
    /// handle can still cover the whole borderless form. WinForms Dock ordering can
    /// let a Fill control extend underneath a later Top control, which caused RC5's
    /// first chat row to be painted behind the toolbar. Give the body an explicit
    /// rectangle below the toolbar instead of relying on sibling Dock ordering.
    /// </summary>
    protected override void OnLayout(LayoutEventArgs levent)
    {
        base.OnLayout(levent);

        // Forms can receive layout callbacks while their constructor is still
        // assigning readonly controls, so do nothing until the body exists.
        if (_topPanel is null || _messages is null || _emptyState is null)
            return;

        if (!_bodyLayoutInitialized)
        {
            _bodyLayoutInitialized = true;
            _messages.Dock = DockStyle.None;
            _emptyState.Dock = DockStyle.None;
        }

        if (_collapsed)
            return;

        var left = Padding.Left;
        var top = Math.Max(Padding.Top, _topPanel.Bottom);
        var width = Math.Max(0, ClientSize.Width - left - Padding.Right);
        var height = Math.Max(0, ClientSize.Height - top - Padding.Bottom);
        var bodyBounds = new Rectangle(left, top, width, height);

        if (_messages.Bounds != bodyBounds)
            _messages.Bounds = bodyBounds;
        if (_emptyState.Bounds != bodyBounds)
            _emptyState.Bounds = bodyBounds;
    }

    internal (Rectangle Toolbar, Rectangle Messages) GetLayoutBoundsForSelfTest() =>
        (_topPanel.Bounds, _messages.Bounds);
}
