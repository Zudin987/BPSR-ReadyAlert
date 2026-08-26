using System.Drawing;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed partial class ChatGeneralSettingsForm
{
    private void RefreshHighlightValidation()
    {
        if (ChatFilterExpression.TryValidate(_highlight.Text, out var error))
        {
            _highlightValidation.ForeColor = ChatUiTheme.Success;
            _highlightValidation.Text = string.IsNullOrWhiteSpace(_highlight.Text)
                ? "● No keyword rule configured — all messages use normal styling."
                : "● Rule is valid — matching is case-insensitive.";
        }
        else
        {
            _highlightValidation.ForeColor = ChatUiTheme.Danger;
            _highlightValidation.Text = "● " + error;
        }
    }

    private void ApplyChanges()
    {
        if (!ChatFilterExpression.TryValidate(_highlight.Text, out var highlightError))
        {
            ShowPage("Alerts");
            MessageBox.Show(this, highlightError, "Highlight rule is invalid", MessageBoxButtons.OK, MessageBoxIcon.Warning);
            _highlight.Focus();
            return;
        }
        if (!ChatHotkey.TryParse(_clickHotkey.Text, out var clickGesture, out var clickError))
        {
            ShowPage("Interaction");
            MessageBox.Show(this, clickError, "Click-through hotkey is invalid", MessageBoxButtons.OK, MessageBoxIcon.Warning);
            _clickHotkey.Focus();
            return;
        }
        if (!ChatHotkey.TryParse(_collapseHotkey.Text, out var collapseGesture, out var collapseError))
        {
            ShowPage("Interaction");
            MessageBox.Show(this, collapseError, "Collapse hotkey is invalid", MessageBoxButtons.OK, MessageBoxIcon.Warning);
            _collapseHotkey.Focus();
            return;
        }
        if (clickGesture.Equals(collapseGesture))
        {
            ShowPage("Interaction");
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
        _settings.ChatSoundVolume = _soundVolume.Value;
        _settings.ChannelColors = new Dictionary<int, string>(_channelColorsWorking);
        _settings.BlockedUsers = _blockedWorking.Select(CloneBlockedUser).ToList();
        _settings.Normalize();

        if (Owner is ChatOverlayForm overlay)
            overlay.ApplySettingsFromOpenDialog();

        _applyStatus.Text = "Saved ✓";
    }

    private void ResetToDefaultsAndApply()
    {
        var answer = MessageBox.Show(
            this,
            "Reset Chat Overlay to its default settings?\r\n\r\n" +
            "This resets appearance, hotkeys, filters, sounds, channel colors, blocked users, and custom tabs. " +
            "The overlay's current window position and size will be kept.",
            "Reset Chat Overlay",
            MessageBoxButtons.YesNo,
            MessageBoxIcon.Warning,
            MessageBoxDefaultButton.Button2);
        if (answer != DialogResult.Yes) return;

        var defaults = new ChatOverlaySettings();
        defaults.Normalize();

        // Factory-reset the chat configuration while deliberately preserving the
        // current window placement so the overlay does not jump during live use.
        _settings.Tabs = defaults.Tabs.Select(x => x.Clone()).ToList();
        _settings.LastSelectedTabId = _settings.Tabs[0].Id;
        _blockedWorking = [];
        _channelColorsWorking = new Dictionary<int, string>(defaults.ChannelColors);
        LoadControlsFrom(defaults);
        ApplyChanges();
        _applyStatus.Text = "Defaults restored ✓";
    }

    private void LoadControlsFrom(ChatOverlaySettings source)
    {
        _topMost.Checked = source.TopMost;
        _compact.Checked = source.CompactMode;
        _showTime.Checked = source.ShowTime;
        _timeAgo.Checked = source.ShowTimeAsAgo;
        _timeAgo.Enabled = source.ShowTime;
        _hideStickers.Checked = source.HideStickers;
        _bold.Checked = source.BoldMessageText;
        _shadow.Checked = source.TextShadow;
        _separators.Checked = source.ShowSeparators;
        _zebra.Checked = source.ShowZebraStripes;
        _colorBand.Checked = source.ShowColorBand;

        _backgroundOpacity.Value = Math.Clamp(source.BackgroundOpacity, _backgroundOpacity.Minimum, _backgroundOpacity.Maximum);
        _toolbarOpacity.Value = Math.Clamp(source.ToolbarOpacity, _toolbarOpacity.Minimum, _toolbarOpacity.Maximum);
        _textOpacity.Value = Math.Clamp(source.TextOpacity, _textOpacity.Minimum, _textOpacity.Maximum);
        _windowOpacity.Value = Math.Clamp(source.WindowOpacity, _windowOpacity.Minimum, _windowOpacity.Maximum);

        if (!_fontFamily.Items.Contains(source.FontFamily)) _fontFamily.Items.Add(source.FontFamily);
        _fontFamily.SelectedItem = source.FontFamily;
        _fontSize.Value = Math.Clamp((decimal)source.FontSize, _fontSize.Minimum, _fontSize.Maximum);

        _clickThrough.Checked = source.ClickThrough;
        _clickHotkey.Text = source.ClickThroughHotkey;
        _collapseHotkey.Text = source.CollapseHotkey;
        _collapseSide.SelectedItem = source.CollapseSide;
        _maxHistory.Value = Math.Clamp(source.MaxHistory, (int)_maxHistory.Minimum, (int)_maxHistory.Maximum);

        _highlight.Text = source.HighlightIfMatches;
        _highlightColorValue = source.HighlightColor;
        ConfigureColorButton(_highlightColor, _highlightColorValue, "Choose highlight color");
        _highlightSound.Checked = source.HighlightSoundEnabled;
        _highlightSoundPath.Text = source.HighlightSoundPath;

        _privateHighlight.Checked = source.PrivateHighlightEnabled;
        _privateColorValue = source.PrivateHighlightColor;
        ConfigureColorButton(_privateColor, _privateColorValue, "Choose Private / Talk color");
        _privateSound.Checked = source.PrivateSoundEnabled;
        _privateSoundPath.Text = source.PrivateSoundPath;
        _soundVolume.Value = Math.Clamp(source.ChatSoundVolume, _soundVolume.Minimum, _soundVolume.Maximum);

        _blockedWorking = source.BlockedUsers.Select(CloneBlockedUser).ToList();
        _channelColorsWorking = new Dictionary<int, string>(source.ChannelColors);
        RefreshHighlightValidation();
    }

    private static void ConfigureColorButton(Button button, string colorValue, string text)
    {
        button.Text = text;
        button.Width = 230;
        button.Height = 34;
        button.BackColor = ChatColorUtil.Parse(colorValue, Color.DimGray);
        button.ForeColor = ContrastText(button.BackColor);
        button.FlatStyle = FlatStyle.Flat;
        button.FlatAppearance.BorderSize = 1;
        button.FlatAppearance.BorderColor = ChatUiTheme.BorderStrong;
        button.Cursor = Cursors.Hand;
        button.Margin = new Padding(0, 6, 0, 10);
    }

    private static void ChooseColor(ref string target, Button button)
    {
        using var dialog = new ColorDialog { FullOpen = true, Color = ChatColorUtil.Parse(target, Color.DimGray) };
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
        if (dialog.ShowDialog(this) == DialogResult.OK) target.Text = dialog.FileName;
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
