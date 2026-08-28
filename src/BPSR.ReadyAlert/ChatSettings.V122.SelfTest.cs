namespace BPSR.ReadyAlert;

internal sealed partial class ChatGeneralSettingsForm
{
    internal int GetV122MaxBoundedInputWidthForSelfTest() =>
        new[]
        {
            _fontFamily.Width,
            _clickHotkey.Width,
            _collapseHotkey.Width,
            _ttsOwnUsername.Width
        }.Max();
}
