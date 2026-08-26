using System.ComponentModel;
using System.Drawing;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal static class ChatUiTheme
{
    internal static readonly Color Window = Color.FromArgb(20, 23, 28);
    internal static readonly Color Surface = Color.FromArgb(26, 30, 36);
    internal static readonly Color SurfaceRaised = Color.FromArgb(31, 36, 44);
    internal static readonly Color SurfaceHover = Color.FromArgb(39, 45, 55);
    internal static readonly Color Border = Color.FromArgb(50, 58, 70);
    internal static readonly Color BorderStrong = Color.FromArgb(67, 77, 91);
    internal static readonly Color Text = Color.FromArgb(239, 243, 247);
    internal static readonly Color Muted = Color.FromArgb(163, 174, 187);
    internal static readonly Color MutedDim = Color.FromArgb(124, 135, 149);
    internal static readonly Color Accent = Color.FromArgb(73, 132, 255);
    internal static readonly Color AccentHover = Color.FromArgb(91, 146, 255);
    internal static readonly Color AccentPressed = Color.FromArgb(55, 112, 229);
    internal static readonly Color Success = Color.FromArgb(91, 211, 145);
    internal static readonly Color Warning = Color.FromArgb(245, 191, 91);
    internal static readonly Color Danger = Color.FromArgb(244, 113, 116);
    internal static readonly Color Input = Color.FromArgb(18, 21, 26);

    internal static Font UiFont(float size = 9F, FontStyle style = FontStyle.Regular) =>
        new("Segoe UI", size, style, GraphicsUnit.Point);

    internal static void ApplyForm(Form form)
    {
        form.BackColor = Window;
        form.ForeColor = Text;
        form.Font = UiFont();
        form.AutoScaleMode = AutoScaleMode.Dpi;
        form.AutoScaleDimensions = new SizeF(96F, 96F);
    }

    internal static void StylePrimaryButton(Button button)
    {
        StyleButtonBase(button);
        button.BackColor = Accent;
        button.ForeColor = Color.White;
        button.FlatAppearance.MouseOverBackColor = AccentHover;
        button.FlatAppearance.MouseDownBackColor = AccentPressed;
        button.FlatAppearance.BorderSize = 0;
        button.Padding = new Padding(14, 0, 14, 0);
    }

    internal static void StyleSecondaryButton(Button button)
    {
        StyleButtonBase(button);
        button.BackColor = SurfaceRaised;
        button.ForeColor = Text;
        button.FlatAppearance.BorderSize = 1;
        button.FlatAppearance.BorderColor = BorderStrong;
        button.FlatAppearance.MouseOverBackColor = SurfaceHover;
        button.FlatAppearance.MouseDownBackColor = Surface;
        button.Padding = new Padding(12, 0, 12, 0);
    }

    internal static void StyleGhostButton(Button button)
    {
        StyleButtonBase(button);
        button.BackColor = Surface;
        button.ForeColor = Muted;
        button.FlatAppearance.BorderSize = 0;
        button.FlatAppearance.MouseOverBackColor = SurfaceHover;
        button.FlatAppearance.MouseDownBackColor = SurfaceRaised;
    }

    private static void StyleButtonBase(Button button)
    {
        button.FlatStyle = FlatStyle.Flat;
        button.UseVisualStyleBackColor = false;
        button.Cursor = Cursors.Hand;
        button.Height = Math.Max(button.Height, 34);
    }

    internal static void StyleTextBox(TextBox box, bool multiline = false)
    {
        box.BackColor = Input;
        box.ForeColor = Text;
        box.BorderStyle = BorderStyle.FixedSingle;
        box.Margin = new Padding(0);
        if (multiline)
        {
            box.Multiline = true;
            box.AcceptsReturn = true;
            box.ScrollBars = ScrollBars.Vertical;
        }
    }

    internal static void StyleComboBox(ComboBox combo)
    {
        combo.BackColor = Input;
        combo.ForeColor = Text;
        combo.DropDownStyle = ComboBoxStyle.DropDownList;
        combo.FlatStyle = FlatStyle.Flat;
    }

    internal static void StyleNumeric(NumericUpDown numeric)
    {
        numeric.BackColor = Input;
        numeric.ForeColor = Text;
        numeric.BorderStyle = BorderStyle.FixedSingle;
    }

    internal static void StyleCheckBox(CheckBox check)
    {
        check.AutoSize = true;
        check.ForeColor = Text;
        check.BackColor = Color.Transparent;
        check.Margin = new Padding(0, 4, 0, 4);
    }

    internal static Label Heading(string text, float size = 16F) => new()
    {
        AutoSize = true,
        Text = text,
        ForeColor = Text,
        Font = UiFont(size, FontStyle.Bold),
        Margin = Padding.Empty
    };

    internal static Label Subheading(string text) => new()
    {
        AutoSize = true,
        MaximumSize = new Size(820, 0),
        Text = text,
        ForeColor = Muted,
        Font = UiFont(9F),
        Margin = new Padding(0, 6, 0, 0)
    };

    internal static Label FieldLabel(string text) => new()
    {
        AutoSize = true,
        Text = text,
        ForeColor = Text,
        Font = UiFont(9F, FontStyle.Bold),
        Margin = Padding.Empty
    };

    internal static Label Hint(string text) => new()
    {
        AutoSize = true,
        MaximumSize = new Size(780, 0),
        Text = text,
        ForeColor = Muted,
        Font = UiFont(8.5F),
        Margin = Padding.Empty
    };

    internal static Panel Divider() => new()
    {
        Dock = DockStyle.Top,
        Height = 1,
        BackColor = Border
    };
}

internal sealed class ChatCardPanel : Panel
{
    internal ChatCardPanel()
    {
        DoubleBuffered = true;
        BackColor = ChatUiTheme.Surface;
        ForeColor = ChatUiTheme.Text;
        Padding = new Padding(18);
        Margin = new Padding(0, 0, 0, 14);
    }

    protected override void OnPaint(PaintEventArgs e)
    {
        base.OnPaint(e);
        using var pen = new Pen(ChatUiTheme.Border);
        var rect = ClientRectangle;
        rect.Width -= 1;
        rect.Height -= 1;
        if (rect.Width > 0 && rect.Height > 0)
            e.Graphics.DrawRectangle(pen, rect);
    }
}

internal sealed class ChatNavButton : Button
{
    private bool _selected;

    [Browsable(false)]
    [DesignerSerializationVisibility(DesignerSerializationVisibility.Hidden)]
    internal bool Selected
    {
        get => _selected;
        set
        {
            if (_selected == value) return;
            _selected = value;
            UpdateVisuals();
        }
    }

    internal ChatNavButton()
    {
        FlatStyle = FlatStyle.Flat;
        FlatAppearance.BorderSize = 0;
        Height = 42;
        TextAlign = ContentAlignment.MiddleLeft;
        Padding = new Padding(15, 0, 8, 0);
        Margin = new Padding(0, 0, 0, 4);
        Cursor = Cursors.Hand;
        ForeColor = ChatUiTheme.Muted;
        BackColor = ChatUiTheme.Window;
        Font = ChatUiTheme.UiFont(9F, FontStyle.Bold);
        UseVisualStyleBackColor = false;
        UpdateVisuals();
    }

    private void UpdateVisuals()
    {
        BackColor = _selected ? ChatUiTheme.SurfaceRaised : ChatUiTheme.Window;
        ForeColor = _selected ? ChatUiTheme.Text : ChatUiTheme.Muted;
        FlatAppearance.MouseOverBackColor = _selected ? ChatUiTheme.SurfaceRaised : ChatUiTheme.Surface;
        FlatAppearance.MouseDownBackColor = ChatUiTheme.SurfaceRaised;
        Invalidate();
    }

    protected override void OnPaint(PaintEventArgs pevent)
    {
        base.OnPaint(pevent);
        if (!_selected) return;
        using var brush = new SolidBrush(ChatUiTheme.Accent);
        pevent.Graphics.FillRectangle(brush, 0, 6, 3, Math.Max(1, Height - 12));
    }
}

internal sealed class ChatTabButton : Button
{
    private bool _selected;

    [Browsable(false)]
    [DesignerSerializationVisibility(DesignerSerializationVisibility.Hidden)]
    internal bool Selected
    {
        get => _selected;
        set
        {
            if (_selected == value) return;
            _selected = value;
            ApplyState();
        }
    }

    internal ChatTabButton()
    {
        AutoSize = true;
        AutoSizeMode = AutoSizeMode.GrowAndShrink;
        MinimumSize = new Size(72, 32);
        Height = 32;
        FlatStyle = FlatStyle.Flat;
        FlatAppearance.BorderSize = 0;
        Padding = new Padding(14, 0, 14, 0);
        Margin = new Padding(0, 4, 4, 4);
        Cursor = Cursors.Hand;
        Font = ChatUiTheme.UiFont(9F, FontStyle.Bold);
        UseVisualStyleBackColor = false;
        ApplyState();
    }

    private void ApplyState()
    {
        BackColor = _selected ? ChatUiTheme.SurfaceRaised : ChatUiTheme.Surface;
        ForeColor = _selected ? ChatUiTheme.Text : ChatUiTheme.Muted;
        FlatAppearance.MouseOverBackColor = _selected ? ChatUiTheme.SurfaceRaised : ChatUiTheme.SurfaceHover;
        FlatAppearance.MouseDownBackColor = ChatUiTheme.SurfaceRaised;
        Invalidate();
    }

    protected override void OnPaint(PaintEventArgs pevent)
    {
        base.OnPaint(pevent);
        if (!_selected) return;
        using var brush = new SolidBrush(ChatUiTheme.Accent);
        pevent.Graphics.FillRectangle(brush, 10, Height - 3, Math.Max(1, Width - 20), 3);
    }
}
