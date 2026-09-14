# BPSR ReadyAlert

A lightweight native Windows companion for **Blue Protocol: Star Resonance** with combat meters, encounter tools, alerts and chat overlays.

<p align="center">
  <img src="docs/images/meter-raid.webp" alt="BPSR ReadyAlert DPS Meter in Raid mode" width="900">
</p>

**[Download ReadyAlert](https://github.com/Zudin987/BPSR-ReadyAlert/releases/latest/download/BPSR-ReadyAlert.exe)** · [Latest release](https://github.com/Zudin987/BPSR-ReadyAlert/releases/latest) · [Report an issue](https://github.com/Zudin987/BPSR-ReadyAlert/issues)

## Highlights

- **DPS Meter** — Damage, Heal and Tank views with Normal, Compact and Raid layouts.
- **Tracker & Mechanics** — target/boss timers, mechanics, consumables and event tracking.
- **Encounter History** — keeps local encounter records with an HTML archive for review and sharing.
- **Ready & Queue alerts** — native alerts for ready checks and queue events.
- **Chat Overlay** — view-only game chat with tabs plus optional translation and TTS.
- **Native & lightweight** — Rust + Win32, no browser runtime, with automatic updates built in.

## Tracker & Mechanics

<p align="center">
  <img src="docs/images/tracker-mechanics.webp" alt="BPSR ReadyAlert Tracker and Mechanics overlay" width="700">
</p>

## Install

**Requirements:** 64-bit Windows and [Npcap](https://npcap.com/#download).

1. Install Npcap.
2. Download and run `BPSR-ReadyAlert.exe` from the latest release.
3. Open BPSR. If no data appears, select the network adapter carrying the game connection in **Settings**.
4. Enable the overlays and alerts you want.

ReadyAlert can stay in the system tray while you play. **Ctrl+Shift+F10** toggles the DPS and Mechanics overlays.

## Updates

ReadyAlert checks for published updates and can update itself. Release downloads and SHA-256 checksums remain available on the Releases page if you prefer to update manually.

## Notes

ReadyAlert reads game network traffic through Npcap. It does **not** inject into the game or automate gameplay. Game updates and regional differences can occasionally require a ReadyAlert update.

Unofficial community project, not affiliated with BPSR's developers or publishers.

<details>
<summary><strong>Build from source</strong></summary>

```powershell
scripts\prepare-build-assets.ps1
cargo test --manifest-path rust/Cargo.toml --all-targets --locked
cargo build --manifest-path rust/Cargo.toml --release --locked
```

See [Development](docs/DEVELOPMENT.md) for the source layout and full native validation workflow.

</details>

[License](LICENSE) · [Third-party notices and credits](THIRD_PARTY_NOTICES.md)
