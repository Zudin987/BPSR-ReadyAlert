using System.ComponentModel;
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

    [Browsable(false)]
    [DesignerSerializationVisibility(DesignerSerializationVisibility.Hidden)]
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

    [Browsable(false)]
    [DesignerSerializationVisibility(DesignerSerializationVisibility.Hidden)]
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

    [Browsable(false)]
    [DesignerSerializationVisibility(DesignerSerializationVisibility.Hidden)]
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
            if (IsHandleCreated)
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

    protected override bool IsInputKey(Keys keyData)
    {
        var key = keyData & Keys.KeyCode;
        if (key is Keys.Left or Keys.Right or Keys.Up or Keys.Down or
            Keys.PageUp or Keys.PageDown or Keys.Home or Keys.End)
            return true;
        return base.IsInputKey(keyData);
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

    protected override void OnMouseCaptureChanged(EventArgs e)
    {
        base.OnMouseCaptureChanged(e);
        if (!Capture) _dragging = false;
    }

    protected override void OnEnabledChanged(EventArgs e)
    {
        if (!Enabled)
        {
            _dragging = false;
            if (Capture) Capture = false;
        }
        base.OnEnabledChanged(e);
        Invalidate();
    }

    protected override void OnKeyDown(KeyEventArgs e)
    {
        base.OnKeyDown(e);
        if (!Enabled) return;

        var span = Math.Max(0, _maximum - _minimum);
        var small = Math.Max(1, Math.Min(5, span));
        var large = Math.Max(1, Math.Min(10, span));
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

        var left = ScalePx(6);
        var rightMargin = ScalePx(7);
        var trackHeight = ScalePx(4);
        var thumbWidth = ScalePx(10);
        var thumbHeight = ScalePx(12);
        var right = Math.Max(left + 1, Width - rightMargin);
        var centerY = Height / 2;
        var track = new Rectangle(
            left,
            centerY - trackHeight / 2,
            Math.Max(1, right - left),
            trackHeight);
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

        var thumb = new Rectangle(
            thumbX - thumbWidth / 2,
            centerY - thumbHeight / 2,
            thumbWidth,
            thumbHeight);
        e.Graphics.FillRectangle(thumbBrush, thumb);

        if (Focused && ShowFocusCues)
        {
            var focus = ClientRectangle;
            focus.Inflate(-1, -1);
            ControlPaint.DrawFocusRectangle(e.Graphics, focus, ForeColor, BackColor);
        }
    }

    internal bool TreatsKeyAsInputForSelfTest(Keys keyData) => IsInputKey(keyData);

    private void SetFromMouse(int x)
    {
        var left = ScalePx(6);
        var rightMargin = ScalePx(7);
        var width = Math.Max(1, Width - left - rightMargin);
        var ratio = Math.Clamp((x - left) / (double)width, 0d, 1d);
        Value = _minimum + (int)Math.Round((_maximum - _minimum) * ratio);
    }

    private int ScalePx(int logicalPixels) =>
        Math.Max(1, (int)Math.Round(logicalPixels * Math.Max(96, DeviceDpi) / 96d));
}
