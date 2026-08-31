using System.Diagnostics;
using System.Threading;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal static class Program
{
    [STAThread]
    private static void Main(string[] args)
    {
        // CI runs the final published single-file EXE in this mode. Besides proving
        // the bundle starts, run deterministic chat parser/filter/settings and UI checks.
        if (args.Any(a => string.Equals(a, "--build-smoke-test", StringComparison.OrdinalIgnoreCase)))
        {
            try
            {
                ApplicationConfiguration.Initialize();
                RunSmokeStep(ChatSelfTest.Run, 11);
                RunSmokeStep(ChatUiSelfTest.Run, 12);
                RunSmokeStep(ChatRc8SelfTest.Run, 13);
                RunSmokeStep(ChatRc9SelfTest.Run, 14);
                RunSmokeStep(ChatRc10SelfTest.Run, 15);
                RunSmokeStep(ChatV111SelfTest.Run, 16);
                RunSmokeStep(ChatRc2SelfTest.Run, 17);
                RunSmokeStep(ChatV120SelfTest.Run, 18);
                RunSmokeStep(UiUxV121SelfTest.Run, 19);
                RunSmokeStep(TtsVolumeIsolationSelfTest.Run, 20);
                RunSmokeStep(SettingsUiV122SelfTest.Run, 21);
                RunSmokeStep(SettingsUiV123SelfTest.Run, 22);
                RunSmokeStep(SettingsUiV124SelfTest.Run, 23);
                RunSmokeStep(SettingsUiV125SelfTest.Run, 24);
                RunSmokeStep(UiSmoothnessV126SelfTest.Run, 25);
                RunSmokeStep(PartyAlertV130SelfTest.Run, 26);
                RunSmokeStep(CoreAlertV131SelfTest.Run, 27);
                RunSmokeStep(UiPerformanceV132SelfTest.Run, 28);
                RunSmokeStep(PlayerIdentityV133SelfTest.Run, 29);
                RunSmokeStep(CaptureRecoveryV134SelfTest.Run, 30);
                RunSmokeStep(RelayCompatibilityV135SelfTest.Run, 31);
                RunSmokeStep(ChatLocalLogV136SelfTest.Run, 32);
                Environment.ExitCode = 0;
                return;
            }
            catch (Exception ex)
            {
                try
                {
                    Console.Error.WriteLine("BPSR ReadyAlert smoke test failed:");
                    Console.Error.WriteLine(ex);
                }
                catch { }
                try
                {
                    File.WriteAllText(
                        Path.Combine(AppContext.BaseDirectory, "smoke-test-error.txt"),
                        ex.ToString());
                }
                catch { }

                if (Environment.ExitCode == 0) Environment.ExitCode = 1;
                return;
            }
        }

        using var mutex = new Mutex(initiallyOwned: true, name: @"Global\BPSR-ReadyAlert", createdNew: out var createdNew);
        if (!createdNew)
        {
            MessageBox.Show(
                "BPSR Ready Alert is already running in the system tray.",
                "BPSR Ready Alert",
                MessageBoxButtons.OK,
                MessageBoxIcon.Information);
            return;
        }

        var startup = Stopwatch.StartNew();
        ApplicationConfiguration.Initialize();

        try
        {
            var paths = AppPaths.Create();
            AppLog.Initialize(paths.LogPath);
            AppLog.Write($"startup: version={AppVersion.Current} exe={Environment.ProcessPath}");

            // Chat-log directory creation, startup retention cleanup, and all later
            // chat filesystem I/O stay on this service's dedicated background thread.
            ChatLocalLogService.Initialize(paths.ChatLogsDir);

            RuntimeAssets.Ensure(paths);

            var settingsStore = new SettingsStore(paths.SettingsPath);
            var settings = settingsStore.Load();
            var launcher = new ResonanceLogsLauncher(settings, settingsStore);

            ChatNotificationEngine.Configure(settings.Chat, paths.AlertSoundPath);
            if (settings.SpeechTranslation.TranslationEnabled || settings.SpeechTranslation.TtsEnabled)
                PlayerIdentityCaptureBridge.ConfigureSpeechEngine(settings.SpeechTranslation);

            var beforeNpcap = startup.ElapsedMilliseconds;
            NpcapCapturePlan capturePlan;
            try
            {
                capturePlan = NpcapDeviceSelector.SelectPlan(settings);
            }
            catch (InvalidOperationException ex) when (
                ex.Message.Contains("no capture adapters", StringComparison.OrdinalIgnoreCase))
            {
                AppLog.Write("startup: no Npcap adapter currently available; entering recovery mode");
                capturePlan = CaptureRecoveryPlanner.CreateWaitingPlan(settings.NpcapDeviceName);
            }
            AppLog.Write($"startup: Npcap plan selected in {startup.ElapsedMilliseconds - beforeNpcap} ms");

            if (!string.IsNullOrWhiteSpace(settings.NpcapDeviceName))
            {
                capturePlan = CaptureRecoveryPlanner.PreserveUnavailableManual(
                    settings.NpcapDeviceName.Trim(),
                    capturePlan);
                AppLog.Write("startup: preserving manual Npcap adapter for self-healing recovery=" + settings.NpcapDeviceName);
            }

            var context = new TrayApplicationContext(paths, settings, settingsStore, launcher, capturePlan);
            AppLog.Write($"startup: tray context ready in {startup.ElapsedMilliseconds} ms");

            QueueStartMenuShortcutRefresh(paths.AppIconPath);

            using var autoLaunchTimer = settings.AutoLaunchResonanceLogs
                ? CreateDeferredAutoLaunchTimer(launcher)
                : null;
            autoLaunchTimer?.Start();

            Application.Run(context);
        }
        catch (DllNotFoundException ex)
        {
            AppLog.Write("startup: Npcap missing " + ex);
            MessageBox.Show(
                "BPSR Ready Alert could not find Npcap.\r\n\r\n" +
                "Install Npcap, or make sure the same Npcap installation used by Resonance Logs CN is working.\r\n\r\n" +
                "Details: " + ex.Message,
                "BPSR Ready Alert - Npcap Required",
                MessageBoxButtons.OK,
                MessageBoxIcon.Error);
        }
        catch (Exception ex)
        {
            AppLog.Write("startup: fatal " + ex);
            MessageBox.Show(
                "BPSR Ready Alert could not start.\r\n\r\n" + ex.Message +
                "\r\n\r\nIf this keeps happening, open %LOCALAPPDATA%\\BPSR-ReadyAlert\\readyalert.log.",
                "BPSR Ready Alert - Error",
                MessageBoxButtons.OK,
                MessageBoxIcon.Error);
        }
        finally
        {
            ChatLocalLogService.Shutdown();
            ChatSpeechTranslationEngine.Shutdown();
            try { mutex.ReleaseMutex(); } catch { }
        }
    }

    private static void QueueStartMenuShortcutRefresh(string iconPath)
    {
        var thread = new Thread(() =>
        {
            try { StartMenuShortcut.TryCreateOrRefresh(iconPath); }
            catch (Exception ex) { AppLog.Write("shortcut: background refresh failed " + ex.Message); }
        })
        {
            IsBackground = true,
            Name = "BPSR-ReadyAlert-ShortcutRefresh"
        };

        try { thread.SetApartmentState(ApartmentState.STA); }
        catch { }
        thread.Start();
    }

    private static System.Windows.Forms.Timer CreateDeferredAutoLaunchTimer(ResonanceLogsLauncher launcher)
    {
        var timer = new System.Windows.Forms.Timer { Interval = 100 };
        timer.Tick += (_, _) =>
        {
            timer.Stop();
            try { launcher.EnsureRunningInteractive(); }
            catch (Exception ex) { AppLog.Write("launcher: deferred auto-launch failed " + ex); }
        };
        return timer;
    }

    private static void RunSmokeStep(Action step, int failureCode)
    {
        try
        {
            Environment.ExitCode = failureCode;
            step();
            Environment.ExitCode = 0;
        }
        catch (Exception ex)
        {
            var specificCode = Environment.ExitCode;
            Environment.ExitCode = specificCode >= 100 ? specificCode : failureCode;
            var owner = step.Method.DeclaringType?.Name ?? "unknown";
            throw new InvalidOperationException(
                $"Smoke step {failureCode} ({owner}.{step.Method.Name}) failed.",
                ex);
        }
    }
}
