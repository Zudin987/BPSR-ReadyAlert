using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed partial class ChatGeneralSettingsForm
{
    // One compact cleanup switch replaces the two overlapping content-cleanup
    // options. It drives both legacy settings fields for backward-compatible JSON.
    private readonly CheckBox _hideRichNoise = new() { Text = "Hide emoji-only + linked items / Hypertext" };
    private bool _v120ContentFiltersInstalled;

    private void InstallV120ContentFilters()
    {
        if (_v120ContentFiltersInstalled) return;
        _v120ContentFiltersInstalled = true;

        ChatUiTheme.StyleSettingsCheckBox(_hideRichNoise);
        MakeV124CheckboxInteractive(_hideRichNoise);

        if (_hideStickers.Parent is not FlowLayoutPanel behavior) return;

        behavior.Controls.Add(_hideRichNoise);
        var stickerIndex = behavior.Controls.GetChildIndex(_hideStickers);
        behavior.Controls.SetChildIndex(_hideRichNoise, Math.Min(stickerIndex + 1, behavior.Controls.Count - 1));
    }
}
