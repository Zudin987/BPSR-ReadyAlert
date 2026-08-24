using System.Reflection;

namespace BPSR.ReadyAlert;

internal static class RuntimeAssets
{
    internal static void Ensure(AppPaths paths)
    {
        EnsureResource(
            "BPSR.ReadyAlert.Assets.LetsDoThis.wav",
            paths.AlertSoundPath);
    }

    private static void EnsureResource(string resourceName, string destination)
    {
        if (File.Exists(destination) && new FileInfo(destination).Length > 0)
            return;

        var assembly = Assembly.GetExecutingAssembly();
        using var resource = assembly.GetManifestResourceStream(resourceName)
            ?? throw new InvalidOperationException($"Embedded resource is missing: {resourceName}");

        var directory = Path.GetDirectoryName(destination)
            ?? throw new InvalidOperationException("Could not resolve asset directory.");
        Directory.CreateDirectory(directory);

        var temp = destination + ".new";
        using (var output = new FileStream(temp, FileMode.Create, FileAccess.Write, FileShare.None))
            resource.CopyTo(output);

        if (!File.Exists(temp) || new FileInfo(temp).Length == 0)
        {
            File.Delete(temp);
            throw new InvalidDataException($"Embedded asset extraction failed: {Path.GetFileName(destination)}");
        }

        File.Move(temp, destination, overwrite: true);
        AppLog.Write("assets: extracted " + destination);
    }
}
