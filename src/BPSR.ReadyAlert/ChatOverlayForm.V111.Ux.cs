using System.Drawing;
using System.Runtime.InteropServices;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed partial class ChatOverlayForm
{
    private ChatListBoxUxController? _v111ListUx;

    protected override void OnShown(EventArgs e)
    {
        base.OnShown(e);
        _v111ListUx ??= new ChatListBoxUxController(_messages, UpdateFollowLatestFromViewport);
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
                "message " + i);
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
            "newest message");

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

    private static int SafeItemHeight(ListBox list, int index)
    {
        try { return Math.Max(1, list.GetItemHeight(index)); }
        catch { return Math.Max(1, list.ItemHeight); }
    }
}

internal static class ChatSenderColor
{
    private static readonly Color[] Palette =
    [
        Color.FromArgb(120, 199, 255),
        Color.FromArgb(255, 159, 182),
        Color.FromArgb(142, 208, 129),
        Color.FromArgb(255, 209, 102),
        Color.FromArgb(182, 156, 255),
        Color.FromArgb(103, 216, 197),
        Color.FromArgb(255, 155, 113),
        Color.FromArgb(134, 168, 255),
        Color.FromArgb(227, 156, 255),
        Color.FromArgb(127, 226, 167),
        Color.FromArgb(246, 178, 107),
        Color.FromArgb(168, 218, 220),
        Color.FromArgb(255, 179, 230),
        Color.FromArgb(196, 225, 127),
        Color.FromArgb(208, 162, 247),
        Color.FromArgb(118, 215, 234)
    ];

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
        var index = (int)(key % (ulong)Palette.Length);
        return Palette[index];
    }

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
    private const int WheelDelta = 120;

    private readonly ListBox _list;
    private readonly Action _viewportChanged;
    private readonly System.Windows.Forms.Timer _timer;
    private int _wheelRemainder;
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
        ApplyDarkScrollTheme();
        Application.AddMessageFilter(this);
    }

    public bool PreFilterMessage(ref Message m)
    {
        if (_disposed || !_list.IsHandleCreated || m.HWnd != _list.Handle || m.Msg != WmMouseWheel)
            return false;

        var packed = m.WParam.ToInt64();
        var delta = unchecked((short)((packed >> 16) & 0xFFFF));
        QueueWheel(delta);
        return true;
    }

    private void QueueWheel(int delta)
    {
        if (_list.Items.Count == 0) return;
        _wheelRemainder += delta;
        var detents = _wheelRemainder / WheelDelta;
        if (detents == 0) return;
        _wheelRemainder -= detents * WheelDelta;

        var maxTop = ChatListScrollMath.GetBottomAlignedTopIndex(_list);
        var lines = SystemInformation.MouseWheelScrollLines;
        if (lines < 0)
            lines = Math.Max(3, ChatListScrollMath.EstimateVisibleRows(_list) - 1);
        lines = Math.Clamp(lines, 1, 8);

        var anchor = _timer.Enabled ? _targetTopIndex : Math.Clamp(_list.TopIndex, 0, maxTop);
        _targetTopIndex = Math.Clamp(anchor - detents * lines, 0, maxTop);
        AdvanceSmoothScroll();
        if (_list.TopIndex != _targetTopIndex)
            _timer.Start();
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

    private void ListHandleCreated(object? sender, EventArgs e) => ApplyDarkScrollTheme();

    private void ApplyDarkScrollTheme()
    {
        if (!_list.IsHandleCreated) return;
        try { _ = SetWindowTheme(_list.Handle, "DarkMode_Explorer", null); }
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

    [DllImport("uxtheme.dll", CharSet = CharSet.Unicode)]
    private static extern int SetWindowTheme(IntPtr hwnd, string? pszSubAppName, string? pszSubIdList);
}
