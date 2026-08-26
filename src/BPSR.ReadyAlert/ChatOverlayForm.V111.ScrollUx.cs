using System.Runtime.InteropServices;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed partial class ChatOverlayForm
{
    private const int WmMouseWheel = 0x020A;
    private const int WheelDelta = 120;
    private const int WheelRowsPerNotch = 1;
    private const string DarkScrollbarTheme = "DarkMode_Explorer";

    private ChatWheelMessageFilter? _chatWheelFilter;
    private bool _scrollUxInstalled;

    [DllImport("uxtheme.dll", CharSet = CharSet.Unicode, ExactSpelling = true)]
    private static extern int SetWindowTheme(IntPtr hwnd, string? pszSubAppName, string? pszSubIdList);

    protected override void OnShown(EventArgs e)
    {
        base.OnShown(e);
        InstallScrollUx();
    }

    private void InstallScrollUx()
    {
        if (_scrollUxInstalled || IsDisposed) return;
        _scrollUxInstalled = true;

        ApplyDarkMessageScrollbar();
        _messages.HandleCreated += (_, _) => ApplyDarkMessageScrollbar();

        _chatWheelFilter = new ChatWheelMessageFilter(this);
        Application.AddMessageFilter(_chatWheelFilter);
        Disposed += (_, _) => RemoveScrollUxFilter();
    }

    private void RemoveScrollUxFilter()
    {
        if (_chatWheelFilter is null) return;
        try { Application.RemoveMessageFilter(_chatWheelFilter); } catch { }
        _chatWheelFilter = null;
    }

    private void ApplyDarkMessageScrollbar()
    {
        if (!OperatingSystem.IsWindows() || !_messages.IsHandleCreated) return;
        try
        {
            _ = SetWindowTheme(_messages.Handle, DarkScrollbarTheme, null);
            _messages.Invalidate();
        }
        catch (DllNotFoundException) { }
        catch (EntryPointNotFoundException) { }
    }

    private void ScrollMessageRows(int rows)
    {
        if (_collapsed || _messages.Items.Count == 0 || rows == 0) return;

        var current = Math.Max(0, _messages.TopIndex);
        var target = Math.Clamp(current + rows, 0, _messages.Items.Count - 1);
        if (target != current)
            _messages.TopIndex = target;

        // Update Smart Scroll immediately instead of waiting for the previous
        // BeginInvoke-based viewport notification. This makes the overlay react to
        // the wheel in the same input frame and removes the delayed feel.
        UpdateFollowLatestFromViewport();
        _messages.Invalidate();
    }

    internal (int WheelRows, string ScrollbarTheme) GetV111ScrollUxMetricsForSelfTest() =>
        (WheelRowsPerNotch, DarkScrollbarTheme);

    private sealed class ChatWheelMessageFilter(ChatOverlayForm owner) : IMessageFilter
    {
        private int _wheelRemainder;

        public bool PreFilterMessage(ref Message m)
        {
            if (m.Msg != WmMouseWheel || owner.IsDisposed || !owner._messages.IsHandleCreated ||
                m.HWnd != owner._messages.Handle)
                return false;

            var raw = m.WParam.ToInt64();
            var delta = unchecked((short)((raw >> 16) & 0xFFFF));
            if (delta == 0) return true;

            _wheelRemainder += delta;
            var notches = _wheelRemainder / WheelDelta;
            if (notches == 0) return true;

            _wheelRemainder -= notches * WheelDelta;
            owner.ScrollMessageRows(-notches * WheelRowsPerNotch);
            return true;
        }
    }
}
