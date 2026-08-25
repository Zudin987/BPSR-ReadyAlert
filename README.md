# BPSR Ready Alert

A small Windows companion for **Blue Protocol: Star Resonance** that plays a sound when matchmaking reaches the accept/Ready popup, a party Ready Check starts, or a party/dungeon activity vote opens.

**Website:** https://zudin987.github.io/projects/readyalert/

It is designed to run beside the official **Resonance Logs CN** DPS meter without modifying or replacing its files.

## Quick start

1. Download `BPSR-ReadyAlert.exe` from the latest GitHub Release.
2. Make sure **Npcap** is installed. If Resonance Logs CN already uses Npcap, you normally need nothing extra.
3. Run Ready Alert. On first run, select `resonance-logs-cn.exe` if it is not found automatically.
4. After setup, you can launch **BPSR Ready Alert only**; it starts Resonance Logs CN when needed.

The app stays in the system tray. Right-click the tray icon for **Test Alert Sound**, alert toggles, **Network Adapter**, **Alert Volume**, logs, or the Resonance Logs CN path.

Ready Alert does not request Administrator elevation by default. If your Npcap installation restricts capture to administrators, Windows may require you to run it as administrator.

## Network adapter

Ready Alert captures **one Npcap adapter** at a time.

- By default it follows Resonance Logs CN's saved Npcap device when available.
- Otherwise it chooses an active physical Ethernet/Wi-Fi adapter and avoids common VPN/VM/tunnel adapters.
- Use **Network Adapter** in the tray menu to choose one manually.
- Choose **Follow Resonance Logs CN / Auto** to remove the manual override.

The selected adapter is saved and capture restarts immediately when changed.

## Alerts and volume

Use tray-menu toggles to enable/disable individual alerts. **Alert Volume** changes only Ready Alert's own playback level; it does not change Windows master volume.

Duplicate notifications are suppressed for a short period so one Ready event does not repeatedly play the sound.

## Installation footprint and safety

There is no installer; the release is one self-contained EXE. Npcap is an external dependency and is not bundled or modified.

Ready Alert stores its extracted alert sound/icon and local app data under:

```text
%LOCALAPPDATA%\BPSR-ReadyAlert\
```

It never copies files into the Resonance Logs CN directory and does not replace the CN executable, DLLs, updater, or settings. It does not inject into or modify the BPSR game process.

Packet/protocol details can change after game updates. If an alert stops working, use **Open Log** from the tray menu when reporting it.

## Pin to Start

Ready Alert creates/refreshes a Start Menu shortcut. Search for **BPSR Ready Alert**, right-click it, then choose **Pin to Start**. Windows keeps the final pin action user-controlled.

## Build

Local build requires the **.NET 10 SDK**:

```powershell
./scripts/prepare-build-assets.ps1
dotnet publish src/BPSR.ReadyAlert/BPSR.ReadyAlert.csproj -c Release -r win-x64 --self-contained true -o dist
```

`prepare-build-assets.ps1` reconstructs the bundled alert WAV and application icon from committed source payloads. Npcap is intentionally not redistributed by the build.

## License

BPSR Ready Alert is licensed under the [MIT License](LICENSE). See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for third-party components and protocol references.
