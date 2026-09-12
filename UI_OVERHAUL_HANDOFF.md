# BPSR ReadyAlert UI Overhaul Handoff

## Overall Goal
Create an unmistakably modern 2026 desktop gaming utility using **Google Pixel / Material 3 clarity + Samsung One UI softness/organization**, adapted to compact desktop density. Keep native Rust + Win32, lightweight performance, existing minimum dimensions, information density, responsive priorities, and feature semantics. The Win32 architecture stays; the old Win32 appearance does not.

Compact Mode is a first-class UI state. Normal / Compact / Raid / Raid+Compact must all receive the same visual system.

## Current Branch / PR
- Branch: `feat/compact-meter-mode`
- PR: #120 `Add dedicated Compact Mode to native meter UI`
- Base: `main` v1.28.0
- Pre-overhaul Compact head: `ab7df982a4940ac50817164dd2a5dac0f5054228`
- Last known green Windows Rust CI: run `34700831809`

## Last Known Good Commit
`ab7df982a4940ac50817164dd2a5dac0f5054228` — Compact Mode implementation + semantic toolbar-state fix + raid-aware copy rendering. Full Windows Rust CI passed before this visual-overhaul continuation.

## Completed
- Existing v1.28 native UI modernization foundation is already merged into this branch ancestry.
- Shared `rust/src/ui_theme.rs` already provides semantic dark tokens, Segoe UI Variable Text, cached native brushes/fonts, dark DWM title bars, themed controls, owner-drawn buttons/combo support, and hover/pressed/focus/disabled states.
- Dedicated persistent Compact Mode exists with permanent `C` control.
- Compact rows are intentionally thinner than Normal rows (~24 logical px).
- Compact hides lower-priority presentation including Ability Score, Illusion Score, Imagine display, build/spec detail, and extra chrome.
- Separate Normal / Compact / Raid Normal / Raid Compact sizing persistence exists.
- Semantic `ToolbarAction` state mapping replaced positional highlight mapping.
- `Live`, `Raid`, and `Compact` active visuals derive from their semantic state; encounter arrows do not inherit neighboring state.
- Copy as Image now routes through raid-aware painting and respects Compact presentation.
- Chat manual-scroll reading-position protection and long-message wrapping were already implemented in the inherited build chain; preserve them.

## Partial
- The previous modernization remains visually too conservative: several surfaces still structurally read as classic Win32 with dark colors.
- Common controls need stronger custom presentation and more obvious Pixel/One UI shape/state language.
- DPS Normal/Compact/Raid/Raid+Compact need a stronger shared chrome/table visual pass without changing density.

## Not Started
- Stronger shared design primitives / reusable rounded tonal helpers.
- Strong Settings/sidebar structural restyle.
- Modern checkbox/toggle/input/list presentation across generated settings surfaces.
- Stronger DPS toolbar/table visual system for all four meter presentation states.
- Chat/Mechanics/Event Tracker visual pass on top of already-correct behavior.
- Benchmark/small-dialog visual pass beyond the inherited theme.
- Final DPI/pixel polish using actual Windows screenshots.

## Design System
Current baseline tokens before this pass:
- Background: `rgb(15,19,23)`
- Sidebar / Surface 1: `rgb(21,27,33)`
- Raised / Input: `rgb(26,32,39)`
- Hover: `rgb(32,40,48)`
- Pressed: `rgb(37,46,55)`
- Text primary: `rgb(241,244,247)`
- Text secondary: `rgb(181,190,200)`
- Muted: `rgb(131,144,157)`
- Accent: teal `rgb(56,184,166)`
- Font: Segoe UI Variable Text with native fallback
- Body / secondary / heading logical font heights: -14 / -13 / -17
- Shared control / button / nav heights: 28 / 30 / 30

Target shape direction:
- Major panel: 8-12 px equivalent radius
- Section surface: 8-10 px
- Standard control/input: 6-8 px
- Toolbar button: 5-7 px
- Dense DPS rows remain nearly rectangular; use restrained tint/accent/contribution treatment instead of card-per-row.

Target interaction language:
- Primary: restrained teal tonal fill, dark foreground
- Secondary: neutral elevated tonal fill
- Selected/active: clear accent-tonal fill, not a 1 px-only indicator
- Hover/pressed/focus/disabled: explicit and stable, no layout shift
- Checkbox: compact rounded-square custom presentation with centered check
- Toggle: only for genuine persistent on/off options
- Combo/edit/list: dark tonal field/row surface, custom focus/selection treatment where practical

## Protected Layout Values
Inherited v1.28 runtime contracts:
- Settings fixed outer: 800 × 440
- Event Tracker fixed outer: 720 × 440
- DPS Settings fixed outer: 620 × 420
- Mechanics Settings fixed outer: 720 × 430
- Benchmark fixed outer: 440 × 285
- Toolbar: 34 px
- Collapsed widget: 25 px
- Normal DPS row: 33 px
- Mechanics attribute row: 38 px
- Mechanics consumable row: 44 px
- Mechanics event row: 36 px
- DPS detail row: 30 px
- Imagine badge: 25 px, 3 px gap

Compact Mode contracts from PR #120:
- Compact minimum logical width: 300 px
- Compact minimum logical height: 140 px
- Compact default non-raid logical width: 420 px
- Compact default raid logical width: 760 px
- Compact player row height: 24 px
- Raid Compact remains two columns: ranks 1-10 and 11-20

Normal-mode protected minima come from the inherited generated overlay functions and must not be increased. Exact current generated values/breakpoints must be re-recorded after inspecting the latest generated source.

Responsive rules:
1. use practical empty width;
2. reduce nonessential gaps;
3. shrink flexible name region;
4. compact decoration/labels;
5. rebalance columns;
6. hide lowest-priority useful data last.

Compact Mode must not re-add intentionally hidden Ability Score, Illusion Score, Imagine presentation, or low-priority build detail.

## State Mapping
Protected semantic toolbar state:
- `ToolbarAction::Live` active iff `history_index.is_none()`.
- `ToolbarAction::Raid` active iff raid mode is enabled.
- `ToolbarAction::Compact` active iff Compact Mode is enabled.
- `<`, `>`, History, Copy, Benchmark, Reset, Settings, overflow, collapse/close are actions, not persistent modes unless explicitly modeled.
- Never derive selected state from action-array index.

## Export / Copy Rendering
Copy as Image must reflect the current presentation:
- Normal + non-raid -> Normal non-raid.
- Compact + non-raid -> Compact non-raid.
- Normal + Raid -> Normal Raid.
- Compact + Raid -> Compact Raid.
- Compact exports must not reintroduce intentionally hidden information.
- Prefer shared live/layout painting logic rather than a stale default export renderer.

## Files Modified
Current PR already modifies:
- `rust/build.rs`
- `rust/build/legacy/build_v1290_compact_meter.rs`
- `rust/build/legacy/build_v1290_compact_stage1.rs` ... `stage7.rs`
- `UI_OVERHAUL_HANDOFF.md`

Additional files for this visual pass will be recorded here as they are changed.

## Screenshots Reviewed
The inherited v1.28 handoff records a completed 27-screenshot audit. Do not restart that audit. This continuation request supplies the stronger design specification; use it to push the existing implementation substantially further while preserving the earlier spacing/density decisions.

## Verification Completed
- Pre-overhaul Compact branch Windows CI run `34700831809`: unit/regression tests, Clippy, native release build, EXE smoke test, size budget, packaging, generated-source diagnostics — all passed.

## Known Issues
- The app can still read as “classic Win32 with a dark theme” because shape/composition/control rendering remains conservative.
- Stock-looking checkboxes, inputs, list selections, combo affordances, and some dialog composition remain high-priority modernization targets.
- DPI/pixel visual acceptance cannot be truthfully completed without a real Windows screenshot/runtime pass.
- Historical assertion-guarded generated-source chain is architecture debt; preserve it for this UI task instead of refactoring unrelated systems.

## Exact Next Steps
1. Inspect the latest generated source for PR #120 and record exact Normal minima, responsive breakpoints, toolbar/row allocation, and Compact rendering anchors.
2. Strengthen `rust/src/ui_theme.rs` with lightweight rounded/tonal drawing primitives and reusable modern control state helpers; no new heavy runtime.
3. Apply those primitives to Settings/common controls and small dialogs without changing protected outer dimensions.
4. Redesign DPS toolbar/table chrome for Normal + Compact + Raid + Raid Compact while preserving all state semantics, row density, and responsive hide order.
5. Modernize Chat + Mechanics + Event Tracker presentation while preserving the already-fixed chat scroll-follow behavior.
6. Batch validation: source review -> targeted compile/tests -> one Windows CI run after coherent changes. Do not poll repeatedly.
7. Update this handoff with exact final tokens, breakpoints, files, validation state, known limitations, and next action before stopping.