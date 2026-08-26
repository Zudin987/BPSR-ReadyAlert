using System.Drawing;
using System.Runtime.InteropServices;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed partial class ChatOverlayForm
{
    private ChatListBoxUxController? _v111ListUx;
    private ChatDarkScrollBar? _v111ScrollBar;

    protected override void OnShown(EventArgs e)
    {
        base.OnShown(e);

        _v111ListUx ??= new ChatListBoxUxController(
            _messages,
            () =>
            {
                UpdateFollowLatestFromViewport();
                _v111ScrollBar?.SyncFromList();
            });

        if (_v111ScrollBar is null)
        {
            _v111ScrollBar = new ChatDarkScrollBar(
                _messages,
                delta => _v111ListUx?.HandleWheelDelta(delta),
                UpdateFollowLatestFromViewport);
            Controls.Add(_v111ScrollBar);
            _v111ScrollBar.BringToFront();

            _messages.LocationChanged += (_, _) => PositionV111ScrollBar();
            _messages.SizeChanged += (_, _) => PositionV111ScrollBar();
            _messages.VisibleChanged += (_, _) => SyncV111ScrollUx();
        }

        PositionV111ScrollBar();
        SyncV111ScrollUx();
    }

    private void PositionV111ScrollBar()
    {
        if (_v111ScrollBar is null || _v111ScrollBar.IsDisposed) return;
        var width = Math.Max(12, SystemInformation.VerticalScrollBarWidth);
        _v111ScrollBar.Bounds = new Rectangle(
            Math.Max(_messages.Left, _messages.Right - width),
            _messages.Top,
            width,
            Math.Max(0, _messages.Height));
        _v111ScrollBar.BringToFront();
        _v111ScrollBar.SyncFromList();
    }

    private void SyncV111ScrollUx()
    {
        _v111ScrollBar?.SyncFromList();
    }

    private void CancelV111SmoothScroll()
    {
        _v111ListUx?.CancelAndSyncToCurrent();
    }

    internal (bool FollowLatest, bool AtBottom, int TopIndex, int Count) GetV111ScrollStateForSelfTest() =>
        (_followLatest, IsNearBottom(), _messages.TopIndex, _messages.Items.Count);

    internal bool RunV111HistoryTrimSelfTest()
    {
        _ = _messages.Handle;
        _history.Clear();
        _messages.Items.Clear();
        _settings.Chat.MaxHistory = 10;
        _followLatest = true;
        _unseenMessages = 0;

        for (var i = 0; i < 10; i++)
        {
            var message = new ChatMessageEvent(
                1000 + i,
                "User" + i,
                80,
                ChatChannel.World,
                DateTime.Now.AddSeconds(i),
                ChatMessageKind.Text,
                "message " + i,
                i + 1);
            _history.Add(message);
            _messages.Items.Add(CreateDisplayItem(message));
        }

        ScrollToLatest();
        if (!IsNearBottom()) return false;

        var newest = new ChatMessageEvent(
            9999,
            "Newest",
            80,
            ChatChannel.World,
            DateTime.Now.AddMinutes(1),
            ChatMessageKind.Text,
            "newest message",
            11);

        _history.Add(newest);
        RemoveOverflowHistoryFromView();
        var wasFollowing = _followLatest && IsNearBottom();
        _messages.Items.Add(CreateDisplayItem(newest));
        if (wasFollowing) ScrollToLatest();

        return _history.Count == 10 && _followLatest && IsNearBottom();
    }
}

internal static class ChatListScrollMath
{
    internal static int GetBottomAlignedTopIndex(ListBox list)
    {
        var count = list.Items.Count;
        if (count <= 1) return 0;

        var availableHeight = Math.Max(1, list.ClientSize.Height);
        var usedHeight = 0;
        var top = count - 1;

        for (var i = count - 1; i >= 0; i--)
        {
            var itemHeight = SafeItemHeight(list, i);
            if (usedHeight > 0 && usedHeight + itemHeight > availableHeight)
                break;

            usedHeight += itemHeight;
            top = i;
        }

        return Math.Clamp(top, 0, count - 1);
    }

    internal static bool IsAtBottom(ListBox list)
    {
        if (list.Items.Count == 0) return true;
        return list.TopIndex >= GetBottomAlignedTopIndex(list);
    }

    internal static void ScrollToBottom(ListBox list)
    {
        if (list.Items.Count == 0) return;
        list.TopIndex = GetBottomAlignedTopIndex(list);
    }

    internal static int EstimateVisibleRows(ListBox list)
    {
        if (list.Items.Count == 0) return 1;
        var availableHeight = Math.Max(1, list.ClientSize.Height);
        var usedHeight = 0;
        var count = 0;
        for (var i = Math.Clamp(list.TopIndex, 0, list.Items.Count - 1); i < list.Items.Count; i++)
        {
            usedHeight += SafeItemHeight(list, i);
            count++;
            if (usedHeight >= availableHeight) break;
        }
        return Math.Max(1, count);
    }

    internal static int SafeItemHeight(ListBox list, int index)
    {
        try { return Math.Max(1, list.GetItemHeight(index)); }
        catch { return Math.Max(1, list.ItemHeight); }
    }
}

internal static class ChatWheelMath
{
    internal static int AccumulateRows(int delta, int systemLines, int visibleRows, ref double rowRemainder)
    {
        if (delta == 0 || systemLines == 0) return 0;
        var rowsPerDetent = systemLines < 0
            ? Math.Max(1, visibleRows - 1)
            : Math.Max(1, systemLines);

        rowRemainder += delta / 120d * rowsPerDetent;
        if (Math.Abs(rowRemainder) < 0.5d) return 0;

        var rows = Math.Sign(rowRemainder) * Math.Max(1, (int)Math.Floor(Math.Abs(rowRemainder) + 0.5d));
        rowRemainder -= rows;
        return Math.Clamp(rows, -200, 200);
    }
}

internal static class ChatSenderColor
{
    internal static Color ForMessage(ChatMessageEvent message)
    {
        ulong key;
        if (message.SenderId != 0)
        {
            key = unchecked((ulong)message.SenderId);
            key *= 11400714819323198485UL;
        }
        else
        {
            key = StableHash(message.SenderName ?? string.Empty);
        }

        key ^= key >> 33;
        key *= 0xff51afd7ed558ccdUL;
        key ^= key >> 33;
        key *= 0xc4ceb9fe1a85ec53UL;
        key ^= key >> 33;

        // 48 hue slots x three small saturation/lightness variants gives far more
        // identity separation than the old 16-color palette while remaining bright
        // enough on ReadyAlert's dark/translucent backgrounds.
        var hueSlot = (int)(key % 48UL);
        var hue = (hueSlot * 137.50776405003785) % 360d;
        var saturation = 0.58d + ((key >> 8) % 3UL) * 0.055d;
        var lightness = 0.68d + ((key >> 12) % 3UL) * 0.035d;
        return HslToColor(hue, saturation, Math.Min(0.76d, lightness));
    }

    private static Color HslToColor(double hue, double saturation, double lightness)
    {
        var c = (1d - Math.Abs(2d * lightness - 1d)) * saturation;
        var h = hue / 60d;
        var x = c * (1d - Math.Abs(h % 2d - 1d));
        (double r, double g, double b) = h switch
        {
            < 1d => (c, x, 0d),
            < 2d => (x, c, 0d),
            < 3d => (0d, c, x),
            < 4d => (0d, x, c),
            < 5d => (x, 0d, c),
            _ => (c, 0d, x)
        };
        var m = lightness - c / 2d;
        return Color.FromArgb(
            ClampByte((r + m) * 255d),
            ClampByte((g + m) * 255d),
            ClampByte((b + m) * 255d));
    }

    private static int ClampByte(double value) => Math.Clamp((int)Math.Round(value), 0, 255);

    private static ulong StableHash(string value)
    {
        const ulong offset = 14695981039346656037UL;
        const ulong prime = 1099511628211UL;
        var hash = offset;
        foreach (var c in value)
        {
            hash ^= char.ToLowerInvariant(c);
            hash *= prime;
        }
        return hash;
    }
}

internal sealed class ChatListBoxUxController : IMessageFilter, IDisposable
{
    private const int WmMouseWheel = 0x020A;

    private readonly ListBox _list;
    private readonly Action _viewportChanged;
    private readonly System.Windows.Forms.Timer _timer;
    private double _wheelRowRemainder;
    private int _targetTopIndex;
    private bool _disposed;

    internal ChatListBoxUxController(ListBox list, Action viewportChanged)
    {
        _list = list;
        _viewportChanged = viewportChanged;
        _targetTopIndex = Math.Max(0, list.TopIndex);
        _timer = new System.Windows.Forms.Timer { Interval = 12 };
        _timer.Tick += (_, _) => AdvanceSmoothScroll();
        _list.HandleCreated += ListHandleCreated;
        _list.Disposed += ListDisposed;
        ApplyNativeThemeAndHideScrollBar();
        Application.AddMessageFilter(this);
    }

    public bool PreFilterMessage(ref Message m)
    {
        if (_disposed || !_list.IsHandleCreated || m.HWnd != _list.Handle || m.Msg != WmMouseWheel)
            return false;

        var packed = m.WParam.ToInt64();
        var delta = unchecked((short)((packed >> 16) & 0xFFFF));
        HandleWheelDelta(delta);
        return true;
    }

    internal void HandleWheelDelta(int delta)
    {
        if (_disposed || _list.Items.Count == 0) return;
        var visibleRows = ChatListScrollMath.EstimateVisibleRows(_list);
        var rows = ChatWheelMath.AccumulateRows(
            delta,
            SystemInformation.MouseWheelScrollLines,
            visibleRows,
            ref _wheelRowRemainder);
        if (rows == 0) return;

        var maxTop = ChatListScrollMath.GetBottomAlignedTopIndex(_list);
        var anchor = _timer.Enabled ? _targetTopIndex : Math.Clamp(_list.TopIndex, 0, maxTop);
        _targetTopIndex = Math.Clamp(anchor - rows, 0, maxTop);
        AdvanceSmoothScroll();
        if (_list.TopIndex != _targetTopIndex)
            _timer.Start();
    }

    internal void CancelAndSyncToCurrent()
    {
        if (_disposed) return;
        _timer.Stop();
        _wheelRowRemainder = 0d;
        var maxTop = _list.Items.Count == 0 ? 0 : ChatListScrollMath.GetBottomAlignedTopIndex(_list);
        _targetTopIndex = Math.Clamp(Math.Max(0, _list.TopIndex), 0, maxTop);
    }

    private void AdvanceSmoothScroll()
    {
        if (_disposed || _list.IsDisposed || _list.Items.Count == 0)
        {
            _timer.Stop();
            return;
        }

        var maxTop = ChatListScrollMath.GetBottomAlignedTopIndex(_list);
        _targetTopIndex = Math.Clamp(_targetTopIndex, 0, maxTop);
        var current = Math.Clamp(_list.TopIndex, 0, maxTop);
        var difference = _targetTopIndex - current;
        if (difference == 0)
        {
            _timer.Stop();
            _viewportChanged();
            return;
        }

        var magnitude = Math.Abs(difference);
        var step = magnitude <= 3 ? 1 : Math.Min(3, (magnitude + 2) / 3);
        _list.TopIndex = Math.Clamp(current + Math.Sign(difference) * step, 0, maxTop);
        _list.Invalidate();
        _viewportChanged();
    }

    private void ListHandleCreated(object? sender, EventArgs e) => ApplyNativeThemeAndHideScrollBar();

    private void ApplyNativeThemeAndHideScrollBar()
    {
        if (!_list.IsHandleCreated) return;
        try { _ = SetWindowTheme(_list.Handle, "DarkMode_Explorer", null); }
        catch (DllNotFoundException) { }
        catch (EntryPointNotFoundException) { }
        try { _ = ShowScrollBar(_list.Handle, 1, false); }
        catch (DllNotFoundException) { }
        catch (EntryPointNotFoundException) { }
    }

    private void ListDisposed(object? sender, EventArgs e) => Dispose();

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;
        _timer.Stop();
        _timer.Dispose();
        _list.HandleCreated -= ListHandleCreated;
        _list.Disposed -= ListDisposed;
        Application.RemoveMessageFilter(this);
    }

    internal (int TargetTopIndex, bool TimerEnabled) GetStateForSelfTest() => (_targetTopIndex, _timer.Enabled);
    internal void AdvanceForSelfTest() => AdvanceSmoothScroll();

    [DllImport("uxtheme.dll", CharSet = CharSet.Unicode)]
    private static extern int SetWindowTheme(IntPtr hwnd, string? pszSubAppName, string? pszSubIdList);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool ShowScrollBar(IntPtr hWnd, int wBar, bool bShow);
}

internal sealed class ChatDarkScrollBar : Control
{
    private readonly ListBox _list;
    private readonly Action<int> _wheel;
    private readonly Action _viewportChanged;
    private bool _dragging;
    private int _dragOffset;

    internal ChatDarkScrollBar(ListBox list, Action<int> wheel, Action viewportChanged)
    {
        _list = list;
        _wheel = wheel;
        _viewportChanged = viewportChanged;
        TabStop = false;
        BackColor = Color.FromArgb(20, 24, 29);
        Cursor = Cursors.Hand;
        SetStyle(ControlStyles.AllPaintingInWmPaint | ControlStyles.OptimizedDoubleBuffer | ControlStyles.UserPaint, true);
    }

    internal void SyncFromList()
    {
        if (IsDisposed) return;
        var maxTop = _list.Items.Count == 0 ? 0 : ChatListScrollMath.GetBottomAlignedTopIndex(_list);
        Visible = _list.Visible && maxTop > 0;
        Invalidate();
    }

    protected override void OnPaint(PaintEventArgs e)
    {
        base.OnPaint(e);
        e.Graphics.Clear(Color.FromArgb(20, 24, 29));
        if (_list.Items.Count == 0) return;

        using var track = new SolidBrush(Color.FromArgb(34, 40, 48));
        using var thumb = new SolidBrush(Color.FromArgb(91, 104, 122));
        var trackRect = new Rectangle(Math.Max(2, Width / 2 - 3), 3, Math.Min(7, Math.Max(4, Width - 4)), Math.Max(1, Height - 6));
        e.Graphics.FillRectangle(track, trackRect);
        var thumbRect = GetThumbRectangle(trackRect);
        e.Graphics.FillRectangle(thumb, thumbRect);
    }

    protected override void OnMouseDown(MouseEventArgs e)
    {
        base.OnMouseDown(e);
        if (e.Button != MouseButtons.Left) return;
        var track = new Rectangle(0, 3, Width, Math.Max(1, Height - 6));
        var thumb = GetThumbRectangle(track);
        if (thumb.Contains(e.Location))
        {
            _dragging = true;
            _dragOffset = e.Y - thumb.Top;
            Capture = true;
            return;
        }

        var page = Math.Max(1, ChatListScrollMath.EstimateVisibleRows(_list) - 1);
        var maxTop = ChatListScrollMath.GetBottomAlignedTopIndex(_list);
        _list.TopIndex = Math.Clamp(_list.TopIndex + (e.Y < thumb.Top ? -page : page), 0, maxTop);
        _viewportChanged();
        Invalidate();
    }

    protected override void OnMouseMove(MouseEventArgs e)
    {
        base.OnMouseMove(e);
        if (!_dragging || _list.Items.Count == 0) return;
        var maxTop = ChatListScrollMath.GetBottomAlignedTopIndex(_list);
        if (maxTop <= 0) return;

        var trackTop = 3;
        var trackHeight = Math.Max(1, Height - 6);
        var thumb = GetThumbRectangle(new Rectangle(0, trackTop, Width, trackHeight));
        var travel = Math.Max(1, trackHeight - thumb.Height);
        var y = Math.Clamp(e.Y - _dragOffset - trackTop, 0, travel);
        _list.TopIndex = Math.Clamp((int)Math.Round(y / (double)travel * maxTop), 0, maxTop);
        _viewportChanged();
        Invalidate();
    }

    protected override void OnMouseUp(MouseEventArgs e)
    {
        base.OnMouseUp(e);
        if (e.Button != MouseButtons.Left) return;
        _dragging = false;
        Capture = false;
    }

    protected override void OnMouseWheel(MouseEventArgs e)
    {
        base.OnMouseWheel(e);
        _wheel(e.Delta);
        Invalidate();
    }

    private Rectangle GetThumbRectangle(Rectangle track)
    {
        var count = Math.Max(1, _list.Items.Count);
        var visibleRows = Math.Min(count, ChatListScrollMath.EstimateVisibleRows(_list));
        var maxTop = ChatListScrollMath.GetBottomAlignedTopIndex(_list);
        var height = Math.Clamp((int)Math.Round(track.Height * (visibleRows / (double)count)), 24, Math.Max(24, track.Height));
        height = Math.Min(track.Height, height);
        var travel = Math.Max(0, track.Height - height);
        var ratio = maxTop <= 0 ? 0d : Math.Clamp(_list.TopIndex, 0, maxTop) / (double)maxTop;
        var y = track.Top + (int)Math.Round(travel * ratio);
        return new Rectangle(track.Left, y, Math.Max(1, track.Width), height);
    }
}
