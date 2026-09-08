# BPSR Ready Alert — Native Rust rewrite

This directory is the native Windows rewrite of BPSR Ready Alert. It intentionally keeps the existing `%LOCALAPPDATA%\\BPSR-ReadyAlert\\settings.json`, chat-log folder and Npcap behavior compatible with v1.3.6 while removing the .NET/WinForms runtime from the hot path.

## Architecture

- `npcap.rs`: dynamically loads the installed Npcap `wpcap.dll`; no import library is bundled.
- `game_filter.rs`: maps BPSR/StarSEA TCP endpoints to game process IDs through Windows TCP owner tables.
- `capture.rs`: zero-copy packet intake, bounded TCP reassembly, mid-stream BPSR frame synchronization, zstd decoding and alert dispatch.
- `proto.rs`: allocation-light protobuf field scanning for Ready/queue/party/chat/player-identity events.
- `chat.rs`: bounded local-log and translation/TTS workers; network/audio work never runs on the capture thread.
- `win.rs`: native Win32 tray icon, notification balloons and chat overlay; no WebView/Electron/WinForms runtime.

The old C# source remains in `src/BPSR.ReadyAlert` during the migration so live behavior can be compared and releases can fall back safely until the native build has passed Windows CI and live Npcap testing.

## Build

```powershell
./scripts/prepare-build-assets.ps1
cargo build --manifest-path rust/Cargo.toml --release
```

Output: `rust/target/release/BPSR-ReadyAlert.exe`.

## Compatibility notes

The Rust build reads and writes the same camelCase `settings.json`. Core alert toggles, desktop notifications, Npcap adapter selection, chat tabs/filters, local logs, translation and Guild/Party TTS settings are preserved. Advanced chat appearance fields remain readable for compatibility; the native overlay deliberately uses one Windows text surface instead of recreating WinForms owner-draw controls.
