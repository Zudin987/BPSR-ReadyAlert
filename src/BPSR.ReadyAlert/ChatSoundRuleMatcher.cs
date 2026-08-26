namespace BPSR.ReadyAlert;

internal static class ChatSoundRuleMatcher
{
    internal static ChatSoundRule? FindFirstMatch(IReadOnlyList<ChatSoundRule> rules, string searchable)
    {
        var count = Math.Min(rules.Count, 3);
        for (var i = 0; i < count; i++)
        {
            var rule = rules[i];
            if (!rule.Enabled || string.IsNullOrWhiteSpace(rule.Match)) continue;
            if (ChatFilterExpression.IsMatch(searchable, rule.Match)) return rule;
        }
        return null;
    }
}
