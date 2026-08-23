namespace BPSR.ReadyAlert;

internal sealed class AppPaths
{
    internal string Root { get; }
    internal string RuntimeDir { get; }
    internal string WinDivertDllPath => Path.Combine(RuntimeDir, "WinDivert.dll");
    internal string WinDivertDriverPath => Path.Combine(RuntimeDir, "WinDivert64.sys");
    internal string WinDivertLicensePath => Path.Combine(RuntimeDir, "WinDivert-LICENSE.txt");
    internal string AssetsDir { get; }
    internal string AlertSoundPath => Path.Combine(AssetsDir, "LetsDoThis.wav");
    internal string SettingsPath => Path.Combine(Root, "settings.json");
    internal string LogPath => Path.Combine(Root, "readyalert.log");

    private AppPaths(string root, string runtimeDir, string assetsDir)
    {
        Root = root;
        RuntimeDir = runtimeDir;
        AssetsDir = assetsDir;
    }

    internal static AppPaths Create()
    {
        var root = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
            "BPSR-ReadyAlert");

        // Version the native runtime folder so a future WinDivert update never needs
        // to overwrite a driver image that Windows may still have mapped.
        var runtimeDir = Path.Combine(root, "runtime", "WinDivert-2.2.2-c1e060ee");
        var assetsDir = Path.Combine(root, "assets");

        Directory.CreateDirectory(root);
        Directory.CreateDirectory(runtimeDir);
        Directory.CreateDirectory(assetsDir);
        return new AppPaths(root, runtimeDir, assetsDir);
    }
}
