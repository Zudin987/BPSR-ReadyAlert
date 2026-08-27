using System.Text;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed partial class ChatGeneralSettingsForm
{
    private bool _v121DirtyTrackingInstalled;
    private bool _v121DirtyTrackingReady;
    private bool _v121SuppressDirtyTracking;
    private bool _v121EverSaved;
    private string _v121SavedFingerprint = string.Empty;

    /// <summary>
    /// Settings apply live but the window intentionally stays open. Track the
    /// current editor state so an old "Saved" badge can never remain visible after
    /// another edit, and make closing with unapplied changes an explicit choice.
    /// </summary>
    private void InstallV121UsabilityTracking()
    {
        if (_v121DirtyTrackingInstalled) return;
        _v121DirtyTrackingInstalled = true;

        if (FindButtonByText(this, "Close") is { } close)
            close.Text = "Cancel";

        ReplaceV121LabelText(
            this,
            "One shared volume for all three sound rules and Private / Talk sounds.",
            "Keyword rules and Private / Talk sounds only. Independent of Ready / Queue and TTS volume.");
        ReplaceV121LabelText(
            this,
            "A single standardized volume keeps sound setup simple.",
            "One shared level for chat keyword and Private / Talk sounds; other ReadyAlert audio volumes stay independent.");
        ReplaceV121LabelText(
            this,
            "Click a box and press the combination you want. Backspace clears it.",
            "Click a box and press the combination you want. Both recovery shortcuts are required; Backspace clears the current entry before choosing another.");

        _soundVolume.AccessibleName = "Chat alert volume";
        _soundVolume.AccessibleDescription = "Controls keyword rules and Private or Talk chat sounds only.";
        _ttsVolume.AccessibleName = "TTS volume";
        _ttsVolume.AccessibleDescription = "Controls spoken Guild and Party or Team chat only.";

        SubscribeV121Changes(this);
        _applyStatus.TextChanged += (_, _) =>
        {
            if (_v121SuppressDirtyTracking) return;
            if (_applyStatus.Text is not ("Saved ✓" or "Defaults restored ✓")) return;

            _v121SuppressDirtyTracking = true;
            try
            {
                // RegisterHotkeys can safely turn click-through back OFF when its
                // recovery shortcut cannot register. Reflect that runtime correction
                // back into the still-open editor instead of leaving a false ON box.
                _clickThrough.Checked = _settings.ClickThrough;
                _clickHotkey.Text = _settings.ClickThroughHotkey;
                _collapseHotkey.Text = _settings.CollapseHotkey;
                _collapseSide.SelectedItem = _settings.CollapseSide;
                if (_speechSettings is not null)
                {
                    _ttsEnabled.Checked = _speechSettings.TtsEnabled;
                    _ttsGuild.Checked = _speechSettings.TtsGuild;
                    _ttsParty.Checked = _speechSettings.TtsPartyTeam;
                    _ttsVolume.Value = Math.Clamp(_speechSettings.TtsVolume, _ttsVolume.Minimum, _ttsVolume.Maximum);
                    _ttsVolumeValue.Text = _ttsVolume.Value + "%";
                }
            }
            finally
            {
                _v121SuppressDirtyTracking = false;
            }

            _v121SavedFingerprint = CaptureV121EditorFingerprint();
            _v121EverSaved = true;
            _applyStatus.ForeColor = ChatUiTheme.Success;
        };

        Activated += (_, _) => RefreshV121DirtyStatus();
        FormClosing += V121SettingsFormClosing;

        _v121SavedFingerprint = CaptureV121EditorFingerprint();
        _v121DirtyTrackingReady = true;
    }

    private void SubscribeV121Changes(Control parent)
    {
        foreach (Control child in parent.Controls)
        {
            switch (child)
            {
                case CheckBox check:
                    check.CheckedChanged += V121EditorValueChanged;
                    break;
                case TextBox box:
                    box.TextChanged += V121EditorValueChanged;
                    break;
                case NumericUpDown numeric:
                    numeric.ValueChanged += V121EditorValueChanged;
                    break;
                case TrackBar slider:
                    slider.ValueChanged += V121EditorValueChanged;
                    break;
                case ComboBox combo:
                    combo.SelectedIndexChanged += V121EditorValueChanged;
                    break;
            }

            if (child.HasChildren)
                SubscribeV121Changes(child);
        }
    }

    private void V121EditorValueChanged(object? sender, EventArgs e) => RefreshV121DirtyStatus();

    private void RefreshV121DirtyStatus()
    {
        if (!_v121DirtyTrackingReady || _v121SuppressDirtyTracking) return;

        var dirty = !string.Equals(
            CaptureV121EditorFingerprint(),
            _v121SavedFingerprint,
            StringComparison.Ordinal);

        if (dirty)
        {
            if (_applyStatus.Text != "Unsaved")
                _applyStatus.Text = "Unsaved";
            _applyStatus.ForeColor = ChatUiTheme.Warning;
        }
        else if (_applyStatus.Text == "Unsaved")
        {
            _applyStatus.Text = _v121EverSaved ? "Saved ✓" : string.Empty;
            _applyStatus.ForeColor = _v121EverSaved ? ChatUiTheme.Success : ChatUiTheme.Muted;
        }
    }

    private void V121SettingsFormClosing(object? sender, FormClosingEventArgs e)
    {
        if (!_v121DirtyTrackingReady || _v121SuppressDirtyTracking || e.CloseReason != CloseReason.UserClosing)
            return;

        if (string.Equals(CaptureV121EditorFingerprint(), _v121SavedFingerprint, StringComparison.Ordinal))
            return;

        var answer = MessageBox.Show(
            this,
            "Discard the changes you have not saved?\r\n\r\nSaved settings and changes already applied with 'Save changes' will be kept.",
            "Unsaved Chat Overlay changes",
            MessageBoxButtons.YesNo,
            MessageBoxIcon.Warning,
            MessageBoxDefaultButton.Button2);
        if (answer != DialogResult.Yes)
            e.Cancel = true;
    }

    private string CaptureV121EditorFingerprint()
    {
        var builder = new StringBuilder(2048);
        AppendV121ControlValues(this, builder, "root");

        builder.Append("|highlightColor=").Append(_highlightColorValue);
        builder.Append("|privateColor=").Append(_privateColorValue);

        foreach (var pair in _channelColorsWorking.OrderBy(x => x.Key))
            builder.Append("|channel:").Append(pair.Key).Append('=').Append(pair.Value);
        foreach (var user in _blockedWorking.OrderBy(x => x.Id))
            builder.Append("|blocked:").Append(user.Id).Append(':').Append(user.Name);

        return builder.ToString();
    }

    private static void AppendV121ControlValues(Control parent, StringBuilder builder, string path)
    {
        for (var i = 0; i < parent.Controls.Count; i++)
        {
            var child = parent.Controls[i];
            var childPath = path + '/' + i;
            switch (child)
            {
                case CheckBox check:
                    builder.Append('|').Append(childPath).Append("=check:").Append(check.Checked ? '1' : '0');
                    break;
                case TextBox box:
                    builder.Append('|').Append(childPath).Append("=text:").Append(box.Text);
                    break;
                case NumericUpDown numeric:
                    builder.Append('|').Append(childPath).Append("=num:").Append(numeric.Value);
                    break;
                case TrackBar slider:
                    builder.Append('|').Append(childPath).Append("=track:").Append(slider.Value);
                    break;
                case ComboBox combo:
                    builder.Append('|').Append(childPath).Append("=combo:").Append(combo.SelectedItem?.ToString() ?? string.Empty);
                    break;
            }

            if (child.HasChildren)
                AppendV121ControlValues(child, builder, childPath);
        }
    }

    private static void ReplaceV121LabelText(Control parent, string oldText, string newText)
    {
        foreach (Control child in parent.Controls)
        {
            if (child is Label label && string.Equals(label.Text, oldText, StringComparison.Ordinal))
                label.Text = newText;
            if (child.HasChildren)
                ReplaceV121LabelText(child, oldText, newText);
        }
    }

    private static Button? FindButtonByText(Control parent, string text)
    {
        foreach (Control child in parent.Controls)
        {
            if (child is Button button && string.Equals(button.Text, text, StringComparison.Ordinal))
                return button;
            var nested = FindButtonByText(child, text);
            if (nested is not null) return nested;
        }
        return null;
    }

    internal string GetV121SaveStateForSelfTest()
    {
        InstallV121UsabilityTracking();
        RefreshV121DirtyStatus();
        return _applyStatus.Text;
    }

    internal string GetV121CancelButtonTextForSelfTest()
    {
        InstallV121UsabilityTracking();
        return FindButtonByText(this, "Cancel")?.Text ?? string.Empty;
    }

    internal void SetV121TtsVolumeForSelfTest(int volume)
    {
        InstallV121UsabilityTracking();
        _ttsVolume.Value = Math.Clamp(volume, _ttsVolume.Minimum, _ttsVolume.Maximum);
        RefreshV121DirtyStatus();
    }
}
