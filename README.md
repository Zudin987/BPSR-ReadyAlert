# BPSR Ready Alert

Lightweight native Windows companion for **Blue Protocol: Star Resonance**, written in **Rust**.

**Website:** https://zudin987.github.io/projects/readyalert/

## Use

1. Install **Npcap**.
2. Download `BPSR-ReadyAlert.exe` from [Releases](https://github.com/Zudin987/BPSR-ReadyAlert/releases/latest).
3. Open the EXE and select Resonance Logs CN / the correct adapter if auto-detection misses it.
4. Leave ReadyAlert running in the system tray.
5. Enable only the alerts/chat features you want.

## Features

- Queue Pop, Ready Check, party invite and party request sounds.
- Optional desktop notifications.
- View-only BPSR chat overlay with filters and keyword alerts.
- Optional English translation and Guild/Party TTS.
- Native Npcap capture, process filtering and TCP reassembly without a .NET runtime.

ReadyAlert uses one shared Npcap capture path. It does **not** inject into BPSR, replace game files, send chat, or automate gameplay.

Translation/TTS depend on no-key web endpoints and may be rate-limited or changed upstream; core alerts still work independently.

## Build

Run `scripts/prepare-build-assets.ps1`, then:

```powershell
cargo build --manifest-path rust/Cargo.toml --release
```

The executable is written to `rust/target/release/BPSR-ReadyAlert.exe`.

[Latest release](https://github.com/Zudin987/BPSR-ReadyAlert/releases/latest) · [Rust notes](rust/README.md) · [License](LICENSE) · [Third-party notices](THIRD_PARTY_NOTICES.md)
