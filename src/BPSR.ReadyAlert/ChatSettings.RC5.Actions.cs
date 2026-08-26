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

    private void SaveAndClose()
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
        _settings.ChannelColors = _channelColorsWorking;
        _settings.BlockedUsers = _blockedWorking;
        DialogResult = DialogResult.OK;
        Close();
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
