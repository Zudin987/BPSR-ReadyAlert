using System.Diagnostics;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal static class UiSmoothnessV126SelfTest
{
    internal static void Run()
    {
        TestSettingsOpenAndTransitions();
        TestLauncherProcessScan();
    }

    private static void TestSettingsOpenAndTransitions()
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
        var constructMs = timer.ElapsedMilliseconds;

        timer.Restart();
        form.PrewarmV124ForOwner(owner);
        var prewarmMs = timer.ElapsedMilliseconds;

        timer.Restart();
        form.PrepareV124ForOpen(owner);
        var prepareMs = timer.ElapsedMilliseconds;

        timer.Restart();
        for (var i = 0; i < 20; i++)
        {
            form.ShowV122PageForSelfTest("Interaction");
            form.ShowV122PageForSelfTest("Alerts");
            form.ShowV122PageForSelfTest("Speech");
            form.ShowV122PageForSelfTest("Advanced");
            form.ShowV122PageForSelfTest("Appearance");
        }
        var switchBurstMs = timer.ElapsedMilliseconds;

        var metrics =
            $"v1.2.6 settings timing: construct={constructMs}ms prewarm={prewarmMs}ms " +
            $"cachedPrepare={prepareMs}ms switches100={switchBurstMs}ms";
        try { Console.Error.WriteLine(metrics); } catch { }
        AppLog.Write("selftest: " + metrics);

        // These are deliberately broad release regression gates, not microbenchmarks.
        // Their purpose is to catch a future accidental return to multi-second form
        // realization, gear clicks, or recursive page-layout work while remaining
        // stable on hosted CI.
        Check(181, constructMs < 1_500,
            $"Settings construction exceeded 1500 ms ({constructMs} ms)");
        Check(182, prewarmMs < 1_500,
            $"hidden Settings prewarm exceeded 1500 ms ({prewarmMs} ms)");
        Check(183, prepareMs < 1_500,
            $"cached Settings preparation exceeded 1500 ms ({prepareMs} ms)");
        Check(184, switchBurstMs < 1_000,
            $"100 realized Settings tab switches exceeded 1000 ms ({switchBurstMs} ms)");
    }

    private static void TestLauncherProcessScan()
    {
        Check(185,
            ResonanceLogsLauncher.LooksLikeResonanceLogsProcessNameForSelfTest("resonance-logs-cn"),
            "launcher recognizes the normal Resonance Logs process name");
        Check(186,
            !ResonanceLogsLauncher.LooksLikeResonanceLogsProcessNameForSelfTest("explorer"),
            "launcher rejects unrelated process names");

        var path = Path.Combine(Path.GetTempPath(), $"BPSR-ReadyAlert-v126-launcher-{Guid.NewGuid():N}.json");
        try
        {
            var settings = new AppSettings();
            var launcher = new ResonanceLogsLauncher(settings, new SettingsStore(path));
            var timer = Stopwatch.StartNew();
            _ = launcher.IsRunning();
            var scanMs = timer.ElapsedMilliseconds;
            try { Console.Error.WriteLine($"v1.2.6 launcher process scan: {scanMs}ms"); } catch { }
            AppLog.Write($"selftest: v1.2.6 process-name scan={scanMs}ms");
            Check(187, scanMs < 1_500,
                $"process-name-only Resonance Logs scan exceeded 1500 ms ({scanMs} ms)");
        }
        finally
        {
            TryDelete(path);
            TryDelete(path + ".bak");
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

    private static void Check(int code, bool condition, string name)
    {
        if (condition) return;
        Environment.ExitCode = code;
        throw new InvalidOperationException("v1.2.6 smoothness self-test failed: " + name);
    }
}
