using System.Drawing;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

/// <summary>
/// Lightweight ImGui/ZDPS-style slider used by Settings. The existing TrackBar
/// remains the backing value/control for compatibility and dirty-state tracking;
/// this control is only the compact visual/input surface.
/// </summary>
internal sealed class ChatCompactSlider : Control
{
    private int _minimum;
    private int _maximum = 100;
    private int _value;
    private bool _dragging;

    internal event EventHandler? ValueChanged;

    internal int Minimum
    {
        get => _minimum;
        set
        {
            _minimum = value;
            if (_maximum < _minimum) _maximum = _minimum;
            Value = _value;
            Invalidate();
        }
    }

    internal int Maximum
    {
        get => _maximum;
        set
        {
            _maximum = Math.Max(value, _minimum);
            Value = _value;
            Invalidate();
        }
    }

    internal int Value
    {
        get => _value;
        set
        {
            var clamped = Math.Clamp(value, _minimum, _maximum);
            if (_value == clamped) return;
            _value = clamped;
            Invalidate();
            ValueChanged?.Invoke(this, EventArgs.Empty);
            AccessibilityNotifyClients(AccessibleEvents.ValueChange, -1);
        }
    }

    internal ChatCompactSlider()
    {
        SetStyle(
            ControlStyles.AllPaintingInWmPaint |
            ControlStyles.OptimizedDoubleBuffer |
            ControlStyles.ResizeRedraw |
            ControlStyles.UserPaint |
            ControlStyles.Selectable,
            true);
        TabStop = true;
        Cursor = Cursors.Hand;
        Height = 24;
        MinimumSize = new Size(80, 24);
        AccessibleRole = AccessibleRole.Slider;
    }

    protected override void OnMouseDown(MouseEventArgs e)
    {
        base.OnMouseDown(e);
        if (!Enabled || e.Button != MouseButtons.Left) return;
        Focus();
        _dragging = true;
        Capture = true;
        SetFromMouse(e.X);
    }

    protected override void OnMouseMove(MouseEventArgs e)
    {
        base.OnMouseMove(e);
        if (_dragging && Enabled) SetFromMouse(e.X);
    }

    protected override void OnMouseUp(MouseEventArgs e)
    {
        base.OnMouseUp(e);
        if (e.Button != MouseButtons.Left) return;
        _dragging = false;
        Capture = false;
    }

    protected override void OnKeyDown(KeyEventArgs e)
    {
        base.OnKeyDown(e);
        if (!Enabled) return;

        var small = Math.Max(1, (_maximum - _minimum) / 20);
        var large = Math.Max(1, (_maximum - _minimum) / 10);
        var handled = true;
        switch (e.KeyCode)
        {
            case Keys.Left:
            case Keys.Down:
                Value -= small;
                break;
            case Keys.Right:
            case Keys.Up:
                Value += small;
                break;
            case Keys.PageDown:
                Value -= large;
                break;
            case Keys.PageUp:
                Value += large;
                break;
            case Keys.Home:
                Value = _minimum;
                break;
            case Keys.End:
                Value = _maximum;
                break;
            default:
                handled = false;
                break;
        }

        if (handled)
        {
            e.Handled = true;
            e.SuppressKeyPress = true;
        }
    }

    protected override void OnPaint(PaintEventArgs e)
    {
        base.OnPaint(e);
        e.Graphics.SmoothingMode = System.Drawing.Drawing2D.SmoothingMode.AntiAlias;

        var left = 6;
        var right = Math.Max(left + 1, Width - 7);
        var centerY = Height / 2;
        var track = new Rectangle(left, centerY - 2, Math.Max(1, right - left), 4);
        var ratio = _maximum == _minimum ? 0d : (_value - _minimum) / (double)(_maximum - _minimum);
        var thumbX = left + (int)Math.Round(track.Width * ratio);

        var trackColor = Enabled ? Color.FromArgb(89, 89, 89) : Color.FromArgb(61, 61, 61);
        var fillColor = Enabled ? Color.FromArgb(0, 145, 214) : Color.FromArgb(72, 91, 101);
        var thumbColor = Enabled ? Color.FromArgb(31, 168, 229) : Color.FromArgb(93, 100, 104);

        using var trackBrush = new SolidBrush(trackColor);
        using var fillBrush = new SolidBrush(fillColor);
        using var thumbBrush = new SolidBrush(thumbColor);
        e.Graphics.FillRectangle(trackBrush, track);
        if (thumbX > left)
            e.Graphics.FillRectangle(fillBrush, new Rectangle(left, track.Top, thumbX - left, track.Height));

        var thumb = new Rectangle(thumbX - 5, centerY - 6, 10, 12);
        e.Graphics.FillRectangle(thumbBrush, thumb);

        if (Focused && ShowFocusCues)
        {
            var focus = ClientRectangle;
            focus.Inflate(-1, -1);
            ControlPaint.DrawFocusRectangle(e.Graphics, focus, ForeColor, BackColor);
        }
    }

    private void SetFromMouse(int x)
    {
        var left = 6;
        var width = Math.Max(1, Width - 13);
        var ratio = Math.Clamp((x - left) / (double)width, 0d, 1d);
        Value = _minimum + (int)Math.Round((_maximum - _minimum) * ratio);
    }
}
