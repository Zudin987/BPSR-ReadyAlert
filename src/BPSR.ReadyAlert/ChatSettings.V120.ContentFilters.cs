using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed partial class ChatGeneralSettingsForm
{
    private readonly CheckBox _hideEmoji = new() { Text = "Hide emoji-only messages" };
    private readonly CheckBox _hideLinkedItems = new() { Text = "Hide linked items / Hypertext" };
    private bool _v120ContentFiltersInstalled;

    private void InstallV120ContentFilters()
    {
        if (_v120ContentFiltersInstalled) return;
        _v120ContentFiltersInstalled = true;

        ChatUiTheme.StyleCheckBox(_hideEmoji);
        ChatUiTheme.StyleCheckBox(_hideLinkedItems);

        if (_hideStickers.Parent is not FlowLayoutPanel behavior) return;

        behavior.Controls.Add(_hideEmoji);
        behavior.Controls.Add(_hideLinkedItems);
        var stickerIndex = behavior.Controls.GetChildIndex(_hideStickers);
        behavior.Controls.SetChildIndex(_hideEmoji, Math.Min(stickerIndex + 1, behavior.Controls.Count - 1));
        behavior.Controls.SetChildIndex(_hideLinkedItems, Math.Min(stickerIndex + 2, behavior.Controls.Count - 1));
    }
}
