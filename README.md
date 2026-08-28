# BPSR Ready Alert

Windows companion for **Blue Protocol: Star Resonance** Ready/queue alerts with an optional view-only chat overlay.

**Website:** https://zudin987.github.io/projects/readyalert/

- **Ready / Queue alerts** — sound alerts and optional desktop notifications.
- **Chat Overlay** — World, Guild/Team, All and custom tabs with filters, keyword sounds, click-through and Always on Top.
- **English translation** — optional no-key translation for World, Guild and Party / Team chat.
- **English TTS** — optional Guild / Party speech only, with its own volume and quick toggle.
- **Independent audio controls** — Ready / Queue, Chat alerts and TTS each use a separate volume.
- **Compact Settings** — ZDPS-inspired top tabs, flat sections, and slim accessible sliders.
- **Portable** — self-contained Windows x64 EXE; no .NET installation required.

## Use

1. Install **Npcap**.
2. Download `BPSR-ReadyAlert.exe` from [Releases](https://github.com/Zudin987/BPSR-ReadyAlert/releases/latest).
3. Open the EXE and select Resonance Logs CN / the correct network adapter if auto-detection misses it.
4. Leave ReadyAlert running in the system tray.
5. Enable **Chat Overlay** only if you want the chat window, translation or TTS.

ReadyAlert does **not** inject into BPSR, replace game files, send chat, or automate gameplay.

Translation/TTS use no-key Google Translate/gTTS-style web endpoints. They can be rate-limited or changed upstream; failures do not stop normal Ready/Queue alerts or local chat capture.

[Latest release](https://github.com/Zudin987/BPSR-ReadyAlert/releases/latest) · [License](LICENSE) · [Third-party notices](THIRD_PARTY_NOTICES.md)
