using Microsoft.Win32;
using System.Diagnostics;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed class ResonanceLogsLauncher
{
    private readonly AppSettings _settings;
    private readonly SettingsStore _store;

    internal ResonanceLogsLauncher(AppSettings settings, SettingsStore store)
    {
        _settings = settings;
        _store = store;
    }

    internal bool EnsureRunningInteractive()
    {
        if (IsRunning()) return true;

        var exe = FindExecutable();
        if (exe is null)
            exe = AskUserForExecutable();

        if (exe is null)
        {
            AppLog.Write("launcher: Resonance Logs CN path not found/selected");
            return false;
        }

        SavePath(exe);
        try
        {
            Process.Start(new ProcessStartInfo
            {
                FileName = exe,
                WorkingDirectory = Path.GetDirectoryName(exe) ?? string.Empty,
                UseShellExecute = true
            });
            AppLog.Write("launcher: started " + exe);
            return true;
        }
        catch (Exception ex)
        {
            AppLog.Write("launcher: start failed " + ex);
            MessageBox.Show(
                "Could not start Resonance Logs CN.\r\n\r\n" + ex.Message,
                "BPSR Ready Alert",
                MessageBoxButtons.OK,
                MessageBoxIcon.Error);
            return false;
        }
    }

    internal void ChangeExecutableInteractive()
    {
        var selected = AskUserForExecutable();
        if (selected is null) return;
        SavePath(selected);
    }

    internal bool IsRunning()
    {
        foreach (var process in Process.GetProcesses())
        {
            using (process)
            {
                try
                {
                    var fileName = process.MainModule?.FileName;
                    if (LooksLikeResonanceLogsExecutable(fileName)) return true;
                }
                catch { }

                try
                {
                    if (process.ProcessName.Equals("resonance-logs-cn", StringComparison.OrdinalIgnoreCase))
                        return true;
                }
                catch { }
            }
        }
        return false;
    }

    private string? FindExecutable()
    {
        if (IsValidExecutable(_settings.ResonanceLogsPath))
            return _settings.ResonanceLogsPath;

        foreach (var process in Process.GetProcesses())
        {
            using (process)
            {
                try
                {
                    var path = process.MainModule?.FileName;
                    if (LooksLikeResonanceLogsExecutable(path) && IsValidExecutable(path))
                        return path;
                }
                catch { }
            }
        }

        foreach (var candidate in GetCommonPaths())
            if (IsValidExecutable(candidate)) return candidate;

        var registryPath = FindFromUninstallRegistry();
        if (IsValidExecutable(registryPath)) return registryPath;

        return null;
    }

    private string? AskUserForExecutable()
    {
        using var dialog = new OpenFileDialog
        {
            Title = "Select resonance-logs-cn.exe",
            Filter = "Resonance Logs CN (resonance-logs-cn.exe)|resonance-logs-cn.exe|Applications (*.exe)|*.exe",
            CheckFileExists = true,
            Multiselect = false
        };

        if (IsValidExecutable(_settings.ResonanceLogsPath))
            dialog.InitialDirectory = Path.GetDirectoryName(_settings.ResonanceLogsPath);

        if (dialog.ShowDialog() != DialogResult.OK) return null;
        return IsValidExecutable(dialog.FileName) ? dialog.FileName : null;
    }

    private void SavePath(string path)
    {
        _settings.ResonanceLogsPath = Path.GetFullPath(path);
        _store.Save(_settings);
    }

    private static bool IsValidExecutable(string? path) =>
        !string.IsNullOrWhiteSpace(path) &&
        File.Exists(path) &&
        path.EndsWith(".exe", StringComparison.OrdinalIgnoreCase);

    private static bool LooksLikeResonanceLogsExecutable(string? path)
    {
        if (string.IsNullOrWhiteSpace(path)) return false;
        var name = Path.GetFileName(path);
        return name.Equals("resonance-logs-cn.exe", StringComparison.OrdinalIgnoreCase) ||
               name.Contains("resonance-logs", StringComparison.OrdinalIgnoreCase);
    }

    private static IEnumerable<string> GetCommonPaths()
    {
        var local = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
        var programFiles = Environment.GetFolderPath(Environment.SpecialFolder.ProgramFiles);
        var programFilesX86 = Environment.GetFolderPath(Environment.SpecialFolder.ProgramFilesX86);

        yield return Path.Combine(local, "Programs", "resonance-logs-cn", "resonance-logs-cn.exe");
        yield return Path.Combine(local, "resonance-logs-cn", "resonance-logs-cn.exe");
        yield return Path.Combine(local, "Programs", "Resonance Logs CN", "resonance-logs-cn.exe");
        if (!string.IsNullOrWhiteSpace(programFiles))
            yield return Path.Combine(programFiles, "resonance-logs-cn", "resonance-logs-cn.exe");
        if (!string.IsNullOrWhiteSpace(programFilesX86))
            yield return Path.Combine(programFilesX86, "resonance-logs-cn", "resonance-logs-cn.exe");
    }

    private static string? FindFromUninstallRegistry()
    {
        var locations = new[]
        {
            (RegistryHive.CurrentUser, RegistryView.Default),
            (RegistryHive.LocalMachine, RegistryView.Registry64),
            (RegistryHive.LocalMachine, RegistryView.Registry32)
        };

        foreach (var (hive, view) in locations)
        {
            try
            {
                using var baseKey = RegistryKey.OpenBaseKey(hive, view);
                using var uninstall = baseKey.OpenSubKey(@"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall");
                if (uninstall is null) continue;

                foreach (var subName in uninstall.GetSubKeyNames())
                {
                    using var sub = uninstall.OpenSubKey(subName);
                    var displayName = sub?.GetValue("DisplayName") as string;
                    if (string.IsNullOrWhiteSpace(displayName) ||
                        !displayName.Contains("resonance", StringComparison.OrdinalIgnoreCase) ||
                        !displayName.Contains("logs", StringComparison.OrdinalIgnoreCase))
                        continue;

                    var installLocation = sub?.GetValue("InstallLocation") as string;
                    if (!string.IsNullOrWhiteSpace(installLocation))
                    {
                        var candidate = Path.Combine(installLocation.Trim('"'), "resonance-logs-cn.exe");
                        if (IsValidExecutable(candidate)) return candidate;
                    }

                    var displayIcon = sub?.GetValue("DisplayIcon") as string;
                    var iconPath = NormalizeExecutableValue(displayIcon);
                    if (LooksLikeResonanceLogsExecutable(iconPath) && IsValidExecutable(iconPath))
                        return iconPath;

                    var uninstallString = sub?.GetValue("UninstallString") as string;
                    var uninstaller = NormalizeExecutableValue(uninstallString);
                    if (!string.IsNullOrWhiteSpace(uninstaller))
                    {
                        var dir = Path.GetDirectoryName(uninstaller);
                        if (!string.IsNullOrWhiteSpace(dir))
                        {
                            var candidate = Path.Combine(dir, "resonance-logs-cn.exe");
                            if (IsValidExecutable(candidate)) return candidate;
                        }
                    }
                }
            }
            catch { }
        }

        return null;
    }

    private static string? NormalizeExecutableValue(string? raw)
    {
        if (string.IsNullOrWhiteSpace(raw)) return null;
        raw = raw.Trim();
        if (raw.StartsWith('"'))
        {
            var end = raw.IndexOf('"', 1);
            if (end > 1) return raw[1..end];
        }

        var exeIndex = raw.IndexOf(".exe", StringComparison.OrdinalIgnoreCase);
        if (exeIndex >= 0) return raw[..(exeIndex + 4)].Trim('"', ' ');
        return raw.Trim('"');
    }
}
