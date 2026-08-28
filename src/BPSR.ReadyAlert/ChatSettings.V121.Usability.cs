using System.Text;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed partial class ChatGeneralSettingsForm
{
    private bool _v121DirtyTrackingInstalled;
    private bool _v121DirtyTrackingReady;
    private bool _v121SuppressDirtyTracking;
    private bool _v121EverSaved;
    private bool _v121AppliedNotPersisted;
    private string _v121SavedFingerprint = string.Empty;
    private readonly System.Windows.Forms.Timer _v121DirtyRefreshTimer = new() { Interval = 80 };

    private void InstallV121UsabilityTracking()
    {
        if (_v121DirtyTrackingInstalled) return;
        _v121DirtyTrackingInstalled = true;

        _soundVolume.AccessibleName = "Chat alert volume";
        _soundVolume.AccessibleDescription = "Controls keyword rules and Private or Talk chat sounds only.";
        _ttsVolume.AccessibleName = "TTS volume";
        _ttsVolume.AccessibleDescription = "Controls spoken Guild and Party or Team chat only.";

        _v121DirtyRefreshTimer.Tick += (_, _) =>
        {
            _v121DirtyRefreshTimer.Stop();
            RefreshV121DirtyStatus();
        };
        Disposed += (_, _) =>
        {
            _v121DirtyRefreshTimer.Stop();
            _v121DirtyRefreshTimer.Dispose();
        };

        SubscribeV121Changes(this);
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

    private void V121EditorValueChanged(object? sender, EventArgs e)
    {
        if (!_v121DirtyTrackingReady || _v121SuppressDirtyTracking) return;

        if (_applyStatus.Text != "Unsaved")
            _applyStatus.Text = "Unsaved";
        _applyStatus.ForeColor = ChatUiTheme.Warning;

        _v121DirtyRefreshTimer.Stop();
        _v121DirtyRefreshTimer.Start();
    }

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
            return;
        }

        if (_v121AppliedNotPersisted)
        {
            _applyStatus.Text = "Applied — not saved";
            _applyStatus.ForeColor = ChatUiTheme.Danger;
        }
        else
        {
            _applyStatus.Text = _v121EverSaved ? "Saved ✓" : string.Empty;
            _applyStatus.ForeColor = _v121EverSaved ? ChatUiTheme.Success : ChatUiTheme.SettingsMuted;
        }
    }

    /// <summary>
    /// A successful Save must establish the exact current editor state as the new
    /// close baseline. Re-load normalized runtime values while tracking is suppressed
    /// so no queued change event can turn a just-saved dialog back into "Unsaved".
    /// </summary>
    private void MarkV121SavedBaseline()
    {
        InstallV121UsabilityTracking();
        _v121DirtyRefreshTimer.Stop();
        var previous = _v121SuppressDirtyTracking;
        _v121SuppressDirtyTracking = true;
        try
        {
            LoadControlsFrom(_settings);
            if (_speechSettings is not null)
                LoadSpeechTranslationControls(_speechSettings);
        }
        finally
        {
            _v121SuppressDirtyTracking = previous;
        }

        _v121SavedFingerprint = CaptureV121EditorFingerprint();
        _v121EverSaved = true;
        _v121AppliedNotPersisted = false;
        _applyStatus.Text = "Saved ✓";
        _applyStatus.ForeColor = ChatUiTheme.Success;
    }

    private void MarkV121AppliedButNotPersisted()
    {
        InstallV121UsabilityTracking();
        _v121DirtyRefreshTimer.Stop();
        var previous = _v121SuppressDirtyTracking;
        _v121SuppressDirtyTracking = true;
        try
        {
            LoadControlsFrom(_settings);
            if (_speechSettings is not null)
                LoadSpeechTranslationControls(_speechSettings);
        }
        finally
        {
            _v121SuppressDirtyTracking = previous;
        }

        _v121SavedFingerprint = CaptureV121EditorFingerprint();
        _v121EverSaved = false;
        _v121AppliedNotPersisted = true;
        _applyStatus.Text = "Applied — not saved";
        _applyStatus.ForeColor = ChatUiTheme.Danger;
    }

    private void V121SettingsFormClosing(object? sender, FormClosingEventArgs e)
    {
        if (!_v121DirtyTrackingReady || _v121SuppressDirtyTracking || e.CloseReason != CloseReason.UserClosing)
            return;

        _v121DirtyRefreshTimer.Stop();
        var warningKind = GetV121CloseWarningKind();
        if (warningKind.Length == 0) return;

        var (message, title) = warningKind switch
        {
            "dirty+persistence" => (
                "Some edits have not been applied, and the last applied settings could not be saved to disk.\r\n\r\n" +
                "Closing will discard the unapplied edits. The settings already applied to this session may also be lost after ReadyAlert restarts. Close anyway?",
                "Changes are not safely saved"),
            "persistence" => (
                "These settings are active for this ReadyAlert session, but Windows could not save them to disk.\r\n\r\n" +
                "Closing keeps them active until ReadyAlert exits, but they may be lost after restart. Close anyway?",
                "Settings are not saved"),
            _ => (
                "Discard the changes you have not saved?",
                "Unsaved Chat Overlay changes")
        };

        var answer = MessageBox.Show(
            this,
            message,
            title,
            MessageBoxButtons.YesNo,
            MessageBoxIcon.Warning,
            MessageBoxDefaultButton.Button2);
        if (answer != DialogResult.Yes)
            e.Cancel = true;
    }

    private string GetV121CloseWarningKind()
    {
        var dirty = !string.Equals(
            CaptureV121EditorFingerprint(),
            _v121SavedFingerprint,
            StringComparison.Ordinal);
        if (dirty && _v121AppliedNotPersisted) return "dirty+persistence";
        if (dirty) return "dirty";
        return _v121AppliedNotPersisted ? "persistence" : string.Empty;
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

    private static bool IsDescendantOf(Control child, Control? ancestor)
    {
        if (ancestor is null) return false;
        for (Control? current = child; current is not null; current = current.Parent)
            if (ReferenceEquals(current, ancestor)) return true;
        return false;
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
        return FindButtonByText(this, "Close")?.Text ?? string.Empty;
    }

    internal void SetV121TtsVolumeForSelfTest(int volume)
    {
        InstallV121UsabilityTracking();
        _ttsVolume.Value = Math.Clamp(volume, _ttsVolume.Minimum, _ttsVolume.Maximum);
        _v121DirtyRefreshTimer.Stop();
        RefreshV121DirtyStatus();
    }

    internal void SetV121AppliedNotPersistedForSelfTest(bool value)
    {
        InstallV121UsabilityTracking();
        _v121AppliedNotPersisted = value;
        _v121SavedFingerprint = CaptureV121EditorFingerprint();
    }

    internal void MarkV121SavedForSelfTest()
    {
        MarkV121SavedBaseline();
    }

    internal string GetV121CloseWarningKindForSelfTest()
    {
        InstallV121UsabilityTracking();
        return GetV121CloseWarningKind();
    }

    internal (bool DistinctControls, bool ChatOnAlertsPage, bool TtsOnSpeechPage, string ChatName, string TtsName)
        GetV121VolumeSeparationForSelfTest()
    {
        InstallV121UsabilityTracking();
        _pages.TryGetValue("Alerts", out var alerts);
        _pages.TryGetValue("Speech", out var speech);
        return (
            !ReferenceEquals(_soundVolume, _ttsVolume),
            IsDescendantOf(_soundVolume, alerts.Page),
            IsDescendantOf(_ttsVolume, speech.Page),
            _soundVolume.AccessibleName ?? string.Empty,
            _ttsVolume.AccessibleName ?? string.Empty);
    }
}
