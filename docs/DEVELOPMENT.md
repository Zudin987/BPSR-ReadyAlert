# Development

Production code lives in `rust/` and stays native: Rust, Win32 and Npcap.

## Layout

- `rust/src/` — application and runtime code.
- `rust/data/` — exact game-data mapping tables, including supplemental seasonal IDs.
- `rust/build/legacy/` — historical assertion-guarded source-generation chain still required by the current build.
- `assets/source/` — encoded source assets reconstructed by the build-preparation scripts.
- `scripts/` — build preparation and repository tooling.
- `docs/images/` — curated images used by the project README.
- `docs/qa/` — durable QA and render-acceptance notes.

The versioned and `legacy`-named Rust files are not automatically obsolete: several are active inputs to the current generated-source chain. New work should prefer normal source files over adding another patch stage, but existing stages must not be removed without first replacing their generated behavior.

## Build

```powershell
scripts\prepare-build-assets.ps1
cargo test --manifest-path rust/Cargo.toml --all-targets --locked
cargo clippy --manifest-path rust/Cargo.toml --all-targets --locked
cargo build --manifest-path rust/Cargo.toml --release --locked
```

Output: `rust\target\release\BPSR-ReadyAlert.exe`

The Windows CI additionally runs the native EXE smoke test, size budget, generated-source diagnostics and native UI render diagnostics.
