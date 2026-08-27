using System.Drawing;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed partial class ChatGeneralSettingsForm
{
    private ChatSpeechTranslationSettings? _speechSettings;
    private readonly CheckBox _translateEnabled = new() { Text = "Translate non-English chat to English" };
    private readonly CheckBox _translateWorld = new() { Text = "World" };
    private readonly CheckBox _translateGuild = new() { Text = "Guild" };
    private readonly CheckBox _translateParty = new() { Text = "Party / Team" };
    private readonly CheckBox _translateShowOverlay = new() { Text = "Show English translation under the original message" };
    private readonly CheckBox _ttsEnabled = new() { Text = "Enable chat text-to-speech" };
    private readonly CheckBox _ttsGuild = new() { Text = "Guild" };
    private readonly CheckBox _ttsParty = new() { Text = "Party / Team" };
    private readonly CheckBox _ttsReadSender = new() { Text = "Read sender name before the message" };
    private readonly TextBox _ttsOwnUsername = new();
    private readonly TrackBar _ttsVolume = new();
    private readonly Label _ttsVolumeValue = new();

    internal ChatGeneralSettingsForm(
        ChatOverlaySettings settings,
        ChatSpeechTranslationSettings speechSettings)
        : this(settings)
    {
        _speechSettings = speechSettings;
        RegisterPage("Speech", "Speech & translation", BuildSpeechTranslationPage());

        // Keep the troubleshooting page last in the sidebar.
        if (_pages.TryGetValue("Speech", out var speech) && _pages.TryGetValue("Advanced", out var advanced))
        {
            var advancedIndex = _navHost.Controls.GetChildIndex(advanced.Button);
            _navHost.Controls.SetChildIndex(speech.Button, advancedIndex);
        }
    }

    private Control BuildSpeechTranslationPage()
    {
        var source = _speechSettings ?? new ChatSpeechTranslationSettings();
        source.Normalize();
        LoadSpeechTranslationControls(source);

        var page = CreatePage(
            "Speech & translation",
            "Translate World / Guild / Party chat to English and optionally read Guild / Party messages aloud.");
        var stack = (TableLayoutPanel)page.Tag!;

        foreach (var check in new[]
                 {
                     _translateEnabled, _translateWorld, _translateGuild, _translateParty, _translateShowOverlay,
                     _ttsEnabled, _ttsGuild, _ttsParty, _ttsReadSender
                 })
            ChatUiTheme.StyleCheckBox(check);

        _translateEnabled.CheckedChanged += (_, _) => RefreshSpeechControlState();
        _ttsEnabled.CheckedChanged += (_, _) => RefreshSpeechControlState();

        var translateChannels = new FlowLayoutPanel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            FlowDirection = FlowDirection.LeftToRight,
            WrapContents = false,
            Margin = Padding.Empty
        };
        _translateWorld.Width = 105;
        _translateGuild.Width = 105;
        _translateParty.Width = 180;
        translateChannels.Controls.Add(_translateWorld);
        translateChannels.Controls.Add(_translateGuild);
        translateChannels.Controls.Add(_translateParty);

        var translation = MakeSingleColumnTable();
        AddStack(translation, _translateEnabled);
        AddStack(translation, MakeFieldBlock(
            "Translate channels",
            "World is independent. Guild = BPSR Union. Party / Team covers Team and Group chat.",
            translateChannels));
        AddStack(translation, _translateShowOverlay);
        AddPageCard(stack, MakeCard(
            "English translation",
            "The original BPSR message appears immediately. A successful non-English translation is added underneath it later.",
            translation));

        var ttsChannels = new FlowLayoutPanel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            FlowDirection = FlowDirection.LeftToRight,
            WrapContents = false,
            Margin = Padding.Empty
        };
        _ttsGuild.Width = 120;
        _ttsParty.Width = 180;
        ttsChannels.Controls.Add(_ttsGuild);
        ttsChannels.Controls.Add(_ttsParty);

        _ttsOwnUsername.Width = 330;
        _ttsOwnUsername.MaxLength = 128;
        ChatUiTheme.StyleTextBox(_ttsOwnUsername);

        var tts = MakeSingleColumnTable();
        AddStack(tts, _ttsEnabled);
        AddStack(tts, MakeFieldBlock(
            "Read aloud channels",
            "Speech is intentionally limited to Guild and Party / Team. World chat is never read aloud.",
            ttsChannels));
        AddStack(tts, _ttsReadSender);
        AddStack(tts, MakeFieldBlock(
            "My BPSR username — never read my own messages",
            "Exact name match, case-insensitive. Leave empty if you want your own messages read too.",
            _ttsOwnUsername));
        AddStack(tts, MakeSliderRow(
            "TTS volume",
            "Separate from ReadyAlert keyword/Ready Check sound volume.",
            _ttsVolume,
            _ttsVolumeValue,
            source.TtsVolume,
            0));
        AddStack(tts, MakeActionRow(
            "Test Google English TTS",
            "Downloads one short no-key Google English TTS sample and plays it using ReadyAlert's TTS volume.",
            "Test voice",
            () => _ = TestGoogleTtsInteractiveAsync()));
        AddPageCard(stack, MakeCard(
            "Google English TTS speech",
            "Uses Google's no-key English (en) Translate TTS voice. Messages are queued one at a time so speech never overlaps.",
            tts));

        AddPageCard(stack, MakeInfoBanner(
            "No API key — Google Translate web service",
            "This uses undocumented Google Translate/gTTS-style web endpoints, not Google Cloud. Only messages needed for enabled translation/TTS channels are sent to Google. Google can rate-limit or change the service without notice; failures never block the overlay or packet capture.",
            ChatUiTheme.Accent));

        AddPageCard(stack, MakeInfoBanner(
            "Translation used by TTS",
            "For TTS-enabled channels, ReadyAlert asks Google to auto-detect/translate the message to English first. Google English TTS then speaks that English text. If translation fails, the original text is still attempted.",
            ChatUiTheme.Success));

        RefreshSpeechControlState();
        return page;
    }

    private void RefreshSpeechControlState()
    {
        var translate = _translateEnabled.Checked;
        _translateWorld.Enabled = translate;
        _translateGuild.Enabled = translate;
        _translateParty.Enabled = translate;
        _translateShowOverlay.Enabled = translate;

        var tts = _ttsEnabled.Checked;
        _ttsGuild.Enabled = tts;
        _ttsParty.Enabled = tts;
        _ttsReadSender.Enabled = tts;
        _ttsOwnUsername.Enabled = tts;
        _ttsVolume.Enabled = tts;
    }

    private async Task TestGoogleTtsInteractiveAsync()
    {
        try
        {
            await ChatSpeechTranslationEngine.TestTtsAsync(_ttsVolume.Value);
        }
        catch (Exception ex)
        {
            AppLog.Write("tts: interactive test failed " + ex.Message);
            MessageBox.Show(
                this,
                "Google English TTS test failed.\r\n\r\n" + ex.Message +
                "\r\n\r\nThe normal chat overlay is unaffected. Check readyalert.log for details.",
                "TTS test failed",
                MessageBoxButtons.OK,
                MessageBoxIcon.Warning);
        }
    }

    private void ApplySpeechTranslationSettings()
    {
        if (_speechSettings is null) return;
        _speechSettings.TranslationEnabled = _translateEnabled.Checked;
        _speechSettings.TranslationWorld = _translateWorld.Checked;
        _speechSettings.TranslationGuild = _translateGuild.Checked;
        _speechSettings.TranslationPartyTeam = _translateParty.Checked;
        _speechSettings.ShowTranslationInOverlay = _translateShowOverlay.Checked;
        _speechSettings.TtsEnabled = _ttsEnabled.Checked;
        _speechSettings.TtsGuild = _ttsGuild.Checked;
        _speechSettings.TtsPartyTeam = _ttsParty.Checked;
        _speechSettings.ReadSenderName = _ttsReadSender.Checked;
        _speechSettings.IgnoreOwnUsername = _ttsOwnUsername.Text;
        _speechSettings.TtsVolume = _ttsVolume.Value;
        _speechSettings.Normalize();
        ChatSpeechTranslationEngine.Configure(_speechSettings);
    }

    private void LoadSpeechTranslationControls(ChatSpeechTranslationSettings source)
    {
        _translateEnabled.Checked = source.TranslationEnabled;
        _translateWorld.Checked = source.TranslationWorld;
        _translateGuild.Checked = source.TranslationGuild;
        _translateParty.Checked = source.TranslationPartyTeam;
        _translateShowOverlay.Checked = source.ShowTranslationInOverlay;
        _ttsEnabled.Checked = source.TtsEnabled;
        _ttsGuild.Checked = source.TtsGuild;
        _ttsParty.Checked = source.TtsPartyTeam;
        _ttsReadSender.Checked = source.ReadSenderName;
        _ttsOwnUsername.Text = source.IgnoreOwnUsername;

        if (_ttsVolume.Minimum == 0 && _ttsVolume.Maximum == 10)
        {
            _ttsVolume.Minimum = 0;
            _ttsVolume.Maximum = 100;
            _ttsVolume.TickFrequency = 10;
            _ttsVolume.SmallChange = 5;
            _ttsVolume.LargeChange = 10;
        }
        _ttsVolume.Value = Math.Clamp(source.TtsVolume, _ttsVolume.Minimum, _ttsVolume.Maximum);
        _ttsVolumeValue.Text = _ttsVolume.Value + "%";
        _ttsVolume.Scroll -= TtsVolumeScrolled;
        _ttsVolume.Scroll += TtsVolumeScrolled;
        RefreshSpeechControlState();
    }

    private void TtsVolumeScrolled(object? sender, EventArgs e) =>
        _ttsVolumeValue.Text = _ttsVolume.Value + "%";
}
