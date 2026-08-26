using System.Drawing;
using System.Drawing.Text;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed class ChatTabEditorForm : Form
{
    private static readonly (string Label, ChatChannel Channel)[] ChannelChoices =
    [
        ("Null / Other", ChatChannel.Null),
        ("World", ChatChannel.World),
        ("Local / Scene", ChatChannel.Local),
        ("Group", ChatChannel.Group),
        ("Team", ChatChannel.Team),
        ("Private", ChatChannel.Private),
        ("Union / Guild", ChatChannel.Union),
        ("System", ChatChannel.System),
        ("Top Notice", ChatChannel.TopNotice),
        ("Newbie", ChatChannel.Newbie),
        ("Play", ChatChannel.Play)
    ];

    private readonly ChatTabSettings _tab;
    private readonly TextBox _name = new();
    private readonly CheckedListBox _channels = new();
    private readonly NumericUpDown _minLevel = new();
    private readonly TextBox _show = new();
    private readonly TextBox _hide = new();
    private readonly Label _validation = new();
    private readonly Button _save = new();

    internal ChatTabEditorForm(ChatTabSettings tab, bool isNew)
    {
        _tab = tab;
        AutoScaleMode = AutoScaleMode.Dpi;
        AutoScaleDimensions = new SizeF(96F, 96F);
        Text = isNew ? "Add Chat Tab" : $"Edit Chat Tab - {tab.Name}";
        StartPosition = FormStartPosition.CenterParent;
        FormBorderStyle = FormBorderStyle.Sizable;
        MaximizeBox = true;
        MinimizeBox = false;
        ShowInTaskbar = false;
        ClientSize = new Size(760, 690);
        MinimumSize = new Size(700, 620);
        BackColor = Color.FromArgb(31, 33, 37);
        ForeColor = Color.Gainsboro;
        Font = new Font("Segoe UI", 9F);

        var footer = new Panel
        {
            Dock = DockStyle.Bottom,
            Height = 58,
            Padding = new Padding(12, 10, 12, 10),
            BackColor = Color.FromArgb(37, 40, 45)
        };
        var footerButtons = new FlowLayoutPanel
        {
            Dock = DockStyle.Fill,
            FlowDirection = FlowDirection.RightToLeft,
            WrapContents = false
        };
        _save.Text = "Save";
        _save.Width = 96;
        _save.Height = 32;
        var cancel = new Button { Text = "Cancel", Width = 96, Height = 32, DialogResult = DialogResult.Cancel };
        _save.Click += (_, _) => SaveAndClose();
        footerButtons.Controls.Add(_save);
        footerButtons.Controls.Add(cancel);
        footer.Controls.Add(footerButtons);

        var scrollHost = new Panel
        {
            Dock = DockStyle.Fill,
            AutoScroll = true,
            Padding = new Padding(18, 14, 18, 18),
            BackColor = BackColor
        };

        var table = new TableLayoutPanel
        {
            AutoSize = true,
            AutoSizeMode = AutoSizeMode.GrowAndShrink,
            ColumnCount = 2,
            Dock = DockStyle.Top,
            Margin = Padding.Empty,
            Padding = Padding.Empty
        };
        table.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 150));
        table.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100));

        _name.Text = tab.Name;
        _name.MaxLength = 40;
        _name.Dock = DockStyle.Fill;
        _name.Margin = new Padding(0, 2, 0, 10);
        AddLabel(table, "Tab name", 0);
        table.Controls.Add(_name, 1, 0);

        _minLevel.Minimum = 1;
        _minLevel.Maximum = 100;
        _minLevel.Value = Math.Clamp(tab.MinLevel, 1, 100);
        _minLevel.Width = 110;
        _minLevel.Margin = new Padding(0, 2, 0, 12);
        AddLabel(table, "Minimum level", 1);
        table.Controls.Add(_minLevel, 1, 1);

        _channels.CheckOnClick = true;
        _channels.BackColor = Color.FromArgb(42, 45, 50);
        _channels.ForeColor = Color.Gainsboro;
        _channels.BorderStyle = BorderStyle.FixedSingle;
        _channels.Height = 190;
        _channels.Dock = DockStyle.Fill;
        _channels.Margin = new Padding(0, 2, 0, 14);
        for (var i = 0; i < ChannelChoices.Length; i++)
        {
            _channels.Items.Add(ChannelChoices[i].Label);
            if (tab.Channels.Contains((int)ChannelChoices[i].Channel))
                _channels.SetItemChecked(i, true);
        }
        AddLabel(table, "Channels", 2, topAligned: true);
        table.Controls.Add(_channels, 1, 2);

        ConfigureFilterBox(_show, tab.ShowIfMatches);
        AddLabel(table, "Show only if matches", 3, topAligned: true);
        table.Controls.Add(_show, 1, 3);

        ConfigureFilterBox(_hide, tab.HideIfMatches);
        AddLabel(table, "Hide if matches", 4, topAligned: true);
        table.Controls.Add(_hide, 1, 4);

        var help = new Label
        {
            AutoSize = true,
            MaximumSize = new Size(660, 0),
            ForeColor = Color.Silver,
            Margin = new Padding(0, 4, 0, 8),
            Text = "Easy filters: serum | food | raid   •   one pattern per line = OR   •   boss AND hard\r\n" +
                   "Matching ignores letter case. Advanced regex still works, for example (raid|dungeon)."
        };
        table.SetColumnSpan(help, 2);
        table.Controls.Add(help, 0, 5);

        _validation.AutoSize = true;
        _validation.MaximumSize = new Size(660, 0);
        _validation.Margin = new Padding(0, 8, 0, 6);
        table.SetColumnSpan(_validation, 2);
        table.Controls.Add(_validation, 0, 6);

        _show.TextChanged += (_, _) => RefreshValidation();
        _hide.TextChanged += (_, _) => RefreshValidation();

        scrollHost.Controls.Add(table);
        Controls.Add(scrollHost);
        Controls.Add(footer);
        AcceptButton = _save;
        CancelButton = cancel;
        RefreshValidation();
    }

    private static void AddLabel(TableLayoutPanel table, string text, int row, bool topAligned = false)
    {
        table.Controls.Add(new Label
        {
            Text = text,
            AutoSize = false,
            Dock = DockStyle.Fill,
            TextAlign = topAligned ? ContentAlignment.TopLeft : ContentAlignment.MiddleLeft,
            Padding = topAligned ? new Padding(0, 6, 0, 0) : Padding.Empty,
            Margin = new Padding(0, 0, 12, 8)
        }, 0, row);
    }

    private static void ConfigureFilterBox(TextBox box, string value)
    {
        box.Text = value;
        box.Multiline = true;
        box.AcceptsReturn = true;
        box.ScrollBars = ScrollBars.Vertical;
        box.Dock = DockStyle.Fill;
        box.Height = 108;
        box.BackColor = Color.FromArgb(42, 45, 50);
        box.ForeColor = Color.Gainsboro;
        box.BorderStyle = BorderStyle.FixedSingle;
        box.Margin = new Padding(0, 2, 0, 14);
    }

    private void RefreshValidation()
    {
        if (!ChatFilterExpression.TryValidate(_show.Text, out var showError))
        {
            _validation.ForeColor = Color.LightCoral;
            _validation.Text = "Show filter: " + showError;
            _save.Enabled = false;
            return;
        }

        if (!ChatFilterExpression.TryValidate(_hide.Text, out var hideError))
        {
            _validation.ForeColor = Color.LightCoral;
            _validation.Text = "Hide filter: " + hideError;
            _save.Enabled = false;
            return;
        }

        _validation.ForeColor = Color.LightGreen;
        _validation.Text = "✓ Filters are valid. Matching is case-insensitive.";
        _save.Enabled = true;
    }

    private void SaveAndClose()
    {
        var name = _name.Text.Trim();
        if (name.Length == 0)
        {
            MessageBox.Show(this, "Enter a tab name.", "BPSR Chat", MessageBoxButtons.OK, MessageBoxIcon.Warning);
            return;
        }

        var channels = new List<int>();
        for (var i = 0; i < ChannelChoices.Length; i++)
        {
            if (_channels.GetItemChecked(i))
                channels.Add((int)ChannelChoices[i].Channel);
        }
        if (channels.Count == 0)
        {
            MessageBox.Show(this, "Select at least one chat channel.", "BPSR Chat", MessageBoxButtons.OK, MessageBoxIcon.Warning);
            return;
        }

        if (!ChatFilterExpression.TryValidate(_show.Text, out var showError))
        {
            MessageBox.Show(this, showError, "Show filter is invalid", MessageBoxButtons.OK, MessageBoxIcon.Warning);
            return;
        }
        if (!ChatFilterExpression.TryValidate(_hide.Text, out var hideError))
        {
            MessageBox.Show(this, hideError, "Hide filter is invalid", MessageBoxButtons.OK, MessageBoxIcon.Warning);
            return;
        }

        _tab.Name = name;
        _tab.Channels = channels;
        _tab.MinLevel = (int)_minLevel.Value;
        _tab.ShowIfMatches = _show.Text.Trim();
        _tab.HideIfMatches = _hide.Text.Trim();
        DialogResult = DialogResult.OK;
        Close();
    }
}

internal sealed class ChatGeneralSettingsForm : Form
{
    private readonly ChatOverlaySettings _settings;
    private List<ChatBlockedUser> _blockedWorking;
    private Dictionary<int, string> _channelColorsWorking;

    private readonly CheckBox _topMost = new() { Text = "Always on top" };
    private readonly CheckBox _compact = new() { Text = "Compact messages" };
    private readonly CheckBox _showTime = new() { Text = "Show timestamps" };
    private readonly CheckBox _timeAgo = new() { Text = "Use relative time (20s, 3m, 2h)" };
    private readonly CheckBox _hideStickers = new() { Text = "Hide stickers" };
    private readonly CheckBox _bold = new() { Text = "Bold message text" };
    private readonly CheckBox _shadow = new() { Text = "Text shadow" };
    private readonly CheckBox _separators = new() { Text = "Message separators" };
    private readonly CheckBox _zebra = new() { Text = "Alternating row shading" };
    private readonly CheckBox _colorBand = new() { Text = "Channel color strip" };
    private readonly CheckBox _clickThrough = new() { Text = "Click-through (mouse passes to the game)" };
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
    private readonly CheckBox _highlightSound = new() { Text = "Play sound when highlight matches" };
    private readonly TextBox _highlightSoundPath = new();
    private readonly CheckBox _privateHighlight = new() { Text = "Highlight Private / Talk messages" };
    private readonly Button _privateColor = new();
    private readonly CheckBox _privateSound = new() { Text = "Play sound for Private / Talk" };
    private readonly TextBox _privateSoundPath = new();
    private string _highlightColorValue;
    private string _privateColorValue;

    internal ChatGeneralSettingsForm(ChatOverlaySettings settings)
    {
        _settings = settings;
        _blockedWorking = settings.BlockedUsers.Select(CloneBlockedUser).ToList();
        _channelColorsWorking = new Dictionary<int, string>(settings.ChannelColors);
        _highlightColorValue = settings.HighlightColor;
        _privateColorValue = settings.PrivateHighlightColor;

        AutoScaleMode = AutoScaleMode.Dpi;
        AutoScaleDimensions = new SizeF(96F, 96F);
        Text = "Chat Overlay Settings";
        StartPosition = FormStartPosition.CenterParent;
        FormBorderStyle = FormBorderStyle.Sizable;
        MaximizeBox = true;
        MinimizeBox = false;
        ShowInTaskbar = false;
        ClientSize = new Size(720, 650);
        MinimumSize = new Size(650, 560);
        BackColor = Color.FromArgb(31, 33, 37);
        ForeColor = Color.Gainsboro;
        Font = new Font("Segoe UI", 9F);

        var footer = new Panel
        {
            Dock = DockStyle.Bottom,
            Height = 58,
            Padding = new Padding(12, 10, 12, 10),
            BackColor = Color.FromArgb(37, 40, 45)
        };
        var footerButtons = new FlowLayoutPanel
        {
            Dock = DockStyle.Fill,
            FlowDirection = FlowDirection.RightToLeft,
            WrapContents = false
        };
        var save = new Button { Text = "Save", Width = 96, Height = 32 };
        var cancel = new Button { Text = "Cancel", Width = 96, Height = 32, DialogResult = DialogResult.Cancel };
        save.Click += (_, _) => SaveAndClose();
        footerButtons.Controls.Add(save);
        footerButtons.Controls.Add(cancel);
        footer.Controls.Add(footerButtons);

        var tabs = new TabControl { Dock = DockStyle.Fill };
        tabs.TabPages.Add(BuildAppearancePage());
        tabs.TabPages.Add(BuildInteractionPage());
        tabs.TabPages.Add(BuildAlertsPage());
        tabs.TabPages.Add(BuildAdvancedPage());

        Controls.Add(tabs);
        Controls.Add(footer);
        AcceptButton = save;
        CancelButton = cancel;
    }

    private TabPage BuildAppearancePage()
    {
        var page = MakePage("Appearance");
        var panel = MakeVerticalPanel();
        page.Controls.Add(panel);

        _topMost.Checked = _settings.TopMost;
        _compact.Checked = _settings.CompactMode;
        _showTime.Checked = _settings.ShowTime;
        _timeAgo.Checked = _settings.ShowTimeAsAgo;
        _bold.Checked = _settings.BoldMessageText;
        _shadow.Checked = _settings.TextShadow;
        _separators.Checked = _settings.ShowSeparators;
        _zebra.Checked = _settings.ShowZebraStripes;
        _colorBand.Checked = _settings.ShowColorBand;
        foreach (var check in new[] { _topMost, _compact, _showTime, _timeAgo, _bold, _shadow, _separators, _zebra, _colorBand })
            AddCheck(panel, check);
        _showTime.CheckedChanged += (_, _) => _timeAgo.Enabled = _showTime.Checked;
        _timeAgo.Enabled = _showTime.Checked;

        panel.Controls.Add(MakeSectionTitle("Font"));
        _fontFamily.DropDownStyle = ComboBoxStyle.DropDownList;
        _fontFamily.Width = 300;
        try
        {
            using var fonts = new InstalledFontCollection();
            var names = fonts.Families.Select(x => x.Name).Distinct(StringComparer.OrdinalIgnoreCase).OrderBy(x => x).ToArray();
            _fontFamily.Items.AddRange(names);
        }
        catch { }
        if (!_fontFamily.Items.Contains(_settings.FontFamily)) _fontFamily.Items.Add(_settings.FontFamily);
        _fontFamily.SelectedItem = _settings.FontFamily;
        panel.Controls.Add(MakeControlRow("Font family", _fontFamily));

        _fontSize.Minimum = 8;
        _fontSize.Maximum = 24;
        _fontSize.DecimalPlaces = 1;
        _fontSize.Increment = 0.5M;
        _fontSize.Value = (decimal)Math.Clamp(_settings.FontSize, 8F, 24F);
        panel.Controls.Add(MakeControlRow("Font size", _fontSize));

        panel.Controls.Add(MakeSectionTitle("Opacity / contrast"));
        panel.Controls.Add(MakeSliderRow("Background strength", _backgroundOpacity, _backgroundOpacityValue, _settings.BackgroundOpacity, 10));
        panel.Controls.Add(MakeSliderRow("Toolbar strength", _toolbarOpacity, _toolbarOpacityValue, _settings.ToolbarOpacity, 15));
        panel.Controls.Add(MakeSliderRow("Text opacity", _textOpacity, _textOpacityValue, _settings.TextOpacity, 40));
        panel.Controls.Add(MakeSliderRow("Whole window opacity", _windowOpacity, _windowOpacityValue, _settings.WindowOpacity, 25));
        panel.Controls.Add(MakeHint("Background, toolbar and text are rendered separately. Whole-window opacity is the final Windows transparency applied to everything."));
        return page;
    }

    private TabPage BuildInteractionPage()
    {
        var page = MakePage("Interaction");
        var panel = MakeVerticalPanel();
        page.Controls.Add(panel);

        _hideStickers.Checked = _settings.HideStickers;
        _clickThrough.Checked = _settings.ClickThrough;
        AddCheck(panel, _hideStickers);
        AddCheck(panel, _clickThrough);
        panel.Controls.Add(MakeHint("When click-through is ON you cannot click the overlay. Use the global hotkey below to turn it back OFF."));

        panel.Controls.Add(MakeSectionTitle("Global hotkeys"));
        _clickHotkey.Text = _settings.ClickThroughHotkey;
        _collapseHotkey.Text = _settings.CollapseHotkey;
        panel.Controls.Add(MakeControlRow("Click-through toggle", _clickHotkey));
        panel.Controls.Add(MakeControlRow("Collapse / expand", _collapseHotkey));
        panel.Controls.Add(MakeHint("Click a hotkey box, then press the combination you want. Backspace clears it."));

        panel.Controls.Add(MakeSectionTitle("Screen-edge collapse"));
        _collapseSide.DropDownStyle = ComboBoxStyle.DropDownList;
        _collapseSide.Items.AddRange(["Left", "Right", "Top", "Bottom"]);
        _collapseSide.SelectedItem = _settings.CollapseSide;
        panel.Controls.Add(MakeControlRow("Collapse side", _collapseSide));

        _maxHistory.Minimum = 10;
        _maxHistory.Maximum = 500;
        _maxHistory.Increment = 10;
        _maxHistory.Value = Math.Clamp(_settings.MaxHistory, 10, 500);
        panel.Controls.Add(MakeControlRow("Max chat history", _maxHistory));
        return page;
    }

    private TabPage BuildAlertsPage()
    {
        var page = MakePage("Highlights & Sounds");
        var panel = MakeVerticalPanel();
        page.Controls.Add(panel);

        panel.Controls.Add(MakeSectionTitle("Keyword / regex highlight"));
        _highlight.Multiline = true;
        _highlight.AcceptsReturn = true;
        _highlight.ScrollBars = ScrollBars.Vertical;
        _highlight.Width = 600;
        _highlight.Height = 90;
        _highlight.BackColor = Color.FromArgb(42, 45, 50);
        _highlight.ForeColor = Color.Gainsboro;
        _highlight.Text = _settings.HighlightIfMatches;
        _highlight.TextChanged += (_, _) => RefreshHighlightValidation();
        panel.Controls.Add(_highlight);
        panel.Controls.Add(MakeHint("Same safe filter syntax as tabs: serum | food | raid, one pattern per line, AND, or advanced regex. Sender name and message text are both checked."));

        _highlightValidation.AutoSize = true;
        _highlightValidation.Margin = new Padding(3, 2, 3, 8);
        panel.Controls.Add(_highlightValidation);

        ConfigureColorButton(_highlightColor, _highlightColorValue, "Highlight color...");
        _highlightColor.Click += (_, _) => ChooseColor(ref _highlightColorValue, _highlightColor);
        panel.Controls.Add(_highlightColor);

        _highlightSound.Checked = _settings.HighlightSoundEnabled;
        AddCheck(panel, _highlightSound);
        _highlightSoundPath.ReadOnly = true;
        _highlightSoundPath.Text = _settings.HighlightSoundPath;
        panel.Controls.Add(MakePathRow(_highlightSoundPath, () => BrowseSound(_highlightSoundPath)));
        panel.Controls.Add(MakeHint("Leave the sound path empty to use ReadyAlert's built-in alert sound."));

        panel.Controls.Add(MakeSectionTitle("Private / Talk"));
        _privateHighlight.Checked = _settings.PrivateHighlightEnabled;
        AddCheck(panel, _privateHighlight);
        ConfigureColorButton(_privateColor, _privateColorValue, "Private highlight color...");
        _privateColor.Click += (_, _) => ChooseColor(ref _privateColorValue, _privateColor);
        panel.Controls.Add(_privateColor);
        _privateSound.Checked = _settings.PrivateSoundEnabled;
        AddCheck(panel, _privateSound);
        _privateSoundPath.ReadOnly = true;
        _privateSoundPath.Text = _settings.PrivateSoundPath;
        panel.Controls.Add(MakePathRow(_privateSoundPath, () => BrowseSound(_privateSoundPath)));

        RefreshHighlightValidation();
        return page;
    }

    private TabPage BuildAdvancedPage()
    {
        var page = MakePage("Advanced");
        var panel = MakeVerticalPanel();
        page.Controls.Add(panel);

        panel.Controls.Add(MakeSectionTitle("Customization"));
        var colors = MakeWideButton("Channel colors...");
        colors.Click += (_, _) =>
        {
            using var dialog = new ChannelColorsForm(_channelColorsWorking);
            if (dialog.ShowDialog(this) == DialogResult.OK)
                _channelColorsWorking = dialog.Result;
        };
        panel.Controls.Add(colors);

        var blocked = MakeWideButton("Manage blocked users...");
        blocked.Click += (_, _) =>
        {
            using var dialog = new BlockedUsersForm(_blockedWorking);
            dialog.ShowDialog(this);
        };
        panel.Controls.Add(blocked);

        panel.Controls.Add(MakeSectionTitle("Diagnostics"));
        var debug = MakeWideButton("Chat capture status...");
        debug.Click += (_, _) =>
        {
            using var dialog = new ChatDebugStatusForm();
            dialog.ShowDialog(this);
        };
        panel.Controls.Add(debug);
        panel.Controls.Add(MakeHint("This status page reads counters from ReadyAlert's shared CaptureEngine chat consumer. It does not start another Npcap capture."));
        return page;
    }

    private void RefreshHighlightValidation()
    {
        if (ChatFilterExpression.TryValidate(_highlight.Text, out var error))
        {
            _highlightValidation.ForeColor = Color.LightGreen;
            _highlightValidation.Text = string.IsNullOrWhiteSpace(_highlight.Text)
                ? "No keyword highlight rule is configured."
                : "✓ Highlight filter is valid and case-insensitive.";
        }
        else
        {
            _highlightValidation.ForeColor = Color.LightCoral;
            _highlightValidation.Text = error;
        }
    }

    private void SaveAndClose()
    {
        if (!ChatFilterExpression.TryValidate(_highlight.Text, out var highlightError))
        {
            MessageBox.Show(this, highlightError, "Highlight filter is invalid", MessageBoxButtons.OK, MessageBoxIcon.Warning);
            return;
        }

        if (!ChatHotkey.TryParse(_clickHotkey.Text, out var clickGesture, out var clickError))
        {
            MessageBox.Show(this, clickError, "Click-through hotkey is invalid", MessageBoxButtons.OK, MessageBoxIcon.Warning);
            return;
        }
        if (!ChatHotkey.TryParse(_collapseHotkey.Text, out var collapseGesture, out var collapseError))
        {
            MessageBox.Show(this, collapseError, "Collapse hotkey is invalid", MessageBoxButtons.OK, MessageBoxIcon.Warning);
            return;
        }
        if (clickGesture.Equals(collapseGesture))
        {
            MessageBox.Show(this, "Click-through and Collapse cannot use the same hotkey.", "Hotkeys", MessageBoxButtons.OK, MessageBoxIcon.Warning);
            return;
        }

        _settings.TopMost = _topMost.Checked;
        _settings.CompactMode = _compact.Checked;
        _settings.ShowTime = _showTime.Checked;
        _settings.ShowTimeAsAgo = _showTime.Checked && _timeAgo.Checked;
        _settings.HideStickers = _hideStickers.Checked;
        _settings.BoldMessageText = _bold.Checked;
        _settings.TextShadow = _shadow.Checked;
        _settings.ShowSeparators = _separators.Checked;
        _settings.ShowZebraStripes = _zebra.Checked;
        _settings.ShowColorBand = _colorBand.Checked;
        _settings.BackgroundOpacity = _backgroundOpacity.Value;
        _settings.ToolbarOpacity = _toolbarOpacity.Value;
        _settings.TextOpacity = _textOpacity.Value;
        _settings.WindowOpacity = _windowOpacity.Value;
        _settings.FontFamily = _fontFamily.SelectedItem?.ToString() ?? "Segoe UI";
        _settings.FontSize = (float)_fontSize.Value;
        _settings.ClickThrough = _clickThrough.Checked;
        _settings.ClickThroughHotkey = clickGesture.DisplayText;
        _settings.CollapseHotkey = collapseGesture.DisplayText;
        _settings.CollapseSide = _collapseSide.SelectedItem?.ToString() ?? "Right";
        _settings.MaxHistory = (int)_maxHistory.Value;
        _settings.HighlightIfMatches = _highlight.Text.Trim();
        _settings.HighlightColor = _highlightColorValue;
        _settings.HighlightSoundEnabled = _highlightSound.Checked;
        _settings.HighlightSoundPath = _highlightSoundPath.Text;
        _settings.PrivateHighlightEnabled = _privateHighlight.Checked;
        _settings.PrivateHighlightColor = _privateColorValue;
        _settings.PrivateSoundEnabled = _privateSound.Checked;
        _settings.PrivateSoundPath = _privateSoundPath.Text;
        _settings.ChannelColors = _channelColorsWorking;
        _settings.BlockedUsers = _blockedWorking;
        DialogResult = DialogResult.OK;
        Close();
    }

    private static TabPage MakePage(string name) => new(name)
    {
        BackColor = Color.FromArgb(31, 33, 37),
        ForeColor = Color.Gainsboro,
        Padding = new Padding(4)
    };

    private static FlowLayoutPanel MakeVerticalPanel() => new()
    {
        Dock = DockStyle.Fill,
        FlowDirection = FlowDirection.TopDown,
        WrapContents = false,
        AutoScroll = true,
        Padding = new Padding(14),
        BackColor = Color.FromArgb(31, 33, 37)
    };

    private static void AddCheck(Control parent, CheckBox check)
    {
        check.AutoSize = true;
        check.Margin = new Padding(3, 3, 3, 7);
        parent.Controls.Add(check);
    }

    private static Label MakeSectionTitle(string text) => new()
    {
        Text = text,
        AutoSize = true,
        Font = new Font("Segoe UI", 9F, FontStyle.Bold),
        ForeColor = Color.WhiteSmoke,
        Margin = new Padding(3, 14, 3, 8)
    };

    private static Label MakeHint(string text) => new()
    {
        Text = text,
        AutoSize = true,
        MaximumSize = new Size(610, 0),
        ForeColor = Color.Silver,
        Margin = new Padding(3, 2, 3, 8)
    };

    private static FlowLayoutPanel MakeControlRow(string label, Control control)
    {
        var row = new FlowLayoutPanel { Width = 620, Height = 38, FlowDirection = FlowDirection.LeftToRight, WrapContents = false };
        row.Controls.Add(new Label { Text = label, Width = 165, Height = 30, TextAlign = ContentAlignment.MiddleLeft });
        control.Margin = new Padding(0, 3, 0, 0);
        row.Controls.Add(control);
        return row;
    }

    private static FlowLayoutPanel MakeSliderRow(string label, TrackBar slider, Label value, int current, int minimum)
    {
        var row = new FlowLayoutPanel { Width = 620, Height = 48, FlowDirection = FlowDirection.LeftToRight, WrapContents = false };
        row.Controls.Add(new Label { Text = label, Width = 165, TextAlign = ContentAlignment.MiddleLeft, Height = 35 });
        slider.Minimum = minimum;
        slider.Maximum = 100;
        slider.TickFrequency = 10;
        slider.Value = Math.Clamp(current, minimum, 100);
        slider.Width = 330;
        value.Text = slider.Value + "%";
        value.Width = 55;
        value.Height = 35;
        value.TextAlign = ContentAlignment.MiddleLeft;
        slider.ValueChanged += (_, _) => value.Text = slider.Value + "%";
        row.Controls.Add(slider);
        row.Controls.Add(value);
        return row;
    }

    private static Button MakeWideButton(string text) => new()
    {
        Text = text,
        Width = 600,
        Height = 34,
        Margin = new Padding(3, 3, 3, 7)
    };

    private static FlowLayoutPanel MakePathRow(TextBox pathBox, Action browse)
    {
        var row = new FlowLayoutPanel { Width = 620, Height = 38, FlowDirection = FlowDirection.LeftToRight, WrapContents = false };
        pathBox.Width = 490;
        pathBox.BackColor = Color.FromArgb(42, 45, 50);
        pathBox.ForeColor = Color.Gainsboro;
        var browseButton = new Button { Text = "Browse...", Width = 100, Height = 28 };
        browseButton.Click += (_, _) => browse();
        row.Controls.Add(pathBox);
        row.Controls.Add(browseButton);
        return row;
    }

    private static void ConfigureColorButton(Button button, string colorValue, string text)
    {
        button.Text = text;
        button.Width = 240;
        button.Height = 32;
        button.BackColor = ChatColorUtil.Parse(colorValue, Color.DimGray);
        button.ForeColor = ContrastText(button.BackColor);
        button.Margin = new Padding(3, 3, 3, 8);
    }

    private static void ChooseColor(ref string target, Button button)
    {
        using var dialog = new ColorDialog
        {
            FullOpen = true,
            Color = ChatColorUtil.Parse(target, Color.DimGray)
        };
        if (dialog.ShowDialog() != DialogResult.OK) return;
        target = ChatColorUtil.ToHtml(dialog.Color);
        button.BackColor = dialog.Color;
        button.ForeColor = ContrastText(dialog.Color);
    }

    private void BrowseSound(TextBox target)
    {
        using var dialog = new OpenFileDialog
        {
            Title = "Choose WAV notification sound",
            Filter = "WAV audio (*.wav)|*.wav|All files (*.*)|*.*",
            CheckFileExists = true
        };
        if (dialog.ShowDialog(this) == DialogResult.OK)
            target.Text = dialog.FileName;
    }

    private static Color ContrastText(Color color) =>
        color.R * 299 + color.G * 587 + color.B * 114 >= 150_000 ? Color.Black : Color.White;

    private static ChatBlockedUser CloneBlockedUser(ChatBlockedUser user) => new()
    {
        Id = user.Id,
        Name = user.Name,
        BlockedAtUtc = user.BlockedAtUtc
    };
}

internal sealed class HotkeyCaptureTextBox : TextBox
{
    internal HotkeyCaptureTextBox()
    {
        ReadOnly = true;
        Width = 240;
        BackColor = Color.FromArgb(42, 45, 50);
        ForeColor = Color.Gainsboro;
        ShortcutsEnabled = false;
        KeyDown += CaptureKeyDown;
        PreviewKeyDown += (_, e) => e.IsInputKey = true;
    }

    private void CaptureKeyDown(object? sender, KeyEventArgs e)
    {
        if (e.KeyCode is Keys.Back or Keys.Delete)
        {
            Text = string.Empty;
            e.SuppressKeyPress = true;
            return;
        }

        var text = ChatHotkey.FromKeyData(e.KeyData);
        if (!string.IsNullOrWhiteSpace(text)) Text = text;
        e.SuppressKeyPress = true;
    }
}

internal sealed class ChannelColorsForm : Form
{
    private readonly Dictionary<int, string> _working;
    internal Dictionary<int, string> Result => new(_working);

    internal ChannelColorsForm(Dictionary<int, string> current)
    {
        _working = new Dictionary<int, string>(current);
        var defaults = ChatOverlaySettings.CreateDefaultChannelColors();
        foreach (var pair in defaults)
            if (!_working.ContainsKey(pair.Key)) _working[pair.Key] = pair.Value;

        AutoScaleMode = AutoScaleMode.Dpi;
        Text = "Chat Channel Colors";
        StartPosition = FormStartPosition.CenterParent;
        ShowInTaskbar = false;
        ClientSize = new Size(500, 520);
        MinimumSize = new Size(460, 440);
        BackColor = Color.FromArgb(31, 33, 37);
        ForeColor = Color.Gainsboro;
        Font = new Font("Segoe UI", 9F);

        var footer = new FlowLayoutPanel
        {
            Dock = DockStyle.Bottom,
            Height = 54,
            FlowDirection = FlowDirection.RightToLeft,
            Padding = new Padding(10),
            BackColor = Color.FromArgb(37, 40, 45)
        };
        var save = new Button { Text = "Save", Width = 90, DialogResult = DialogResult.OK };
        var cancel = new Button { Text = "Cancel", Width = 90, DialogResult = DialogResult.Cancel };
        var reset = new Button { Text = "Reset defaults", Width = 110 };
        footer.Controls.Add(save);
        footer.Controls.Add(cancel);
        footer.Controls.Add(reset);

        var table = new TableLayoutPanel
        {
            Dock = DockStyle.Fill,
            AutoScroll = true,
            ColumnCount = 2,
            Padding = new Padding(14)
        };
        table.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 45));
        table.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 55));

        void Rebuild()
        {
            table.SuspendLayout();
            table.Controls.Clear();
            table.RowStyles.Clear();
            var channels = Enum.GetValues<ChatChannel>();
            table.RowCount = channels.Length;
            for (var i = 0; i < channels.Length; i++)
            {
                var channel = channels[i];
                var key = (int)channel;
                var name = ChannelLabel(channel);
                var label = new Label { Text = name, Dock = DockStyle.Fill, TextAlign = ContentAlignment.MiddleLeft };
                var button = new Button { Text = _working[key], Width = 170, Height = 28, BackColor = ChatColorUtil.Parse(_working[key], Color.Gray) };
                button.ForeColor = Contrast(button.BackColor);
                button.Click += (_, _) =>
                {
                    using var color = new ColorDialog { FullOpen = true, Color = ChatColorUtil.Parse(_working[key], Color.Gray) };
                    if (color.ShowDialog(this) != DialogResult.OK) return;
                    _working[key] = ChatColorUtil.ToHtml(color.Color);
                    button.Text = _working[key];
                    button.BackColor = color.Color;
                    button.ForeColor = Contrast(color.Color);
                };
                table.RowStyles.Add(new RowStyle(SizeType.Absolute, 38));
                table.Controls.Add(label, 0, i);
                table.Controls.Add(button, 1, i);
            }
            table.ResumeLayout();
        }

        reset.Click += (_, _) =>
        {
            _working.Clear();
            foreach (var pair in ChatOverlaySettings.CreateDefaultChannelColors()) _working[pair.Key] = pair.Value;
            Rebuild();
        };

        Rebuild();
        Controls.Add(table);
        Controls.Add(footer);
        AcceptButton = save;
        CancelButton = cancel;
    }

    private static string ChannelLabel(ChatChannel channel) => channel switch
    {
        ChatChannel.Null => "Null / Other",
        ChatChannel.Local => "Local / Scene",
        ChatChannel.Union => "Union / Guild",
        ChatChannel.TopNotice => "Top Notice",
        _ => channel.ToString()
    };

    private static Color Contrast(Color color) =>
        color.R * 299 + color.G * 587 + color.B * 114 >= 150_000 ? Color.Black : Color.White;
}

internal sealed class BlockedUsersForm : Form
{
    private readonly List<ChatBlockedUser> _blockedUsers;
    private readonly ListBox _list = new();

    internal BlockedUsersForm(List<ChatBlockedUser> blockedUsers)
    {
        _blockedUsers = blockedUsers;
        AutoScaleMode = AutoScaleMode.Dpi;
        AutoScaleDimensions = new SizeF(96F, 96F);
        Text = "Blocked Chat Users";
        StartPosition = FormStartPosition.CenterParent;
        ShowInTaskbar = false;
        ClientSize = new Size(460, 340);
        MinimumSize = new Size(420, 300);
        BackColor = Color.FromArgb(31, 33, 37);
        ForeColor = Color.Gainsboro;
        Font = new Font("Segoe UI", 9F);

        _list.Dock = DockStyle.Fill;
        _list.BackColor = Color.FromArgb(42, 45, 50);
        _list.ForeColor = Color.Gainsboro;
        RefreshList();

        var bottom = new FlowLayoutPanel
        {
            Dock = DockStyle.Bottom,
            Height = 50,
            FlowDirection = FlowDirection.RightToLeft,
            Padding = new Padding(8),
            BackColor = Color.FromArgb(37, 40, 45)
        };
        var close = new Button { Text = "Close", Width = 90, DialogResult = DialogResult.OK };
        var unblock = new Button { Text = "Unblock", Width = 90 };
        unblock.Click += (_, _) => UnblockSelected();
        bottom.Controls.Add(close);
        bottom.Controls.Add(unblock);

        Controls.Add(_list);
        Controls.Add(bottom);
        AcceptButton = close;
    }

    private void RefreshList()
    {
        _list.Items.Clear();
        foreach (var user in _blockedUsers.OrderBy(x => x.Name, StringComparer.OrdinalIgnoreCase))
            _list.Items.Add(new BlockedUserItem(user));
    }

    private void UnblockSelected()
    {
        if (_list.SelectedItem is not BlockedUserItem item) return;
        _blockedUsers.Remove(item.User);
        RefreshList();
    }

    private sealed class BlockedUserItem(ChatBlockedUser user)
    {
        internal ChatBlockedUser User { get; } = user;
        public override string ToString() => $"{User.Name}  [UID {User.Id}]";
    }
}

internal sealed class ChatDebugStatusForm : Form
{
    private readonly TextBox _status = new();
    private readonly System.Windows.Forms.Timer _timer = new() { Interval = 500 };

    internal ChatDebugStatusForm()
    {
        AutoScaleMode = AutoScaleMode.Dpi;
        Text = "Chat Capture Status";
        StartPosition = FormStartPosition.CenterParent;
        ShowInTaskbar = false;
        ClientSize = new Size(540, 330);
        MinimumSize = new Size(500, 300);
        BackColor = Color.FromArgb(31, 33, 37);
        ForeColor = Color.Gainsboro;
        Font = new Font("Segoe UI", 9F);

        _status.Dock = DockStyle.Fill;
        _status.Multiline = true;
        _status.ReadOnly = true;
        _status.ScrollBars = ScrollBars.Vertical;
        _status.BackColor = Color.FromArgb(25, 27, 31);
        _status.ForeColor = Color.Gainsboro;
        _status.Font = new Font(FontFamily.GenericMonospace, 9F);

        var bottom = new FlowLayoutPanel
        {
            Dock = DockStyle.Bottom,
            Height = 50,
            FlowDirection = FlowDirection.RightToLeft,
            Padding = new Padding(8),
            BackColor = Color.FromArgb(37, 40, 45)
        };
        var close = new Button { Text = "Close", Width = 90, DialogResult = DialogResult.OK };
        var copy = new Button { Text = "Copy status", Width = 100 };
        copy.Click += (_, _) =>
        {
            if (!string.IsNullOrEmpty(_status.Text)) Clipboard.SetText(_status.Text);
        };
        bottom.Controls.Add(close);
        bottom.Controls.Add(copy);

        Controls.Add(_status);
        Controls.Add(bottom);
        AcceptButton = close;

        _timer.Tick += (_, _) => RefreshStatus();
        FormClosed += (_, _) => _timer.Stop();
        RefreshStatus();
        _timer.Start();
    }

    private void RefreshStatus()
    {
        var status = ChatCaptureBridge.GetStatus();
        _status.Text =
            "BPSR ReadyAlert Chat RC3\r\n" +
            "----------------------------------------\r\n" +
            $"Enabled:              {status.Enabled}\r\n" +
            "Capture pipeline:      Shared ReadyAlert CaptureEngine\r\n" +
            "Second Npcap capture:  No\r\n" +
            $"Service ID:           {ChatProtocol.ServiceId}\r\n" +
            $"Method:               0x{ChatProtocol.NotifyNewestChitChatMsgs:X2}\r\n" +
            $"Matching notifies:    {status.MatchingNotifies}\r\n" +
            $"Parsed messages:      {status.ParsedMessages}\r\n" +
            $"Parse failures:       {status.ParseFailures}\r\n" +
            $"Queue drops:          {status.DroppedQueuedMessages}\r\n" +
            $"Pending UI queue:     {status.QueueCount}\r\n" +
            $"Last payload bytes:   {status.LastPayloadLength}\r\n" +
            $"Last message UTC:     {(status.LastMessageUtc?.ToString("yyyy-MM-dd HH:mm:ss") ?? "Never")}\r\n\r\n" +
            "If messages are not appearing, keep this window open while someone sends chat.\r\n" +
            "Matching notifies should increase first; Parsed messages should then increase.";
    }
}
