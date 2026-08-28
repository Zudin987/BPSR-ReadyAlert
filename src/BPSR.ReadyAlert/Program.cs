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
                Environment.ExitCode = 0;
                return;
            }
            catch (Exception ex)
            {
                // Do not hide the useful reason behind a numeric smoke-test code.
                // GitHub Actions can normally inherit stderr from this WinExe. Keep a
                // sidecar too so a local/published smoke run remains diagnosable even
                // when the parent process does not expose console handles.
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

        ApplicationConfiguration.Initialize();

        try
        {
            var paths = AppPaths.Create();
            AppLog.Initialize(paths.LogPath);
            AppLog.Write($"startup: version={AppVersion.Current} exe={Environment.ProcessPath}");

            RuntimeAssets.Ensure(paths);

            var settingsStore = new SettingsStore(paths.SettingsPath);
            var settings = settingsStore.Load();
            var launcher = new ResonanceLogsLauncher(settings, settingsStore);

            // Prime independent background snapshots before capture can emit the
            // first chat packet. The optional Google worker is not started at all
            // for users who leave both translation and TTS disabled.
            ChatNotificationEngine.Configure(settings.Chat, paths.AlertSoundPath);
            if (settings.SpeechTranslation.TranslationEnabled || settings.SpeechTranslation.TtsEnabled)
                ChatSpeechTranslationEngine.Configure(settings.SpeechTranslation);

            StartMenuShortcut.TryCreateOrRefresh(paths.AppIconPath);
            if (settings.AutoLaunchResonanceLogs)
                launcher.EnsureRunningInteractive();

            var capturePlan = NpcapDeviceSelector.SelectPlan(settings);

            if (!string.IsNullOrWhiteSpace(settings.NpcapDeviceName) &&
                !capturePlan.AvailableDevices.Any(d =>
                    string.Equals(d.Name, settings.NpcapDeviceName, StringComparison.OrdinalIgnoreCase)))
            {
                AppLog.Write("settings: clearing unavailable NpcapDeviceName=" + settings.NpcapDeviceName);
                settings.NpcapDeviceName = string.Empty;
                settingsStore.Save(settings);
            }

            Application.Run(new TrayApplicationContext(paths, settings, settingsStore, launcher, capturePlan));
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
            ChatSpeechTranslationEngine.Shutdown();
            try { mutex.ReleaseMutex(); } catch { }
        }
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
            // A nested assertion is free to use its own diagnostics internally, but
            // the process exit code must identify the top-level smoke suite that
            // actually failed. This prevents stale assertion numbers from masking
            // which regression group needs inspection.
            Environment.ExitCode = failureCode;
            var owner = step.Method.DeclaringType?.Name ?? "unknown";
            throw new InvalidOperationException(
                $"Smoke step {failureCode} ({owner}.{step.Method.Name}) failed.",
                ex);
        }
    }
}
