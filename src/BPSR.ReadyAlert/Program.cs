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
                ChatSelfTest.Run();
                ChatUiSelfTest.Run();
                return;
            }
            catch (Exception ex)
            {
                // WinExe has no attached console in Actions. Use temporary distinct
                // exit codes to identify which RC5 UI-fit assertion failed without
                // weakening the assertions themselves.
                var detail = ex.ToString();
                Environment.ExitCode = detail.Contains("tab editor", StringComparison.OrdinalIgnoreCase) ? 11
                    : detail.Contains("settings", StringComparison.OrdinalIgnoreCase) ? 12
                    : detail.Contains("themed buttons", StringComparison.OrdinalIgnoreCase) ? 13
                    : 10;
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
            try { mutex.ReleaseMutex(); } catch { }
        }
    }
}
