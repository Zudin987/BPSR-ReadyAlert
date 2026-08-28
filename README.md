# BPSR Ready Alert

Windows companion for **Blue Protocol: Star Resonance** Ready/queue and party alerts with an optional view-only chat overlay.

**Website:** https://zudin987.github.io/projects/readyalert/

- **Ready / Queue alerts** — sound alerts and optional desktop notifications.
- **Party alerts** — detects incoming party invitations and party join requests, using the core ReadyAlert sound plus optional desktop notifications.
- **Chat Overlay** — World, Guild/Team, All and custom tabs with filters, keyword sounds, click-through and Always on Top.
- **English translation** — optional no-key translation for World, Guild and Party / Team chat.
- **English TTS** — optional Guild / Party speech only, with its own volume and quick toggle.
- **Independent audio controls** — core ReadyAlert, Chat alerts and TTS each keep separate volume paths.
- **Compact Settings** — ZDPS-inspired top tabs, flat sections, and slim accessible sliders.
- **Portable** — self-contained Windows x64 EXE; no .NET installation required.

## Use

1. Install **Npcap**.
2. Download `BPSR-ReadyAlert.exe` from [Releases](https://github.com/Zudin987/BPSR-ReadyAlert/releases/latest).
3. Open the EXE and select Resonance Logs CN / the correct network adapter if auto-detection misses it.
4. Leave ReadyAlert running in the system tray.
5. Enable **Desktop Notification** in the tray if you want Windows notifications in addition to sounds.
6. Enable **Chat Overlay** only if you want the chat window, translation or TTS.

Party invite/request detection is part of core ReadyAlert and works even when Chat Overlay, translation and TTS are disabled.

ReadyAlert does **not** inject into BPSR, replace game files, send chat, or automate gameplay.

Translation/TTS use no-key Google Translate/gTTS-style web endpoints. They can be rate-limited or changed upstream; failures do not stop normal core alerts or local chat capture.

[Latest release](https://github.com/Zudin987/BPSR-ReadyAlert/releases/latest) · [License](LICENSE) · [Third-party notices](THIRD_PARTY_NOTICES.md)
