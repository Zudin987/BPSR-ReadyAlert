# BPSR ReadyAlert UI Overhaul Handoff

## Overall Goal
Modernize the existing compact native Rust/Win32 utility using Fluent 2 for visual language, PowerToys for Settings, Task Manager for tables and Xbox Game Bar for overlays. No Electron/Tauri/web runtime or new heavy dependencies. Preserve minimum sizes, normal spacing, row density, responsive priorities and saved preferences. Exhaust reclaimable space before hiding data.

## Current Branch / PR
- Branch: `feat/native-ui-modernization`
- PR: #119 `Modernize ReadyAlert native UI without changing density`
- Base: `main` v1.27.2, `67b7fb27faa06c60d887b57f7eae1c2fea027ccb`
- Latest code commit before this documentation-only update: `86ae00c3c906a36ddb8e70bb01569ecfb96f3365`

## Completed
- Read the full uploaded implementation brief.
- Individually reviewed all 27 uploaded PNGs, including 5 existing offline archive pages. Do not redo this audit unless a concrete regression appears.
- Inspected the active assertion-guarded build chain through v1.27.1 and the generated runtime source inputs.
- Verified the existing Chat Overlay reading-position fix and later translation-wrap hardening are already present in the generated chain; preserve rather than duplicate them.
- Recorded current DPS/mechanics row geometry and responsive thresholds below.
- Improved `rust/src/ui_theme.rs` without adding dependencies or altering fixed window dimensions:
  - semantic accent foreground for readable primary-button labels;
  - cached native dark brushes;
  - reliable owner-draw hover tracking using native Win32 subclassing;
  - generic `theme_control()` detects BUTTON controls, so generated Settings/Event Tracker/feature-settings buttons receive hover invalidation without adding another legacy patch stage;
  - disabled owner-draw text uses the existing muted semantic token.
- Modernized `rust/src/benchmark_ui.rs` in place:
  - preserved the exact 440 × 285 footprint and existing field/button coordinates;
  - added native dark title bar/background/input/static treatment;
  - Start is the primary accent action and Cancel remains neutral;
  - no benchmark behavior or timing semantics changed.
- Opened PR #119 after the coherent implementation checkpoint.

## Important Design Decisions
- Keep existing palette identity and teal accent, class-colored full rows and dense table geometry.
- Settings fixed outer dimensions: 800 × 440; Event Tracker editor 720 × 440; DPS settings 620 × 420; Mechanics settings 720 × 430. Windows screenshots exclude invisible frame margins.
- Benchmark remains 440 × 285.
- Current app uses Windows DPI virtualization; do not switch the process to per-monitor awareness without implementing full scaling across all windows.
- Event Tracker live events are embedded in Dungeon Mechanics; there is no distinct event overlay to invent.
- Existing offline HTML archives open in the user's browser. They are not the native app's runtime.

## Protected Existing Behavior
Staged Apply in main Settings/Event Tracker; immediate feature settings; native keyboard and tray behavior; compact overlay chrome, saved bounds/opacity/collapse, class colors and meter hide order. Do not change protocol/telemetry/archive format.

## Verified Runtime Layout Contracts
### Shared overlay constants
From `src/feature_overlays_v170.rs` feeding the current generated overlay:
- Toolbar: 34 px
- Collapsed widget: 25 px
- DPS control region: 31 px
- DPS row: 32 px
- Mechanics attribute row: 29 px
- Mechanics consumable row: 44 px
- Mechanics event row: 36 px
- DPS detail row: 30 px
- Imagine badge: 25 px with 3 px gap

### DPS meter row allocation
The generated v1.16 meter layout still underpins the current v1.27 toolbar/history additions:
- deaths 30 px; share 56 px; total 88 px; active rate 90 px; 5 px metric gap;
- player identity begins at row-left + 25;
- player-name allocation is flexible (roughly 42% of identity area, clamped 92–205 px) and ellipsizes only when constrained;
- battle-imagine badges are shown only once row width reaches 690 px;
- aggregate summary expands at 720 px;
- capture/status copy is shown from 820 px;
- numeric metric columns are right-aligned;
- full-row class/spec coloring and self-player emphasis remain intentional.

### Current v1.27 toolbar priority tiers
The v1.27 history/benchmark toolbar keeps four width tiers instead of hiding actions early:
- Comfortable: History 28, arrows 30, Live 46, Raid 44, Copy 48, Benchmark 76, Reset 52, gap 4.
- Compact: History 24, arrows 24, Live 30, Raid 28, Copy 28, Benchmark 28, Reset 28, gap 3.
- Dense: History 22, arrows 20, Live 28, Raid 26, Copy 26, Benchmark 26, Reset 26, gap 2.
- Minimum: History 22, arrows 18, Live 26, Raid 24, Copy 24, Benchmark 24, Reset 24, gap 1.
- Toolbar action vertical bounds remain 5..29.

### Chat Overlay behavior already verified
- When the user has scrolled away from the bottom, incoming visible messages increment the preserved `scroll_from_bottom` reading position instead of forcing the viewport back to latest.
- An unseen-message counter is maintained and the UI offers a “Back to latest” / new-message affordance.
- Returning to the tail clears the unseen count.
- The later v1.24.1 generated patch removes the earlier hard 3/4-line wrapping cap, so long translated/chat text can use the measured required height.
- Do not reimplement either fix in direct source unless the build chain itself is replaced.

## Files Modified on This Branch
- `UI_OVERHAUL_HANDOFF.md`
- `rust/src/ui_theme.rs`
- `rust/src/benchmark_ui.rs`

## Screenshot Audit Notes
All 27 screenshots were audited before implementation. Key retained decisions:
- Main Settings: keep compact two-column geometry; polish shared control/nav hierarchy only.
- Chat Settings / Colors / Speech / Tabs / Sounds / Network / Blocked Users: preserve footprint, grouping and density; common control rendering is the main modernization surface.
- DPS Settings 620×420 and Mechanics Settings 720×430 stay fixed and compact.
- Event Tracker editor stays 720×440 with staged Apply; its live events remain inside Dungeon Mechanics.
- Benchmark: dark-theme the existing tiny dialog, not a full-page rewrite.
- Chat Overlay: keep configured message density; preserve manual-scroll reading position and translation wrapping.
- DPS/raid meter: preserve 20-player density, full-row class color, self outline, flexible identity and tuned side data.
- Tray menu: native menu semantics are intentional.
- Offline archive pages remain browser content and are outside the native runtime modernization pass.

## Remaining Known Limitation / Follow-up
- Event Tracker native disabled checkboxes/fields may still inherit some legacy Win32 disabled glyph treatment because they are not custom-rendered. Do not replace them with heavy/custom controls merely for aesthetics; only address this if a lightweight native fix is visually proven on Windows.
- Windows visual/DPI testing has not been performed in this environment. Do not claim it has.
- The current generated-code architecture still depends on the historical patch chain; this UI pass intentionally does not refactor that architecture.

## Validation State
- No local Rust toolchain was available in this environment, so local compilation was not used as an edit/debug loop.
- Rust CI is the intended single Windows integration checkpoint after the coherent implementation batch. The workflow runs tests, Clippy, native release build, smoke test and the 20 MiB size budget.
- A documentation-only handoff update does not retrigger Rust CI because the workflow path filter is scoped to Rust/assets/build files.

## Exact Next Steps
1. Inspect the current PR-head Rust CI once. If it failed, read the useful error output, batch-fix related compile/test issues, then rerun via the next code push. Do not poll repeatedly.
2. If CI is green, inspect PR #119 diff/status and leave it open for review; do not merge/release unless explicitly requested.
3. If a Windows screenshot/runtime pass later exposes a specific control defect, make a narrow correction without changing fixed dimensions, row heights, hide priorities or chat-scroll semantics.

## Do Not Revisit Unless Broken
All 27 screenshots have been audited. Keep dense row/caption/settings geometry, class palette, existing minimum/fixed dimensions, staged/immediate setting semantics, chat reading-position behavior and the current responsive priorities. No protocol/telemetry/archive-format changes.
