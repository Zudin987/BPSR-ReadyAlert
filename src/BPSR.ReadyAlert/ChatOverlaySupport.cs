using System.ComponentModel;
using System.Drawing;
using System.Runtime.InteropServices;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal readonly record struct ChatHotkeyGesture(Keys Key, bool Ctrl, bool Shift, bool Alt)
{
    internal uint NativeModifiers
    {
        get
        {
            uint value = ChatNativeMethods.ModNoRepeat;
            if (Ctrl) value |= ChatNativeMethods.ModControl;
            if (Shift) value |= ChatNativeMethods.ModShift;
            if (Alt) value |= ChatNativeMethods.ModAlt;
            return value;
        }
    }

    internal string DisplayText
    {
        get
        {
            var parts = new List<string>(4);
            if (Ctrl) parts.Add("Ctrl");
            if (Shift) parts.Add("Shift");
            if (Alt) parts.Add("Alt");
            parts.Add(ChatHotkey.FormatKey(Key));
            return string.Join('+', parts);
        }
    }
}

internal static class ChatHotkey
{
    internal static bool TryParse(string? text, out ChatHotkeyGesture gesture, out string error)
    {
        gesture = default;
        error = string.Empty;
        if (string.IsNullOrWhiteSpace(text))
        {
            error = "Choose a hotkey.";
            return false;
        }

        var ctrl = false;
        var shift = false;
        var alt = false;
        Keys key = Keys.None;

        foreach (var rawPart in text.Split('+', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries))
        {
            if (rawPart.Equals("Ctrl", StringComparison.OrdinalIgnoreCase) ||
                rawPart.Equals("Control", StringComparison.OrdinalIgnoreCase))
            {
                ctrl = true;
                continue;
            }
            if (rawPart.Equals("Shift", StringComparison.OrdinalIgnoreCase))
            {
                shift = true;
                continue;
            }
            if (rawPart.Equals("Alt", StringComparison.OrdinalIgnoreCase))
            {
                alt = true;
                continue;
            }

            if (key != Keys.None)
            {
                error = "A hotkey can contain only one non-modifier key.";
                return false;
            }

            if (!TryParseKey(rawPart, out key))
            {
                error = $"'{rawPart}' is not a supported hotkey key.";
                return false;
            }
        }

        if (key == Keys.None)
        {
            error = "Press a normal key together with any modifiers you want.";
            return false;
        }

        gesture = new ChatHotkeyGesture(key, ctrl, shift, alt);
        return true;
    }

    internal static string FromKeyData(Keys keyData)
    {
        var key = keyData & Keys.KeyCode;
        if (key is Keys.ControlKey or Keys.ShiftKey or Keys.Menu or Keys.None)
            return string.Empty;

        var ctrl = (keyData & Keys.Control) == Keys.Control;
        var shift = (keyData & Keys.Shift) == Keys.Shift;
        var alt = (keyData & Keys.Alt) == Keys.Alt;
        return new ChatHotkeyGesture(key, ctrl, shift, alt).DisplayText;
    }

    internal static string FormatKey(Keys key)
    {
        var keyValue = (int)key;
        if (keyValue >= (int)Keys.D0 && keyValue <= (int)Keys.D9)
            return (keyValue - (int)Keys.D0).ToString();
        if (keyValue >= (int)Keys.NumPad0 && keyValue <= (int)Keys.NumPad9)
            return "Num" + (keyValue - (int)Keys.NumPad0);
        if (keyValue >= (int)Keys.A && keyValue <= (int)Keys.Z)
            return key.ToString();
        if (keyValue >= (int)Keys.F1 && keyValue <= (int)Keys.F24)
            return key.ToString();

        return key switch
        {
            Keys.Insert => "Insert",
            Keys.Delete => "Delete",
            Keys.Home => "Home",
            Keys.End => "End",
            Keys.PageUp => "PageUp",
            Keys.PageDown => "PageDown",
            Keys.Up => "Up",
            Keys.Down => "Down",
            Keys.Left => "Left",
            Keys.Right => "Right",
            Keys.Space => "Space",
            Keys.Oemtilde => "`",
            Keys.OemMinus => "-",
            Keys.Oemplus => "=",
            _ => key.ToString()
        };
    }

    private static bool TryParseKey(string text, out Keys key)
    {
        key = Keys.None;
        if (text.Length == 1)
        {
            var c = text[0];
            if (char.IsLetter(c))
            {
                key = (Keys)Enum.Parse(typeof(Keys), char.ToUpperInvariant(c).ToString());
                return true;
            }
            if (char.IsDigit(c))
            {
                key = (Keys)((int)Keys.D0 + (c - '0'));
                return true;
            }
            if (c == '`') { key = Keys.Oemtilde; return true; }
            if (c == '-') { key = Keys.OemMinus; return true; }
            if (c == '=') { key = Keys.Oemplus; return true; }
        }

        if (text.StartsWith("Num", StringComparison.OrdinalIgnoreCase) &&
            text.Length == 4 && char.IsDigit(text[3]))
        {
            key = (Keys)((int)Keys.NumPad0 + (text[3] - '0'));
            return true;
        }

        if (Enum.TryParse<Keys>(text, true, out var parsed))
        {
            var keyCode = parsed & Keys.KeyCode;
            if (keyCode != Keys.None && keyCode is not Keys.ControlKey and not Keys.ShiftKey and not Keys.Menu)
            {
                key = keyCode;
                return true;
            }
        }

        return false;
    }
}

internal static class ChatNativeMethods
{
    internal const int WmHotKey = 0x0312;
    internal const int WmNcHitTest = 0x0084;
    internal const int WmNclButtonDown = 0x00A1;
    internal const int HtCaption = 2;
    internal const int HtLeft = 10;
    internal const int HtRight = 11;
    internal const int HtTop = 12;
    internal const int HtTopLeft = 13;
    internal const int HtTopRight = 14;
    internal const int HtBottom = 15;
    internal const int HtBottomLeft = 16;
    internal const int HtBottomRight = 17;

    internal const uint ModAlt = 0x0001;
    internal const uint ModControl = 0x0002;
    internal const uint ModShift = 0x0004;
    internal const uint ModNoRepeat = 0x4000;

    private const int GwlExStyle = -20;
    private const long WsExTransparent = 0x00000020L;

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    internal static extern bool RegisterHotKey(IntPtr hWnd, int id, uint fsModifiers, uint vk);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    internal static extern bool UnregisterHotKey(IntPtr hWnd, int id);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool ReleaseCapture();

    [DllImport("user32.dll")]
    private static extern IntPtr SendMessage(IntPtr hWnd, int msg, IntPtr wParam, IntPtr lParam);

    [DllImport("user32.dll", EntryPoint = "GetWindowLongPtrW", SetLastError = true)]
    private static extern IntPtr GetWindowLongPtr64(IntPtr hWnd, int nIndex);

    [DllImport("user32.dll", EntryPoint = "SetWindowLongPtrW", SetLastError = true)]
    private static extern IntPtr SetWindowLongPtr64(IntPtr hWnd, int nIndex, IntPtr dwNewLong);

    [DllImport("user32.dll", EntryPoint = "GetWindowLongW", SetLastError = true)]
    private static extern int GetWindowLong32(IntPtr hWnd, int nIndex);

    [DllImport("user32.dll", EntryPoint = "SetWindowLongW", SetLastError = true)]
    private static extern int SetWindowLong32(IntPtr hWnd, int nIndex, int dwNewLong);

    internal static void BeginWindowDrag(IntPtr handle)
    {
        _ = ReleaseCapture();
        _ = SendMessage(handle, WmNclButtonDown, new IntPtr(HtCaption), IntPtr.Zero);
    }

    internal static bool SetClickThrough(IntPtr handle, bool enabled)
    {
        if (handle == IntPtr.Zero) return false;

        var style = GetWindowLongPtr(handle, GwlExStyle).ToInt64();
        var next = enabled ? style | WsExTransparent : style & ~WsExTransparent;
        if (next == style) return true;

        Marshal.SetLastPInvokeError(0);
        _ = SetWindowLongPtr(handle, GwlExStyle, new IntPtr(next));
        return Marshal.GetLastPInvokeError() == 0;
    }

    private static IntPtr GetWindowLongPtr(IntPtr hWnd, int nIndex) =>
        IntPtr.Size == 8 ? GetWindowLongPtr64(hWnd, nIndex) : new IntPtr(GetWindowLong32(hWnd, nIndex));

    private static IntPtr SetWindowLongPtr(IntPtr hWnd, int nIndex, IntPtr value) =>
        IntPtr.Size == 8 ? SetWindowLongPtr64(hWnd, nIndex, value) : new IntPtr(SetWindowLong32(hWnd, nIndex, value.ToInt32()));
}

internal sealed class ChatMessageListBox : ListBox
{
    private const int WmVScroll = 0x0115;
    private Font? _stableControlFont;
    private bool _isolateOwnerDrawFont;
    private bool _normalizingFont;

    internal event EventHandler? ViewportChanged;

    internal ChatMessageListBox()
    {
        DrawMode = DrawMode.OwnerDrawVariable;
        IntegralHeight = false;
        BorderStyle = BorderStyle.None;
        SelectionMode = SelectionMode.One;
        SetStyle(ControlStyles.OptimizedDoubleBuffer, true);

        // The real message fonts are owned/disposed by ChatOverlayForm's custom
        // renderer. Keep the native ListBox FONT property on a process-owned,
        // stable system font so WinForms can always create/recreate the HWND safely.
        // This deliberately prevents a transient render Font from becoming the
        // control Font and later being disposed while WinForms still references it.
        _stableControlFont = SystemFonts.MessageBoxFont;
        base.Font = _stableControlFont;
        _isolateOwnerDrawFont = true;
    }

    protected override void OnFontChanged(EventArgs e)
    {
        if (_isolateOwnerDrawFont && !_normalizingFont && _stableControlFont is not null &&
            !ReferenceEquals(Font, _stableControlFont))
        {
            _normalizingFont = true;
            try
            {
                base.Font = _stableControlFont;
            }
            finally
            {
                _normalizingFont = false;
            }
            return;
        }

        base.OnFontChanged(e);
    }

    protected override void OnMouseWheel(MouseEventArgs e)
    {
        base.OnMouseWheel(e);
        RaiseViewportChangedSoon();
    }

    protected override void OnKeyUp(KeyEventArgs e)
    {
        base.OnKeyUp(e);
        if (e.KeyCode is Keys.Up or Keys.Down or Keys.PageUp or Keys.PageDown or Keys.Home or Keys.End)
            RaiseViewportChangedSoon();
    }

    protected override void WndProc(ref Message m)
    {
        base.WndProc(ref m);
        if (m.Msg == WmVScroll)
            RaiseViewportChangedSoon();
    }

    private void RaiseViewportChangedSoon()
    {
        if (IsDisposed || !IsHandleCreated) return;
        try { BeginInvoke(new Action(() => ViewportChanged?.Invoke(this, EventArgs.Empty))); }
        catch (InvalidOperationException) { }
        catch (Win32Exception) { }
    }
}

internal static class ChatColorUtil
{
    internal static Color Parse(string? html, Color fallback)
    {
        if (string.IsNullOrWhiteSpace(html)) return fallback;
        try { return ColorTranslator.FromHtml(html); }
        catch { return fallback; }
    }

    internal static string ToHtml(Color color) => $"#{color.R:X2}{color.G:X2}{color.B:X2}";

    internal static Color Blend(Color foreground, Color background, int foregroundPercent)
    {
        var t = Math.Clamp(foregroundPercent, 0, 100) / 100d;
        return Color.FromArgb(
            (int)Math.Round(background.R + (foreground.R - background.R) * t),
            (int)Math.Round(background.G + (foreground.G - background.G) * t),
            (int)Math.Round(background.B + (foreground.B - background.B) * t));
    }
}
