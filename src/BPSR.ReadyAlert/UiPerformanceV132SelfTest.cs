using System.Diagnostics;
using System.Drawing;
using System.Reflection;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

/// <summary>
/// Release-level UI timing checks executed from the published single-file EXE.
/// The limits are deliberately broad hosted-CI regression gates, while the emitted
/// metrics give the final audit concrete startup/navigation/render numbers.
/// </summary>
internal static class UiPerformanceV132SelfTest
{
    private const int ChatMessageCount = 200;
    private const int ChatTabSwitchCount = 40;
    private const int RepaintCount = 60;

    internal static void Run()
    {
        var metrics = new List<string>
        {
            "BPSR ReadyAlert v1.3.2 UI performance audit",
            $"os={Environment.OSVersion}",
            $"framework={Environment.Version}",
            $"processArch={System.Runtime.InteropServices.RuntimeInformation.ProcessArchitecture}"
        };
        var metricsPath = Path.Combine(AppContext.BaseDirectory, "ui-performance-v132.txt");
        var settingsPath = Path.Combine(Path.GetTempPath(), $"BPSR-ReadyAlert-v132-ui-{Guid.NewGuid():N}.json");

        try
        {
            TestOverlayStartupNavigationAndPaint(settingsPath, metrics);
            TestSettingsNavigation(metrics);
            TestCoreAudioPreload(metrics);
            metrics.Add("result=PASS");
        }
        catch (Exception ex)
        {
            metrics.Add("result=FAIL");
            metrics.Add("failure=" + ex.Message.Replace(Environment.NewLine, " | ", StringComparison.Ordinal));
            throw;
        }
        finally
        {
            try { File.WriteAllLines(metricsPath, metrics); } catch { }
            TryDelete(settingsPath);
            TryDelete(settingsPath + ".bak");
            TryDelete(settingsPath + ".new");
        }
    }

    private static void TestOverlayStartupNavigationAndPaint(string settingsPath, List<string> metrics)
    {
        var settings = new AppSettings { ChatOverlayEnabled = true };
        settings.Chat.MaxHistory = ChatMessageCount;
        settings.Chat.Normalize();

        // Keep the benchmark entirely local: translation/TTS background behavior is
        // covered separately and no network request should be part of a UI timing gate.
        settings.SpeechTranslation.TranslationEnabled = false;
        settings.SpeechTranslation.TtsEnabled = false;
        settings.SpeechTranslation.Normalize();
        ChatCaptureBridge.Enabled = false;

        var store = new SettingsStore(settingsPath);
        var timer = Stopwatch.StartNew();
        using var form = new ChatOverlayForm(settings, store, string.Empty, string.Empty)
        {
            ShowInTaskbar = false
        };
        var constructMs = timer.Elapsed.TotalMilliseconds;
        Check(191, constructMs < 1_500,
            $"overlay construction exceeded 1500 ms ({constructMs:F1} ms)");

        timer.Restart();
        form.ShowOverlay();
        Application.DoEvents();
        var showMs = timer.Elapsed.TotalMilliseconds;
        Check(192, showMs < 1_500,
            $"overlay first show exceeded 1500 ms ({showMs:F1} ms)");

        // Do not let the 350 ms idle Settings prewarm contaminate the chat-navigation
        // timings below. Settings construction/prewarm has its own focused measurement.
        form.StopV132SettingsPrewarmForSelfTest();

        var channels = new[]
        {
            ChatChannel.World,
            ChatChannel.Local,
            ChatChannel.Team,
            ChatChannel.Union,
            ChatChannel.Private,
            ChatChannel.Group,
            ChatChannel.Newbie
        };

        timer.Restart();
        for (var i = 0; i < ChatMessageCount; i++)
        {
            form.AddMessage(new ChatMessageEvent(
                10_000 + i,
                "PerfUser" + i,
                80,
                channels[i % channels.Length],
                DateTime.Now.AddSeconds(-i),
                ChatMessageKind.Text,
                "Representative BPSR chat row for ReadyAlert UI performance testing " + i,
                i + 1));
        }
        Application.DoEvents();
        var ingestMs = timer.Elapsed.TotalMilliseconds;
        Check(193, ingestMs < 2_500,
            $"200-message steady UI ingestion exceeded 2500 ms ({ingestMs:F1} ms)");
        Check(194, form.V132VisibleMessageCountForSelfTest > 0,
            "overlay performance fixture produced no visible rows");

        // Warm the per-channel cache before allocation measurement.
        foreach (var channel in channels)
            _ = form.GetV132ChannelColorForSelfTest(channel);
        var allocationBefore = GC.GetAllocatedBytesForCurrentThread();
        for (var i = 0; i < 10_000; i++)
            _ = form.GetV132ChannelColorForSelfTest(channels[i % channels.Length]);
        var colorCacheAllocatedBytes = GC.GetAllocatedBytesForCurrentThread() - allocationBefore;
        Check(195, colorCacheAllocatedBytes < 32 * 1024,
            $"cached channel-color lookup allocated {colorCacheAllocatedBytes} bytes for 10k lookups");

        // Verify cache invalidation follows a live user color edit rather than keeping
        // stale render output forever.
        settings.Chat.ChannelColors[(int)ChatChannel.World] = "#010203";
        var firstCustom = form.GetV132ChannelColorForSelfTest(ChatChannel.World);
        settings.Chat.ChannelColors[(int)ChatChannel.World] = "#040506";
        var secondCustom = form.GetV132ChannelColorForSelfTest(ChatChannel.World);
        Check(196, firstCustom != secondCustom && form.V132ChannelColorCacheCountForSelfTest <= channels.Length,
            "channel render cache did not track a live color change correctly");

        Check(207, form.RebuildV132TabBarDisposesOldControlsForSelfTest(),
            "chat tab rebuild left replaced buttons/context menus undisposed");

        var tabIds = settings.Chat.Tabs.Select(x => x.Id).ToArray();
        var switchDurations = new double[ChatTabSwitchCount];
        for (var i = 0; i < ChatTabSwitchCount; i++)
        {
            timer.Restart();
            form.SelectV132TabForSelfTest(tabIds[(i + 1) % tabIds.Length]);
            Application.DoEvents();
            switchDurations[i] = timer.Elapsed.TotalMilliseconds;
        }
        var switchTotalMs = switchDurations.Sum();
        var switchAverageMs = switchDurations.Average();
        var switchMaxMs = switchDurations.Max();
        Check(197, switchTotalMs < 4_000 && switchMaxMs < 500,
            $"chat tab navigation was too slow (total={switchTotalMs:F1} ms max={switchMaxMs:F1} ms)");

        // Selecting a tab must not block the UI on SettingsStore.Save(), whose durable
        // path calls Flush(flushToDisk:true). Selection is persisted by normal lifecycle
        // saves (hide/collapse/shutdown/settings changes) instead.
        Check(198, !File.Exists(settingsPath),
            "chat tab navigation synchronously wrote settings.json");
        var selectedBeforeSave = settings.Chat.LastSelectedTabId;
        Check(199, store.Save(settings), "explicit lifecycle settings save failed");
        var reloaded = store.Load();
        Check(200, reloaded.Chat.LastSelectedTabId == selectedBeforeSave,
            "deferred selected-tab persistence did not survive an explicit settings save");

        // Force real WinForms paints through the owner-drawn chat path. This is not a
        // game-FPS benchmark; it is a deterministic synthetic repaint throughput check
        // that catches expensive per-row allocation/layout regressions.
        form.Opacity = 0.01d;
        form.Refresh();
        Application.DoEvents();
        var frameDurations = new double[RepaintCount];
        for (var i = 0; i < RepaintCount; i++)
        {
            timer.Restart();
            form.Refresh();
            frameDurations[i] = timer.Elapsed.TotalMilliseconds;
        }
        var repaintTotalMs = Math.Max(0.001, frameDurations.Sum());
        var repaintAverageMs = repaintTotalMs / RepaintCount;
        var repaintMaxMs = frameDurations.Max();
        var syntheticFps = 1000d / Math.Max(0.001, repaintAverageMs);
        Check(201, repaintAverageMs < 50 && repaintMaxMs < 250,
            $"overlay repaint path was too slow (avg={repaintAverageMs:F2} ms max={repaintMaxMs:F2} ms)");

        metrics.Add($"overlay.construct.ms={constructMs:F2}");
        metrics.Add($"overlay.firstShow.ms={showMs:F2}");
        metrics.Add($"overlay.ingest200.ms={ingestMs:F2}");
        metrics.Add($"overlay.tabSwitch40.total.ms={switchTotalMs:F2}");
        metrics.Add($"overlay.tabSwitch40.avg.ms={switchAverageMs:F2}");
        metrics.Add($"overlay.tabSwitch40.max.ms={switchMaxMs:F2}");
        metrics.Add($"overlay.repaint60.avg.ms={repaintAverageMs:F2}");
        metrics.Add($"overlay.repaint60.max.ms={repaintMaxMs:F2}");
        metrics.Add($"overlay.repaint.syntheticFps={syntheticFps:F1}");
        metrics.Add($"overlay.colorCache.10k.allocatedBytes={colorCacheAllocatedBytes}");
        metrics.Add("overlay.tabRebuild.resourceCleanup=PASS");
    }

    private static void TestSettingsNavigation(List<string> metrics)
    {
        var chat = new ChatOverlaySettings();
        chat.Normalize();
        var speech = new ChatSpeechTranslationSettings();
        speech.Normalize();

        using var owner = new Form { ShowInTaskbar = false, Opacity = 0d };
        owner.Show();
        Application.DoEvents();

        var timer = Stopwatch.StartNew();
        using var form = new ChatGeneralSettingsForm(chat, speech)
        {
            ShowInTaskbar = false,
            Opacity = 0d
        };
        var constructMs = timer.Elapsed.TotalMilliseconds;

        timer.Restart();
        form.PrewarmV124ForOwner(owner);
        var prewarmMs = timer.Elapsed.TotalMilliseconds;

        timer.Restart();
        form.PrepareV124ForOpen(owner);
        var prepareMs = timer.Elapsed.TotalMilliseconds;

        timer.Restart();
        for (var i = 0; i < 20; i++)
        {
            form.ShowV122PageForSelfTest("Interaction");
            form.ShowV122PageForSelfTest("Alerts");
            form.ShowV122PageForSelfTest("Speech");
            form.ShowV122PageForSelfTest("Advanced");
            form.ShowV122PageForSelfTest("Appearance");
        }
        var switch100Ms = timer.Elapsed.TotalMilliseconds;

        Check(202, constructMs < 1_500, $"Settings construction exceeded 1500 ms ({constructMs:F1} ms)");
        Check(203, prewarmMs < 1_500, $"Settings prewarm exceeded 1500 ms ({prewarmMs:F1} ms)");
        Check(204, prepareMs < 1_500, $"Settings cached prepare exceeded 1500 ms ({prepareMs:F1} ms)");
        Check(205, switch100Ms < 1_000, $"100 Settings page switches exceeded 1000 ms ({switch100Ms:F1} ms)");

        metrics.Add($"settings.construct.ms={constructMs:F2}");
        metrics.Add($"settings.prewarm.ms={prewarmMs:F2}");
        metrics.Add($"settings.cachedPrepare.ms={prepareMs:F2}");
        metrics.Add($"settings.pageSwitch100.ms={switch100Ms:F2}");
    }

    private static void TestCoreAudioPreload(List<string> metrics)
    {
        var assembly = Assembly.GetExecutingAssembly();
        var resources = new[]
        {
            "BPSR.ReadyAlert.Assets.Queue.wav",
            "BPSR.ReadyAlert.Assets.ReadyCheck.wav",
            "BPSR.ReadyAlert.Assets.PartyInvite.wav",
            "BPSR.ReadyAlert.Assets.PartyRequest.wav"
        };
        var tempFiles = new List<string>(resources.Length);

        try
        {
            foreach (var resourceName in resources)
            {
                var path = Path.Combine(Path.GetTempPath(), $"BPSR-ReadyAlert-v132-perf-{Guid.NewGuid():N}.wav");
                using var source = assembly.GetManifestResourceStream(resourceName)
                    ?? throw new InvalidOperationException("missing audio resource " + resourceName);
                using (var output = File.Create(path)) source.CopyTo(output);
                tempFiles.Add(path);
            }

            var timer = Stopwatch.StartNew();
            using var queue = new AlertAudioPlayer(tempFiles[0], 100);
            using var ready = new AlertAudioPlayer(tempFiles[1], 100);
            using var invite = new AlertAudioPlayer(tempFiles[2], 100);
            using var request = new AlertAudioPlayer(tempFiles[3], 100);
            var preloadMs = timer.Elapsed.TotalMilliseconds;
            Check(206, preloadMs < 1_000,
                $"four core alert WAV preloads exceeded 1000 ms ({preloadMs:F1} ms)");
            metrics.Add($"audio.preload4.ms={preloadMs:F2}");
        }
        finally
        {
            foreach (var path in tempFiles) TryDelete(path);
        }
    }

    private static void TryDelete(string path)
    {
        try { if (File.Exists(path)) File.Delete(path); } catch { }
    }

    private static void Check(int code, bool condition, string message)
    {
        if (condition) return;
        Environment.ExitCode = code;
        throw new InvalidOperationException("v1.3.2 UI performance self-test failed: " + message);
    }
}
