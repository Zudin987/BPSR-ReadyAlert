using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed partial class ChatGeneralSettingsForm
{
    private readonly CheckBox _hideEmoji = new() { Text = "Hide emoji-only messages (<sprite=1> … <sprite=100>)" };
    private readonly CheckBox _hideLinkedItems = new() { Text = "Hide linked-item / Hypertext messages" };
    private bool _v120ContentFiltersInstalled;

    protected override void OnShown(EventArgs e)
    {
        base.OnShown(e);
        InstallV120ContentFilters();
    }

    private void InstallV120ContentFilters()
    {
        if (_v120ContentFiltersInstalled) return;
        _v120ContentFiltersInstalled = true;

        var speech = _speechSettings ?? new ChatSpeechTranslationSettings();
        _hideEmoji.Checked = speech.HideEmojiMessages;
        _hideLinkedItems.Checked = speech.HideLinkedItemMessages;
        ChatUiTheme.StyleCheckBox(_hideEmoji);
        ChatUiTheme.StyleCheckBox(_hideLinkedItems);

        _hideEmoji.CheckedChanged += (_, _) =>
        {
            if (_speechSettings is not null)
                _speechSettings.HideEmojiMessages = _hideEmoji.Checked;
        };
        _hideLinkedItems.CheckedChanged += (_, _) =>
        {
            if (_speechSettings is not null)
                _speechSettings.HideLinkedItemMessages = _hideLinkedItems.Checked;
        };

        if (_hideStickers.Parent is FlowLayoutPanel behavior)
        {
            behavior.Controls.Add(_hideEmoji);
            behavior.Controls.Add(_hideLinkedItems);
            var stickerIndex = behavior.Controls.GetChildIndex(_hideStickers);
            behavior.Controls.SetChildIndex(_hideEmoji, Math.Min(stickerIndex + 1, behavior.Controls.Count - 1));
            behavior.Controls.SetChildIndex(_hideLinkedItems, Math.Min(stickerIndex + 2, behavior.Controls.Count - 1));
        }

        var reset = FindButtonByText(this, "Reset to defaults");
        if (reset is not null)
        {
            // The existing Reset handler asks for confirmation and writes this status
            // only after the user accepts. Clear it before Click so cancelling Reset
            // cannot accidentally reset these v1.2-only controls.
            reset.MouseDown += (_, _) => _applyStatus.Text = string.Empty;
            reset.Click += (_, _) =>
            {
                if (!string.Equals(_applyStatus.Text, "Defaults restored ✓", StringComparison.Ordinal))
                    return;

                var defaults = new ChatSpeechTranslationSettings();
                _hideEmoji.Checked = defaults.HideEmojiMessages;
                _hideLinkedItems.Checked = defaults.HideLinkedItemMessages;
                if (_speechSettings is not null)
                {
                    _speechSettings.HideEmojiMessages = defaults.HideEmojiMessages;
                    _speechSettings.HideLinkedItemMessages = defaults.HideLinkedItemMessages;
                    _speechSettings.Normalize();
                }
                if (Owner is ChatOverlayForm overlay)
                    overlay.ApplyV120SpeechSettingsFromOpenDialog();
            };
        }
    }

    private static Button? FindButtonByText(Control root, string text)
    {
        foreach (Control child in root.Controls)
        {
            if (child is Button button && string.Equals(button.Text, text, StringComparison.Ordinal))
                return button;
            var nested = FindButtonByText(child, text);
            if (nested is not null) return nested;
        }
        return null;
    }
}
