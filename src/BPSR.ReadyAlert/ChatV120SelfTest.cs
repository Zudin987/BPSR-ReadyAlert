namespace BPSR.ReadyAlert;

internal static class ChatV120SelfTest
{
    internal static void Run()
    {
        TestChannelSelection();
        TestOwnUsernameFilter();
        TestSettingsNormalization();
        TestTtsChunking();
    }

    private static void TestChannelSelection()
    {
        Assert(ChatSpeechTranslationSettings.ChannelEnabled(ChatChannel.Union, guild: true, partyTeam: false),
            "Guild toggle includes Union chat");
        Assert(!ChatSpeechTranslationSettings.ChannelEnabled(ChatChannel.Team, guild: true, partyTeam: false),
            "Guild-only toggle excludes Team chat");
        Assert(ChatSpeechTranslationSettings.ChannelEnabled(ChatChannel.Team, guild: false, partyTeam: true),
            "Party toggle includes Team chat");
        Assert(ChatSpeechTranslationSettings.ChannelEnabled(ChatChannel.Group, guild: false, partyTeam: true),
            "Party toggle includes Group chat");
        Assert(!ChatSpeechTranslationSettings.ChannelEnabled(ChatChannel.World, guild: true, partyTeam: true),
            "World chat is never selected by Guild/Party speech toggles");
        Assert(!ChatSpeechTranslationSettings.ChannelEnabled(ChatChannel.Private, guild: true, partyTeam: true),
            "Private chat is not implicitly selected");
    }

    private static void TestOwnUsernameFilter()
    {
        var settings = new ChatSpeechTranslationSettings { IgnoreOwnUsername = "  MrEz  " };
        settings.Normalize();
        Assert(settings.IgnoreOwnUsername == "MrEz", "own username is trimmed");
        Assert(settings.IsOwnUsername("mrez"), "own username comparison is case-insensitive");
        Assert(settings.IsOwnUsername(" MrEz "), "sender whitespace does not bypass own-user filter");
        Assert(!settings.IsOwnUsername("MrEz2"), "own username requires an exact name match");
        Assert(!settings.IsOwnUsername("Mr"), "partial sender names do not match own-user filter");

        settings.IgnoreOwnUsername = string.Empty;
        settings.Normalize();
        Assert(!settings.IsOwnUsername("MrEz"), "blank username disables own-user filtering");
    }

    private static void TestSettingsNormalization()
    {
        var settings = new ChatSpeechTranslationSettings
        {
            IgnoreOwnUsername = "Name\r\nInjected\0",
            TtsVolume = 900
        };
        settings.Normalize();
        Assert(settings.TtsVolume == 100, "TTS volume is capped at 100 percent");
        Assert(!settings.IgnoreOwnUsername.Contains('\r') &&
               !settings.IgnoreOwnUsername.Contains('\n') &&
               !settings.IgnoreOwnUsername.Contains('\0'),
            "own username strips line/null control characters");

        settings.TtsVolume = -10;
        settings.Normalize();
        Assert(settings.TtsVolume == 0, "TTS volume is floored at zero");
    }

    private static void TestTtsChunking()
    {
        var text = string.Join(' ', Enumerable.Repeat("party-ready-check-message", 45));
        var chunks = ChatSpeechTranslationEngine.SplitForTts(text, 180);
        Assert(chunks.Count > 1, "long TTS text is split into multiple requests");
        Assert(chunks.All(x => x.Length is > 0 and <= 180), "every Google TTS chunk respects the length ceiling");
        Assert(string.Join(' ', chunks).Replace("  ", " ", StringComparison.Ordinal).Length > 0,
            "chunking preserves non-empty speech content");

        var emoji = string.Concat(Enumerable.Repeat("hello🙂 ", 40));
        var emojiChunks = ChatSpeechTranslationEngine.SplitForTts(emoji, 31);
        Assert(emojiChunks.All(x => !x.EndsWith('\uD83D')), "TTS chunking never ends on a dangling high surrogate");
        Assert(emojiChunks.All(x => x.Length == 0 || !char.IsLowSurrogate(x[0])), "TTS chunking never starts on a dangling low surrogate");
    }

    private static void Assert(bool condition, string name)
    {
        if (!condition) throw new InvalidOperationException("Chat v1.2.0 self-test failed: " + name);
    }
}
