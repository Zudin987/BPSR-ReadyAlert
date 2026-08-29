# BPSR Ready Alert

Windows companion for **Blue Protocol: Star Resonance** Ready/queue and party alerts with an optional view-only chat overlay.

**Website:** https://zudin987.github.io/projects/readyalert/

- **Ready / Queue alerts** — dedicated Queue Pop and Ready Check sounds with optional desktop notifications.
- **Party alerts** — detects incoming party invitations and party join requests, each with its own sound and independent tray toggle.
- **Chat Overlay** — World, Guild/Team, All and custom tabs with filters, keyword sounds, click-through and Always on Top.
- **English translation** — optional no-key translation for World, Guild and Party / Team chat.
- **English TTS** — optional Guild / Party speech only, with its own volume and quick toggle.
- **Automatic player identity** — detects your local BPSR character name and UID from the existing capture stream so your own Guild / Party messages can be skipped by TTS; an optional manual username override remains available.
- **Independent audio controls** — Queue/Ready/Party sounds share one core volume; Chat alerts and TTS keep separate volume paths.
- **Compact Settings** — ZDPS-inspired top tabs, flat sections, and slim accessible sliders.
- **Portable** — self-contained Windows x64 EXE; no .NET installation required.

## Use

1. Install **Npcap**.
2. Download `BPSR-ReadyAlert.exe` from [Releases](https://github.com/Zudin987/BPSR-ReadyAlert/releases/latest).
3. Open the EXE and select Resonance Logs CN / the correct network adapter if auto-detection misses it.
4. Leave ReadyAlert running in the system tray.
5. Use the tray toggles to independently enable/disable Queue Pop, Ready Check, Party Invite and Party Request alerts.
6. Enable **Desktop Notification** in the tray if you want Windows notifications in addition to sounds.
7. Enable **Chat Overlay** only if you want the chat window, translation or TTS. The single Chat Overlay toggle also controls whether the overlay is shown.
8. Under **Speech & translation**, ReadyAlert shows the detected BPSR username/UID. Leave the manual override empty to use detection, or enter your exact in-game name if detection is incorrect.

Party invite/request and local-player identity detection are part of core ReadyAlert and work even when Chat Overlay, translation and TTS are disabled.

ReadyAlert does **not** inject into BPSR, replace game files, send chat, or automate gameplay.

Translation/TTS use no-key Google Translate/gTTS-style web endpoints. They can be rate-limited or changed upstream; failures do not stop normal core alerts or local chat capture.

[Latest release](https://github.com/Zudin987/BPSR-ReadyAlert/releases/latest) · [License](LICENSE) · [Third-party notices](THIRD_PARTY_NOTICES.md)
