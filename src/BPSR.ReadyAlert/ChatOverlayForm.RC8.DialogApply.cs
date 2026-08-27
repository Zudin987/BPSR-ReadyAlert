namespace BPSR.ReadyAlert;

internal sealed partial class ChatOverlayForm
{
    /// <summary>
    /// Applies settings while the modal Settings window remains open. Keeping the
    /// dialog open makes tuning opacity/fonts/hotkeys much faster and avoids the
    /// old save-close-reopen loop. The return value tells the dialog whether the
    /// settings were also durably persisted to disk.
    /// </summary>
    internal bool ApplySettingsFromOpenDialog()
    {
        _settings.Chat.Normalize();
        RemoveOverflowHistoryFromView();
        var saved = _settingsStore.Save(_settings);
        ApplyWindowSettings(registerHotkeys: true);
        RebuildTabBar();
        RebuildVisibleMessages(keepScroll: true);
        UpdateEmptyState();
        return saved;
    }

    /// <summary>
    /// Applies Add/Edit Tab changes live while the editor stays open. For a new
    /// tab, the first Apply inserts it; later Apply presses update the same tab.
    /// </summary>
    internal void ApplyTabFromOpenDialog(ChatTabSettings source, bool isNew)
    {
        var existing = _settings.Chat.Tabs.FirstOrDefault(x => x.Id == source.Id);
        if (existing is null)
        {
            if (!isNew) return;
            existing = source;
            _settings.Chat.Tabs.Add(existing);
        }
        else if (!ReferenceEquals(existing, source))
        {
            existing.Name = source.Name;
            existing.Channels = new List<int>(source.Channels);
            existing.MinLevel = source.MinLevel;
            existing.ShowIfMatches = source.ShowIfMatches;
            existing.HideIfMatches = source.HideIfMatches;
        }

        _settings.Chat.LastSelectedTabId = existing.Id;
        _settings.Chat.Normalize();
        _settingsStore.Save(_settings);
        RebuildTabBar();
        RebuildVisibleMessages(keepScroll: false);
        UpdateEmptyState();
    }
}
