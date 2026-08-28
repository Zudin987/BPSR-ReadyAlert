using System.Drawing;
using System.Drawing.Text;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed partial class ChatGeneralSettingsForm
{
    private bool _fontFamiliesLoaded;

    private Control BuildAppearancePage()
    {
        InitializeAppearanceValues();
        var page = CreatePage("Appearance", "Chat layout and window opacity.");
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
            ChatUiTheme.StyleSettingsCheckBox(checks[i]);
            checks[i].Dock = DockStyle.Fill;
            layoutGrid.Controls.Add(checks[i], i % 2, i / 2);
        }
        _showTime.CheckedChanged += (_, _) => _timeAgo.Enabled = _showTime.Checked;
        _timeAgo.Enabled = _showTime.Checked;
        AddPageCard(stack, MakeCard("Message layout", string.Empty, layoutGrid));

        var typography = MakeFieldTable();
        AddFieldRow(typography, "Font family", string.Empty, _fontFamily, 0);
        AddFieldRow(typography, "Font size", "8–24 pt", _fontSize, 1);
        AddPageCard(stack, MakeCard("Typography", string.Empty, typography));

        var opacity = MakeSingleColumnTable();
        AddStack(opacity, MakeSliderRow(
            "Window opacity",
            "Whole Chat Overlay window",
            _windowOpacity,
            _windowOpacityValue,
            _settings.WindowOpacity,
            25));
        AddPageCard(stack, MakeCard("Window opacity", string.Empty, opacity));
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

        ChatUiTheme.StyleSettingsComboBox(_fontFamily);
        if (!_fontFamily.Items.Contains(_settings.FontFamily))
            _fontFamily.Items.Add(_settings.FontFamily);
        _fontFamily.SelectedItem = _settings.FontFamily;
        _fontFamily.DropDown += (_, _) => EnsureInstalledFontFamiliesLoaded();
        _fontFamily.Width = 260;

        _fontSize.Minimum = 8;
        _fontSize.Maximum = 24;
        _fontSize.DecimalPlaces = 1;
        _fontSize.Increment = 0.5M;
        _fontSize.Value = (decimal)Math.Clamp(_settings.FontSize, 8F, 24F);
        _fontSize.Width = 90;
        ChatUiTheme.StyleSettingsNumeric(_fontSize);
    }

    private void EnsureInstalledFontFamiliesLoaded()
    {
        if (_fontFamiliesLoaded) return;
        _fontFamiliesLoaded = true;

        var selected = _fontFamily.SelectedItem?.ToString() ?? _settings.FontFamily;
        try
        {
            using var fonts = new InstalledFontCollection();
            var names = fonts.Families
                .Select(x => x.Name)
                .Append(selected)
                .Distinct(StringComparer.OrdinalIgnoreCase)
                .OrderBy(x => x, StringComparer.OrdinalIgnoreCase)
                .ToArray();

            _fontFamily.BeginUpdate();
            try
            {
                _fontFamily.Items.Clear();
                _fontFamily.Items.AddRange(names);
            }
            finally
            {
                _fontFamily.EndUpdate();
            }
        }
        catch
        {
            if (!_fontFamily.Items.Contains(selected))
                _fontFamily.Items.Add(selected);
        }

        _fontFamily.SelectedItem = selected;
    }

    private Control BuildInteractionPage()
    {
        _hideStickers.Checked = _settings.HideStickers;
        ChatUiTheme.StyleSettingsCheckBox(_hideStickers);

        var page = CreatePage("Interaction", "Cleanup, click-through hotkey and docking.");
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
        AddPageCard(stack, MakeCard("Overlay behavior", string.Empty, behavior));

        _clickHotkey.Text = _settings.ClickThroughHotkey;
        var hotkeys = MakeFieldTable();
        AddFieldRow(hotkeys, "Click-through toggle", "Default: Ctrl+Shift+F10", _clickHotkey, 0);
        AddPageCard(stack, MakeCard("Keyboard shortcut", string.Empty, hotkeys));

        ChatUiTheme.StyleSettingsComboBox(_collapseSide);
        _collapseSide.Items.AddRange(["Left", "Right", "Top", "Bottom"]);
        _collapseSide.SelectedItem = _settings.CollapseSide;
        _collapseSide.Width = 140;
        _maxHistory.Minimum = 10;
        _maxHistory.Maximum = 500;
        _maxHistory.Increment = 10;
        _maxHistory.Value = Math.Clamp(_settings.MaxHistory, 10, 500);
        _maxHistory.Width = 90;
        ChatUiTheme.StyleSettingsNumeric(_maxHistory);
        var docking = MakeFieldTable();
        AddFieldRow(docking, "Collapse side", string.Empty, _collapseSide, 0);
        AddFieldRow(docking, "Chat history", "Messages kept in memory", _maxHistory, 1);
        AddPageCard(stack, MakeCard("Docking & memory", string.Empty, docking));
        return page;
    }

    private Control BuildAlertsPage()
    {
        var page = CreatePage("Alerts", "Keyword highlighting and chat sounds.");
        var stack = (TableLayoutPanel)page.Tag!;

        var volumeContent = MakeSingleColumnTable();
        AddStack(volumeContent, MakeSliderRow(
            "Chat alert volume",
            "Chat sounds only; Ready / Queue and TTS stay separate.",
            _soundVolume,
            _soundVolumeValue,
            _settings.ChatSoundVolume,
            0));
        AddPageCard(stack, MakeCard("Sound volume", string.Empty, volumeContent));

        _highlight.Text = _settings.HighlightIfMatches;
        _highlight.Multiline = false;
        _highlight.ScrollBars = ScrollBars.None;
        _highlight.Height = 24;
        ChatUiTheme.StyleSettingsTextBox(_highlight);
        _highlight.TextChanged += (_, _) => RefreshHighlightValidation();
        _highlightValidation.AutoSize = true;
        _highlightValidation.Font = ChatUiTheme.UiFont(8.5F, FontStyle.Bold);
        _highlightValidation.Margin = new Padding(18, 3, 0, 3);

        ConfigureColorButton(_highlightColor, _highlightColorValue, "Highlight color");
        _highlightColor.Click += (_, _) => ChooseColor(ref _highlightColorValue, _highlightColor);

        var keywordContent = MakeSingleColumnTable();
        AddStack(keywordContent, MakeFieldBlock("Visual highlight rule", "Example: serum | food | raid", _highlight));
        AddStack(keywordContent, _highlightValidation);
        AddStack(keywordContent, _highlightColor);
        AddPageCard(stack, MakeCard("Visual keyword highlight", string.Empty, keywordContent));

        for (var i = 0; i < V124SoundRuleCount; i++)
            AddPageCard(stack, BuildSoundRuleCard(i));

        _privateHighlight.Checked = _settings.PrivateHighlightEnabled;
        _privateSound.Checked = _settings.PrivateSoundEnabled;
        ChatUiTheme.StyleSettingsCheckBox(_privateHighlight);
        ChatUiTheme.StyleSettingsCheckBox(_privateSound);
        ConfigureColorButton(_privateColor, _privateColorValue, "Private / Talk color");
        _privateColor.Click += (_, _) => ChooseColor(ref _privateColorValue, _privateColor);
        _privateSoundPath.ReadOnly = true;
        _privateSoundPath.Text = _settings.PrivateSoundPath;
        ChatUiTheme.StyleSettingsTextBox(_privateSoundPath);

        var privateContent = MakeSingleColumnTable();
        AddStack(privateContent, _privateHighlight);
        AddStack(privateContent, _privateColor);
        AddStack(privateContent, _privateSound);
        AddStack(privateContent, MakePathRow(_privateSoundPath, () => BrowseSound(_privateSoundPath), "Sound file", "Empty = built-in alert sound."));
        AddPageCard(stack, MakeCard("Private / Talk", string.Empty, privateContent));

        RefreshHighlightValidation();
        for (var i = 0; i < V124SoundRuleCount; i++) RefreshSoundRuleValidation(i);
        return page;
    }

    private Control BuildSoundRuleCard(int index)
    {
        var existing = index < _settings.HighlightSoundRules.Count ? _settings.HighlightSoundRules[index] : null;
        _soundRuleEnabled[index].Checked = existing?.Enabled ?? false;
        _soundRuleMatch[index].Text = existing?.Match ?? string.Empty;
        _soundRulePath[index].Text = existing?.SoundPath ?? string.Empty;

        ChatUiTheme.StyleSettingsCheckBox(_soundRuleEnabled[index]);
        _soundRuleMatch[index].Multiline = false;
        _soundRuleMatch[index].ScrollBars = ScrollBars.None;
        _soundRuleMatch[index].Height = 24;
        ChatUiTheme.StyleSettingsTextBox(_soundRuleMatch[index]);
        _soundRulePath[index].ReadOnly = true;
        ChatUiTheme.StyleSettingsTextBox(_soundRulePath[index]);

        _soundRuleValidation[index].AutoSize = true;
        _soundRuleValidation[index].Font = ChatUiTheme.UiFont(8.5F, FontStyle.Bold);
        _soundRuleValidation[index].Margin = new Padding(18, 3, 0, 3);
        _soundRuleMatch[index].TextChanged += (_, _) => RefreshSoundRuleValidation(index);
        _soundRuleEnabled[index].CheckedChanged += (_, _) => RefreshSoundRuleValidation(index);

        var content = MakeSingleColumnTable();
        AddStack(content, _soundRuleEnabled[index]);
        AddStack(content, MakeFieldBlock("Match", "Use | for OR, AND for all terms, or regex.", _soundRuleMatch[index]));
        AddStack(content, _soundRuleValidation[index]);
        AddStack(content, MakePathRow(_soundRulePath[index], () => BrowseSound(_soundRulePath[index]), "Sound file", "Empty = built-in alert sound."));

        var priority = index == 0
            ? "Highest priority; first matching sound wins."
            : "Used only when Sound rule 1 does not match.";
        return MakeCard($"Sound rule {index + 1}", priority, content);
    }

    private Control BuildAdvancedPage()
    {
        var page = CreatePage("Advanced", "Customization and diagnostics.");
        var stack = (TableLayoutPanel)page.Tag!;

        var customize = MakeSingleColumnTable();
        AddStack(customize, MakeActionRow("Channel colors", string.Empty, "Customize…", () =>
        {
            using var dialog = new ChannelColorsForm(_channelColorsWorking);
            if (dialog.ShowDialog(this) == DialogResult.OK) _channelColorsWorking = dialog.Result;
        }));
        AddStack(customize, MakeActionRow("Blocked users", string.Empty, "Manage…", () =>
        {
            using var dialog = new BlockedUsersForm(_blockedWorking);
            dialog.ShowDialog(this);
        }));
        AddPageCard(stack, MakeCard("Customization", string.Empty, customize));

        var diagnostics = MakeActionRow("Chat capture status", "Shared capture counters", "Open status…", () =>
        {
            using var dialog = new ChatDebugStatusForm();
            dialog.ShowDialog(this);
        });
        AddPageCard(stack, MakeCard("Diagnostics", string.Empty, diagnostics));

        AddPageCard(stack, MakeInfoBanner("Local and view-only", "ReadyAlert reads the existing BPSR packet stream; it cannot send chat or automate gameplay.", ChatUiTheme.Accent));
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
