# BPSR ReadyAlert

Lightweight Windows combat companion for **Blue Protocol: Star Resonance**. See combat results, party status, encounter mechanics and queue alerts without juggling separate overlays.

[Download latest release](https://github.com/Zudin987/BPSR-ReadyAlert/releases/latest) · [Project website](https://zudin987.github.io/projects/readyalert/)

<p align="center">
  <img src="docs/images/meter-normal.png" alt="ReadyAlert combat meter showing player damage, classes and Imagines" width="679">
</p>

## What it does

- **Combat meter:** Damage, DPS, healing, tank statistics and damage received, with Normal, Compact, Raid and Compact Raid layouts.
- **Party status:** Food and Serum activity, revive availability and Imagine tiers.
- **Encounter helpers:** Enrage countdown, supported boss mechanics and Imagine timing trackers.
- **History:** Save encounters locally, open HTML reports and compare two fights side by side.
- **Optional chat tools:** View game chat, translate supported non-English messages to English and read Guild/Party messages aloud.
- **Alerts:** See queue and party notifications alongside the combat tools.

[More screenshots](docs/images/) · [Encounter History details and screenshots](https://zudin987.github.io/projects/readyalert/#history)

## Requirements

- **64-bit Windows** and a supported BPSR client.
- [Npcap](https://npcap.com/#download), installed separately to read game network traffic.
- Internet access for optional online translation and speech features.

## Get started

1. Install Npcap, then download and run `BPSR-ReadyAlert.exe` from the latest release.
2. Open BPSR and enable the overlays and alerts you want in **Settings**.
3. If the meter has no game data, choose the network adapter used by BPSR in **Settings**.

**Ctrl+Shift+F10** shows or hides the combat and mechanics overlays.

## Important

ReadyAlert reads game network traffic through Npcap. It does not inject into the game or automate gameplay. This is an unofficial community tool, not affiliated with the game's developers or publishers. Game updates can affect compatibility.

[Build from source](docs/DEVELOPMENT.md) · [License](LICENSE) · [Third-party notices and credits](THIRD_PARTY_NOTICES.md)
