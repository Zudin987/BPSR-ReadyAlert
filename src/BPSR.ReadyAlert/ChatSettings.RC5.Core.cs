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

    private readonly ChatBufferedPanel _contentHost = new();
    private readonly FlowLayoutPanel _navHost = new();
    private readonly Dictionary<string, (ChatNavButton Button, ChatSettingsPagePanel Page)> _pages = new(StringComparer.Ordinal);
    private string _activePageKey = string.Empty;
    private readonly CheckBox _topMost = new() { Text = "Always on top" };
    private readonly CheckBox _compact = new() { Text = "Compact messages" };
    private readonly CheckBox _showTime = new() { Text = "Timestamps" };
    private readonly CheckBox _timeAgo = new() { Text = "Relative time (20s, 3m, 2h)" };
    private readonly CheckBox _hideStickers = new() { Text = "Hide stickers" };
    private readonly CheckBox _bold = new() { Text = "Bold text" };
    private readonly CheckBox _shadow = new() { Text = "Text shadow" };
    private readonly CheckBox _separators = new() { Text = "Message dividers" };
    private readonly CheckBox _zebra = new() { Text = "Alternating rows" };
    private readonly CheckBox _colorBand = new() { Text = "Channel color strip" };
    private readonly CheckBox _clickThrough = new() { Text = "Click-through" };
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

    private readonly CheckBox[] _soundRuleEnabled =
    [
        new() { Text = "Enable sound rule 1" },
        new() { Text = "Enable sound rule 2" },
        new() { Text = "Enable sound rule 3" }
    ];
    private readonly TextBox[] _soundRuleMatch = [new(), new(), new()];
    private readonly TextBox[] _soundRulePath = [new(), new(), new()];
    private readonly Label[] _soundRuleValidation = [new(), new(), new()];

    private readonly CheckBox _privateHighlight = new() { Text = "Highlight Private / Talk" };
    private readonly Button _privateColor = new();
    private readonly CheckBox _privateSound = new() { Text = "Private / Talk sound" };
    private readonly TextBox _privateSoundPath = new();
    private readonly TrackBar _soundVolume = new();
    private readonly Label _soundVolumeValue = new();
    private readonly Label _applyStatus = new();

    internal ChatGeneralSettingsForm(ChatOverlaySettings settings)
        : this(settings, deferCompactUi: false)
    {
    }

    private ChatGeneralSettingsForm(ChatOverlaySettings settings, bool deferCompactUi)
    {
        _settings = settings;
        _blockedWorking = settings.BlockedUsers.Select(CloneBlockedUser).ToList();
        _channelColorsWorking = new Dictionary<int, string>(settings.ChannelColors);
        _highlightColorValue = settings.HighlightColor;
        _privateColorValue = settings.PrivateHighlightColor;

        // Building five nested AutoSize page trees while layout is live makes the
        // constructor repeatedly measure partially assembled controls. Keep the shell,
        // nav strip and content host suspended until the tree is complete; native
        // realization/prewarm will perform the first useful layout exactly once.
        SuspendLayout();
        _contentHost.SuspendLayout();
        _navHost.SuspendLayout();
        try
        {
            ChatUiTheme.ApplySettingsForm(this);
            Text = "Settings";
            StartPosition = FormStartPosition.CenterParent;
            FormBorderStyle = FormBorderStyle.Sizable;
            MaximizeBox = true;
            MinimizeBox = false;
            ShowInTaskbar = false;
            ClientSize = new Size(760, 680);
            MinimumSize = new Size(620, 500);

            var footer = BuildFooter();
            var shell = new Panel { Dock = DockStyle.Fill, BackColor = ChatUiTheme.SettingsWindow };
            var topTabs = BuildTopTabs();
            _contentHost.Dock = DockStyle.Fill;
            _contentHost.BackColor = ChatUiTheme.SettingsWindow;
            _contentHost.Padding = new Padding(5, 4, 5, 4);

            shell.Controls.Add(_contentHost);
            shell.Controls.Add(topTabs);
            Controls.Add(shell);
            Controls.Add(footer);

            RegisterPage("Appearance", "Appearance", BuildAppearancePage());
            RegisterPage("Interaction", "Interaction", BuildInteractionPage());
            RegisterPage("Alerts", "Alerts", BuildAlertsPage());
            RegisterPage("Advanced", "Advanced", BuildAdvancedPage());
            ShowPage("Appearance");
            if (!deferCompactUi)
                InstallV122CompactUi();
        }
        finally
        {
            _navHost.ResumeLayout(performLayout: false);
            _contentHost.ResumeLayout(performLayout: false);
            ResumeLayout(performLayout: false);
        }
    }

    private Panel BuildFooter()
    {
        var footer = new Panel
        {
            Dock = DockStyle.Bottom,
            Height = 52,
            BackColor = ChatUiTheme.SettingsWindow,
            Padding = Padding.Empty
        };

        var border = new Panel
        {
            Dock = DockStyle.Top,
            Height = 1,
            BackColor = ChatUiTheme.SettingsBorder
        };

        var actions = new TableLayoutPanel
        {
            Dock = DockStyle.Fill,
            ColumnCount = 7,
            RowCount = 1,
            Margin = Padding.Empty,
            Padding = new Padding(4, 8, 4, 6),
            BackColor = ChatUiTheme.SettingsWindow
        };
        actions.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 120F));
        actions.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 8F));
        actions.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 108F));
        actions.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100F));
        actions.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 150F));
        actions.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 8F));
        actions.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 120F));
        actions.RowStyles.Add(new RowStyle(SizeType.Percent, 100F));

        var save = new Button { Text = "Save", Dock = DockStyle.Fill, Margin = Padding.Empty };
        var reset = new Button { Text = "Reset", Dock = DockStyle.Fill, Margin = Padding.Empty };
        var cancel = new Button { Text = "Close", Dock = DockStyle.Fill, DialogResult = DialogResult.Cancel, Margin = Padding.Empty };
        ChatUiTheme.StyleSettingsSaveButton(save);
        ChatUiTheme.StyleSettingsButton(reset);
        ChatUiTheme.StyleSettingsCloseButton(cancel);

        _applyStatus.Dock = DockStyle.Fill;
        _applyStatus.TextAlign = ContentAlignment.MiddleRight;
        _applyStatus.ForeColor = ChatUiTheme.Success;
        _applyStatus.Font = ChatUiTheme.UiFont(8.5F, FontStyle.Bold);
        _applyStatus.Text = string.Empty;

        save.Click += (_, _) => ApplyChanges();
        reset.Click += (_, _) => ResetToDefaultsAndApply();
        actions.Controls.Add(save, 0, 0);
        actions.Controls.Add(reset, 2, 0);
        actions.Controls.Add(_applyStatus, 4, 0);
        actions.Controls.Add(cancel, 6, 0);

        footer.Controls.Add(actions);
        footer.Controls.Add(border);
        AcceptButton = save;
        CancelButton = cancel;
        return footer;
    }

    private Panel BuildTopTabs()
    {
        var host = new Panel
        {
            Dock = DockStyle.Top,
            Height = 34,
            BackColor = ChatUiTheme.SettingsWindow,
            Padding = Padding.Empty
        };

        _navHost.Dock = DockStyle.Fill;
        _navHost.FlowDirection = FlowDirection.LeftToRight;
        _navHost.WrapContents = false;
        _navHost.AutoScroll = true;
        _navHost.Padding = new Padding(4, 3, 4, 2);
        _navHost.Margin = Padding.Empty;
        _navHost.BackColor = ChatUiTheme.SettingsWindow;

        var accentLine = new Panel
        {
            Dock = DockStyle.Bottom,
            Height = 1,
            BackColor = ChatUiTheme.SettingsAccent
        };

        host.Controls.Add(_navHost);
        host.Controls.Add(accentLine);
        return host;
    }

    private void RegisterPage(string key, string navText, Control page)
    {
        if (page is not ChatSettingsPagePanel settingsPage)
            throw new InvalidOperationException("Settings pages must use ChatSettingsPagePanel.");

        var button = new ChatNavButton { Text = navText, Height = 28 };
        button.Click += (_, _) => ShowPage(key);
        _navHost.Controls.Add(button);

        settingsPage.Dock = DockStyle.Fill;
        settingsPage.Visible = true;
        settingsPage.ActivePage = false;
        _contentHost.Controls.Add(settingsPage);

        // Speech is registered by the derived constructor after Appearance is
        // already active. New background pages must not cover the active page.
        if (_activePageKey.Length > 0)
            settingsPage.SendToBack();

        _pages[key] = (button, settingsPage);
    }

    private void ShowPage(string key)
    {
        if (!_pages.TryGetValue(key, out var target)) return;
        if (string.Equals(_activePageKey, key, StringComparison.Ordinal))
        {
            target.Page.ActivePage = true;
            _contentHost.SuspendLayout();
            try { _contentHost.Controls.SetChildIndex(target.Page, 0); }
            finally { _contentHost.ResumeLayout(performLayout: false); }
            return;
        }

        _contentHost.SuspendLayout();
        _navHost.SuspendLayout();
        try
        {
            if (_activePageKey.Length > 0 && _pages.TryGetValue(_activePageKey, out var previous))
            {
                if (previous.Page.ContainsFocus)
                    target.Button.Select();

                previous.Button.Selected = false;
                previous.Page.ActivePage = false;
            }

            target.Button.Selected = true;
            target.Page.ActivePage = true;

            // Critical v1.2.4 change: all pages stay visible and fully realized.
            // Changing sibling z-order is cheap; toggling Visible on these AutoSize
            // trees caused recursive layout/handle work on every tab click.
            _contentHost.Controls.SetChildIndex(target.Page, 0);
            _activePageKey = key;
        }
        finally
        {
            _navHost.ResumeLayout(performLayout: false);
            _contentHost.ResumeLayout(performLayout: false);
        }
    }
}
