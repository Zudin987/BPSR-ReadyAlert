using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal static class ChatRc9SelfTest
{
    internal static void Run()
    {
        Environment.ExitCode = 91;
        TestLegacyRc8SoundMigration();
        Environment.ExitCode = 92;
        TestThreeRuleCapAndPriority();
        Environment.ExitCode = 93;
        TestSoundRuleUi();
        Environment.ExitCode = 0;
    }

    private static void TestLegacyRc8SoundMigration()
    {
        var settings = new ChatOverlaySettings
        {
            HighlightIfMatches = "PA",
            HighlightSoundEnabled = true,
            HighlightSoundPath = "boss.wav"
        };
        settings.Normalize();

        Assert(settings.HighlightSoundRules.Count == 1, "RC8 single sound migrates to rule 1");
        Assert(settings.HighlightSoundRules[0].Enabled, "migrated rule remains enabled");
        Assert(settings.HighlightSoundRules[0].Match == "PA", "migrated rule keeps match text");
        Assert(settings.HighlightSoundRules[0].SoundPath == "boss.wav", "migrated rule keeps sound path");
        Assert(!settings.HighlightSoundEnabled && settings.HighlightSoundPath.Length == 0, "legacy sound fields clear after migration");
    }

    private static void TestThreeRuleCapAndPriority()
    {
        var settings = new ChatOverlaySettings
        {
            HighlightSoundRules =
            [
                new() { Enabled = true, Match = "PA", SoundPath = "a.wav" },
                new() { Enabled = true, Match = "raid", SoundPath = "b.wav" },
                new() { Enabled = true, Match = "serum", SoundPath = "c.wav" },
                new() { Enabled = true, Match = "food", SoundPath = "d.wav" }
            ]
        };
        settings.Normalize();
        Assert(settings.HighlightSoundRules.Count == 3, "sound rules are capped at three");

        var first = ChatSoundRuleMatcher.FindFirstMatch(settings.HighlightSoundRules, "PA raid serum");
        Assert(ReferenceEquals(first, settings.HighlightSoundRules[0]), "first matching sound rule wins");

        // Avoid words such as "party" here because simple PA intentionally uses
        // normal substring/regex matching and would correctly match the "pa" in it.
        var second = ChatSoundRuleMatcher.FindFirstMatch(settings.HighlightSoundRules, "need RAID group");
        Assert(ReferenceEquals(second, settings.HighlightSoundRules[1]), "later rule matches when earlier rule does not");

        settings.HighlightSoundRules[0].Enabled = false;
        first = ChatSoundRuleMatcher.FindFirstMatch(settings.HighlightSoundRules, "PA raid");
        Assert(ReferenceEquals(first, settings.HighlightSoundRules[1]), "disabled higher-priority rule is skipped");

        var blank = new ChatOverlaySettings
        {
            HighlightSoundRules = [new() { Enabled = true, Match = "   ", SoundPath = "a.wav" }]
        };
        blank.Normalize();
        Assert(!blank.HighlightSoundRules[0].Enabled, "blank enabled rule is normalized off to prevent match-all sound");
    }

    private static void TestSoundRuleUi()
    {
        var settings = new ChatOverlaySettings
        {
            HighlightSoundRules =
            [
                new() { Enabled = true, Match = "PA", SoundPath = string.Empty },
                new() { Enabled = true, Match = "raid", SoundPath = string.Empty }
            ]
        };
        settings.Normalize();

        using var form = new ChatGeneralSettingsForm(settings);
        _ = form.Handle;
        form.CreateControl();
        PerformLayoutTree(form);

        for (var i = 1; i <= 3; i++)
            Assert(FindCheckBox(form, $"Enable sound rule {i}") is not null, $"sound rule {i} editor exists");

        Assert(FindControlText(form, "Cooldown") is null, "no cooldown UI is exposed");
        Assert(FindControlText(form, "Chat alert volume") is not null, "one shared chat alert volume is present");
    }

    private static CheckBox? FindCheckBox(Control parent, string text)
    {
        foreach (Control child in parent.Controls)
        {
            if (child is CheckBox box && string.Equals(box.Text, text, StringComparison.Ordinal)) return box;
            var nested = FindCheckBox(child, text);
            if (nested is not null) return nested;
        }
        return null;
    }

    private static Control? FindControlText(Control parent, string text)
    {
        foreach (Control child in parent.Controls)
        {
            if (child.Text.Contains(text, StringComparison.OrdinalIgnoreCase)) return child;
            var nested = FindControlText(child, text);
            if (nested is not null) return nested;
        }
        return null;
    }

    private static void PerformLayoutTree(Control parent)
    {
        parent.PerformLayout();
        foreach (Control child in parent.Controls) PerformLayoutTree(child);
    }

    private static void Assert(bool condition, string name)
    {
        if (!condition) throw new InvalidOperationException("RC9 self-test failed: " + name);
    }
}
