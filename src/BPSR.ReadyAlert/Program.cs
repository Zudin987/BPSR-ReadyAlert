using System.Threading;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal static class Program
{
    [STAThread]
    private static void Main()
    {
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

            StartMenuShortcut.TryCreateOrRefresh();
            if (settings.AutoLaunchResonanceLogs)
                launcher.EnsureRunningInteractive();

            var captureSelection = NpcapDeviceSelector.Select();
            Application.Run(new TrayApplicationContext(paths, settings, settingsStore, launcher, captureSelection));
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
