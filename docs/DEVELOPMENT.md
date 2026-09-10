# Development

Production code lives in `rust/` and stays native: Rust, Win32 and Npcap.

## Layout

- `rust/src/` — application/runtime code.
- `rust/data/` — exact game-data mapping tables.
- `rust/build/legacy/` — historical assertion-guarded source-generation chain used by the current build.
- `assets/source/` — encoded source assets reconstructed by the build-prep script.
- `scripts/` — build preparation utilities.

New work should prefer normal source files over adding another legacy build-patch stage.

## Build

```powershell
scripts\prepare-build-assets.ps1
cargo test --manifest-path rust/Cargo.toml --all-targets
cargo build --manifest-path rust/Cargo.toml --release
```

Output: `rust\target\release\BPSR-ReadyAlert.exe`
