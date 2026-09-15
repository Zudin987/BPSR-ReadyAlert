# BPSR ReadyAlert

A lightweight Windows companion for **Blue Protocol: Star Resonance**. Track fights, get ready alerts, and keep game chat on screen.

**[Download for Windows](https://github.com/Zudin987/BPSR-ReadyAlert/releases/latest/download/BPSR-ReadyAlert.exe)** · [Release notes](https://github.com/Zudin987/BPSR-ReadyAlert/releases/latest)

<p align="center">
  <img src="docs/images/meter-normal.png" alt="DPS meter in Normal mode with player damage, classes and Imagines" width="679">
  <br>
  <em>DPS meter - Normal mode (more screenshots below)</em>
</p>

## Main features

- **Combat meter** - See damage, healing and tank stats in Normal, Compact or 20-player Raid layouts.
- **Tracker & Mechanics** - Keep key stats, food and serum durations, and mechanic timers visible.
- **Encounter History** - Review past fights in a local archive and share HTML reports.
- **Ready & Queue alerts** - Get notified about ready checks and matchmaking queues.
- **Chat Overlay** - Read game chat in custom tabs, with optional translation and text-to-speech (TTS).

## Get started

**Requires:** 64-bit Windows and [Npcap](https://npcap.com/#download).

1. Install Npcap.
2. [Download ReadyAlert](https://github.com/Zudin987/BPSR-ReadyAlert/releases/latest/download/BPSR-ReadyAlert.exe) and run `BPSR-ReadyAlert.exe`.
3. Open BPSR, then enable the overlays and alerts you want in **Settings**.
4. If no game data appears, select the network adapter used by your game connection in **Settings**.

Press **Ctrl+Shift+F10** to show or hide the combat and mechanics overlays. ReadyAlert can stay in the system tray and update itself when a new release is available.

## Screenshots

<details>
<summary><strong>More meter layouts - Raid, Compact and Compact Raid</strong></summary>

**Raid - 20 players in two columns**

<p align="center">
  <img src="docs/images/meter-raid.png" alt="Raid meter showing 20 players in two columns with damage and revive status" width="668">
</p>

**Compact - a smaller view of the essentials**

<p align="center">
  <img src="docs/images/meter-compact.png" alt="Compact meter with a single player list and damage values" width="402">
</p>

**Compact Raid - the raid list in less space**

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
<summary><strong>Chat Overlay</strong></summary>

<p align="center">
  <img src="docs/images/chat-overlay.png" alt="Chat Overlay with channel tabs, game messages and a text-to-speech control" width="617">
</p>

</details>

## About

ReadyAlert reads game network traffic through Npcap. It does not inject into the game or automate gameplay. This is an unofficial community project.

[Build from source](docs/DEVELOPMENT.md) · [License](LICENSE) · [Third-party notices and credits](THIRD_PARTY_NOTICES.md)
