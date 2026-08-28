namespace BPSR.ReadyAlert;

internal sealed partial class ChatGeneralSettingsForm
{
    // v1.2.4 intentionally exposes only two prioritized keyword sound rules.
    private const int V124SoundRuleCount = 2;

    // The three old internal opacity values remain in settings JSON for backward
    // compatibility but are no longer user-configurable. Keep one stable rendering
    // preset and expose only the real whole-window opacity control.
    private const int V124FixedBackgroundOpacity = 82;
    private const int V124FixedToolbarOpacity = 92;
    private const int V124FixedTextOpacity = 100;
}
