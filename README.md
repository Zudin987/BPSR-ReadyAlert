# BPSR ReadyAlert

Lightweight native Windows companion for **Blue Protocol: Star Resonance**. Rust + Win32 + Npcap. No injection or gameplay automation.

**Download:** [BPSR-ReadyAlert.exe](https://github.com/Zudin987/BPSR-ReadyAlert/releases/latest/download/BPSR-ReadyAlert.exe)  
Requires [Npcap](https://npcap.com/#download).

## Features

- DPS / Heal / Tank meter with history, exports and player inspection.
- Live target HP + Enrage, Dungeon Mechanics, Food/Serum and Imagine tracking.
- Ready/queue/party alerts plus a view-only chat overlay with optional translation/TTS.

## Run

1. Install Npcap.
2. Download the EXE.
3. Run it. If capture is empty, select the correct network adapter in Settings.

`Ctrl+Shift+F10` toggles the DPS + Mechanics overlays.

## Build

```powershell
scripts\prepare-build-assets.ps1
cargo build --manifest-path rust/Cargo.toml --release
```

[Website](https://zudin987.github.io/projects/readyalert/) · [Releases](https://github.com/Zudin987/BPSR-ReadyAlert/releases/latest) · [Development](docs/DEVELOPMENT.md) · [License](LICENSE) · [Third-party notices](THIRD_PARTY_NOTICES.md)
