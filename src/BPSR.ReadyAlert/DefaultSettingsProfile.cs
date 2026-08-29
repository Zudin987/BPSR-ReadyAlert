namespace BPSR.ReadyAlert;

/// <summary>
/// Canonical fresh-install and Reset-to-defaults profile. Keep this centralized so
/// the first-run experience and Chat Overlay reset action cannot drift apart.
/// Existing settings.json values are loaded normally and are not overwritten.
/// </summary>
internal static class DefaultSettingsProfile
{
    internal static ChatOverlaySettings CreateChatOverlay()
    {
        var settings = new ChatOverlaySettings
        {
            TopMost = true,
            CompactMode = false,
            ShowTime = true,
            ShowTimeAsAgo = true,
            HideStickers = true,
            BackgroundOpacity = 82,
            ToolbarOpacity = 92,
            TextOpacity = 100,
            WindowOpacity = 100,
            FontFamily = "Segoe UI",
            FontSize = 12F,
            BoldMessageText = false,
            TextShadow = true,
            ShowSeparators = true,
            ShowZebraStripes = true,
            ShowColorBand = true,
            ClickThrough = false,
            CollapseSide = "Left",
            Tabs =
            [
                new ChatTabSettings
                {
                    Id = 639233255393111833L,
                    Name = "All",
                    Channels = [1, 2, 3, 4, 5, 6, 9],
                    MinLevel = 50,
                    ShowIfMatches = string.Empty,
                    HideIfMatches = string.Empty
                },
                new ChatTabSettings
                {
                    Id = 639233255393111900L,
                    Name = "Guild&Team",
                    Channels = [3, 4, 5, 6],
                    MinLevel = 1,
                    ShowIfMatches = string.Empty,
                    HideIfMatches = string.Empty
                },
                new ChatTabSettings
                {
                    Id = 639233255393111918L,
                    Name = "Guild",
                    Channels = [4],
                    MinLevel = 1,
                    ShowIfMatches = string.Empty,
                    HideIfMatches = string.Empty
                },
                new ChatTabSettings
                {
                    Id = 639235625391474596L,
                    Name = "Team",
                    Channels = [3, 6],
                    MinLevel = 1,
                    ShowIfMatches = string.Empty,
                    HideIfMatches = string.Empty
                }
            ]
        };

        settings.Normalize();
        return settings;
    }

    internal static ChatSpeechTranslationSettings CreateSpeechTranslation()
    {
        var settings = new ChatSpeechTranslationSettings
        {
            TranslationEnabled = true,
            TranslationWorld = true,
            TranslationGuild = true,
            TranslationPartyTeam = true,
            ShowTranslationInOverlay = true,
            TtsEnabled = true,
            TtsGuild = false,
            TtsPartyTeam = true,
            ReadSenderName = true,
            TtsVolume = 100,
            HideEmojiMessages = true,
            HideLinkedItemMessages = true
        };

        settings.Normalize();
        return settings;
    }
}
