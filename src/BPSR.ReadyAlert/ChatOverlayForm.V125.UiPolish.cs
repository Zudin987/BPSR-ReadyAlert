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
    /// strip scrollbar-free and only compact tab names when the complete set genuinely
    /// cannot fit in the width that is available.
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
        for (var i = 0; i < buttons.Length; i++)
        {
            var measured = TextRenderer.MeasureText(
                buttons[i].Text,
                buttons[i].Font,
                Size.Empty,
                TextFormatFlags.SingleLine | TextFormatFlags.NoPadding).Width;

            // Do not cap the natural width. A long tab should keep its complete name
            // whenever there is room; ellipsis is a fallback for actual overflow only.
            preferred[i] = Math.Max(64, measured + 28);
        }

        var fitted = FitV126TabWidths(preferred, usable);

        _tabBar.SuspendLayout();
        try
        {
            for (var i = 0; i < buttons.Length; i++)
            {
                var button = buttons[i];
                button.AutoSize = false;
                button.MinimumSize = new Size(0, 32);
                button.Padding = new Padding(8, 0, 8, 0);
                button.Width = fitted[i];
                button.AutoEllipsis = fitted[i] < preferred[i];

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

    /// <summary>
    /// Preserve every tab's preferred width when possible. If the row actually
    /// overflows, cap only the longer tabs first instead of forcing all tabs to an
    /// equal width. This keeps short labels such as General, Guild / Team and Team
    /// readable for as long as the available width permits.
    /// </summary>
    private static int[] FitV126TabWidths(int[] preferred, int usable)
    {
        if (preferred.Length == 0) return [];

        usable = Math.Max(1, usable);
        var natural = preferred.Select(x => Math.Max(1, x)).ToArray();
        if (natural.Sum() <= usable) return natural;

        var minimum = Math.Max(1, Math.Min(64, usable / natural.Length));
        var minimumTotal = natural.Sum(x => Math.Min(x, minimum));

        // Extremely narrow windows can make even the normal compact minimum
        // impossible. Distribute the remaining pixels deterministically in that case.
        if (minimumTotal > usable)
        {
            var equal = Math.Max(1, usable / natural.Length);
            var remainder = Math.Max(0, usable - (equal * natural.Length));
            return natural
                .Select((width, index) => Math.Min(width, equal + (index < remainder ? 1 : 0)))
                .ToArray();
        }

        // Find the largest shared cap that fits. Tabs already shorter than the cap
        // keep their full natural width; only genuinely long tabs are shortened.
        var low = minimum;
        var high = natural.Max();
        while (low < high)
        {
            var mid = low + ((high - low + 1) / 2);
            var total = natural.Sum(x => Math.Min(x, mid));
            if (total <= usable)
                low = mid;
            else
                high = mid - 1;
        }

        var fitted = natural.Select(x => Math.Min(x, low)).ToArray();
        var leftover = usable - fitted.Sum();

        // The binary cap can leave fewer than one pixel per truncated tab unused.
        // Give those pixels back without exceeding any tab's natural width.
        for (var i = 0; i < fitted.Length && leftover > 0; i++)
        {
            if (fitted[i] >= natural[i]) continue;
            fitted[i]++;
            leftover--;
        }

        return fitted;
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

    internal static int[] FitV126TabWidthsForSelfTest(int[] preferred, int usable) =>
        FitV126TabWidths(preferred, usable);
}
