using System.Drawing;
using System.Drawing.Text;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed partial class ChatGeneralSettingsForm
{
    private Control BuildAppearancePage()
    {
        InitializeAppearanceValues();
        var page = CreatePage(
            "Appearance",
            "Tune readability without making the overlay feel heavy. Changes apply after you save.");
        var stack = (TableLayoutPanel)page.Tag!;

        var layoutGrid = new TableLayoutPanel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            ColumnCount = 2,
            RowCount = 0,
            Margin = Padding.Empty
        };
        layoutGrid.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 50F));
        layoutGrid.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 50F));
        var checks = new[] { _topMost, _compact, _showTime, _timeAgo, _bold, _shadow, _separators, _zebra, _colorBand };
        for (var i = 0; i < checks.Length; i++)
        {
            ChatUiTheme.StyleCheckBox(checks[i]);
            checks[i].Dock = DockStyle.Fill;
            layoutGrid.Controls.Add(checks[i], i % 2, i / 2);
        }
        _showTime.CheckedChanged += (_, _) => _timeAgo.Enabled = _showTime.Checked;
        _timeAgo.Enabled = _showTime.Checked;
        AddPageCard(stack, MakeCard("Message layout", "Choose the information and visual cues shown for each chat line.", layoutGrid));

        var typography = MakeFieldTable();
        AddFieldRow(typography, "Font family", "Uses fonts installed in Windows.", _fontFamily, 0);
        AddFieldRow(typography, "Font size", "8–24 pt. Larger text may use more vertical space.", _fontSize, 1);
        AddPageCard(stack, MakeCard("Typography", "Keep the message text comfortable to read over the game.", typography));

        var opacity = new TableLayoutPanel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            ColumnCount = 1,
            RowCount = 0,
            Margin = Padding.Empty
        };
        opacity.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100F));
        AddStack(opacity, MakeSliderRow("Chat background", "Darkness behind messages", _backgroundOpacity, _backgroundOpacityValue, _settings.BackgroundOpacity, 10));
        AddStack(opacity, MakeSliderRow("Toolbar", "Strength of the top navigation bar", _toolbarOpacity, _toolbarOpacityValue, _settings.ToolbarOpacity, 15));
        AddStack(opacity, MakeSliderRow("Text", "Text contrast inside the overlay", _textOpacity, _textOpacityValue, _settings.TextOpacity, 40));
        AddStack(opacity, MakeSliderRow("Whole window", "Final Windows transparency applied to everything", _windowOpacity, _windowOpacityValue, _settings.WindowOpacity, 25));
        AddPageCard(stack, MakeCard("Transparency & contrast", "The first three controls are independent; Whole window opacity is applied last.", opacity));
        return page;
    }

    private void InitializeAppearanceValues()
    {
        _topMost.Checked = _settings.TopMost;
        _compact.Checked = _settings.CompactMode;
        _showTime.Checked = _settings.ShowTime;
        _timeAgo.Checked = _settings.ShowTimeAsAgo;
        _bold.Checked = _settings.BoldMessageText;
        _shadow.Checked = _settings.TextShadow;
        _separators.Checked = _settings.ShowSeparators;
        _zebra.Checked = _settings.ShowZebraStripes;
        _colorBand.Checked = _settings.ShowColorBand;

        ChatUiTheme.StyleComboBox(_fontFamily);
        try
        {
            using var fonts = new InstalledFontCollection();
            _fontFamily.Items.AddRange(fonts.Families.Select(x => x.Name).Distinct(StringComparer.OrdinalIgnoreCase).OrderBy(x => x).ToArray());
        }
        catch { }
        if (!_fontFamily.Items.Contains(_settings.FontFamily)) _fontFamily.Items.Add(_settings.FontFamily);
        _fontFamily.SelectedItem = _settings.FontFamily;
        _fontFamily.Width = 330;

        _fontSize.Minimum = 8;
        _fontSize.Maximum = 24;
        _fontSize.DecimalPlaces = 1;
        _fontSize.Increment = 0.5M;
        _fontSize.Value = (decimal)Math.Clamp(_settings.FontSize, 8F, 24F);
        _fontSize.Width = 120;
        ChatUiTheme.StyleNumeric(_fontSize);
    }

    private Control BuildInteractionPage()
    {
        _hideStickers.Checked = _settings.HideStickers;
        _clickThrough.Checked = _settings.ClickThrough;
        ChatUiTheme.StyleCheckBox(_hideStickers);
        ChatUiTheme.StyleCheckBox(_clickThrough);

        var page = CreatePage(
            "Interaction",
            "Control how the overlay behaves while you play. Hotkeys work globally while ReadyAlert is running.");
        var stack = (TableLayoutPanel)page.Tag!;

        var behavior = new FlowLayoutPanel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            FlowDirection = FlowDirection.TopDown,
            WrapContents = false,
            Margin = Padding.Empty
        };
        behavior.Controls.Add(_hideStickers);
        behavior.Controls.Add(_clickThrough);
        behavior.Controls.Add(MakeInfoBanner(
            "Click-through safety",
            "When click-through is ON, mouse clicks pass to the game. Use the recovery hotkey below to turn it OFF. If that hotkey cannot register, ReadyAlert automatically disables click-through.",
            ChatUiTheme.Warning));
        AddPageCard(stack, MakeCard("Overlay behavior", "Keep the overlay out of the way without making it impossible to recover.", behavior));

        _clickHotkey.Text = _settings.ClickThroughHotkey;
        _collapseHotkey.Text = _settings.CollapseHotkey;
        var hotkeys = MakeFieldTable();
        AddFieldRow(hotkeys, "Click-through toggle", "Default: Ctrl+Shift+F10", _clickHotkey, 0);
        AddFieldRow(hotkeys, "Collapse / expand", "Default: Ctrl+Shift+F9", _collapseHotkey, 1);
        AddPageCard(stack, MakeCard("Keyboard shortcuts", "Click a box and press the combination you want. Backspace clears it.", hotkeys));

        ChatUiTheme.StyleComboBox(_collapseSide);
        _collapseSide.Items.AddRange(["Left", "Right", "Top", "Bottom"]);
        _collapseSide.SelectedItem = _settings.CollapseSide;
        _collapseSide.Width = 160;
        _maxHistory.Minimum = 10;
        _maxHistory.Maximum = 500;
        _maxHistory.Increment = 10;
        _maxHistory.Value = Math.Clamp(_settings.MaxHistory, 10, 500);
        _maxHistory.Width = 120;
        ChatUiTheme.StyleNumeric(_maxHistory);
        var docking = MakeFieldTable();
        AddFieldRow(docking, "Collapse side", "Edge used when the overlay is collapsed.", _collapseSide, 0);
        AddFieldRow(docking, "Chat history", "Messages kept in memory. 200 is a good default and stays lightweight.", _maxHistory, 1);
        AddPageCard(stack, MakeCard("Docking & memory", "Choose where the compact edge handle lives and how much recent chat is retained.", docking));
        return page;
    }

    private Control BuildAlertsPage()
    {
        var page = CreatePage(
            "Highlights & sounds",
            "Highlight important messages visually and optionally play a local sound. Nothing is sent outside your PC.");
        var stack = (TableLayoutPanel)page.Tag!;

        _highlight.Text = _settings.HighlightIfMatches;
        _highlight.Height = 104;
        ChatUiTheme.StyleTextBox(_highlight, multiline: true);
        _highlight.TextChanged += (_, _) => RefreshHighlightValidation();
        _highlightValidation.AutoSize = true;
        _highlightValidation.Font = ChatUiTheme.UiFont(8.7F, FontStyle.Bold);
        _highlightValidation.Margin = new Padding(0, 8, 0, 4);

        ConfigureColorButton(_highlightColor, _highlightColorValue, "Choose highlight color");
        _highlightColor.Click += (_, _) => ChooseColor(ref _highlightColorValue, _highlightColor);
        _highlightSound.Checked = _settings.HighlightSoundEnabled;
        ChatUiTheme.StyleCheckBox(_highlightSound);
        _highlightSoundPath.ReadOnly = true;
        _highlightSoundPath.Text = _settings.HighlightSoundPath;
        ChatUiTheme.StyleTextBox(_highlightSoundPath);

        var keywordContent = MakeSingleColumnTable();
        AddStack(keywordContent, MakeFieldBlock(
            "Match rule",
            "Example: serum | food | raid. One pattern per line is OR. Sender name and message text are both checked.",
            _highlight));
        AddStack(keywordContent, _highlightValidation);
        AddStack(keywordContent, _highlightColor);
        AddStack(keywordContent, _highlightSound);
        AddStack(keywordContent, MakePathRow(_highlightSoundPath, () => BrowseSound(_highlightSoundPath), "Sound file", "Leave empty to use ReadyAlert's built-in alert sound."));
        AddPageCard(stack, MakeCard("Keyword highlight", "Use the same safe, case-insensitive filter syntax as chat tabs.", keywordContent));

        _privateHighlight.Checked = _settings.PrivateHighlightEnabled;
        _privateSound.Checked = _settings.PrivateSoundEnabled;
        ChatUiTheme.StyleCheckBox(_privateHighlight);
        ChatUiTheme.StyleCheckBox(_privateSound);
        ConfigureColorButton(_privateColor, _privateColorValue, "Choose Private / Talk color");
        _privateColor.Click += (_, _) => ChooseColor(ref _privateColorValue, _privateColor);
        _privateSoundPath.ReadOnly = true;
        _privateSoundPath.Text = _settings.PrivateSoundPath;
        ChatUiTheme.StyleTextBox(_privateSoundPath);

        var privateContent = MakeSingleColumnTable();
        AddStack(privateContent, _privateHighlight);
        AddStack(privateContent, _privateColor);
        AddStack(privateContent, _privateSound);
        AddStack(privateContent, MakePathRow(_privateSoundPath, () => BrowseSound(_privateSoundPath), "Sound file", "Leave empty to use ReadyAlert's built-in alert sound."));
        AddPageCard(stack, MakeCard("Private / Talk", "Give direct messages a stronger visual or audio cue.", privateContent));

        RefreshHighlightValidation();
        return page;
    }

    private Control BuildAdvancedPage()
    {
        var page = CreatePage(
            "Advanced",
            "Less-used customization and troubleshooting tools. These do not start another packet capture.");
        var stack = (TableLayoutPanel)page.Tag!;

        var customize = MakeSingleColumnTable();
        AddStack(customize, MakeActionRow(
            "Channel colors",
            "Change the color used for World, Local, Team, Guild and other channel labels.",
            "Customize…",
            () =>
            {
                using var dialog = new ChannelColorsForm(_channelColorsWorking);
                if (dialog.ShowDialog(this) == DialogResult.OK) _channelColorsWorking = dialog.Result;
            }));
        AddStack(customize, MakeActionRow(
            "Blocked users",
            "Review players blocked from the overlay and unblock them if needed.",
            "Manage…",
            () =>
            {
                using var dialog = new BlockedUsersForm(_blockedWorking);
                dialog.ShowDialog(this);
            }));
        AddPageCard(stack, MakeCard("Customization", "Personalize channel identity and maintain your local block list.", customize));

        var diagnostics = MakeActionRow(
            "Chat capture status",
            "Live counters from ReadyAlert's existing shared CaptureEngine chat consumer. Useful when the overlay is empty.",
            "Open status…",
            () =>
            {
                using var dialog = new ChatDebugStatusForm();
                dialog.ShowDialog(this);
            });
        AddPageCard(stack, MakeCard("Diagnostics", "Quickly tell whether BPSR chat packets are reaching the parser and UI queue.", diagnostics));

        AddPageCard(stack, MakeInfoBanner(
            "Local and view-only",
            "Chat Overlay only reads the same BPSR packet stream ReadyAlert already captures. It cannot send chat, inject into BPSR, or automate gameplay.",
            ChatUiTheme.Accent));
        return page;
    }

    private static TableLayoutPanel MakeSingleColumnTable()
    {
        var table = new TableLayoutPanel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            AutoSizeMode = AutoSizeMode.GrowAndShrink,
            ColumnCount = 1,
            RowCount = 0,
            Margin = Padding.Empty
        };
        table.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100F));
        return table;
    }
}
