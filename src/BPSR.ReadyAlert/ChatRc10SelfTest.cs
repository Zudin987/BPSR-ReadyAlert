namespace BPSR.ReadyAlert;

internal static class ChatRc10SelfTest
{
    internal static void Run()
    {
        TestSoundRulesIgnoreSenderName();
    }

    private static void TestSoundRulesIgnoreSenderName()
    {
        var rules = new List<ChatSoundRule>
        {
            new() { Enabled = true, Match = "serum", SoundPath = "serum.wav" }
        };

        var senderOnly = new ChatMessageEvent(
            SenderId: 12345,
            SenderName: "Serum",
            SenderLevel: 80,
            Channel: ChatChannel.World,
            Timestamp: DateTime.Now,
            Kind: ChatMessageKind.Text,
            Text: "hello everyone");

        Assert(
            ChatSoundRuleMatcher.FindFirstMatch(rules, senderOnly) is null,
            "sender name alone must not trigger a sound rule");

        var messageMatch = senderOnly with { SenderName = "SomeoneElse", Text = "Need SERUM for boss" };
        Assert(
            ReferenceEquals(ChatSoundRuleMatcher.FindFirstMatch(rules, messageMatch), rules[0]),
            "message text still triggers sound rule case-insensitively");

        var senderAndMessageMatch = senderOnly with { Text = "serum available" };
        Assert(
            ReferenceEquals(ChatSoundRuleMatcher.FindFirstMatch(rules, senderAndMessageMatch), rules[0]),
            "sender name does not prevent a real message-text match");
    }

    private static void Assert(bool condition, string name)
    {
        if (!condition) throw new InvalidOperationException("RC10 self-test failed: " + name);
    }
}
