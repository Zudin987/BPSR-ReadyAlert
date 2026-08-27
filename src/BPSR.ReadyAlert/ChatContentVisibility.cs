using System.Text.RegularExpressions;

namespace BPSR.ReadyAlert;

internal static class ChatContentVisibility
{
    private static readonly Regex SpriteOnlyRegex = new(
        @"^(?:\s*<sprite=(?:100|[1-9][0-9]?)>\s*)+$",
        RegexOptions.IgnoreCase | RegexOptions.CultureInvariant | RegexOptions.Compiled,
        TimeSpan.FromMilliseconds(50));

    private static volatile bool _hideEmoji;
    private static volatile bool _hideLinkedItems;

    internal static void Configure(bool hideEmoji, bool hideLinkedItems)
    {
        _hideEmoji = hideEmoji;
        _hideLinkedItems = hideLinkedItems;
    }

    internal static bool ShouldHideInOverlay(ChatMessageEvent message)
    {
        if (_hideEmoji && IsSpriteOnlyEmoji(message.Text)) return true;
        if (_hideLinkedItems && IsLinkedItem(message)) return true;
        return false;
    }

    internal static bool ShouldSkipSpeech(ChatMessageEvent message) =>
        IsSpriteOnlyEmoji(message.Text) || IsLinkedItem(message);

    internal static bool IsSpriteOnlyEmoji(string? text)
    {
        if (string.IsNullOrWhiteSpace(text)) return false;
        try { return SpriteOnlyRegex.IsMatch(text); }
        catch (RegexMatchTimeoutException) { return false; }
    }

    internal static bool IsLinkedItem(ChatMessageEvent message) =>
        message.Kind == ChatMessageKind.Hypertext ||
        (!string.IsNullOrWhiteSpace(message.Text) &&
         message.Text.Contains("Hypertext", StringComparison.OrdinalIgnoreCase));
}
