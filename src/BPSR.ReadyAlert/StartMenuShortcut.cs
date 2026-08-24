using System.Runtime.InteropServices;

namespace BPSR.ReadyAlert;

internal static class StartMenuShortcut
{
    internal static bool TryCreateOrRefresh(string? iconPath = null)
    {
        object? shell = null;
        object? shortcut = null;
        try
        {
            var exe = Environment.ProcessPath;
            if (string.IsNullOrWhiteSpace(exe) || !File.Exists(exe)) return false;

            var programs = Environment.GetFolderPath(Environment.SpecialFolder.Programs);
            Directory.CreateDirectory(programs);
            var shortcutPath = Path.Combine(programs, "BPSR Ready Alert.lnk");

            // Recreate the shortcut rather than editing it in place. Together with the
            // versioned icon file this avoids Windows keeping the old cached icon.
            if (File.Exists(shortcutPath))
            {
                try { File.Delete(shortcutPath); } catch { }
            }

            var shellType = Type.GetTypeFromProgID("WScript.Shell");
            if (shellType is null) return false;

            shell = Activator.CreateInstance(shellType);
            if (shell is null) return false;

            dynamic dynamicShell = shell;
            shortcut = dynamicShell.CreateShortcut(shortcutPath);
            dynamic dynamicShortcut = shortcut;
            dynamicShortcut.TargetPath = exe;
            dynamicShortcut.WorkingDirectory = Path.GetDirectoryName(exe) ?? string.Empty;
            dynamicShortcut.Description = "BPSR Ready Alert";
            dynamicShortcut.IconLocation = !string.IsNullOrWhiteSpace(iconPath) && File.Exists(iconPath)
                ? iconPath + ",0"
                : exe + ",0";
            dynamicShortcut.Save();
            AppLog.Write($"shortcut: refreshed {shortcutPath} icon={dynamicShortcut.IconLocation}");
            return true;
        }
        catch (Exception ex)
        {
            AppLog.Write("shortcut: failed " + ex.Message);
            return false;
        }
        finally
        {
            if (shortcut is not null && Marshal.IsComObject(shortcut))
            {
                try { Marshal.FinalReleaseComObject(shortcut); } catch { }
            }
            if (shell is not null && Marshal.IsComObject(shell))
            {
                try { Marshal.FinalReleaseComObject(shell); } catch { }
            }
        }
    }
}
