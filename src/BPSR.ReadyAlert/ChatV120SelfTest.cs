namespace BPSR.ReadyAlert;

internal static class ChatV120SelfTest
{
    internal static void Run()
    {
        TestChannelSelection();
        TestOwnUsernameFilter();
        TestSettingsNormalization();
        TestTtsChunking();
        TestGoogleEnglishSelection();
        TestContentFilters();
        TestSpeechQueuePriority();
        TestToolbarTtsToggle();
    }

    private static void TestChannelSelection()
    {
        Assert(ChatSpeechTranslationSettings.TranslationChannelEnabled(ChatChannel.World, world: true, guild: false, partyTeam: false),
            "World translation toggle includes World chat");
        Assert(!ChatSpeechTranslationSettings.TranslationChannelEnabled(ChatChannel.World, world: false, guild: true, partyTeam: true),
            "World translation stays independent from Guild/Party toggles");
        Assert(ChatSpeechTranslationSettings.TranslationChannelEnabled(ChatChannel.Union, world: false, guild: true, partyTeam: false),
            "Guild translation toggle includes Union chat");
        Assert(ChatSpeechTranslationSettings.TranslationChannelEnabled(ChatChannel.Team, world: false, guild: false, partyTeam: true),
            "Party translation toggle includes Team chat");
        Assert(ChatSpeechTranslationSettings.TranslationChannelEnabled(ChatChannel.Group, world: false, guild: false, partyTeam: true),
            "Party translation toggle includes Group chat");
        Assert(!ChatSpeechTranslationSettings.TranslationChannelEnabled(ChatChannel.Private, world: true, guild: true, partyTeam: true),
            "Private chat is not implicitly translated");

        Assert(ChatSpeechTranslationSettings.TtsChannelEnabled(ChatChannel.Union, guild: true, partyTeam: false),
            "Guild TTS includes Union chat");
        Assert(ChatSpeechTranslationSettings.TtsChannelEnabled(ChatChannel.Team, guild: false, partyTeam: true),
            "Party TTS includes Team chat");
        Assert(ChatSpeechTranslationSettings.TtsChannelEnabled(ChatChannel.Group, guild: false, partyTeam: true),
            "Party TTS includes Group chat");
        Assert(!ChatSpeechTranslationSettings.TtsChannelEnabled(ChatChannel.World, guild: true, partyTeam: true),
            "World chat is never selected by TTS toggles");
        Assert(!ChatSpeechTranslationSettings.TtsChannelEnabled(ChatChannel.Private, guild: true, partyTeam: true),
            "Private chat is not implicitly spoken");
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
        var chunks = ChatSpeechTranslationEngine.SplitForTts(text, 200);
        Assert(chunks.Count > 1, "long TTS text is split into multiple requests");
        Assert(chunks.All(x => x.Length is > 0 and <= 200), "every Google TTS chunk respects the 200-character ceiling");
        Assert(string.Join(' ', chunks).Replace("  ", " ", StringComparison.Ordinal).Length > 0,
            "chunking preserves non-empty speech content");

        var emoji = string.Concat(Enumerable.Repeat("hello🙂 ", 40));
        var emojiChunks = ChatSpeechTranslationEngine.SplitForTts(emoji, 40);
        Assert(emojiChunks.All(x => !x.EndsWith('\uD83D')), "TTS chunking never ends on a dangling high surrogate");
        Assert(emojiChunks.All(x => x.Length == 0 || !char.IsLowSurrogate(x[0])), "TTS chunking never starts on a dangling low surrogate");
    }

    private static void TestGoogleEnglishSelection()
    {
        Assert(ChatSpeechTranslationEngine.GoogleTtsLanguage == "en",
            "Google TTS uses the English en voice requested for ReadyAlert");
    }

    private static void TestContentFilters()
    {
        Assert(ChatContentVisibility.IsSpriteOnlyEmoji("<sprite=1>"), "sprite 1 is recognized as emoji-only chat");
        Assert(ChatContentVisibility.IsSpriteOnlyEmoji(" <sprite=63> "), "sprite 63 is recognized with whitespace");
        Assert(ChatContentVisibility.IsSpriteOnlyEmoji("<sprite=31><sprite=31> <sprite=100>"),
            "multiple sprite tokens through 100 are recognized");
        Assert(!ChatContentVisibility.IsSpriteOnlyEmoji("<sprite=0>"), "sprite 0 is outside the supported emoji range");
        Assert(!ChatContentVisibility.IsSpriteOnlyEmoji("<sprite=101>"), "sprite 101 is not hidden preemptively");
        Assert(!ChatContentVisibility.IsSpriteOnlyEmoji("hello <sprite=31>"),
            "normal text containing an emoji token is not hidden as an emoji-only message");

        var textEmoji = Message(ChatMessageKind.Text, "<sprite=62>");
        var hypertextKind = Message(ChatMessageKind.Hypertext, "[Hypertext 3000001]");
        var hypertextText = Message(ChatMessageKind.Text, "[Hypertext 1050001] MrHard");
        var ordinaryWord = Message(ChatMessageKind.Text, "I learned about hypertext today");
        var normal = Message(ChatMessageKind.Text, "what is makan nasi");

        Assert(ChatContentVisibility.ShouldSkipSpeech(textEmoji), "sprite-only emoji is never spoken literally");
        Assert(ChatContentVisibility.ShouldSkipSpeech(hypertextKind), "Hypertext kind is never spoken literally");
        Assert(ChatContentVisibility.ShouldSkipSpeech(hypertextText), "Hypertext placeholder text is never spoken literally");
        Assert(!ChatContentVisibility.ShouldSkipSpeech(ordinaryWord), "ordinary use of the word hypertext is not mistaken for a linked item");
        Assert(!ChatContentVisibility.ShouldSkipSpeech(normal), "normal chat remains eligible for speech");

        ChatContentVisibility.Configure(hideEmoji: true, hideLinkedItems: true);
        Assert(ChatContentVisibility.ShouldHideInOverlay(textEmoji), "Hide emoji suppresses sprite-only rows");
        Assert(ChatContentVisibility.ShouldHideInOverlay(hypertextKind), "Hide linked items suppresses Hypertext kind rows");
        Assert(ChatContentVisibility.ShouldHideInOverlay(hypertextText), "Hide linked items suppresses parsed Hypertext placeholder rows");
        Assert(!ChatContentVisibility.ShouldHideInOverlay(ordinaryWord), "Hide linked items preserves ordinary hypertext word usage");
        Assert(!ChatContentVisibility.ShouldHideInOverlay(normal), "content filters preserve normal chat");

        // Normalizing a temporary settings object must not mutate the live global
        // visibility filter. Runtime filter state is applied explicitly by SettingsStore.
        new ChatSpeechTranslationSettings().Normalize();
        Assert(ChatContentVisibility.ShouldHideInOverlay(textEmoji), "settings normalization has no runtime filter side effects");

        ChatContentVisibility.Configure(hideEmoji: false, hideLinkedItems: false);
        Assert(!ChatContentVisibility.ShouldHideInOverlay(textEmoji), "content filters can be explicitly disabled");
        Assert(!ChatContentVisibility.ShouldHideInOverlay(hypertextText), "linked-item filter can be explicitly disabled");
    }

    private static void TestSpeechQueuePriority()
    {
        var worldTranslation = Message(ChatMessageKind.Text, "world translation", ChatChannel.World, 101);
        var guildSpeech = Message(ChatMessageKind.Text, "guild speech", ChatChannel.Union, 202);
        Assert(ChatSpeechTranslationEngine.SpeechPriorityPrecedesTranslationForSelfTest(worldTranslation, guildSpeech),
            "Guild/Party speech priority dequeues ahead of older World translation work");
    }

    private static void TestToolbarTtsToggle()
    {
        var settings = new AppSettings { ChatOverlayEnabled = true };
        settings.Chat.Normalize();
        settings.SpeechTranslation.Normalize();
        var tempPath = Path.Combine(Path.GetTempPath(), $"BPSR-ReadyAlert-tts-toggle-{Guid.NewGuid():N}.json");

        try
        {
            using var form = new ChatOverlayForm(settings, new SettingsStore(tempPath), string.Empty, string.Empty);
            var off = form.GetV120TtsToolbarStateForSelfTest();
            Assert(!off.Enabled, "toolbar TTS starts from the saved disabled state");
            Assert(off.Strikeout, "disabled toolbar TTS uses strikeout text");
            Assert(off.Background.R > off.Background.G, "disabled toolbar TTS uses a red background");
            Assert(off.ActionIndex == 1, "toolbar TTS is directly between +Tab and Settings");

            form.ToggleV120TtsForSelfTest();
            var on = form.GetV120TtsToolbarStateForSelfTest();
            Assert(on.Enabled && settings.SpeechTranslation.TtsEnabled, "toolbar click enables the persisted TTS master switch");
            Assert(!on.Strikeout, "enabled toolbar TTS removes strikeout");
            Assert(on.Background.G > on.Background.R, "enabled toolbar TTS uses a green background");

            form.ToggleV120TtsForSelfTest();
            Assert(!settings.SpeechTranslation.TtsEnabled, "second toolbar click disables TTS again");
        }
        finally
        {
            TryDelete(tempPath);
            TryDelete(tempPath + ".bak");
            TryDelete(tempPath + ".new");
        }
    }

    private static ChatMessageEvent Message(
        ChatMessageKind kind,
        string text,
        ChatChannel channel = ChatChannel.Union,
        long sequenceId = 1) => new(
        123,
        "Tester",
        60,
        channel,
        DateTime.UtcNow,
        kind,
        text,
        sequenceId);

    private static void TryDelete(string path)
    {
        try
        {
            if (File.Exists(path)) File.Delete(path);
        }
        catch { }
    }

    private static void Assert(bool condition, string name)
    {
        if (!condition) throw new InvalidOperationException("Chat v1.2.0 self-test failed: " + name);
    }
}
