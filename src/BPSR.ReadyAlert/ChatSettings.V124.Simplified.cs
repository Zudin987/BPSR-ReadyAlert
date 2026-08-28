namespace BPSR.ReadyAlert;

internal sealed partial class ChatGeneralSettingsForm
{
    private const int V124SoundRuleCount = 2;

    private const int V124FixedBackgroundOpacity = 82;
    private const int V124FixedToolbarOpacity = 92;
    private const int V124FixedTextOpacity = 100;

    internal (
        bool OnlyWindowOpacity,
        bool CombinedCleanup,
        bool ClickThroughCheckboxRemoved,
        bool CollapseHotkeyRemoved,
        bool HighlightSingleLine,
        bool TwoSoundRulesOnly,
        bool SoundRulesSingleLine)
        GetV124SimplifiedUiForSelfTest()
    {
        return (
            _windowOpacity.Parent is not null &&
            _backgroundOpacity.Parent is null &&
            _toolbarOpacity.Parent is null &&
            _textOpacity.Parent is null,
            _hideRichNoise.Parent is not null,
            _clickThrough.Parent is null,
            _collapseHotkey.Parent is null,
            !_highlight.Multiline,
            _soundRuleEnabled[0].Parent is not null &&
            _soundRuleEnabled[1].Parent is not null &&
            _soundRuleEnabled[2].Parent is null,
            !_soundRuleMatch[0].Multiline && !_soundRuleMatch[1].Multiline);
    }
}
