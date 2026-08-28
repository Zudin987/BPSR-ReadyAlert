using System.Drawing;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed partial class ChatGeneralSettingsForm
{
    /// <summary>
    /// Re-synchronize a cached Settings dialog with the live overlay settings before
    /// each modal open. This preserves the old create-per-open semantics: edits that
    /// were discarded on Close do not reappear the next time Settings is opened.
    /// </summary>
    internal void PrepareV124ForOpen(Form? owner = null)
    {
        if (IsDisposed || Disposing) return;

        _v121DirtyRefreshTimer.Stop();
        var previousSuppress = _v121SuppressDirtyTracking;
        _v121SuppressDirtyTracking = true;
        try
        {
            _settings.Normalize();
            _speechSettings?.Normalize();

            LoadControlsFrom(_settings);
            if (_speechSettings is not null)
                LoadSpeechTranslationControls(_speechSettings);

            _blockedWorking = _settings.BlockedUsers.Select(CloneBlockedUser).ToList();
            _channelColorsWorking = new Dictionary<int, string>(_settings.ChannelColors);
            _highlightColorValue = _settings.HighlightColor;
            _privateColorValue = _settings.PrivateHighlightColor;

            _v121EverSaved = false;
            _v121AppliedNotPersisted = false;
            _applyStatus.Text = string.Empty;
            _applyStatus.ForeColor = ChatUiTheme.SettingsMuted;

            DialogResult = DialogResult.None;
            ActiveControl = null;
            ShowPage("Appearance");

            foreach (var page in _pages.Values.Select(x => x.Page))
            {
                if (page.AutoScroll)
                    page.AutoScrollPosition = Point.Empty;
            }

            if (owner is not null)
            {
                Owner = owner;
                TopMost = owner.TopMost;
            }
        }
        finally
        {
            _v121SuppressDirtyTracking = previousSuppress;
        }

        // On the first open OnShown() installs dirty tracking and creates the
        // baseline. Reused forms already have tracking installed, so refresh that
        // baseline after the live values above have been copied back into controls.
        if (_v121DirtyTrackingInstalled)
        {
            _v121SavedFingerprint = CaptureV121EditorFingerprint();
            _v121DirtyTrackingReady = true;
        }
    }

    /// <summary>
    /// Create handles and complete layout while the dialog is still hidden. The
    /// overlay queues this after its own first paint, moving expensive one-time
    /// WinForms realization away from the Settings-button click path.
    /// </summary>
    internal void PrewarmV124ForOwner(Form owner)
    {
        if (IsDisposed || Disposing || Visible) return;

        Owner = owner;
        TopMost = owner.TopMost;
        CreateV124ControlTree(this);
    }

    private static void CreateV124ControlTree(Control control)
    {
        control.CreateControl();
        foreach (Control child in control.Controls)
            CreateV124ControlTree(child);
        control.PerformLayout();
    }

    internal bool AreV124InstalledFontsDeferredForSelfTest() => !_fontFamiliesLoaded;
}
