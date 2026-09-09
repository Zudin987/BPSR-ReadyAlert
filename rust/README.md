# Native Rust implementation

`rust/` contains the production ReadyAlert application.

The project intentionally stays native and small: Win32 UI, Npcap packet capture, Rust protocol parsing and no Electron/Tauri/.NET runtime dependency.

## Runtime structure

- `src/main.rs` — application startup, shared state and module wiring.
- `src/capture_v160.rs` — Npcap worker, TCP reassembly and frame processing. The active build generates a small v1.8.5 wrapper from this source to add capture-health reporting without duplicating the parser.
- `src/telemetry_v170.rs` — compact combat/mechanics telemetry core used by the generated compatibility layer.
- `src/telemetry_adapter_v170.rs` — team/spec/skill/Imagine enrichment.
- `src/telemetry_v181.rs` — scene-freshness and stable-primary-target guard around telemetry events.
- `src/feature_overlays_v170.rs` — native DPS/mechanics overlay base.
- `src/feature_settings_v181.rs` — feature settings persistence, recovery and monitor-bound correction.
- `src/settings_v181.rs` — application settings persistence and crash recovery.
- `src/capture_supervisor.rs` — restarts only the Npcap worker when adapter preference changes.

## Generated compatibility layers

The current production build still has a historical `build_v18x.rs` compatibility chain. Each layer applies assertion-guarded source transformations and fails CI if its expected source anchors drift.

This is intentionally temporary technical debt. Future refactoring should fold the validated generated code back into normal source files so releases no longer depend on a long string-patch chain.

The current entry point is configured in `Cargo.toml` (`build_v185.rs`).

## Combat behavior

ReadyAlert starts an encounter from eligible player damage and is deliberately conservative about automatic resets. One player dying, ordinary add death/despawn and idle time are not sufficient boundaries. Scene changes, validated full-party wipes and the manual Reset button are the trusted boundaries.

The public target shown in the DPS toolbar is stabilized outside the compact parser: a known stronger boss/objective remains selected when smaller adds are hit, while phase transitions can promote a new target after the old target disappears.

## Capture health

The capture worker periodically reports whether:

- the game process is detected,
- recent game packets are arriving,
- valid protocol frames are being reconstructed.

The status is surfaced in the tray and Dungeon Mechanics toolbar. This does not change combat calculations; it is a diagnostic signal that helps users notice a wrong adapter, VPN-route problem or protocol-frame stall before trusting incomplete numbers.

## Build

From the repository root on Windows:

```powershell
scripts\prepare-build-assets.ps1
cargo test --manifest-path rust/Cargo.toml
cargo build --manifest-path rust/Cargo.toml --release
```

The release binary is:

```text
rust\target\release\BPSR-ReadyAlert.exe
```

CI additionally launches the release EXE with `--build-smoke-test`, enforces the native size budget and packages the Windows ZIP.

## Third-party data

See the repository-level `THIRD_PARTY_NOTICES.md`. Upstream game-data/community-derived tables are pinned during builds where practical so a changing remote repository cannot silently alter a released binary.
