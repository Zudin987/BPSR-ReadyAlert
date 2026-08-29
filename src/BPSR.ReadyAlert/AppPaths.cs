namespace BPSR.ReadyAlert;

internal sealed class AppPaths
{
    internal string Root { get; }
    internal string AssetsDir { get; }
    internal string AlertSoundPath => Path.Combine(AssetsDir, $"LetsDoThis-{AppVersion.Current}.wav");
    internal string QueueSoundPath => Path.Combine(AssetsDir, $"Queue-{AppVersion.Current}.wav");
    internal string ReadyCheckSoundPath => Path.Combine(AssetsDir, $"ReadyCheck-{AppVersion.Current}.wav");
    internal string PartyInviteSoundPath => Path.Combine(AssetsDir, $"PartyInvite-{AppVersion.Current}.wav");
    internal string PartyRequestSoundPath => Path.Combine(AssetsDir, $"PartyRequest-{AppVersion.Current}.wav");
    internal string AppIconPath => Path.Combine(AssetsDir, $"App-{AppVersion.Current}.ico");
    internal string SettingsPath => Path.Combine(Root, "settings.json");
    internal string LogPath => Path.Combine(Root, "readyalert.log");

    private AppPaths(string root, string assetsDir)
    {
        Root = root;
        AssetsDir = assetsDir;
    }

    internal static AppPaths Create()
    {
        var root = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
            "BPSR-ReadyAlert");
        var assetsDir = Path.Combine(root, "assets");

        Directory.CreateDirectory(root);
        Directory.CreateDirectory(assetsDir);
        return new AppPaths(root, assetsDir);
    }
}
