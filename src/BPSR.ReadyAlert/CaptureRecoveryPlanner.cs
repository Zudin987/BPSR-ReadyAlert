using System.Net;
using System.Net.NetworkInformation;
using System.Runtime.InteropServices;

namespace BPSR.ReadyAlert;

internal static class CaptureRecoveryPlanner
{
    internal static NpcapCapturePlan Refresh(NpcapCapturePlan current)
    {
        var devices = NpcapCapture.ListDevices();
        if (devices.Count == 0)
            throw new InvalidOperationException("Npcap is installed but no capture adapters are currently available.");

        var manual = CaptureRecoveryPolicy.IsManualPlan(current);
        var preferred = manual
            ? Array.Empty<string>()
            : GetPreferredActiveDeviceNames(devices);
        var selected = SelectDeviceFromSnapshot(current, devices, preferred, manual);
        if (selected is null)
        {
            if (manual)
                throw new InvalidOperationException(
                    "The manually selected Npcap adapter is not currently available: " + current.Primary.DeviceName);
            throw new InvalidOperationException("No usable Npcap adapter is currently available.");
        }

        var sameDevice = string.Equals(
            selected.Name,
            current.Primary.DeviceName,
            StringComparison.OrdinalIgnoreCase);
        var source = manual
            ? "User selected"
            : sameDevice
                ? current.Primary.Source
                : "Recovery auto-selected";

        var refreshed = new NpcapCapturePlan(
            [new NpcapCaptureCandidate(selected.Name, selected.Description, source)],
            SortAvailable(devices),
            current.ResonanceLogsConfigPath);

        // CaptureEngine and TrayApplicationContext intentionally share this plan
        // instance. Replacing its immutable snapshot keeps the tray's adapter label,
        // available-device list and later manual switch/rollback logic synchronized
        // with automatic recovery performed on the background capture thread.
        current.ReplaceWith(refreshed);
        return current;
    }

    internal static NpcapCapturePlan CreateWaitingPlan(string? manualDeviceName)
    {
        var manual = !string.IsNullOrWhiteSpace(manualDeviceName);
        return new NpcapCapturePlan(
            [new NpcapCaptureCandidate(
                manual ? manualDeviceName!.Trim() : string.Empty,
                manual ? "Waiting for manually selected network adapter" : "Waiting for active network adapter",
                manual ? "User selected" : "Recovery auto")],
            Array.Empty<NpcapDevice>(),
            null);
    }

    internal static NpcapCapturePlan PreserveUnavailableManual(
        string manualDeviceName,
        NpcapCapturePlan discoveredPlan)
    {
        var exact = discoveredPlan.AvailableDevices.FirstOrDefault(device =>
            string.Equals(device.Name, manualDeviceName, StringComparison.OrdinalIgnoreCase));
        if (exact is not null)
        {
            return new NpcapCapturePlan(
                [new NpcapCaptureCandidate(exact.Name, exact.Description, "User selected")],
                discoveredPlan.AvailableDevices,
                discoveredPlan.ResonanceLogsConfigPath);
        }

        return new NpcapCapturePlan(
            [new NpcapCaptureCandidate(
                manualDeviceName,
                "Waiting for manually selected network adapter",
                "User selected")],
            discoveredPlan.AvailableDevices,
            discoveredPlan.ResonanceLogsConfigPath);
    }

    internal static NpcapDevice? SelectDeviceFromSnapshot(
        NpcapCapturePlan current,
        IReadOnlyList<NpcapDevice> devices,
        IReadOnlyList<string> preferredActiveDeviceNames,
        bool manual)
    {
        if (manual)
        {
            return devices.FirstOrDefault(device =>
                string.Equals(device.Name, current.Primary.DeviceName, StringComparison.OrdinalIgnoreCase));
        }

        foreach (var preferred in preferredActiveDeviceNames)
        {
            var match = devices.FirstOrDefault(device =>
                string.Equals(device.Name, preferred, StringComparison.OrdinalIgnoreCase));
            if (match is not null) return match;
        }

        var currentDevice = devices.FirstOrDefault(device =>
            string.Equals(device.Name, current.Primary.DeviceName, StringComparison.OrdinalIgnoreCase));
        if (currentDevice is not null) return currentDevice;

        return devices.FirstOrDefault(device => !LooksLikeLoopback(device) && !LooksVirtual(device))
               ?? devices.FirstOrDefault(device => !LooksLikeLoopback(device))
               ?? devices.FirstOrDefault();
    }

    private static IReadOnlyList<string> GetPreferredActiveDeviceNames(IReadOnlyList<NpcapDevice> devices)
    {
        var interfaces = NetworkInterface.GetAllNetworkInterfaces()
            .Where(IsUsableInterface)
            .Select(nic => new
            {
                Nic = nic,
                Score = ScoreInterface(nic),
                NormalizedId = NormalizeGuid(nic.Id),
                IsBestRoute = IsBestIpv4Route(nic)
            })
            .OrderByDescending(item => item.IsBestRoute)
            .ThenByDescending(item => item.Score)
            .ThenByDescending(item => item.Nic.Speed)
            .ToArray();

        var result = new List<string>();
        foreach (var item in interfaces)
        {
            var device = devices.FirstOrDefault(candidate =>
                DeviceMatchesInterface(candidate, item.Nic, item.NormalizedId));
            if (device is null) continue;
            if (!result.Contains(device.Name, StringComparer.OrdinalIgnoreCase))
                result.Add(device.Name);
        }
        return result;
    }

    private static IReadOnlyList<NpcapDevice> SortAvailable(IReadOnlyList<NpcapDevice> devices) =>
        devices
            .OrderBy(LooksLikeLoopback)
            .ThenBy(LooksVirtual)
            .ThenBy(device => device.Description, StringComparer.OrdinalIgnoreCase)
            .ToArray();

    private static bool IsUsableInterface(NetworkInterface nic)
    {
        if (nic.OperationalStatus != OperationalStatus.Up) return false;
        if (nic.NetworkInterfaceType is NetworkInterfaceType.Loopback or NetworkInterfaceType.Tunnel) return false;
        try
        {
            var props = nic.GetIPProperties();
            if (props.UnicastAddresses.Count == 0) return false;
            return props.GatewayAddresses.Any(gateway => IsRealGateway(gateway.Address));
        }
        catch
        {
            return false;
        }
    }

    private static int ScoreInterface(NetworkInterface nic)
    {
        var score = 100;
        if (nic.NetworkInterfaceType is NetworkInterfaceType.Ethernet or NetworkInterfaceType.Wireless80211)
            score += 40;
        var text = (nic.Name + " " + nic.Description).ToLowerInvariant();
        if (VirtualWords.Any(word => text.Contains(word, StringComparison.Ordinal)))
            score -= 100;
        try
        {
            var props = nic.GetIPProperties();
            if (props.UnicastAddresses.Any(address =>
                    address.Address.AddressFamily == System.Net.Sockets.AddressFamily.InterNetwork))
                score += 20;
            if (props.GatewayAddresses.Any(gateway =>
                    gateway.Address.AddressFamily == System.Net.Sockets.AddressFamily.InterNetwork &&
                    IsRealGateway(gateway.Address)))
                score += 20;
        }
        catch { }
        return score;
    }

    private static bool IsBestIpv4Route(NetworkInterface nic)
    {
        try
        {
            var ipv4 = nic.GetIPProperties().GetIPv4Properties();
            if (ipv4 is null) return false;
            return TryGetBestInterfaceIndex(out var index) && ipv4.Index == index;
        }
        catch
        {
            return false;
        }
    }

    private static bool TryGetBestInterfaceIndex(out int index)
    {
        index = -1;
        try
        {
            // 1.1.1.1 is only used as a routing-table destination. No packet or DNS
            // request is sent by GetBestInterface.
            const uint routeProbe = 0x01010101;
            var result = GetBestInterface(routeProbe, out var nativeIndex);
            if (result != 0 || nativeIndex > int.MaxValue) return false;
            index = checked((int)nativeIndex);
            return true;
        }
        catch
        {
            return false;
        }
    }

    private static bool DeviceMatchesInterface(NpcapDevice device, NetworkInterface nic, string normalizedId)
    {
        if (!string.IsNullOrWhiteSpace(normalizedId))
        {
            var normalizedDevice = NormalizeGuid(device.Name);
            if (normalizedDevice.Contains(normalizedId, StringComparison.OrdinalIgnoreCase)) return true;
        }
        if (!string.IsNullOrWhiteSpace(nic.Description) &&
            device.Description.Contains(nic.Description, StringComparison.OrdinalIgnoreCase))
            return true;
        if (!string.IsNullOrWhiteSpace(nic.Name) &&
            device.Description.Contains(nic.Name, StringComparison.OrdinalIgnoreCase))
            return true;
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

    private static bool LooksLikeLoopback(NpcapDevice device)
    {
        var text = (device.Name + " " + device.Description).ToLowerInvariant();
        return text.Contains("loopback", StringComparison.Ordinal);
    }

    private static string NormalizeGuid(string value) =>
        value.Replace("\\Device\\NPF_", string.Empty, StringComparison.OrdinalIgnoreCase)
             .Replace("{", string.Empty)
             .Replace("}", string.Empty)
             .Trim();

    private static bool IsRealGateway(IPAddress address)
    {
        if (IPAddress.Any.Equals(address) || IPAddress.IPv6Any.Equals(address)) return false;
        if (IPAddress.None.Equals(address) || IPAddress.IPv6None.Equals(address)) return false;
        return !address.Equals(IPAddress.Loopback) && !address.Equals(IPAddress.IPv6Loopback);
    }

    [DllImport("iphlpapi.dll")]
    private static extern uint GetBestInterface(uint destinationAddress, out uint bestInterfaceIndex);
}
