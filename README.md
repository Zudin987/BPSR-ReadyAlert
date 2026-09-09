# BPSR Ready Alert

Lightweight native Windows companion for **Blue Protocol: Star Resonance**, written in **Rust**.

**Website:** https://zudin987.github.io/projects/readyalert/

## Features

- Native DPS meter with Damage, Heal and Tank views.
- Local combat history with previous-fight browsing and historical Entity Inspector data.
- Active and Encounter DPS/HPS/DTPS, plus contributor/party filters and configurable row limits.
- Per-player Entity Inspector with skill damage/healing/taken breakdowns.
- Battle Imagine detection with game icons and tiers.
- Dungeon Mechanics overlay with food/serum timers, tracked buffs and character attributes.
- ActorState-based death tracking and conservative party-wipe detection.
- Capture-health status so stale/wrong-adapter capture is visible while playing.
- Queue Pop, Ready Check, party invite and party request sounds.
- Optional desktop notifications.
- View-only BPSR chat overlay with filters and keyword alerts.
- Optional English translation and Guild/Party TTS.
- Native Npcap capture, process filtering and TCP reassembly without a .NET runtime.

ReadyAlert uses one shared Npcap capture path. It does **not** inject into BPSR, replace game files, send chat, or automate gameplay.

## Use

1. Install **Npcap**.
2. Download the latest `BPSR-ReadyAlert-v*-win-x64.zip` from Releases.
3. Extract it and run `BPSR-ReadyAlert.exe`.
4. Select the correct network adapter if automatic selection misses the game connection.
5. Leave ReadyAlert running in the system tray and enable only the overlays/alerts you want.

The Dungeon Mechanics title shows capture health. If it reports stale frames or no recent game packets while you are actively playing, re-check the selected Npcap adapter before trusting combat totals.

## Combat history and meter rates

Completed/reset encounters are stored **locally** under ReadyAlert's app-data `History` folder as compressed, schema-versioned files. History is never uploaded or synchronized to a web service. The number of retained encounters can be configured in DPS Meter Settings.

Use the DPS meter's `<`, `LIVE`, and `>` controls to move between older fights, the current live encounter, and newer saved fights. Clicking a player name while viewing history opens the Entity Inspector using that saved encounter state.

When **Active + encounter rates** is enabled, rows show both values:

- **Active rate (A)** reduces the effect of a player's late start and long per-player downtime.
- **Encounter rate (E)** divides the total by the full encounter duration.

DPS Meter Settings also provides contributor-only filtering, party-only filtering, always-show-self behavior, and Auto / 5 / 10 / 20 / 30 / 50 row caps.

## Combat segmentation

ReadyAlert deliberately avoids resetting a pull because one player dies, an add despawns, or a boss changes phase. Automatic encounter reset is conservative: scene changes and validated full-party wipes are boundaries; manual Reset is always available for training/testing.

The displayed target is also stabilized so a low-HP add does not constantly replace a known stronger boss/objective. A new target may take over after the previous target disappears between phases or when a clearly stronger target is observed.

## Data and attribution

Some protocol identifiers, English skill names and game-asset mappings are derived from community projects and/or game data. See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for attribution and pinned upstream sources.

## Build

Run:

```powershell
scripts\prepare-build-assets.ps1
cargo build --manifest-path rust/Cargo.toml --release
```

The executable is written to `rust/target/release/BPSR-ReadyAlert.exe`.

Translation/TTS depend on no-key web endpoints and may be rate-limited or changed upstream; the core capture, alerts, DPS and mechanics features work independently.

[Latest release](https://github.com/Zudin987/BPSR-ReadyAlert/releases/latest) · [Rust architecture notes](rust/README.md) · [License](LICENSE) · [Third-party notices](THIRD_PARTY_NOTICES.md)
