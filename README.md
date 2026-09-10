# BPSR ReadyAlert

Lightweight Windows companion for **Blue Protocol: Star Resonance** with combat meters, encounter tracking, queue alerts and an optional chat overlay.

[Download latest release](https://github.com/Zudin987/BPSR-ReadyAlert/releases/latest) · [Project website](https://zudin987.github.io/projects/readyalert/) · [Report an issue](https://github.com/Zudin987/BPSR-ReadyAlert/issues)

## Get running

Requires **64-bit Windows** and [Npcap](https://npcap.com/#download), installed separately.

1. Install Npcap.
2. Download `BPSR-ReadyAlert.exe` from the latest release and run it.
3. Open BPSR. If capture is empty, choose the correct network adapter in **Settings**.
4. Enable the overlays and alerts you want.

**Ctrl+Shift+F10** toggles the DPS and Mechanics overlays. The app can stay in the system tray while you play.

## Features

- **Combat:** DPS, Heal and Tank meters, encounter history, Copy as Image and player inspection.
- **Tracking:** live target HP and Enrage timer, Dungeon Mechanics, Food/Serum and Imagine tracking.
- **Alerts and chat:** Ready/queue/party alerts and a view-only chat overlay with optional translation and TTS.

## Capture and troubleshooting

ReadyAlert reads game network traffic through Npcap. It uses native Rust and Win32, with no injection or gameplay automation. Client detection covers CN, Global, TW, JP/KR and SEA; game-data mappings can still vary by region/version.

- **No combat or chat data:** confirm Npcap is installed and the selected adapter carries your game connection.
- **Hidden overlays:** check their Settings controls and the overlay hotkey.
- **Translation or speech fails:** these optional features depend on online services. Provider changes or rate limits can affect them independently of packet capture.
- **A game update breaks parsing:** check Releases for an update. When reporting a problem, include your app version, game region and the steps that reproduce it.

## Build

```powershell
scripts\prepare-build-assets.ps1
cargo build --manifest-path rust/Cargo.toml --release
```

See [Development](docs/DEVELOPMENT.md) for the build workflow and source layout.

Unofficial community tool, not affiliated with BPSR's developers or publishers.

[License](LICENSE) · [Third-party notices and credits](THIRD_PARTY_NOTICES.md)
