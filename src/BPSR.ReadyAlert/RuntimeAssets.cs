using System.Reflection;
using System.Security.Cryptography;

namespace BPSR.ReadyAlert;

internal static class RuntimeAssets
{
    private const string WinDivertDllSha256 = "c1e060ee19444a259b2162f8af0f3fe8c4428a1c6f694dce20de194ac8d7d9a2";
    private const string WinDivertDriverSha256 = "8da085332782708d8767bcace5327a6ec7283c17cfb85e40b03cd2323a90ddc2";
    private const string AlertSoundSha256 = "0befc4c0b6a40ef374fb75c6f4c658850439ee43fa9a3c0d74d904c76627048a";

    internal static void Ensure(AppPaths paths)
    {
        EnsureResource(
            "BPSR.ReadyAlert.WinDivert.WinDivert.dll",
            paths.WinDivertDllPath,
            WinDivertDllSha256);
        EnsureResource(
            "BPSR.ReadyAlert.WinDivert.WinDivert64.sys",
            paths.WinDivertDriverPath,
            WinDivertDriverSha256);
        EnsureResource(
            "BPSR.ReadyAlert.WinDivert.LICENSE",
            paths.WinDivertLicensePath,
            expectedSha256: null);
        EnsureResource(
            "BPSR.ReadyAlert.Assets.LetsDoThis.wav",
            paths.AlertSoundPath,
            AlertSoundSha256);
    }

    private static void EnsureResource(string resourceName, string destination, string? expectedSha256)
    {
        if (File.Exists(destination) && IsExpectedFile(destination, expectedSha256))
            return;

        var assembly = Assembly.GetExecutingAssembly();
        using var resource = assembly.GetManifestResourceStream(resourceName)
            ?? throw new InvalidOperationException($"Embedded resource is missing: {resourceName}");

        var directory = Path.GetDirectoryName(destination)
            ?? throw new InvalidOperationException("Could not resolve runtime asset directory.");
        Directory.CreateDirectory(directory);

        var temp = destination + ".new";
        using (var output = new FileStream(temp, FileMode.Create, FileAccess.Write, FileShare.None))
            resource.CopyTo(output);

        if (!IsExpectedFile(temp, expectedSha256))
        {
            File.Delete(temp);
            throw new InvalidDataException($"Embedded runtime asset failed integrity verification: {Path.GetFileName(destination)}");
        }

        File.Move(temp, destination, overwrite: true);
        AppLog.Write("assets: extracted " + destination);
    }

    private static bool IsExpectedFile(string path, string? expectedSha256)
    {
        try
        {
            if (!File.Exists(path) || new FileInfo(path).Length == 0) return false;
            if (string.IsNullOrWhiteSpace(expectedSha256)) return true;

            using var stream = File.OpenRead(path);
            var hash = Convert.ToHexString(SHA256.HashData(stream)).ToLowerInvariant();
            return string.Equals(hash, expectedSha256, StringComparison.OrdinalIgnoreCase);
        }
        catch
        {
            return false;
        }
    }
}
