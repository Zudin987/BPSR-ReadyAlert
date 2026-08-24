# BPSR Ready Alert

A tiny Windows companion for **Blue Protocol: Star Resonance** that plays a sound when:

- matchmaking reaches the **accept / Ready** popup;
- a party **Ready Check** starts;
- a party/dungeon activity vote opens.

It is designed to run beside the official **Resonance Logs CN** DPS meter without modifying or replacing any of its files, so the CN meter keeps using its normal updater.

## Download / use

1. Open **Releases** and download `BPSR-ReadyAlert.exe` from **Latest Build**.
2. Put the EXE anywhere you want to keep it.
3. Make sure **Npcap** is installed. If you already use Resonance Logs CN with Npcap, there is normally nothing extra to install.
4. Run `BPSR-ReadyAlert.exe`.
5. On first run, Ready Alert tries to find `resonance-logs-cn.exe`. If it cannot, select the EXE once.
6. From then on, launch **BPSR Ready Alert only**. It automatically starts Resonance Logs CN if the meter is not already running.

The app stays in the system tray. Right-click the tray icon for **Test Alert Sound**, alert toggles, **Network Adapter**, **Alert Volume**, log access, or to change the Resonance Logs CN path.

Ready Alert does not request Administrator/UAC elevation by default. Some Npcap installations can be configured to allow capture only to administrators; on such systems Windows may require the app to be run as administrator.

## Network adapter selection

Ready Alert captures **one Npcap adapter only**.

- If you have not selected an adapter in Ready Alert, it first follows Resonance Logs CN's saved `npcapDevice` when Resonance Logs itself is configured for Npcap.
- If that is unavailable, Ready Alert auto-selects an active physical adapter with an IP address, gateway, and MAC address, preferring Ethernet/Wi-Fi and avoiding common VPN/VM/tunnel adapters.
- Use **Network Adapter** in the tray menu to explicitly choose any Npcap adapter yourself. The choice is saved and capture restarts immediately on the selected adapter.
- Choose **Follow Resonance Logs CN / Auto** to remove the Ready Alert override.

Ready Alert does not scan all adapters in parallel.

## Alert volume

Use **Alert Volume** in the tray menu to set Ready Alert's own sound level from **Mute to 100%** in 10% steps. This changes only the alert playback volume; it does not change Windows master volume.

## Pin to Start

The app automatically creates / refreshes a **BPSR Ready Alert** shortcut in your Start Menu. Search for **BPSR Ready Alert**, right-click it, then choose **Pin to Start**.

Windows does not provide a reliable supported API for silently pinning an app, so the final **Pin to Start** click remains user-controlled.

## What gets installed?

There is no installer. The release is one self-contained `BPSR-ReadyAlert.exe`.

Npcap is an external dependency and is **not** bundled or modified by Ready Alert. The app extracts its bundled alert WAV and versioned icon to:

```text
%LOCALAPPDATA%\BPSR-ReadyAlert\assets\
```

It never copies files into the Resonance Logs CN directory and does not replace the CN executable, DLLs, updater, or settings.

## Alert detection

The app passively captures TCP traffic through Npcap and performs TCP/game-frame reassembly using the same message-type rules as BPSR-ZDPS. Captured packets are filtered to TCP endpoints owned by a running BPSR-family game process before game-protocol parsing.

- Ready Check open: `WorldNtf` service `1664308034`, method `0x46` (`NotifyAllMemberReady`).
- Ready Check response/update: method `0x47` (`NotifyCaptainReady`) is observed but does **not** start the alert.
- Match found: `MatchNtf` service `822849903`, method `0x04` (`EnterMatchResult`), then protobuf `MatchInfo.matchStatus == 2` (`WaitReady`).
- Party/dungeon vote: `GrpcTeamNtf` service `966773353`, method `0x0E` (`NotifyTeamActivityState`), then protobuf `TeamActivity.state == 3` (`Voting`).
- Protocol message types `0..8` are consumed correctly; only `FrameDown` (`6`) is recursively decoded for nested server notifications. `FrameUp` (`5`) is not treated as `FrameDown`.
- IPv4 and IPv6 game-frame parsing is supported; BPSR process ownership filtering currently follows ZDPS's IPv4 TCP owner-table approach.
- Ethernet/raw-IP Npcap datalink formats are supported, including common VLAN headers.
- Zstd-compressed notify and FrameDown payloads are supported.
- Duplicate alerts are suppressed for a few seconds.

The default sound is the user-selected `LetsDoThis` alert bundled into the EXE at build time.

## Build

GitHub Actions builds the self-contained Windows x64 EXE. Local build requires the .NET 8 SDK:

```powershell
./scripts/prepare-build-assets.ps1
dotnet publish src/BPSR.ReadyAlert/BPSR.ReadyAlert.csproj -c Release -r win-x64 --self-contained true -o dist
```

`prepare-build-assets.ps1` reconstructs the bundled alert WAV and validates/repairs the bundled application ICO before compilation. Npcap is intentionally not downloaded or redistributed by the build.

## Notes

This is an unofficial community utility. It does not inject into or modify the game process. Packet protocol details can change after game updates; if an alert stops working, use **Open Log** from the tray menu when reporting it.

## License

BPSR Ready Alert source code is licensed under the MIT License. See `THIRD_PARTY_NOTICES.md` for third-party components and protocol references.
