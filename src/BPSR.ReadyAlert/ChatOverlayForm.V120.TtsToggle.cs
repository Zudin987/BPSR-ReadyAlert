using System.Drawing;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed partial class ChatOverlayForm
{
    private const int V120TtsToolbarWidth = 52;
    private Button? _v120TtsToggleButton;
    private Font? _v120TtsOnFont;
    private Font? _v120TtsOffFont;

    private void EnsureV120TtsToolbarButton()
    {
        if (_v120TtsToggleButton is null)
        {
            _v120TtsOnFont = ChatUiTheme.UiFont(8.5F, FontStyle.Bold);
            _v120TtsOffFont = ChatUiTheme.UiFont(8.5F, FontStyle.Bold | FontStyle.Strikeout);
            _v120TtsToggleButton = MakeToolbarButton("TTS", V120TtsToolbarWidth, "Toggle chat text-to-speech");
            _v120TtsToggleButton.AccessibleName = "Chat text-to-speech toggle";
            _v120TtsToggleButton.Click += (_, _) => ToggleV120TtsFromToolbar();

            // The original action bar was sized exactly for +Tab, Settings,
            // Collapse and Hide. Reserve one more fixed toolbar slot and insert it
            // directly between +Tab and Settings, matching the user's requested
            // order without changing the tab strip itself.
            _actionBar.Width = Math.Max(_actionBar.Width, 184 + V120TtsToolbarWidth);
            _actionBar.Controls.Add(_v120TtsToggleButton);
            _actionBar.Controls.SetChildIndex(_v120TtsToggleButton, 1);

            Disposed += (_, _) =>
            {
                _v120TtsOnFont?.Dispose();
                _v120TtsOffFont?.Dispose();
                _v120TtsOnFont = null;
                _v120TtsOffFont = null;
            };
        }

        UpdateV120TtsToolbarButton();
    }

    private void ToggleV120TtsFromToolbar()
    {
        _settings.SpeechTranslation.TtsEnabled = !_settings.SpeechTranslation.TtsEnabled;
        _settings.SpeechTranslation.Normalize();

        // Make the toolbar action effective immediately; disk I/O should never
        // delay stopping/starting speech. Persist afterward and tell the user if the
        // preference is session-only rather than silently pretending it was saved.
        ChatSpeechTranslationEngine.Configure(_settings.SpeechTranslation, _v120TranslationQueue);
        UpdateV120TtsToolbarButton();
        AppLog.Write("tts: toolbar toggle " + (_settings.SpeechTranslation.TtsEnabled ? "on" : "off"));

        if (_settingsStore.Save(_settings)) return;

        AppLog.Write("tts: toolbar toggle applied for session but settings save failed");
        MessageBox.Show(
            this,
            "TTS was changed for this ReadyAlert session, but Windows could not save the preference to disk.\r\n\r\n" +
            "It may revert after ReadyAlert restarts. Check folder permissions or disk availability.",
            "TTS preference was not saved",
            MessageBoxButtons.OK,
            MessageBoxIcon.Warning);
    }

    private void UpdateV120TtsToolbarButton()
    {
        if (_v120TtsToggleButton is null) return;

        var speech = _settings.SpeechTranslation;
        var enabled = speech.TtsEnabled;
        var muted = enabled && speech.TtsVolume <= 0;
        var noChannels = enabled && !speech.TtsGuild && !speech.TtsPartyTeam;
        var inactive = muted || noChannels;

        _v120TtsToggleButton.Text = "TTS";
        if (_v120TtsOnFont is not null && _v120TtsOffFont is not null)
            _v120TtsToggleButton.Font = enabled ? _v120TtsOnFont : _v120TtsOffFont;
        _v120TtsToggleButton.ForeColor = Color.White;

        if (!enabled)
        {
            _v120TtsToggleButton.BackColor = Color.FromArgb(112, 47, 52);
            _v120TtsToggleButton.FlatAppearance.MouseOverBackColor = Color.FromArgb(137, 58, 64);
            _v120TtsToggleButton.FlatAppearance.MouseDownBackColor = Color.FromArgb(91, 38, 43);
        }
        else if (inactive)
        {
            // Amber means the master switch is ON but no speech can currently be
            // heard. This avoids the misleading green state when volume is 0% or
            // both allowed TTS channels are deselected.
            _v120TtsToggleButton.BackColor = Color.FromArgb(126, 83, 31);
            _v120TtsToggleButton.FlatAppearance.MouseOverBackColor = Color.FromArgb(151, 100, 37);
            _v120TtsToggleButton.FlatAppearance.MouseDownBackColor = Color.FromArgb(101, 66, 25);
        }
        else
        {
            _v120TtsToggleButton.BackColor = Color.FromArgb(38, 105, 70);
            _v120TtsToggleButton.FlatAppearance.MouseOverBackColor = Color.FromArgb(46, 126, 83);
            _v120TtsToggleButton.FlatAppearance.MouseDownBackColor = Color.FromArgb(31, 86, 58);
        }

        _v120TtsToggleButton.FlatAppearance.BorderSize = 0;

        var description = !enabled
            ? "TTS is off. Press to turn text-to-speech on."
            : muted
                ? "TTS is on but muted at 0%. Raise TTS volume in Speech & translation settings."
                : noChannels
                    ? "TTS is on but no Guild or Party / Team channel is selected."
                    : "TTS is on. Press to turn text-to-speech off.";
        _v120TtsToggleButton.AccessibleDescription = description;
        _toolTip.SetToolTip(_v120TtsToggleButton, description);
        _v120TtsToggleButton.Invalidate();
    }

    internal (bool Enabled, bool Strikeout, Color Background, int ActionIndex) GetV120TtsToolbarStateForSelfTest()
    {
        EnsureV120TtsToolbarButton();
        return (
            _settings.SpeechTranslation.TtsEnabled,
            (_v120TtsToggleButton!.Font.Style & FontStyle.Strikeout) != 0,
            _v120TtsToggleButton.BackColor,
            _actionBar.Controls.GetChildIndex(_v120TtsToggleButton));
    }

    internal void ToggleV120TtsForSelfTest() => ToggleV120TtsFromToolbar();
}
