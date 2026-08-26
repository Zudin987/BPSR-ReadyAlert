# BPSR Ready Alert

A small Windows companion for **Blue Protocol: Star Resonance** that plays a sound when matchmaking reaches the accept/Ready popup, a party Ready Check starts, or a party/dungeon activity vote opens.

**Website:** https://zudin987.github.io/projects/readyalert/

It is designed to run beside the official **Resonance Logs CN** DPS meter without modifying or replacing its files.

## Quick start

1. Download `BPSR-ReadyAlert.exe` from the latest GitHub Release.
2. Make sure **Npcap** is installed. If Resonance Logs CN already uses Npcap, you normally need nothing extra.
3. Run Ready Alert. On first run, select `resonance-logs-cn.exe` if it is not found automatically.
4. After setup, you can launch **BPSR Ready Alert only**; it starts Resonance Logs CN when needed.

The app stays in the system tray. Right-click the tray icon for **Test Alert Sound**, alert toggles, **Chat Overlay**, **Show Chat / Hide Chat**, **Network Adapter**, **Alert Volume**, logs, or the Resonance Logs CN path.

Ready Alert does not request Administrator elevation by default. If your Npcap installation restricts capture to administrators, Windows may require you to run it as administrator.

## Chat Overlay (v1.1 RC3)

The optional **Chat Overlay** is based on public BPSR-ZDPS protocol definitions and borrows several overlay UX ideas from community chat tools while keeping ReadyAlert's own lightweight shared-capture architecture. It is **off by default** and can be enabled/disabled from the system tray.

RC3 provides:

- borderless floating chat window with no normal Windows title bar;
- World, Guild/Team, and All default tabs;
- custom tabs with add/edit/delete, selectable channels, minimum level, Show-if and Hide-if rules;
- player name/message display, timestamps, compact/expanded layouts and sticker hiding;
- Smart Scroll: follow new chat while at the bottom, pause when the user scrolls up, then show a **new messages** button;
- Always on Top;
- click-through mode with a configurable global recovery hotkey (default `Ctrl+Shift+F10`);
- screen-edge collapse/expand with a configurable hotkey (default `Ctrl+Shift+F9`) and Left/Right/Top/Bottom side;
- configurable font family/size, bold text, text shadow, separators, zebra shading and channel color strip;
- customizable per-channel text colors;
- separate background-surface, toolbar, text and whole-window opacity controls;
- keyword/regex row highlighting with optional sound;
- dedicated Private/Talk highlight and optional sound;
- copy player name / UID and a local blocked-user list;
- configurable maximum in-memory history;
- persistent window position/size, tabs, filters, hotkeys, colors and appearance;
- Per-Monitor-V2 DPI scaling;
- a small **Chat Capture Status** diagnostic page using counters from the same shared capture pipeline;
- validated `settings.json` writes plus automatic `.bak` recovery if the primary settings file becomes unreadable.

The overlay is view-only. It does not send chat messages, inject into BPSR, modify the client, or automate gameplay.

Closing the custom **×** button or choosing **Hide Chat** only hides the window; chat remains enabled so recent messages can still appear when you show it again. Unchecking **Chat Overlay** fully disables chat parsing, closes the chat UI and clears its in-memory history.

### Overlay controls

- Drag the small `≡` grip to move the borderless window.
- Drag any outer edge/corner to resize it.
- Use the arrow button to collapse to the configured screen edge.
- Use `⚙` to open Chat Overlay Settings.
- The default click-through toggle is `Ctrl+Shift+F10`.
- The default collapse/expand toggle is `Ctrl+Shift+F9`.

If a click-through hotkey cannot be registered, ReadyAlert automatically turns click-through back **off** so the overlay cannot become mouse-locked without a recovery key.

### Better Show/Hide filters

ReadyAlert keeps regex support but makes matching **case-insensitive** and adds beginner-friendly multi-filter operators.

Examples:

```text
serum
serum | food | raid
serum
food
raid
boss AND hard
boss&&hard
(raid|dungeon)
```

- `serum | food | raid` means **serum OR food OR raid**.
- `OR` / `||` also means any expression may match.
- A separate pattern on each line is also treated as OR.
- `AND` / `&&` means every expression in that group must match.
- Compact regex alternation still works normally, for example `(raid|dungeon)`.
- Matching is case-insensitive, so `serum` matches `Serum`, `SERUM`, or `SeRuM`.
- Invalid regex is shown in the editor and cannot crash the app.
- Matching has a timeout and filter-size guard so pathological regex cannot hang the overlay indefinitely.

`Show If Matches` includes matching text messages. `Hide If Matches` excludes matching text messages. The global Highlight rule uses the same safe expression engine and checks both sender name and message text.

### Chat network handling

The overlay follows the same BPSR `ChitChatNtf.NotifyNewestChitChatMsgs` path used by ZDPS (`service 164931432`, method `0x01`) and manually decodes only the fields it needs. No Google.Protobuf runtime is added.

Chat does **not** open a second Npcap capture. It consumes ReadyAlert's existing BPSR-owned packet stream after the existing TCP reassembly, FrameDown handling, zstd decompression and Notify framing. When **Chat Overlay** is off, the chat parser and UI are skipped; the stable Ready/Queue capture path continues normally.

Chat history stays in memory only. Overlay settings and the local blocked-user list are persisted in `settings.json`. The store validates a temporary write before replacing the primary file and keeps a `settings.json.bak` recovery copy.

## Network adapter

Ready Alert captures **one selected Npcap adapter** at a time.

- By default it follows Resonance Logs CN's saved Npcap device when available.
- Otherwise it chooses an active physical Ethernet/Wi-Fi adapter and avoids common VPN/VM/tunnel adapters.
- Use **Network Adapter** in the tray menu to choose one manually.
- Choose **Follow Resonance Logs CN / Auto** to remove the manual override.

The selected adapter is saved and capture restarts immediately when changed. Chat automatically follows the exact same shared capture stream.

## Alerts and volume

Use tray-menu toggles to enable/disable individual alerts. **Alert Volume** changes only Ready Alert's own normal alert playback level; it does not change Windows master volume.

Duplicate Ready/Queue notifications are suppressed for a short period so one event does not repeatedly play the sound.

Chat highlight/private notification sounds can use a user-selected WAV. If no custom WAV is selected, the built-in ReadyAlert alert sound is used.

## Installation footprint and safety

There is no installer; the release is one self-contained EXE. Npcap is an external dependency and is not bundled or modified.

Ready Alert stores its extracted alert sound/icon and local app data under:

```text
%LOCALAPPDATA%\BPSR-ReadyAlert\
```

It never copies files into the Resonance Logs CN directory and does not replace the CN executable, DLLs, updater, or settings. It does not inject into or modify the BPSR game process.

Packet/protocol details can change after game updates. If an alert or chat capture stops working, open **Chat Settings → Advanced → Chat capture status...** and/or **Open Log** from the tray menu when reporting it.

## Pin to Start

Ready Alert creates/refreshes a Start Menu shortcut. Search for **BPSR Ready Alert**, right-click it, then choose **Pin to Start**. Windows keeps the final pin action user-controlled.

## Build

Local build requires the **.NET 10 SDK**:

```powershell
./scripts/prepare-build-assets.ps1
dotnet publish src/BPSR.ReadyAlert/BPSR.ReadyAlert.csproj -c Release -r win-x64 --self-contained true -o dist
```

`prepare-build-assets.ps1` reconstructs the bundled alert WAV and application icon from committed source payloads. Npcap is intentionally not redistributed by the build.

GitHub Actions also executes the published EXE with `--build-smoke-test`. That mode validates filter rules, invalid-regex safety, hotkey parsing, Unicode Malay/Chinese protobuf decoding, sticker decoding, timestamps and chat-settings normalization without requiring a live BPSR session.

## License

BPSR Ready Alert is licensed under the [MIT License](LICENSE). See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for third-party components and protocol references.
