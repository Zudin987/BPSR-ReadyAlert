# BPSR ReadyAlert UI Overhaul Handoff

## Scope and visual separation

The native ReadyAlert application remains lightweight Rust + Win32. Its visual direction is compact desktop Pixel / Material 3 clarity + One UI organization, using the existing semantic dark palette and protected information density.

Encounter History is intentionally separate. It uses the offline **Midnight Tactical / combat analytics dashboard** language and must not inherit the native Pixel / One UI styling.

## Current branch / PR

- Branch: `feat/compact-meter-mode`
- PR: #120
- Base: `main` (`a41841a1a020218fc703e93e2025080d9ca0c54e` when this continuation began)
- Main already contains merged PR #119 native-modernization ancestry.
- Pre-continuation Compact checkpoint: `ab7df982a4940ac50817164dd2a5dac0f5054228`
- Implementation checkpoint for this continuation: `5a5b3c53155679baa67ce2f251b09676063ede1f`
- Rust CI: `34703829395` — SUCCESS — unit/regression tests, Clippy, native release build, EXE smoke test, size budget, packaging and generated-source diagnostics all passed
- Do not merge or release unless explicitly requested.

## Completed behavior that must not regress

### Compact / Raid / toolbar / export

- Dedicated persistent `C` Compact toggle remains available.
- Normal + non-Raid, Normal + Raid, Compact + non-Raid, and Compact + Raid are all first-class combinations.
- Compact rows remain intentionally thinner than Normal rows and continue hiding low-priority build information such as Ability Score, Illusion Score, Imagine presentation, and extra header chrome.
- Toolbar selected state is semantic, not positional: `Live`, `Raid`, and `Compact` derive from their actual state, while `<` / `>` and other actions never inherit neighboring toggle state.
- Copy -> As Image uses the same Raid/Compact-aware painter and current presentation state as the live meter.
- Normal/Raid/Compact/Raid+Compact sizing persistence remains separate.

### Protected native behavior

- Existing telemetry/protocol parsing, encounter/DPS calculations, updater, persistence, archive data semantics, tray behavior, Chat, Event Tracker, Dungeon Mechanics, Benchmark, and game integration behavior were not redesigned or refactored.
- Chat manual-scroll reading position, unseen-message indication, back-to-latest behavior, and translation/message wrapping remain intact.
- Wrapped target tooltip behavior remains measured/multiline and bounded instead of reverting to a one-line ellipsis.
- Entity detail surfaces continue using semantic theme colors and the existing compact row density.

## Native UI work completed in this continuation

`rust/src/ui_modern.rs` now supplies a lightweight GDI/Win32-only final control language:

- compact rounded tonal surfaces (6 / 9 / 12 px logical radii)
- neutral and selected tonal fills distinct from the teal accent
- owner-drawn secondary / primary / danger / navigation button states
- coherent hover / pressed / selected / disabled / keyboard-focus states
- owner-drawn combo presentation
- edit/list focus and hover border tracking
- Segoe UI Variable Text-based native drawing
- no GUI runtime or new heavy dependency

The final generated-source pass `build_v1290_native_finish.rs` runs **after** Compact stages and propagates that language without touching protected density/state logic:

- Settings controls, navigation buttons, combos and list surfaces
- Event Tracker controls and combos
- DPS / Mechanics feature settings controls
- DPS toolbar active/hover surfaces
- DPS target summary and wrapped hover tooltip
- entity/detail tab selected surfaces
- Chat toolbar controls, selected tabs, overflow control and jump-to-latest pill

Benchmark remains on the inherited modern native dialog implementation and protected 440 x 285 footprint; it was not unnecessarily rebuilt.

## Generated-source ordering — critical

Final build order is deliberately:

1. historical/prior generated-source chain
2. `build_ui_modernization.rs`
3. Compact stages `build_v1290_compact_stage1.rs` ... `stage7.rs`
4. `build_v1290_native_finish.rs`

The final pass is assertion-guarded and only restyles anchors in the already-Compact-aware generated source. Do not move it before the Compact stages. This ordering is what prevents native modernization and Compact Mode from overwriting each other.

## Protected density / layout contracts

Do not increase these just to make styling easier:

- Settings: 800 x 440
- Event Tracker: 720 x 440
- DPS Settings: 620 x 420
- Dungeon Mechanics Settings: 720 x 430
- Benchmark: 440 x 285
- Normal DPS minimum: inherited existing value (do not increase)
- Mechanics minimum: 400 x 220
- Chat minimum: 360 x 180
- Normal DPS row: ~33 px
- Compact DPS row: ~24 px
- Mechanics attribute: ~38 px
- Mechanics consumable: ~44 px
- Mechanics event: ~36 px
- Entity/detail row: ~30 px
- Badge: ~25 px with ~3 px gap
- Compact minimum: 300 x 140 logical px
- Compact non-Raid default width: 420 logical px
- Compact Raid default width: 760 logical px
- Compact Raid remains two columns, ranks 1-10 / 11-20

Responsive priority remains: reclaim gaps/padding/low-priority presentation before sacrificing player-name width or core combat information; vertical growth should show more rows rather than inflate rows.

## Native palette / semantics

Shared native direction remains approximately:

- BG `rgb(15,19,23)`
- Sidebar/Surface `rgb(21,27,33)`
- Raised/Input `rgb(26,32,39)`
- Hover `rgb(32,40,48)`
- Pressed `rgb(37,46,55)`
- Border `rgb(46,57,68)`
- Border strong `rgb(63,76,88)`
- Text `rgb(241,244,247)`
- Secondary `rgb(181,190,200)`
- Muted `rgb(131,144,157)`
- Disabled `rgb(103,114,125)`
- Accent `rgb(56,184,166)`

Do not randomly retheme individual pages.

## Encounter History status

The renderer now chains:

`encounter_archive.rs` -> `encounter_archive_v1290.rs` -> `encounter_archive_v1272.rs` -> prior renderer/data pipeline.

`v1290` is presentation-only. It injects Midnight Tactical CSS and local JS after the existing fully-local archive is generated. It does **not** alter serialized encounter meaning or invent metrics.

Implemented archive presentation:

- denser tactical hierarchy and stronger section contrast
- semantic damage/healing/tank/critical coloring
- restrained leader-relative damage bars derived from existing row damage
- semantic table-cell treatment derived from existing headers/context
- encounter/player/skill/buff/death/damage-taken/comparison views preserved
- comparison skill breakdown from v1272 preserved
- responsive desktop/small-window rules
- print styling that removes navigation chrome and keeps data readable
- CSP/offline model preserved: no CDN, remote fonts, remote JS, tracking, fetch, websocket, or server dependency

See `ARCHIVE_UI_HANDOFF.md` for archive-specific details.

## Files changed by this continuation

- `rust/src/ui_modern.rs`
- `rust/build/legacy/build_v1290_native_finish.rs`
- `rust/build/legacy/build_v1290_compact_meter.rs`
- `rust/src/encounter_archive.rs`
- `rust/src/encounter_archive_v1290.rs`
- `UI_OVERHAUL_HANDOFF.md`
- `ARCHIVE_UI_HANDOFF.md`

Earlier PR #120 files (`rust/build.rs`, Compact stage1-stage7, etc.) remain part of the branch and were preserved.

## Validation

- Branch divergence inspected first: PR #120 is based cleanly on the merged PR #119/current `main` ancestry (`behind_by: 0` at continuation start).
- Generated source from the prior CI artifact was inspected directly.
- P0 semantic toolbar mapping, permanent `C`, Compact row sizing, Raid+Compact routing, and Raid-aware image rendering were verified in generated source before the new visual pass.
- Final generated-source patch anchors were checked against the actual prior generated-source artifact before push.
- Midnight Tactical inline JavaScript passed a local `node --check` syntax validation.
- Archive upgrade has a Rust regression test for the tactical marker, semantic treatment, comparison preservation, CSP offline policy, and absence of remote source/link dependencies.
- Windows Rust CI run `34703829395`: SUCCESS — unit/regression tests, Clippy, native release build, EXE smoke test, size budget, packaging and generated-source diagnostics all passed.

## Remaining manual verification

A real interactive Windows screenshot/DPI pass has **not** been performed in this environment. CI/smoke testing is not equivalent to visual acceptance. Manual checks should cover Settings, Event Tracker, DPS Normal/Raid/Compact/Raid+Compact, Chat scrolling, Mechanics, target tooltip, entity detail, Copy-as-Image output, multiple DPI scales, and generated Encounter History in a browser.

If CI is green, this interactive Windows visual/DPI pass is the only remaining acceptance item; do not claim it already happened.
