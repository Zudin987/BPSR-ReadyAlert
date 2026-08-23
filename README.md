# BPSR Ready Alert

A tiny Windows companion for **Blue Protocol: Star Resonance** that plays a sound when:

- matchmaking reaches the **accept / Ready** popup;
- a party **Ready Check** starts.

It is designed to be used together with the official **Resonance Logs CN** DPS meter without modifying or replacing any of its files, so the CN meter can keep using its own updater normally.

## Download / use

1. Open **Releases** and download `BPSR-ReadyAlert.exe` from **Latest Build**.
2. Put the EXE anywhere you want to keep it.
3. Run it and accept the Windows Administrator prompt. Administrator access is required by WinDivert packet capture.
4. On first run, BPSR Ready Alert tries to find `resonance-logs-cn.exe`. If it cannot, select the EXE once.
5. From then on, launch **BPSR Ready Alert only**. It automatically starts Resonance Logs CN if the meter is not already running.

The app stays in the system tray. Right-click the tray icon for **Test Alert Sound**, alert toggles, log access, or to change the Resonance Logs CN path.

## Pin to Start

The app automatically creates / refreshes a **BPSR Ready Alert** shortcut in your Start Menu. Search for **BPSR Ready Alert**, right-click it, then choose **Pin to Start**.

Windows does not provide a reliable supported API for silently pinning an app, so the final **Pin to Start** click remains user-controlled.

## What gets installed?

There is no installer. The release is one self-contained `BPSR-ReadyAlert.exe`.

At runtime it extracts its private WinDivert runtime and default alert WAV to:

```text
%LOCALAPPDATA%\BPSR-ReadyAlert\
```

It never copies files into the Resonance Logs CN directory and does not replace the CN executable, DLLs, updater, or settings.

## Alert detection

The app passively watches inbound TCP traffic using WinDivert in sniff / receive-only mode.

- Ready Check: `WorldNtf` service `1664308034`, method `0x46` (`NotifyAllMemberReady`).
- Match found: `MatchNtf` service `822849903`, method `0x04` (`EnterMatchResult`), then protobuf `MatchInfo.matchStatus == 2` (`WaitReady`).
- Zstd-compressed notify and nested frames are supported.
- Duplicate alerts are suppressed for a few seconds.

The default sound is the selected `LetsDoThis.wav` used by BPSR-ZDPS; the build verifies it against the exact SHA-256 of the file selected for this project.

## Build

GitHub Actions builds the self-contained Windows x64 EXE. Local build requires the .NET 8 SDK:

```powershell
./scripts/prepare-build-assets.ps1
dotnet publish src/BPSR.ReadyAlert/BPSR.ReadyAlert.csproj -c Release -r win-x64 --self-contained true -o dist
```

`prepare-build-assets.ps1` downloads the official WinDivert 2.2.2 x64 runtime and the selected `LetsDoThis.wav`, then verifies their SHA-256 hashes before compilation.

## Notes

This is an unofficial community utility. It does not inject into or modify the game process. Packet protocol details can change after game updates; if an alert stops working, use **Open Log** from the tray menu when reporting it.

## License

BPSR Ready Alert source code is licensed under the MIT License. See `THIRD_PARTY_NOTICES.md` for third-party components and protocol references.
