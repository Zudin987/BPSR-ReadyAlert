using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed partial class ChatOverlayForm
{
    private ChatGeneralSettingsForm? _v124SettingsDialog;
    private System.Windows.Forms.Timer? _v124SettingsPrewarmTimer;
    private bool _v124SettingsCacheHooksInstalled;

    /// <summary>
    /// Queue the expensive one-time Settings control realization shortly after the
    /// overlay is already visible. This keeps both overlay startup and the gear-click
    /// path responsive while preserving the all-pages-realized tab-switch model.
    /// </summary>
    private void QueueV124SettingsPrewarm()
    {
        if (IsDisposed || Disposing ||
            _v124SettingsDialog is { IsDisposed: false } ||
            _v124SettingsPrewarmTimer is not null)
            return;

        if (!_v124SettingsCacheHooksInstalled)
        {
            _v124SettingsCacheHooksInstalled = true;
            Disposed += (_, _) => DisposeV124SettingsCache();
        }

        // Give the overlay itself time to paint and settle first. Settings is still
        // prepared long before a normal gear-button click, but overlay appearance is
        // not delayed by constructing the editor synchronously.
        var timer = new System.Windows.Forms.Timer { Interval = 350 };
        _v124SettingsPrewarmTimer = timer;
        timer.Tick += (_, _) =>
        {
            timer.Stop();
            timer.Dispose();
            if (ReferenceEquals(_v124SettingsPrewarmTimer, timer))
                _v124SettingsPrewarmTimer = null;

            if (IsDisposed || Disposing ||
                _v124SettingsDialog is { IsDisposed: false })
                return;

            ChatGeneralSettingsForm? dialog = null;
            try
            {
                dialog = new ChatGeneralSettingsForm(_settings.Chat, _settings.SpeechTranslation);
                dialog.PrewarmV124ForOwner(this);
                _v124SettingsDialog = dialog;
                dialog = null;
            }
            catch (Exception ex)
            {
                try { dialog?.Dispose(); } catch { }
                AppLog.Write("settings: background prewarm failed " + ex.Message);
            }
        };
        timer.Start();
    }

    private ChatGeneralSettingsForm GetV124SettingsDialog()
    {
        if (_v124SettingsDialog is { IsDisposed: false } cached)
            return cached;

        _v124SettingsDialog = new ChatGeneralSettingsForm(_settings.Chat, _settings.SpeechTranslation);
        return _v124SettingsDialog;
    }

    /// <summary>
    /// Open the cached editor. PrepareV124ForOpen restores current persisted/live
    /// values first, so closing with unapplied edits behaves exactly like the old
    /// create-and-dispose-per-click dialog.
    /// </summary>
    private void OpenV124CachedSettingsDialog()
    {
        if (InvokeRequired)
        {
            BeginInvoke(new Action(OpenV124CachedSettingsDialog));
            return;
        }

        _v124SettingsPrewarmTimer?.Stop();
        _v124SettingsPrewarmTimer?.Dispose();
        _v124SettingsPrewarmTimer = null;

        var oldClickThrough = _settings.Chat.ClickThrough;
        var dialog = GetV124SettingsDialog();
        dialog.PrepareV124ForOpen(this);
        if (dialog.ShowDialog(this) != DialogResult.OK) return;

        // Preserve the legacy post-OK path for completeness. Normal Settings saves
        // are applied immediately through the owner while the dialog stays open.
        _settings.Chat.Normalize();
        _settings.SpeechTranslation.Normalize();
        RemoveOverflowHistoryFromView();
        _settingsStore.Save(_settings);
        ChatSpeechTranslationEngine.Configure(_settings.SpeechTranslation, _v120TranslationQueue);
        ApplyWindowSettings(registerHotkeys: true);
        RebuildTabBar();
        RebuildVisibleMessages(keepScroll: true);
        UpdateEmptyState();

        if (!oldClickThrough && _settings.Chat.ClickThrough)
            AppLog.Write("chat: click-through enabled; use " + _settings.Chat.ClickThroughHotkey + " to toggle it");
    }

    private void DisposeV124SettingsCache()
    {
        var timer = _v124SettingsPrewarmTimer;
        _v124SettingsPrewarmTimer = null;
        if (timer is not null)
        {
            try { timer.Stop(); } catch { }
            try { timer.Dispose(); } catch { }
        }

        var dialog = _v124SettingsDialog;
        _v124SettingsDialog = null;
        if (dialog is null || dialog.IsDisposed) return;
        try { dialog.Dispose(); } catch { }
    }

    internal bool HasV124PrewarmedSettingsForSelfTest() =>
        _v124SettingsDialog is { IsDisposed: false, IsHandleCreated: true };
}
