using System.Drawing;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal static class ChatV111SelfTest
{
    internal static void Run()
    {
        TestSenderColorsAreStable();
        TestBottomAlignmentMath();
        TestHistoryTrimKeepsFollowingLatest();
        TestOverlayStaysOutOfAltTab();
    }

    private static void TestSenderColorsAreStable()
    {
        var first = new ChatMessageEvent(123456, "Artemis", 80, ChatChannel.World, DateTime.Now, ChatMessageKind.Text, "hello");
        var renamedSameId = first with { SenderName = "DifferentName" };
        Assert(ChatSenderColor.ForMessage(first) == ChatSenderColor.ForMessage(renamedSameId),
            "same sender ID keeps the same username color");

        var noIdA = first with { SenderId = 0, SenderName = "LaPlace" };
        var noIdB = first with { SenderId = 0, SenderName = "laplace" };
        Assert(ChatSenderColor.ForMessage(noIdA) == ChatSenderColor.ForMessage(noIdB),
            "name fallback color is case-insensitive and stable");

        var colors = Enumerable.Range(1, 12)
            .Select(i => ChatSenderColor.ForMessage(first with { SenderId = i }))
            .Distinct()
            .Count();
        Assert(colors >= 7, "sender palette distributes nearby user IDs across several colors");
    }

    private static void TestBottomAlignmentMath()
    {
        using var list = new ListBox
        {
            Size = new Size(320, 120),
            IntegralHeight = false,
            ItemHeight = 20
        };
        for (var i = 0; i < 20; i++) list.Items.Add("row " + i);
        _ = list.Handle;

        var bottomTop = ChatListScrollMath.GetBottomAlignedTopIndex(list);
        Assert(bottomTop > 0 && bottomTop < list.Items.Count,
            "bottom alignment chooses a real top row instead of putting the final row at the top");
        list.TopIndex = bottomTop;
        Assert(ChatListScrollMath.IsAtBottom(list), "bottom-aligned viewport is recognized as following latest");
    }

    private static void TestHistoryTrimKeepsFollowingLatest()
    {
        var settings = new AppSettings { ChatOverlayEnabled = true };
        settings.Chat.Normalize();
        var tempPath = Path.Combine(Path.GetTempPath(), $"BPSR-ReadyAlert-v111-{Guid.NewGuid():N}.json");

        try
        {
            using var form = new ChatOverlayForm(settings, new SettingsStore(tempPath), string.Empty, string.Empty);
            form.Size = new Size(600, 300);
            _ = form.Handle;
            form.CreateControl();
            Assert(form.RunV111HistoryTrimSelfTest(),
                "history-cap trimming preserves follow-latest instead of jumping to old chat");
            var state = form.GetV111ScrollStateForSelfTest();
            Assert(state.FollowLatest && state.AtBottom,
                "overlay remains at latest chat after the oldest row is evicted");
        }
        finally
        {
            TryDelete(tempPath);
            TryDelete(tempPath + ".bak");
            TryDelete(tempPath + ".new");
        }
    }

    private static void TestOverlayStaysOutOfAltTab()
    {
        var settings = new AppSettings { ChatOverlayEnabled = true };
        settings.Chat.Normalize();
        var tempPath = Path.Combine(Path.GetTempPath(), $"BPSR-ReadyAlert-alttab-{Guid.NewGuid():N}.json");

        try
        {
            using var form = new ChatOverlayForm(settings, new SettingsStore(tempPath), string.Empty, string.Empty);
            var styles = form.GetAltTabWindowStylesForSelfTest();
            Assert(styles.ToolWindow, "chat overlay uses WS_EX_TOOLWINDOW so Windows excludes it from Alt+Tab");
            Assert(!styles.AppWindow, "chat overlay does not force WS_EX_APPWINDOW");
            Assert(!form.ShowInTaskbar, "chat overlay remains hidden from the taskbar");
        }
        finally
        {
            TryDelete(tempPath);
            TryDelete(tempPath + ".bak");
            TryDelete(tempPath + ".new");
        }
    }

    private static void TryDelete(string path)
    {
        try
        {
            if (File.Exists(path)) File.Delete(path);
        }
        catch { }
    }

    private static void Assert(bool condition, string name)
    {
        if (!condition) throw new InvalidOperationException("Chat v1.1.1 self-test failed: " + name);
    }
}
