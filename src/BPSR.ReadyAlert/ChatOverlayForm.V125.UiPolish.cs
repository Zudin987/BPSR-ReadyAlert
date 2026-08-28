using System.Drawing;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed partial class ChatOverlayForm
{
    private bool _v125TabStripInstalled;
    private bool _v125TabLayoutQueued;

    protected override void OnLoad(EventArgs e)
    {
        base.OnLoad(e);
        InstallV125TabStripLayout();
    }

    /// <summary>
    /// Native FlowLayoutPanel AutoScroll adds a bright horizontal scrollbar as soon
    /// as one more custom tab no longer fits. Besides clashing with the overlay theme,
    /// the bar also steals height until the user resizes the window. Keep the tab
    /// strip scrollbar-free and compact the visible tab buttons to the width that is
    /// actually available instead.
    /// </summary>
    private void InstallV125TabStripLayout()
    {
        if (_v125TabStripInstalled) return;
        _v125TabStripInstalled = true;

        _tabBar.AutoScroll = false;
        _tabBar.SizeChanged += (_, _) => QueueV125TabLayout();
        _tabBar.ControlAdded += (_, _) => QueueV125TabLayout();
        _tabBar.ControlRemoved += (_, _) => QueueV125TabLayout();
        QueueV125TabLayout();
    }

    private void QueueV125TabLayout()
    {
        if (_v125TabLayoutQueued || IsDisposed || Disposing) return;

        if (!IsHandleCreated)
        {
            FitV125TabButtons();
            return;
        }

        _v125TabLayoutQueued = true;
        BeginInvoke(new Action(() =>
        {
            _v125TabLayoutQueued = false;
            if (!IsDisposed && !Disposing)
                FitV125TabButtons();
        }));
    }

    private void FitV125TabButtons()
    {
        var buttons = _tabBar.Controls.OfType<ChatTabButton>().ToArray();
        if (buttons.Length == 0) return;

        var available = Math.Max(1, _tabBar.ClientSize.Width - _tabBar.Padding.Horizontal);
        var marginWidth = buttons.Sum(x => x.Margin.Horizontal);
        var usable = Math.Max(1, available - marginWidth);

        var preferred = new int[buttons.Length];
        var preferredTotal = 0;
        for (var i = 0; i < buttons.Length; i++)
        {
            var measured = TextRenderer.MeasureText(
                buttons[i].Text,
                buttons[i].Font,
                Size.Empty,
                TextFormatFlags.SingleLine | TextFormatFlags.NoPadding).Width;
            preferred[i] = Math.Clamp(measured + 28, 64, 160);
            preferredTotal += preferred[i];
        }

        var compact = preferredTotal > usable;
        var equalWidth = compact ? Math.Max(1, usable / buttons.Length) : 0;
        var remainder = compact ? Math.Max(0, usable - (equalWidth * buttons.Length)) : 0;

        _tabBar.SuspendLayout();
        try
        {
            for (var i = 0; i < buttons.Length; i++)
            {
                var button = buttons[i];
                button.AutoSize = false;
                button.MinimumSize = new Size(0, 32);
                button.AutoEllipsis = true;
                button.Padding = new Padding(8, 0, 8, 0);
                button.Width = compact ? equalWidth + (i < remainder ? 1 : 0) : preferred[i];

                if (button.Tag is ChatTabSettings tab)
                    _toolTip.SetToolTip(button, tab.Name);
            }
        }
        finally
        {
            _tabBar.ResumeLayout(performLayout: true);
        }

        _tabBar.Invalidate();
    }

    internal void RebuildV125TabBarForSelfTest()
    {
        InstallV125TabStripLayout();
        RebuildTabBar();
        FitV125TabButtons();
    }

    internal (bool AutoScroll, int TabCount, int OuterWidth, int AvailableWidth, bool AllFit)
        GetV125TabStripMetricsForSelfTest()
    {
        var buttons = _tabBar.Controls.OfType<ChatTabButton>().ToArray();
        var outer = buttons.Sum(x => x.Width + x.Margin.Horizontal);
        var available = Math.Max(0, _tabBar.ClientSize.Width - _tabBar.Padding.Horizontal);
        return (_tabBar.AutoScroll, buttons.Length, outer, available, outer <= available);
    }
}
