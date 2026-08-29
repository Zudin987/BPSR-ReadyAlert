using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed partial class ChatOverlayForm
{
    private ChatGeneralSettingsForm? _v124SettingsDialog;
    private bool _v124SettingsCacheHooksInstalled;

    /// <summary>
    /// v1.3.2 intentionally does not construct Settings from the overlay Shown event.
    /// ChatGeneralSettingsForm is a large WinForms/AutoSize tree and hosted-Windows
    /// timing showed that constructing it on the UI thread could stall the already-
    /// visible overlay for well over a second. Keep the historical hook as a cheap
    /// no-op so startup never pays Settings work the user did not request.
    /// </summary>
    private void QueueV124SettingsPrewarm() => EnsureV124SettingsCacheHooks();

    private void EnsureV124SettingsCacheHooks()
    {
        if (_v124SettingsCacheHooksInstalled) return;
        _v124SettingsCacheHooksInstalled = true;
        Disposed += (_, _) => DisposeV124SettingsCache();
    }

    private ChatGeneralSettingsForm GetV124SettingsDialog()
    {
        if (_v124SettingsDialog is { IsDisposed: false } cached)
            return cached;

        EnsureV124SettingsCacheHooks();
        _v124SettingsDialog = new ChatGeneralSettingsForm(_settings.Chat, _settings.SpeechTranslation);
        return _v124SettingsDialog;
    }

    /// <summary>
    /// Construct Settings only after the user explicitly clicks the gear, then cache
    /// that dialog for the rest of the overlay lifetime. PrepareV124ForOpen restores
    /// current persisted/live values before each modal open, so closing unapplied edits
    /// still behaves like the old create-and-dispose-per-click dialog while repeated
    /// opens remain fast.
    /// </summary>
    private void OpenV124CachedSettingsDialog()
    {
        if (InvokeRequired)
        {
            BeginInvoke(new Action(OpenV124CachedSettingsDialog));
            return;
        }

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
        var dialog = _v124SettingsDialog;
        _v124SettingsDialog = null;
        if (dialog is null || dialog.IsDisposed) return;
        try { dialog.Dispose(); } catch { }
    }

    internal bool HasV124PrewarmedSettingsForSelfTest() =>
        _v124SettingsDialog is { IsDisposed: false, IsHandleCreated: true };

    internal bool HasV132AutomaticSettingsPrewarmForSelfTest() => false;
}
