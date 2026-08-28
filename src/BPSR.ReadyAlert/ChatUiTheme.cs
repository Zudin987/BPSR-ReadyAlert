using System.ComponentModel;
using System.Drawing;
using System.Runtime.InteropServices;
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

    // Settings-only palette. Deliberately close to ZDPS/ImGui's neutral charcoal
    // density without changing the chat overlay's existing visual identity.
    internal static readonly Color SettingsWindow = Color.FromArgb(37, 37, 37);
    internal static readonly Color SettingsSurface = Color.FromArgb(42, 42, 42);
    internal static readonly Color SettingsSurfaceHover = Color.FromArgb(52, 52, 52);
    internal static readonly Color SettingsInput = Color.FromArgb(47, 47, 47);
    internal static readonly Color SettingsBorder = Color.FromArgb(82, 82, 82);
    internal static readonly Color SettingsText = Color.FromArgb(238, 238, 238);
    internal static readonly Color SettingsMuted = Color.FromArgb(166, 166, 166);
    internal static readonly Color SettingsAccent = Color.FromArgb(0, 145, 214);
    internal static readonly Color SettingsAccentHover = Color.FromArgb(15, 161, 229);
    internal static readonly Color SettingsSave = Color.FromArgb(0, 116, 15);
    internal static readonly Color SettingsSaveHover = Color.FromArgb(0, 135, 18);
    internal static readonly Color SettingsClose = Color.FromArgb(168, 0, 12);
    internal static readonly Color SettingsCloseHover = Color.FromArgb(194, 0, 15);

    private const int DwmwaUseImmersiveDarkMode = 20;
    private const int DwmwaUseImmersiveDarkModeLegacy = 19;

    [DllImport("dwmapi.dll", PreserveSig = true)]
    private static extern int DwmSetWindowAttribute(IntPtr hwnd, int attribute, ref int value, int valueSize);

    internal static Font UiFont(float size = 9F, FontStyle style = FontStyle.Regular) =>
        new("Segoe UI", size, style, GraphicsUnit.Point);

    internal static void ApplyForm(Form form)
    {
        form.BackColor = Window;
        form.ForeColor = Text;
        form.Font = UiFont();
        form.AutoScaleMode = AutoScaleMode.Dpi;
        form.AutoScaleDimensions = new SizeF(96F, 96F);
        form.HandleCreated += (_, _) => TryUseDarkTitleBar(form);
        if (form.IsHandleCreated) TryUseDarkTitleBar(form);
    }

    internal static void ApplySettingsForm(Form form)
    {
        ApplyForm(form);
        form.BackColor = SettingsWindow;
        form.ForeColor = SettingsText;
    }

    private static void TryUseDarkTitleBar(Form form)
    {
        if (!OperatingSystem.IsWindows() || !form.IsHandleCreated) return;
        try
        {
            var enabled = 1;
            var result = DwmSetWindowAttribute(form.Handle, DwmwaUseImmersiveDarkMode, ref enabled, sizeof(int));
            if (result != 0)
                _ = DwmSetWindowAttribute(form.Handle, DwmwaUseImmersiveDarkModeLegacy, ref enabled, sizeof(int));
        }
        catch (DllNotFoundException) { }
        catch (EntryPointNotFoundException) { }
    }

    internal static void StylePrimaryButton(Button button)
    {
        StyleButtonBase(button);
        button.Margin = Padding.Empty;
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

    internal static void StyleSettingsButton(Button button)
    {
        StyleSettingsButtonBase(button);
        button.BackColor = SettingsSurface;
        button.ForeColor = SettingsText;
        button.FlatAppearance.BorderSize = 1;
        button.FlatAppearance.BorderColor = SettingsBorder;
        button.FlatAppearance.MouseOverBackColor = SettingsSurfaceHover;
        button.FlatAppearance.MouseDownBackColor = SettingsInput;
        button.Padding = new Padding(10, 0, 10, 0);
    }

    internal static void StyleSettingsSaveButton(Button button)
    {
        StyleSettingsButtonBase(button);
        button.BackColor = SettingsSave;
        button.ForeColor = Color.White;
        button.FlatAppearance.BorderSize = 0;
        button.FlatAppearance.MouseOverBackColor = SettingsSaveHover;
        button.FlatAppearance.MouseDownBackColor = Color.FromArgb(0, 95, 12);
        button.Padding = new Padding(10, 0, 10, 0);
    }

    internal static void StyleSettingsCloseButton(Button button)
    {
        StyleSettingsButtonBase(button);
        button.BackColor = SettingsClose;
        button.ForeColor = Color.White;
        button.FlatAppearance.BorderSize = 0;
        button.FlatAppearance.MouseOverBackColor = SettingsCloseHover;
        button.FlatAppearance.MouseDownBackColor = Color.FromArgb(139, 0, 10);
        button.Padding = new Padding(10, 0, 10, 0);
    }

    private static void StyleButtonBase(Button button)
    {
        button.FlatStyle = FlatStyle.Flat;
        button.UseVisualStyleBackColor = false;
        button.Cursor = Cursors.Hand;
        button.Height = Math.Max(button.Height, 34);
    }

    private static void StyleSettingsButtonBase(Button button)
    {
        button.FlatStyle = FlatStyle.Flat;
        button.UseVisualStyleBackColor = false;
        button.Cursor = Cursors.Hand;
        button.Height = Math.Max(button.Height, 30);
    }

    internal static void StyleTextBox(TextBox box, bool multiline = false)
    {
        box.BackColor = Input;
        box.ForeColor = Text;
        box.BorderStyle = BorderStyle.FixedSingle;
        box.Margin = Padding.Empty;
        if (multiline)
        {
            box.Multiline = true;
            box.AcceptsReturn = true;
            box.ScrollBars = ScrollBars.Vertical;
        }
    }

    internal static void StyleSettingsTextBox(TextBox box, bool multiline = false)
    {
        StyleTextBox(box, multiline);
        box.BackColor = SettingsInput;
        box.ForeColor = SettingsText;
    }

    internal static void StyleComboBox(ComboBox combo)
    {
        combo.BackColor = Input;
        combo.ForeColor = Text;
        combo.DropDownStyle = ComboBoxStyle.DropDownList;
        combo.FlatStyle = FlatStyle.Flat;
    }

    internal static void StyleSettingsComboBox(ComboBox combo)
    {
        StyleComboBox(combo);
        combo.BackColor = SettingsInput;
        combo.ForeColor = SettingsText;
    }

    internal static void StyleNumeric(NumericUpDown numeric)
    {
        numeric.BackColor = Input;
        numeric.ForeColor = Text;
        numeric.BorderStyle = BorderStyle.FixedSingle;
    }

    internal static void StyleSettingsNumeric(NumericUpDown numeric)
    {
        StyleNumeric(numeric);
        numeric.BackColor = SettingsInput;
        numeric.ForeColor = SettingsText;
    }

    internal static void StyleCheckBox(CheckBox check)
    {
        check.AutoSize = true;
        check.ForeColor = Text;
        check.BackColor = Color.Transparent;
        check.Margin = new Padding(0, 4, 0, 4);
    }

    internal static void StyleSettingsCheckBox(CheckBox check)
    {
        check.AutoSize = true;
        check.ForeColor = SettingsText;
        check.BackColor = Color.Transparent;
        check.FlatStyle = FlatStyle.Flat;
        check.UseVisualStyleBackColor = false;
        check.Margin = new Padding(0, 2, 0, 2);
        check.Padding = Padding.Empty;
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
        MaximumSize = new Size(430, 0),
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

    internal static Label SettingsFieldLabel(string text) => new()
    {
        AutoSize = true,
        Text = text,
        ForeColor = SettingsText,
        Font = UiFont(9F),
        Margin = Padding.Empty
    };

    internal static Label Hint(string text) => new()
    {
        AutoSize = true,
        MaximumSize = new Size(430, 0),
        Text = text,
        ForeColor = Muted,
        Font = UiFont(8.5F),
        Margin = Padding.Empty
    };

    internal static Label SettingsHint(string text) => new()
    {
        AutoSize = true,
        MaximumSize = new Size(540, 0),
        Text = text,
        ForeColor = SettingsMuted,
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

// Generic card used by the overlay/support UI. Keep its pre-v1.2.3 semantics so
// compact Settings styling cannot leak into unrelated windows.
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

// Settings-specific flat section. This intentionally has no card border: the
// section heading helper draws the single separator line used by the compact UI.
internal sealed class ChatSettingsSectionPanel : Panel
{
    internal ChatSettingsSectionPanel()
    {
        DoubleBuffered = true;
        BackColor = ChatUiTheme.SettingsWindow;
        ForeColor = ChatUiTheme.SettingsText;
        Padding = Padding.Empty;
        Margin = new Padding(0, 0, 0, 8);
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
        Height = 28;
        AutoSize = true;
        AutoSizeMode = AutoSizeMode.GrowAndShrink;
        MinimumSize = new Size(70, 28);
        TextAlign = ContentAlignment.MiddleCenter;
        Padding = new Padding(10, 0, 10, 0);
        Margin = new Padding(0, 0, 2, 0);
        Cursor = Cursors.Hand;
        ForeColor = ChatUiTheme.SettingsText;
        BackColor = ChatUiTheme.SettingsWindow;
        Font = ChatUiTheme.UiFont(9F);
        UseVisualStyleBackColor = false;
        UpdateVisuals();
    }

    private void UpdateVisuals()
    {
        BackColor = _selected ? ChatUiTheme.SettingsAccent : ChatUiTheme.SettingsWindow;
        ForeColor = ChatUiTheme.SettingsText;
        FlatAppearance.MouseOverBackColor = _selected ? ChatUiTheme.SettingsAccentHover : ChatUiTheme.SettingsSurfaceHover;
        FlatAppearance.MouseDownBackColor = ChatUiTheme.SettingsAccent;
        Invalidate();
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
