# BPSR Ready Alert

A lightweight Windows companion for **Blue Protocol: Star Resonance (BPSR)** that combines fast Ready/Queue alerts with an optional customizable chat overlay.

**Website:** https://zudin987.github.io/projects/readyalert/

ReadyAlert is designed to run beside the official **Resonance Logs CN** DPS meter without modifying or replacing its files.

## What it does

### Ready / matchmaking alerts

ReadyAlert can notify you when:

- matchmaking reaches the accept / Ready popup;
- a party Ready Check starts;
- a party or dungeon activity confirmation/vote opens.

Alerts can use sound, app-local volume control, and optional desktop notifications.

### Chat Overlay

The optional Chat Overlay adds a separate lightweight floating BPSR chat window with:

- World, Guild/Team and All default tabs;
- custom tabs with add/edit/delete;
- selectable channels and minimum-level filtering;
- Show If Matches / Hide If Matches rules;
- case-insensitive matching;
- easy OR syntax such as `serum | food | raid`;
- one expression per line as OR;
- `AND` / `&&` support;
- advanced regex support with timeout and invalid-regex protection;
- timestamps and relative time;
- compact mode;
- font, bold, shadow, zebra rows, separators and channel colors;
- background, toolbar, text and whole-window opacity controls;
- Always on Top;
- click-through mode with a recovery hotkey;
- screen-edge collapse/expand;
- Smart Scroll with a new-message indicator while reading older chat;
- sticker hiding;
- local blocked-user list;
- configurable in-memory history;
- saved window position/size, tabs, filters and appearance;
- Per-Monitor-V2 DPI scaling;
- built-in chat capture diagnostics.

The overlay is **view-only**. It cannot send chat, inject into BPSR, modify the client, or automate gameplay.

## Quick start

1. Download `BPSR-ReadyAlert.exe` from the latest GitHub Release.
2. Make sure **Npcap** is installed. If Resonance Logs CN already uses Npcap, you normally need nothing extra.
3. Run ReadyAlert. On first run, select `resonance-logs-cn.exe` if it is not found automatically.
4. Leave ReadyAlert running in the system tray.

After setup, you can launch **BPSR Ready Alert only**; it can auto-launch Resonance Logs CN when needed.

Right-click the tray icon for Ready/Queue toggles, **Chat Overlay**, **Show Chat / Hide Chat**, **Network Adapter**, **Alert Volume**, logs and Resonance Logs CN settings.

ReadyAlert does not request Administrator elevation by default. If your Npcap installation restricts capture to administrators, Windows may require you to run ReadyAlert as administrator.

## Chat Overlay controls

- Drag the top grip to move the borderless window.
- Drag the outer frame/corners to resize it. The visible border is intentionally small, while the actual resize hit area is larger for easier grabbing.
- Use `⚙` to open Chat Overlay Settings.
- Use the arrow button or `Ctrl+Shift+F9` by default to collapse/expand at the configured screen edge.
- Use `Ctrl+Shift+F10` by default to toggle click-through.
- Use **+ Tab** to add a custom chat tab.

If the click-through hotkey cannot register, ReadyAlert automatically turns click-through back **off** so the overlay cannot become mouse-locked without a recovery key.

Closing the overlay with **×** or choosing **Hide Chat** hides only the window. Chat remains enabled and recent messages stay available. Unchecking **Chat Overlay** fully disables chat processing/UI and clears the in-memory chat history.

## Filters

Show/Hide tab filters are case-insensitive and support beginner-friendly syntax while still allowing real regex.

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

Rules:

- `serum | food | raid` means **serum OR food OR raid**;
- `OR` / `||` also means any expression may match;
- one expression per line is also OR;
- `AND` / `&&` means every expression in that group must match;
- compact regex such as `(raid|dungeon)` still works;
- matching is case-insensitive, so `serum` matches `Serum`, `SERUM` and `SeRuM`;
- short expressions are allowed — for example `PA`;
- invalid regex is shown in the editor and cannot crash the app;
- regex execution has a timeout and size guard so pathological patterns cannot hang the overlay.

Normal tab Show/Hide filtering can match sender name plus message content. This is useful when you want to include or hide a specific player's messages.

## Highlight sounds

ReadyAlert supports **up to 3 prioritized chat sound rules**.

Example:

1. `PA` → `boss.wav`
2. `serum` → `serum.wav`
3. `raid | dungeon` → `raid.wav`

Behavior:

- all three rules share one **Chat alert volume** slider;
- there is no per-rule volume and no cooldown setting;
- if one message matches multiple rules, the **first matching rule wins**, so only one notification sound plays;
- sound-rule matching checks **message content only**, not the sender/player name;
- matching is case-insensitive and uses the same safe OR / AND / regex engine;
- leave a sound path empty to use ReadyAlert's built-in alert sound;
- custom sound volume uses standard 16-bit PCM WAV files.

Private/Talk can also use its own highlight and sound while sharing the same Chat alert volume.

The global visual row-highlight rule is separate from sound rules and can match sender name plus message content.

## Chat settings UX

Settings are designed to be usable without editing configuration files.

- **Save changes** applies settings without closing the Settings window.
- Add/Edit Tab **Save tab** also applies without closing the tab editor.
- **Reset to defaults** restores chat settings after confirmation while preserving the current overlay position and size.
- When Always on Top is enabled, Settings and Add/Edit Tab dialogs stay above the overlay.

## Network handling

ReadyAlert captures **one selected Npcap adapter at a time**.

- By default it follows Resonance Logs CN's saved Npcap device when available.
- Otherwise it chooses an active physical Ethernet/Wi-Fi adapter and avoids common VPN/VM/tunnel adapters.
- Use **Network Adapter** in the tray menu to choose one manually.
- Choose **Follow Resonance Logs CN / Auto** to remove the manual override.

The selected adapter is saved and capture restarts immediately when changed.

### Shared capture architecture

Chat does **not** open a second Npcap capture.

Ready alerts and chat share the same existing pipeline:

`Npcap → BPSR-owned TCP filtering → TCP reassembly → FrameDown handling → zstd decompression → Notify dispatch`

Chat follows `ChitChatNtf.NotifyNewestChitChatMsgs` (`service 164931432`, method `0x01`) and manually decodes only the protobuf fields it needs. No Google.Protobuf runtime is required.

When Chat Overlay is off, chat protobuf/UI processing is skipped while the normal Ready/Queue path continues.

## Alerts and volume

Use tray-menu toggles to enable/disable individual ReadyAlert alerts. **Alert Volume** controls Ready/Queue alert playback only; it does not change Windows master volume.

Chat notification sounds use their separate **Chat alert volume** setting.

Duplicate Ready/Queue notifications are suppressed for a short period so one event does not repeatedly play the alert.

## Settings and recovery

ReadyAlert stores local app data under:

```text
%LOCALAPPDATA%\BPSR-ReadyAlert\
```

Chat history stays in memory only.

Persistent settings include overlay position/size, tabs, filters, appearance, hotkeys, channel colors, blocked users and sound rules. Settings writes are validated using a temporary file before replacement, and ReadyAlert keeps `settings.json.bak` for automatic recovery if the primary settings file becomes unreadable.

## Installation footprint and safety

There is no installer; the release is one self-contained Windows x64 EXE. **Npcap is an external dependency** and is not bundled or modified.

ReadyAlert never copies files into the Resonance Logs CN directory and does not replace its EXE, DLLs, updater or settings. It also does not inject into or modify the BPSR game process.

Packet/protocol details can change after BPSR updates. If alerts or chat capture stop working, use **Chat Settings → Advanced → Chat capture status** and/or **Open Log** from the tray menu when reporting the problem.

## Pin to Start

ReadyAlert creates/refreshes a Start Menu shortcut. Search for **BPSR Ready Alert**, right-click it, then choose **Pin to Start**. Windows keeps the final pin action user-controlled.

## Build

Local build requires the **.NET 10 SDK**:

```powershell
./scripts/prepare-build-assets.ps1
dotnet publish src/BPSR.ReadyAlert/BPSR.ReadyAlert.csproj -c Release -r win-x64 --self-contained true -o dist
```

`prepare-build-assets.ps1` reconstructs the bundled alert WAV and application icon from committed source payloads. Npcap is intentionally not redistributed by the build.

GitHub Actions publishes the self-contained EXE and runs it with `--build-smoke-test`. The smoke suite covers parser/protobuf behavior, Unicode text, filter safety, short filters, sound-rule priority/message-only matching, settings normalization, hotkeys, UI-fit checks, and prior crash regressions.

## Credits and protocol references

The Chat Overlay protocol work references the MIT-licensed **BPSR-ZDPS** project and community chat-overlay behavior documented in `THIRD_PARTY_NOTICES.md`.

## License

BPSR Ready Alert is licensed under the [MIT License](LICENSE). See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for third-party components and protocol references.
