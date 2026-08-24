using System.Net;
using System.Net.NetworkInformation;
using System.Text.Json;

namespace BPSR.ReadyAlert;

internal sealed record NpcapCaptureCandidate(string DeviceName, string Description, string Source);

internal sealed record NpcapCapturePlan(
    IReadOnlyList<NpcapCaptureCandidate> Candidates,
    IReadOnlyList<NpcapDevice> AvailableDevices,
    string? ResonanceLogsConfigPath)
{
    internal NpcapCaptureCandidate Primary => Candidates[0];
}

internal static class NpcapDeviceSelector
{
    internal static NpcapCapturePlan SelectPlan(AppSettings settings)
    {
        var devices = NpcapCapture.ListDevices();
        if (devices.Count == 0)
            throw new InvalidOperationException("Npcap is installed but no capture adapters were found.");

        AppLog.Write($"npcap: version={NpcapCapture.GetVersion()}");
        AppLog.Write($"npcap: enumerated devices={devices.Count}");

        var saved = TryReadResonanceLogsDevice();
        NpcapDevice? selected = null;
        var source = "Fallback";

        if (!string.IsNullOrWhiteSpace(settings.NpcapDeviceName))
        {
            selected = devices.FirstOrDefault(d =>
                string.Equals(d.Name, settings.NpcapDeviceName, StringComparison.OrdinalIgnoreCase));
            if (selected is not null)
            {
                source = "User selected";
            }
            else
            {
                AppLog.Write($"npcap: saved ReadyAlert adapter unavailable: {settings.NpcapDeviceName}; falling back");
            }
        }

        if (selected is null &&
            !string.IsNullOrWhiteSpace(saved.DeviceName) &&
            string.Equals(saved.Method, "Npcap", StringComparison.OrdinalIgnoreCase))
        {
            selected = devices.FirstOrDefault(d =>
                string.Equals(d.Name, saved.DeviceName, StringComparison.OrdinalIgnoreCase));
            if (selected is not null)
            {
                source = "Resonance Logs CN";
                AppLog.Write($"npcap: selected Resonance Logs CN device={selected.Name} description={selected.Description} config={saved.Path}");
            }
        }

        if (selected is null)
        {
            selected = AutoSelect(devices);
            if (selected is not null) source = "Auto-selected";
        }

        selected ??= devices.FirstOrDefault(d => !LooksLikeLoopback(d)) ?? devices[0];

        var candidate = new NpcapCaptureCandidate(selected.Name, selected.Description, source);
        AppLog.Write($"npcap: single-adapter capture device={candidate.DeviceName} description={candidate.Description} source={candidate.Source}");

        var available = devices
            .OrderBy(d => LooksLikeLoopback(d))
            .ThenBy(d => LooksVirtual(d))
            .ThenBy(d => d.Description, StringComparer.OrdinalIgnoreCase)
            .ToArray();

        return new NpcapCapturePlan([candidate], available, saved.Path);
    }

    private static NpcapDevice? AutoSelect(IReadOnlyList<NpcapDevice> devices)
    {
        var interfaces = NetworkInterface.GetAllNetworkInterfaces()
            .Where(IsUsableInterface)
            .Select(n => new
            {
                Interface = n,
                Score = ScoreInterface(n),
                NormalizedId = NormalizeGuid(n.Id)
            })
            .OrderByDescending(x => x.Score)
            .ToArray();

        foreach (var item in interfaces)
        {
            var exact = devices.FirstOrDefault(d =>
                DeviceMatchesInterface(d, item.Interface, item.NormalizedId));
            if (exact is null) continue;
            AppLog.Write($"npcap: auto-selected nic={item.Interface.Name} description={item.Interface.Description} score={item.Score} matched={exact.Name}");
            return exact;
        }

        return devices.FirstOrDefault(d => !LooksLikeLoopback(d) && !LooksVirtual(d))
               ?? devices.FirstOrDefault(d => !LooksLikeLoopback(d));
    }

    private static (string? DeviceName, string? Method, string? Path) TryReadResonanceLogsDevice()
    {
        foreach (var path in EnumerateConfigCandidates())
        {
            try
            {
                if (!File.Exists(path)) continue;
                using var doc = JsonDocument.Parse(File.ReadAllText(path));
                var root = doc.RootElement;
                var device = root.TryGetProperty("npcapDevice", out var npcapDevice)
                    ? npcapDevice.GetString()
                    : null;
                var method = root.TryGetProperty("method", out var methodValue)
                    ? methodValue.GetString()
                    : null;

                AppLog.Write($"npcap: Resonance Logs config path={path} method={method ?? "<missing>"} device={device ?? "<empty>"}");
                if (!string.IsNullOrWhiteSpace(device) || !string.IsNullOrWhiteSpace(method))
                    return (device?.Trim(), method?.Trim(), path);
            }
            catch (Exception ex)
            {
                AppLog.Write($"npcap: failed reading Resonance Logs config path={path}: {ex.Message}");
            }
        }

        return (null, null, null);
    }

    private static IEnumerable<string> EnumerateConfigCandidates()
    {
        var local = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
        var roaming = Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData);
        var seen = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        var results = new List<string>();

        foreach (var root in new[] { local, roaming })
        {
            if (string.IsNullOrWhiteSpace(root)) continue;

            foreach (var appDirName in new[] { "com.resonance-logs-cn", "resonance-logs-cn", "resonance-logs" })
            {
                foreach (var relative in new[]
                {
                    Path.Combine(appDirName, "stores", "packetCapture.json"),
                    Path.Combine(appDirName, "packetCapture.json")
                })
                {
                    var candidate = Path.Combine(root, relative);
                    if (seen.Add(candidate)) results.Add(candidate);
                }
            }

            string[] directories;
            try { directories = Directory.GetDirectories(root, "*resonance*", SearchOption.TopDirectoryOnly); }
            catch { directories = Array.Empty<string>(); }

            foreach (var dir in directories)
            {
                var name = Path.GetFileName(dir);
                if (!name.Contains("log", StringComparison.OrdinalIgnoreCase)) continue;
                foreach (var candidate in new[]
                {
                    Path.Combine(dir, "stores", "packetCapture.json"),
                    Path.Combine(dir, "packetCapture.json")
                })
                {
                    if (seen.Add(candidate)) results.Add(candidate);
                }
            }
        }

        return results;
    }

    private static bool IsUsableInterface(NetworkInterface nic)
    {
        if (nic.OperationalStatus != OperationalStatus.Up) return false;
        if (nic.NetworkInterfaceType is NetworkInterfaceType.Loopback or NetworkInterfaceType.Tunnel) return false;
        if (nic.GetPhysicalAddress().GetAddressBytes().Length == 0) return false;
        try
        {
            var props = nic.GetIPProperties();
            if (props.UnicastAddresses.Count == 0) return false;
            return props.GatewayAddresses.Any(g => IsRealGateway(g.Address));
        }
        catch { return false; }
    }

    private static int ScoreInterface(NetworkInterface nic)
    {
        var score = 100;
        if (nic.NetworkInterfaceType is NetworkInterfaceType.Ethernet or NetworkInterfaceType.Wireless80211) score += 40;
        var text = (nic.Name + " " + nic.Description).ToLowerInvariant();
        if (VirtualWords.Any(word => text.Contains(word, StringComparison.Ordinal))) score -= 100;
        try
        {
            var props = nic.GetIPProperties();
            if (props.UnicastAddresses.Any(a => a.Address.AddressFamily == System.Net.Sockets.AddressFamily.InterNetwork)) score += 20;
            if (props.GatewayAddresses.Any(g => g.Address.AddressFamily == System.Net.Sockets.AddressFamily.InterNetwork && IsRealGateway(g.Address))) score += 20;
        }
        catch { }
        return score;
    }

    private static bool DeviceMatchesInterface(NpcapDevice device, NetworkInterface nic, string normalizedId)
    {
        if (!string.IsNullOrWhiteSpace(normalizedId))
        {
            var normalizedDevice = NormalizeGuid(device.Name);
            if (normalizedDevice.Contains(normalizedId, StringComparison.OrdinalIgnoreCase)) return true;
        }
        if (!string.IsNullOrWhiteSpace(nic.Description) && device.Description.Contains(nic.Description, StringComparison.OrdinalIgnoreCase)) return true;
        if (!string.IsNullOrWhiteSpace(nic.Name) && device.Description.Contains(nic.Name, StringComparison.OrdinalIgnoreCase)) return true;
        return false;
    }

    private static readonly string[] VirtualWords =
    {
        "virtual", "vmware", "hyper-v", "virtualbox", "vpn", "tap", "tunnel",
        "wsl", "tailscale", "zerotier", "wireguard", "loopback", "bluetooth", "wan miniport"
    };

    private static bool LooksVirtual(NpcapDevice device)
    {
        var text = (device.Name + " " + device.Description).ToLowerInvariant();
        return VirtualWords.Any(word => text.Contains(word, StringComparison.Ordinal));
    }

    private static string NormalizeGuid(string value) =>
        value.Replace("\\Device\\NPF_", string.Empty, StringComparison.OrdinalIgnoreCase)
             .Replace("{", string.Empty)
             .Replace("}", string.Empty)
             .Trim();

    private static bool LooksLikeLoopback(NpcapDevice device)
    {
        var text = (device.Name + " " + device.Description).ToLowerInvariant();
        return text.Contains("loopback");
    }

    private static bool IsRealGateway(IPAddress address)
    {
        if (IPAddress.Any.Equals(address) || IPAddress.IPv6Any.Equals(address)) return false;
        if (IPAddress.None.Equals(address) || IPAddress.IPv6None.Equals(address)) return false;
        return !address.Equals(IPAddress.Loopback) && !address.Equals(IPAddress.IPv6Loopback);
    }
}
