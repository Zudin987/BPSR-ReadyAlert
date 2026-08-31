namespace BPSR.ReadyAlert;

internal static class TranslationLanguageLabelV136SelfTest
{
    internal static void Run()
    {
        Assert(ChatOverlayForm.GetV120TranslationSourceLabelForSelfTest("ms") == "MY",
            "Malay source code is presented as MY");
        Assert(ChatOverlayForm.GetV120TranslationSourceLabelForSelfTest("ms-MY") == "MY",
            "Malay regional source code is compacted to MY");
        Assert(ChatOverlayForm.GetV120TranslationSourceLabelForSelfTest("ja") == "JA",
            "other detected languages retain their compact language code");
        Assert(ChatOverlayForm.GetV120TranslationSourceLabelForSelfTest("zh-CN") == "ZH",
            "regional source tags are compacted to their base language");
        Assert(ChatOverlayForm.GetV120TranslationSourceLabelForSelfTest(string.Empty) == "AUTO",
            "missing source language is shown honestly as AUTO");

        var settings = new AppSettings { ChatOverlayEnabled = true };
        settings.Chat.Normalize();
        settings.SpeechTranslation.TranslationEnabled = true;
        settings.SpeechTranslation.TranslationGuild = true;
        settings.SpeechTranslation.ShowTranslationInOverlay = true;
        settings.SpeechTranslation.Normalize();

        var tempPath = Path.Combine(
            Path.GetTempPath(),
            $"BPSR-ReadyAlert-translation-language-{Guid.NewGuid():N}.json");

        try
        {
            using var form = new ChatOverlayForm(
                settings,
                new SettingsStore(tempPath),
                string.Empty,
                string.Empty);

            var message = new ChatMessageEvent(
                42,
                "Tester",
                50,
                ChatChannel.Union,
                DateTime.Now,
                ChatMessageKind.Text,
                "makan nasi",
                9136);

            form.EnqueueV120TranslationForSelfTest(
                new ChatTranslationResult(9136, "eat rice", "ms"));
            form.DrainV120TranslationsForSelfTest();

            Assert(form.GetV120TranslationLabelForSelfTest(message) == "↳ MY → EN: eat rice",
                "overlay translation line includes detected source and target languages");
        }
        finally
        {
            try { File.Delete(tempPath); } catch { }
        }
    }

    private static void Assert(bool condition, string message)
    {
        if (!condition)
            throw new InvalidOperationException("v1.3.6 translation language label self-test failed: " + message);
    }
}
