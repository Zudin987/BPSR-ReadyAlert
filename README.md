# BPSR ReadyAlert

A lightweight Windows combat companion for **Blue Protocol: Star Resonance**. See the information that matters during a fight without stacking multiple tools on top of the game.

[Download latest release](https://github.com/Zudin987/BPSR-ReadyAlert/releases/latest) · [Project website](https://zudin987.github.io/projects/readyalert/)

<p align="center">
  <img src="docs/images/meter-normal.png" alt="BPSR ReadyAlert DPS meter in Normal mode with player damage, classes and Imagines" width="679">
  <br>
  <em>DPS meter - Normal mode</em>
</p>

## Why ReadyAlert?

ReadyAlert is more than a DPS meter. It puts combat information that is easy to miss, hard to calculate, or normally hidden behind extra UI steps directly into one lightweight companion.

- **Party Food & Serum tracking** - See other players' active Food and Serum activity.
- **Revive status** - Know at a glance who can and cannot be revived, including revive-penalty cases.
- **Imagine tiers without hovering** - View Imagine tier information directly from the combat UI.
- **Built-in Enrage tracker** - See how much time remains before the boss enrages.
- **Boss-specific Imagine mechanics** - Built-in trackers for mechanics such as TINA Basilisk and Kartgriff help you time Imagine casts correctly.
- **Encounter History** - Keep past encounters in a local archive, open them as HTML in your browser, and compare fights with Compare mode.
- **Very lightweight** - Built in Rust and packaged at around **10 MB**.
- **Chat translation** - Automatically detect non-English game chat and translate it to English.
- **Guild & Party TTS** - Optionally read Guild and Party chat aloud while you play.

## What it tracks

### Combat

- Damage, DPS and healing
- Tank and damage-received statistics
- Normal, Compact, Raid and Compact Raid layouts
- Encounter history and player inspection

### Party information

- Food and Serum activity
- Revive availability and revive penalties
- Imagine tier information

### Encounter mechanics

- Boss Enrage timer
- Dungeon and encounter mechanics
- Built-in Imagine timing helpers for supported encounters
- Food, Serum and Imagine tracking in one place

### Chat

- View-only game chat overlay
- Custom chat tabs
- Automatic non-English to English translation
- Optional Guild and Party text-to-speech

## Screenshots

<details>
<summary><strong>Combat meter layouts</strong></summary>

**Normal - full combat view**

<p align="center">
  <img src="docs/images/meter-normal.png" alt="DPS meter showing player damage, classes and Imagines in Normal mode" width="679">
</p>

**Raid - 20 players in two columns**

<p align="center">
  <img src="docs/images/meter-raid.png" alt="Raid meter showing 20 players in two columns with damage and revive status" width="668">
</p>

**Compact - essentials in less space**

<p align="center">
  <img src="docs/images/meter-compact.png" alt="Compact meter with a smaller player list and damage values" width="402">
</p>

**Compact Raid - 20 players in a smaller footprint**

<p align="center">
  <img src="docs/images/meter-compact-raid.png" alt="Compact Raid meter showing 20 players in a smaller two-column layout" width="509">
</p>

</details>

<details>
<summary><strong>Tracker & Mechanics</strong></summary>

<p align="center">
  <img src="docs/images/tracker-mechanics.png" alt="Tracker and Mechanics overlay showing key stats, food and serum durations, and mechanic timers" width="387">
</p>

</details>

<details>
<summary><strong>Encounter History</strong></summary>

<p align="center">
  <img src="docs/images/archive-encounter-1.png" alt="BPSR ReadyAlert Encounter History archive showing a saved combat encounter" width="900">
  <br>
  <em>Encounter History - archive overview</em>
</p>

<p align="center">
  <img src="docs/images/archive-encounter-2.png" alt="BPSR ReadyAlert Encounter History details showing encounter information and report actions" width="900">
  <br>
  <em>Encounter History - encounter details</em>
</p>

</details>

<details>
<summary><strong>Chat Overlay</strong></summary>

<p align="center">
  <img src="docs/images/chat-overlay.png" alt="Chat Overlay with channel tabs, game messages and text-to-speech controls" width="617">
</p>

</details>

## Get started

**Requires:** 64-bit Windows and [Npcap](https://npcap.com/#download).

1. Install Npcap.
2. Download and run `BPSR-ReadyAlert.exe` from the latest release.
3. Open BPSR and enable the overlays and alerts you want in **Settings**.
4. If no game data appears, select the network adapter used by your game connection in **Settings**.

Press **Ctrl+Shift+F10** to show or hide the combat and mechanics overlays.

## Encounter History

ReadyAlert keeps encounters in a local archive that can be opened as HTML reports in your browser. Use **Compare** mode to place two encounters side by side and quickly spot differences in performance, party composition and fight context.

## Built for lightweight use

ReadyAlert is built in **Rust** and designed to stay small and responsive. The release is around **10 MB**, making it suitable as a background companion while you play.

## Chat without losing focus

The Chat Overlay can watch supported game chat channels and automatically translate non-English messages to English. Guild and Party chat can also be sent to TTS when enabled, so important messages can be heard without keeping the chat window in view.

## About

ReadyAlert reads game network traffic through Npcap. It does not inject into the game or automate gameplay. This is an unofficial community project.

[Build from source](docs/DEVELOPMENT.md) · [License](LICENSE) · [Third-party notices and credits](THIRD_PARTY_NOTICES.md)
