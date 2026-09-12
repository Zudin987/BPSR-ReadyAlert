# Encounter History UI Handoff

## Design boundary

Encounter History is **not** part of the native Pixel / One UI visual language. Its intended presentation is **Midnight Tactical / combat analytics dashboard**: dense, dark, restrained, data-first, with semantic combat colors and minimal decoration.

It remains a generated local HTML file. No server, CDN, cloud, remote font, remote JavaScript, tracking, or online API is allowed.

## Current implementation

Renderer chain:

`rust/src/encounter_archive.rs`
-> `rust/src/encounter_archive_v1290.rs`
-> `rust/src/encounter_archive_v1272.rs`
-> prior archive renderer/data pipeline

`v1290` is presentation-only. It calls the prior renderer first and then injects one CSS block before `</style>` and one script block before `</body>` using single-anchor checks. Existing data serialization and encounter meaning are untouched.

## Midnight Tactical presentation

Implemented in `encounter_archive_v1290.rs`:

- near-black tactical background and compact panel hierarchy
- cyan accent used sparingly for navigation/focus, not as a combat metric color
- damage red, healing green, mitigation/taken blue, warning gold, critical/death red
- compact 4-6 px surfaces instead of mobile-style cards
- denser encounter list and player/detail panes
- selected encounter/player/tab indicators with narrow accents
- compact metrics and tables with stronger numeric scanning
- semantic table-cell coloration inferred only from existing headers/context
- thin player/compare damage bars derived only from existing `row.damage`, relative to the encounter leader
- Death Recap styling preserved and clarified
- comparison layout and v1272 expandable per-player skill breakdown preserved
- responsive rules for narrower windows
- print rules that remove the sidebar/tools/search and produce a readable local hard-copy/PDF layout

No metric is fabricated. If the current archive record does not contain a value, the existing empty/missing-value behavior remains authoritative.

## Offline / portability constraints

The base archive CSP remains in force, including `connect-src 'none'`. The v1290 CSS/script contains no remote URLs, imports, fetch/XHR/WebSocket calls, or external dependencies.

The archive continues to be generated and opened from the user's local encounter-history folder. Existing archive storage/record semantics are not migrated for this visual pass.

## Views preserved

- encounter list / search
- encounter summary metrics
- player ranking/selection
- Overview
- Skills
- Buffs
- Deaths / death recap
- Damage Taken / absorbed sources
- two-encounter Comparison
- comparison per-player skill breakdown
- empty/missing-value states

## Validation

- Inline v1290 JavaScript passes `node --check` syntax validation.
- New Rust regression test generates a real temporary saved encounter and checks:
  - Midnight Tactical marker exists
  - semantic treatment/script exists
  - comparison skill feature remains present
  - original offline CSP is retained
  - no remote `<link>` or HTTP(S) script/source dependency is introduced
- Code CI: Rust CI `34703829395` — SUCCESS — unit/regression tests, Clippy, native release build, EXE smoke test, size budget, packaging and generated-source diagnostics all passed.

## Manual verification still required

A real browser visual pass has not been performed here. After a Windows build, manually inspect the generated `index.html` at desktop and narrow widths, comparison mode, every player tab, empty states, and print preview. This is visual acceptance only; the offline/data invariants are covered by source/tests.
