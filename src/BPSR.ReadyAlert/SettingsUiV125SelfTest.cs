using System.Drawing;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal static class SettingsUiV125SelfTest
{
    internal static void Run()
    {
        TestOverlayTabStripStaysScrollbarFree();
        TestTabNamesStayFullUntilSpaceRunsOut();
        TestLiteralTabLabelsStayCompleteWhenRoomy();
        TestMessageSeparatorReachesScrollbarEdge();
        TestSettingsPolishContract();
    }

    private static void TestOverlayTabStripStaysScrollbarFree()
    {
        var settings = new AppSettings { ChatOverlayEnabled = true };
        settings.Chat.Normalize();
        settings.Chat.Tabs.Add(new ChatTabSettings
        {
            Name = "Recruitment",
            MinLevel = 1,
            Channels = [(int)ChatChannel.World]
        });
        settings.Chat.Tabs.Add(new ChatTabSettings
        {
            Name = "Very Long Custom Tab",
            MinLevel = 1,
            Channels = [(int)ChatChannel.Team]
        });
        settings.Chat.LastSelectedTabId = settings.Chat.Tabs[^1].Id;

        var path = Path.Combine(Path.GetTempPath(), $"BPSR-ReadyAlert-v125-tabs-{Guid.NewGuid():N}.json");
        try
        {
            using var form = new ChatOverlayForm(settings, new SettingsStore(path), string.Empty, string.Empty)
            {
                ShowInTaskbar = false,
                Opacity = 0d,
                Size = new Size(590, 430)
            };
            form.Show();
            Application.DoEvents();
            form.RebuildV125TabBarForSelfTest();
            Application.DoEvents();

            var metrics = form.GetV125TabStripMetricsForSelfTest();
            Check(170, !metrics.AutoScroll,
                "overlay tab strip does not enable the native horizontal scrollbar");
            Check(171, metrics.TabCount == settings.Chat.Tabs.Count,
                "all configured chat tabs remain present after compact fitting");
            Check(172, metrics.AllFit && metrics.OuterWidth <= metrics.AvailableWidth,
                "tab buttons fit inside the visible tab strip without requiring a resize recovery");
        }
        finally
        {
            TryDelete(path);
            TryDelete(path + ".bak");
        }
    }

    private static void TestTabNamesStayFullUntilSpaceRunsOut()
    {
        var natural = new[] { 78, 112, 82, 236 };

        var roomy = ChatOverlayForm.FitV126TabWidthsForSelfTest(natural, 620);
        Check(181, roomy.SequenceEqual(natural),
            "tab names keep their complete natural widths when the row has enough room");

        var tight = ChatOverlayForm.FitV126TabWidthsForSelfTest(natural, 410);
        Check(182, tight.Sum() <= 410,
            "tab fitting still stays inside the available row when compaction is actually required");
        Check(183, tight[0] == natural[0] && tight[1] == natural[1] && tight[2] == natural[2],
            "short tab names remain full while only the genuinely long tab is shortened first");
        Check(184, tight[3] < natural[3],
            "ellipsis pressure is reserved for a tab that cannot fit at its complete natural width");
    }

    private static void TestLiteralTabLabelsStayCompleteWhenRoomy()
    {
        var settings = new AppSettings { ChatOverlayEnabled = true };
        settings.Chat.Normalize();
        settings.Chat.Tabs.Clear();
        settings.Chat.Tabs.Add(new ChatTabSettings
        {
            Name = "General",
            MinLevel = 1,
            Channels = [(int)ChatChannel.World]
        });
        settings.Chat.Tabs.Add(new ChatTabSettings
        {
            Name = "Guild&Team",
            MinLevel = 1,
            Channels = [(int)ChatChannel.Union, (int)ChatChannel.Team, (int)ChatChannel.Group]
        });
        settings.Chat.Tabs.Add(new ChatTabSettings
        {
            Name = "123456789",
            MinLevel = 1,
            Channels = [(int)ChatChannel.Local]
        });
        settings.Chat.LastSelectedTabId = settings.Chat.Tabs[0].Id;

        var path = Path.Combine(Path.GetTempPath(), $"BPSR-ReadyAlert-v127-tabs-{Guid.NewGuid():N}.json");
        try
        {
            using var form = new ChatOverlayForm(settings, new SettingsStore(path), string.Empty, string.Empty)
            {
                ShowInTaskbar = false,
                Opacity = 0d,
                Size = new Size(760, 430)
            };
            form.Show();
            Application.DoEvents();
            form.RebuildV125TabBarForSelfTest();
            Application.DoEvents();

            var tabs = form.GetV127TabButtonMetricsForSelfTest();
            Check(188, tabs.Select(x => x.Text).SequenceEqual(new[] { "General", "Guild&Team", "123456789" }),
                "tab controls retain the exact configured text including ampersands and final characters");
            Check(189, tabs.All(x => !x.UseMnemonic),
                "chat tab labels render ampersands literally instead of consuming them as keyboard mnemonics");
            Check(190, tabs.All(x => !x.AutoEllipsis),
                "roomy overlay does not enable ellipsis for General, Guild&Team, or 123456789");
            Check(191, tabs.All(x => x.Width >= x.NaturalWidth),
                "roomy tab buttons receive enough renderer-safe width for their complete final glyph");
        }
        finally
        {
            TryDelete(path);
            TryDelete(path + ".bak");
        }
    }

    private static void TestMessageSeparatorReachesScrollbarEdge()
    {
        var bounds = new Rectangle(0, 0, 700, 48);
        var right = ChatOverlayForm.GetV125MessageSeparatorRightForSelfTest(bounds);

        Check(179, right == bounds.Right - 2,
            "message divider uses the full ListBox row width rather than subtracting the overlaid custom scrollbar twice");
        Check(180, bounds.Right - right <= 2,
            "message divider finishes at the visible scrollbar edge without the old empty gap");
    }

    private static void TestSettingsPolishContract()
    {
        var chat = new ChatOverlaySettings();
        chat.Normalize();
        var speech = new ChatSpeechTranslationSettings();
        speech.Normalize();

        using var form = new ChatGeneralSettingsForm(chat, speech)
        {
            ShowInTaskbar = false,
            Opacity = 0d
        };
        form.Show();
        Application.DoEvents();
        form.ShowV122PageForSelfTest("Alerts");
        form.PerformLayout();
        Application.DoEvents();

        Check(173,
            string.Equals(form.GetV125CleanupLabelForSelfTest(), "Hide emoji + linked items / Hypertext", StringComparison.Ordinal),
            "cleanup option removes the extra 'only' wording");

        var layout = form.GetV125AlertInputMetricsForSelfTest();
        Check(174, layout.HighlightFluid && layout.Rule1Fluid && layout.Rule2Fluid && layout.SingleLine,
            "Alert highlight and sound-rule match inputs stay single-line and use fluid width");

        var widths = form.GetV125AlertInputWidthsForSelfTest();
        Check(175, widths.Highlight > 300 && widths.Rule1 > 300 && widths.Rule2 > 300,
            "Alert rule inputs are no longer visually capped at the old 300 px width");
        Check(176, Math.Abs(widths.Highlight - widths.Rule1) <= 2 && Math.Abs(widths.Rule1 - widths.Rule2) <= 2,
            "Alert rule input boxes align to the same width");
        Check(177, widths.AlertsPageWidth > widths.Highlight,
            "aligned Alert inputs remain inside the page client area");

        Check(178, form.AreV125SettingsScrollbarsDarkThemedForSelfTest(),
            "all realized Settings pages request the Windows dark scrollbar theme");
    }

    private static void TryDelete(string path)
    {
        try
        {
            if (File.Exists(path)) File.Delete(path);
        }
        catch { }
    }

    private static void Check(int code, bool condition, string name)
    {
        if (condition) return;
        Environment.ExitCode = code;
        throw new InvalidOperationException("v1.2.5+ UI self-test failed: " + name);
    }
}
