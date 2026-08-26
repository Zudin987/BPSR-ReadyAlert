using System.Drawing;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed partial class ChatGeneralSettingsForm : Form
{
    private readonly ChatOverlaySettings _settings;
    private List<ChatBlockedUser> _blockedWorking;
    private Dictionary<int, string> _channelColorsWorking;
    private string _highlightColorValue;
    private string _privateColorValue;

    private readonly Panel _contentHost = new();
    private readonly FlowLayoutPanel _navHost = new();
    private readonly Dictionary<string, (ChatNavButton Button, Control Page)> _pages = new(StringComparer.Ordinal);
    private readonly CheckBox _topMost = new() { Text = "Keep overlay above other windows" };
    private readonly CheckBox _compact = new() { Text = "Compact one-line messages" };
    private readonly CheckBox _showTime = new() { Text = "Show timestamps" };
    private readonly CheckBox _timeAgo = new() { Text = "Use short relative time (20s, 3m, 2h)" };
    private readonly CheckBox _hideStickers = new() { Text = "Hide sticker messages" };
    private readonly CheckBox _bold = new() { Text = "Bold message text" };
    private readonly CheckBox _shadow = new() { Text = "Text shadow for readability" };
    private readonly CheckBox _separators = new() { Text = "Divider between messages" };
    private readonly CheckBox _zebra = new() { Text = "Alternating row shading" };
    private readonly CheckBox _colorBand = new() { Text = "Channel color strip" };
    private readonly CheckBox _clickThrough = new() { Text = "Click-through mode" };
    private readonly ComboBox _fontFamily = new();
    private readonly NumericUpDown _fontSize = new();
    private readonly NumericUpDown _maxHistory = new();
    private readonly TrackBar _backgroundOpacity = new();
    private readonly TrackBar _toolbarOpacity = new();
    private readonly TrackBar _textOpacity = new();
    private readonly TrackBar _windowOpacity = new();
    private readonly Label _backgroundOpacityValue = new();
    private readonly Label _toolbarOpacityValue = new();
    private readonly Label _textOpacityValue = new();
    private readonly Label _windowOpacityValue = new();
    private readonly HotkeyCaptureTextBox _clickHotkey = new();
    private readonly HotkeyCaptureTextBox _collapseHotkey = new();
    private readonly ComboBox _collapseSide = new();
    private readonly TextBox _highlight = new();
    private readonly Label _highlightValidation = new();
    private readonly Button _highlightColor = new();
    private readonly CheckBox _highlightSound = new() { Text = "Play a sound when the keyword rule matches" };
    private readonly TextBox _highlightSoundPath = new();
    private readonly CheckBox _privateHighlight = new() { Text = "Highlight Private / Talk messages" };
    private readonly Button _privateColor = new();
    private readonly CheckBox _privateSound = new() { Text = "Play a sound for Private / Talk messages" };
    private readonly TextBox _privateSoundPath = new();
    private readonly TrackBar _soundVolume = new();
    private readonly Label _soundVolumeValue = new();
    private readonly Label _applyStatus = new();

    internal ChatGeneralSettingsForm(ChatOverlaySettings settings)
    {
        _settings = settings;
        _blockedWorking = settings.BlockedUsers.Select(CloneBlockedUser).ToList();
        _channelColorsWorking = new Dictionary<int, string>(settings.ChannelColors);
        _highlightColorValue = settings.HighlightColor;
        _privateColorValue = settings.PrivateHighlightColor;

        ChatUiTheme.ApplyForm(this);
        Text = "Chat Overlay Settings";
        StartPosition = FormStartPosition.CenterParent;
        FormBorderStyle = FormBorderStyle.Sizable;
        MaximizeBox = true;
        MinimizeBox = false;
        ShowInTaskbar = false;
        ClientSize = new Size(980, 720);
        MinimumSize = new Size(820, 620);

        var footer = BuildFooter();
        var shell = new Panel { Dock = DockStyle.Fill, BackColor = ChatUiTheme.Window };
        var sidebar = BuildSidebar();
        _contentHost.Dock = DockStyle.Fill;
        _contentHost.BackColor = ChatUiTheme.Window;

        shell.Controls.Add(_contentHost);
        shell.Controls.Add(sidebar);
        Controls.Add(shell);
        Controls.Add(footer);

        RegisterPage("Appearance", "Appearance", BuildAppearancePage());
        RegisterPage("Interaction", "Interaction", BuildInteractionPage());
        RegisterPage("Alerts", "Highlights & sounds", BuildAlertsPage());
        RegisterPage("Advanced", "Advanced", BuildAdvancedPage());
        ShowPage("Appearance");
    }

    private Panel BuildFooter()
    {
        var footer = new Panel
        {
            Dock = DockStyle.Bottom,
            Height = 70,
            BackColor = ChatUiTheme.Surface,
            Padding = Padding.Empty
        };
        var root = new TableLayoutPanel
        {
            Dock = DockStyle.Fill,
            ColumnCount = 1,
            RowCount = 2,
            Margin = Padding.Empty,
            Padding = Padding.Empty,
            BackColor = ChatUiTheme.Surface
        };
        root.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100F));
        root.RowStyles.Add(new RowStyle(SizeType.Absolute, 1F));
        root.RowStyles.Add(new RowStyle(SizeType.Percent, 100F));
        root.Controls.Add(new Panel { Dock = DockStyle.Fill, BackColor = ChatUiTheme.Border, Margin = Padding.Empty }, 0, 0);

        var actions = new TableLayoutPanel
        {
            Dock = DockStyle.Fill,
            ColumnCount = 7,
            RowCount = 1,
            Margin = Padding.Empty,
            Padding = new Padding(20, 14, 20, 14),
            BackColor = ChatUiTheme.Surface
        };
        actions.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 144F));
        actions.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 16F));
        actions.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100F));
        actions.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 90F));
        actions.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 96F));
        actions.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 10F));
        actions.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 128F));
        actions.RowStyles.Add(new RowStyle(SizeType.Percent, 100F));

        var reset = new Button { Text = "Reset to defaults", Dock = DockStyle.Fill, Margin = Padding.Empty };
        var save = new Button { Text = "Save changes", Dock = DockStyle.Fill, Margin = Padding.Empty };
        var close = new Button { Text = "Close", Dock = DockStyle.Fill, DialogResult = DialogResult.Cancel, Margin = Padding.Empty };
        ChatUiTheme.StyleSecondaryButton(reset);
        ChatUiTheme.StylePrimaryButton(save);
        ChatUiTheme.StyleSecondaryButton(close);
        reset.Margin = Padding.Empty;
        save.Margin = Padding.Empty;
        close.Margin = Padding.Empty;

        _applyStatus.Dock = DockStyle.Fill;
        _applyStatus.TextAlign = ContentAlignment.MiddleRight;
        _applyStatus.ForeColor = ChatUiTheme.Success;
        _applyStatus.Font = ChatUiTheme.UiFont(8.5F, FontStyle.Bold);
        _applyStatus.Text = string.Empty;

        reset.Click += (_, _) => ResetToDefaultsAndApply();
        save.Click += (_, _) => ApplyChanges();
        actions.Controls.Add(reset, 0, 0);
        actions.Controls.Add(_applyStatus, 3, 0);
        actions.Controls.Add(close, 4, 0);
        actions.Controls.Add(save, 6, 0);
        root.Controls.Add(actions, 0, 1);
        footer.Controls.Add(root);
        AcceptButton = save;
        CancelButton = close;
        return footer;
    }

    private Panel BuildSidebar()
    {
        var sidebar = new Panel
        {
            Dock = DockStyle.Left,
            Width = 196,
            BackColor = ChatUiTheme.Window,
            Padding = new Padding(16, 18, 14, 14)
        };
        var brand = new Label
        {
            Dock = DockStyle.Top,
            Height = 58,
            Text = "CHAT OVERLAY\r\nSETTINGS",
            ForeColor = ChatUiTheme.Muted,
            Font = ChatUiTheme.UiFont(8.5F, FontStyle.Bold),
            TextAlign = ContentAlignment.TopLeft,
            Padding = new Padding(8, 4, 0, 0)
        };
        _navHost.Dock = DockStyle.Fill;
        _navHost.FlowDirection = FlowDirection.TopDown;
        _navHost.WrapContents = false;
        _navHost.AutoScroll = false;
        _navHost.Padding = new Padding(0, 6, 0, 0);
        _navHost.Margin = Padding.Empty;
        _navHost.SizeChanged += (_, _) =>
        {
            foreach (Control child in _navHost.Controls)
                child.Width = Math.Max(130, _navHost.ClientSize.Width - 2);
        };

        sidebar.Controls.Add(_navHost);
        sidebar.Controls.Add(brand);
        return sidebar;
    }

    private void RegisterPage(string key, string navText, Control page)
    {
        var button = new ChatNavButton { Text = navText, Width = 164 };
        button.Click += (_, _) => ShowPage(key);
        _navHost.Controls.Add(button);
        page.Dock = DockStyle.Fill;
        page.Visible = false;
        _contentHost.Controls.Add(page);
        _pages[key] = (button, page);
    }

    private void ShowPage(string key)
    {
        foreach (var pair in _pages)
        {
            var active = string.Equals(pair.Key, key, StringComparison.Ordinal);
            pair.Value.Button.Selected = active;
            pair.Value.Page.Visible = active;
            if (active) pair.Value.Page.BringToFront();
        }
    }
}
