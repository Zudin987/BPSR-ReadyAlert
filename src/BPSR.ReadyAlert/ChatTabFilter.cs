namespace BPSR.ReadyAlert;

/// <summary>
/// Applies the user-facing Show/Hide rules to the same searchable content the
/// overlay presents. Rules intentionally have no minimum character length: a
/// one- or two-character boss abbreviation such as "PA" is a valid filter.
/// </summary>
internal static class ChatTabFilter
{
    internal static bool PassesTextRules(ChatMessageEvent message, ChatTabSettings tab)
    {
        var searchable = SearchableText(message);

        if (!string.IsNullOrWhiteSpace(tab.ShowIfMatches) &&
            !ChatFilterExpression.IsMatch(searchable, tab.ShowIfMatches))
            return false;

        if (!string.IsNullOrWhiteSpace(tab.HideIfMatches) &&
            ChatFilterExpression.IsMatch(searchable, tab.HideIfMatches))
            return false;

        return true;
    }

    internal static string SearchableText(ChatMessageEvent message)
    {
        var sender = !string.IsNullOrWhiteSpace(message.SenderName)
            ? message.SenderName
            : message.SenderId != 0
                ? message.SenderId.ToString()
                : "System";

        return sender + "\n" + (message.Text ?? string.Empty);
    }
}
