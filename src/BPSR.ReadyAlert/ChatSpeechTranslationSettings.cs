namespace BPSR.ReadyAlert;

internal sealed class ChatSpeechTranslationSettings
{
    public bool TranslationEnabled { get; set; } = true;
    public bool TranslationWorld { get; set; } = true;
    public bool TranslationGuild { get; set; } = true;
    public bool TranslationPartyTeam { get; set; } = true;
    public bool ShowTranslationInOverlay { get; set; } = true;

    public bool TtsEnabled { get; set; } = true;
    public bool TtsGuild { get; set; } = false;
    public bool TtsPartyTeam { get; set; } = true;
    public bool ReadSenderName { get; set; } = true;
    public string IgnoreOwnUsername { get; set; } = string.Empty;
    public int TtsVolume { get; set; } = 100;

    public bool HideEmojiMessages { get; set; } = true;
    public bool HideLinkedItemMessages { get; set; } = true;

    internal void Normalize()
    {
        IgnoreOwnUsername = (IgnoreOwnUsername ?? string.Empty)
            .Replace('\r', ' ')
            .Replace('\n', ' ')
            .Replace('\0', ' ')
            .Trim();
        if (IgnoreOwnUsername.Length > 128) IgnoreOwnUsername = IgnoreOwnUsername[..128];
        TtsVolume = Math.Clamp(TtsVolume, 0, 100);
    }

    internal bool TranslationEnabledFor(ChatChannel channel) =>
        TranslationEnabled && TranslationChannelEnabled(channel, TranslationWorld, TranslationGuild, TranslationPartyTeam);

    internal bool TtsEnabledFor(ChatChannel channel) =>
        TtsEnabled && TtsChannelEnabled(channel, TtsGuild, TtsPartyTeam);

    internal bool IsOwnUsername(string? senderName) =>
        IgnoreOwnUsername.Length > 0 &&
        string.Equals(IgnoreOwnUsername, senderName?.Trim(), StringComparison.OrdinalIgnoreCase);

    internal static bool TranslationChannelEnabled(ChatChannel channel, bool world, bool guild, bool partyTeam) =>
        (world && channel == ChatChannel.World) ||
        (guild && channel == ChatChannel.Union) ||
        (partyTeam && channel is ChatChannel.Team or ChatChannel.Group);

    internal static bool TtsChannelEnabled(ChatChannel channel, bool guild, bool partyTeam) =>
        (guild && channel == ChatChannel.Union) ||
        (partyTeam && channel is ChatChannel.Team or ChatChannel.Group);
}
