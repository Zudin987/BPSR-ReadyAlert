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

        // Keep v1.2.5 geometry out of the visible gear-click path. Normally this was
        // already applied during hidden prewarm; this also covers the soft fallback
        // path where Settings had to be constructed only when the gear was clicked.
        ApplyV125SettingsPolish();

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
    /// Create all Settings handles while the dialog is hidden so the first visible
    /// open does not pay native-control creation costs. Layout is intentionally done
    /// once at the form root: the old recursive PerformLayout() at every node caused
    /// the same nested AutoSize trees to be measured repeatedly and could monopolize
    /// the WinForms UI thread during idle prewarm.
    /// </summary>
    internal void PrewarmV124ForOwner(Form owner)
    {
        if (IsDisposed || Disposing || Visible) return;

        ApplyV125SettingsPolish();
        Owner = owner;
        TopMost = owner.TopMost;

        SuspendLayout();
        try
        {
            CreateV124Handles(this);
        }
        finally
        {
            ResumeLayout(performLayout: false);
        }
        PerformLayout();
    }

    private static void CreateV124Handles(Control control)
    {
        // CreateControl() may skip effectively-hidden descendants. Accessing Handle
        // explicitly realizes each native control, but no nested PerformLayout calls
        // are made here; the root layout after the walk is sufficient.
        if (!control.IsHandleCreated)
            _ = control.Handle;

        foreach (Control child in control.Controls)
            CreateV124Handles(child);
    }

    internal bool AreV124InstalledFontsDeferredForSelfTest() => !_fontFamiliesLoaded;

    internal (bool HandleReady, bool Visible, DialogResult Result, int TtsVolume)
        GetV124ReuseStateForSelfTest() =>
        (IsHandleCreated, Visible, DialogResult, _ttsVolume.Value);
}
