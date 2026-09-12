# BPSR ReadyAlert UI Overhaul Handoff

## Overall Goal
Modernize the existing compact native Rust/Win32 utility into a polished 2026 desktop gaming tool with **Google Pixel / Material 3 clarity + Samsung One UI softness and organization**, adapted to desktop rather than copied from mobile. Preserve the tuned information density, minimum sizes, normal spacing, responsive priorities, saved preferences, and native/lightweight behavior. Exhaust reclaimable space before hiding useful data.

## Current Branch / PR
- Branch: `feat/native-ui-modernization`
- PR: #119 `Modernize ReadyAlert native UI without changing density`
- Base: `main` v1.27.2, `67b7fb27faa06c60d887b57f7eae1c2fea027ccb`
- Current code checkpoint: `e7286497c9f3a0f546091a4a2bed9c26b69c3905` (`polish: finish compact Pixel One UI native pass`)
- Previous known-good code checkpoint: `356b5410a2e44c8db8b7aa8b16835eb5c21e1080`

## Completed
- Read the complete implementation brief and individually audited all 27 supplied screenshots, including compact/resized overlays, Settings pages, Benchmark, detailed DPS views, tray menu and 5 offline archive pages. Do not repeat the screenshot audit unless a concrete regression appears.
- Inspected the assertion-guarded generated-source build chain through v1.27.1 and the final generated runtime source emitted by successful Windows CI #347.
- Verified the existing Chat Overlay reading-position fix and later translation-wrap hardening are already present in the generated chain; preserve them rather than duplicating them.
- Recorded the final generated DPS/mechanics geometry and responsive thresholds below.
- Improved `rust/src/ui_theme.rs` without new runtime dependencies or dimension changes:
  - semantic dark palette and Segoe UI Variable hierarchy;
  - readable dark foreground on teal primary actions;
  - cached native brushes/fonts;
  - native dark title bars;
  - owner-draw hover/pressed/focus/disabled states;
  - reliable hover invalidation through a tiny local Win32 subclass ABI, avoiding dependency growth.
- Modernized `rust/src/benchmark_ui.rs` while preserving its exact 440 × 285 footprint and field/button coordinates:
  - shared dark theme;
  - Start is primary, Cancel neutral;
  - benchmark behavior/timing untouched.
- Added the final assertion-guarded generated UI polish stage `rust/build/legacy/build_ui_modernization.rs` and routed `rust/build.rs` through it. It deliberately patches only final generated UI surfaces that cannot safely be changed earlier in the historical chain.
- Settings interaction polish in the final generated UI:
  - Delete tab is disabled when deletion is impossible (only one tab or no valid selection);
  - Unblock selected is disabled until a blocked user is actually selected;
  - Clear all is disabled when the blocked list is empty;
  - selection changes refresh those states immediately.
- Event Tracker editor combo boxes now explicitly receive the existing `DarkMode_CFD` combo theme, removing the remaining legacy-looking closed dropdown/arrow treatment without changing control dimensions.
- DPS/overlay hover help is now measured and wrapped instead of forcing every tooltip into a 25 px single line with ellipsis. It stays native GDI, caps width at 420 px and measured content height at 96 px, and repositions inside the current overlay bounds.
- Entity detail skill/taken/buff/death tables now reuse shared `RAISED`, `SURFACE`, `TEXT`, `TEXT_SECONDARY`, and `MUTED` tokens instead of isolated legacy gray values. Row height and table density are unchanged.
- Opened and maintained PR #119. Do not merge or release unless explicitly requested.

## Important Design Decisions
- Pixel / Material 3 supplies clarity, coherent states, restrained accent usage and modern dark surfaces.
- One UI supplies section organization, softer grouping and easier Settings scanning.
- This is **not** a mobile UI: no giant padding, giant toggles, card-per-setting layouts or increased row heights.
- Keep the existing teal interaction accent, class/spec full-row identity colors and dense live-data geometry.
- Settings fixed outer dimensions: 800 × 440.
- Event Tracker editor fixed outer dimensions: 720 × 440.
- DPS Settings fixed outer dimensions: 620 × 420.
- Mechanics Settings fixed outer dimensions: 720 × 430.
- Benchmark fixed outer dimensions: 440 × 285.
- Resizable overlay minima remain: DPS 600 × 220; Dungeon Mechanics 400 × 220; Chat Overlay 360 × 180.
- Current app uses Windows DPI virtualization. Do not switch the process to per-monitor awareness without implementing a complete scaling strategy for every window.
- Event Tracker live events are embedded in Dungeon Mechanics; do not invent a separate event overlay.
- Offline HTML archives open in the user's browser and are not the native runtime UI.

## Protected Existing Behavior
- Main Settings and Event Tracker use staged Apply semantics.
- DPS/Mechanics feature Settings save immediately.
- Native keyboard, tray and menu behavior stays native.
- Preserve saved bounds, opacity, collapse state and overlay restore behavior.
- Preserve full-row class/spec coloring, self emphasis and dead-player semantics.
- Preserve the current meter hide/reclaim order; never hide useful data while reclaimable horizontal space remains.
- Preserve current Chat Overlay manual-scroll reading position and unseen-message behavior.
- Preserve protocol, telemetry, encounter/archive format, updater and combat calculations.

## Verified Runtime Layout Contracts
### Final generated overlay constants
Verified against the generated source artifact from successful Windows CI #347, not only the older base input:
- Toolbar: 34 px
- Collapsed widget: 25 px
- Final DPS row: **33 px**
- Final Mechanics attribute row: **38 px**
- Mechanics consumable row: 44 px
- Mechanics event row: 36 px
- DPS detail row: 30 px
- Imagine badge: 25 px with 3 px gap

The older base source still contains pre-patch 32 px DPS rows and 29 px Mechanics attribute rows; do not use those older values as the final runtime contract.

### DPS meter row allocation / responsive behavior
- deaths: 30 px; share: 56 px; total: 88 px; active rate: 90 px; metric gap: 5 px.
- player identity begins at row-left + 25.
- player-name allocation is flexible and ellipsizes only when constrained.
- battle-imagine badges require sufficient width rather than forcing core data out.
- numeric metrics are right-aligned.
- main rows retain class/spec tinting and self/dead emphasis.
- height growth shows more rows; it does not inflate row height.

### v1.27 toolbar priority tiers
- Comfortable: History 28, arrows 30, Live 46, Raid 44, Copy 48, Benchmark 76, Reset 52, gap 4.
- Compact: History 24, arrows 24, Live 30, Raid 28, Copy 28, Benchmark 28, Reset 28, gap 3.
- Dense: History 22, arrows 20, Live 28, Raid 26, Copy 26, Benchmark 26, Reset 26, gap 2.
- Minimum: History 22, arrows 18, Live 26, Raid 24, Copy 24, Benchmark 24, Reset 24, gap 1.
- Toolbar action vertical bounds remain 5..29.

### Chat Overlay behavior already verified
- When the user has scrolled away from the bottom, incoming visible messages preserve the reading position instead of forcing the viewport back to latest.
- An unseen-message counter and “Back to latest” affordance are maintained.
- Returning to the tail clears unseen state.
- The v1.24.1 generated patch removed the previous fixed 3/4-line wrap cap so long translated/chat text can use measured height.
- Do not reimplement these fixes unless the build chain itself is intentionally replaced.

## Files Modified on This Branch
- `UI_OVERHAUL_HANDOFF.md`
- `rust/src/ui_theme.rs`
- `rust/src/benchmark_ui.rs`
- `rust/build.rs`
- `rust/build/legacy/build_ui_modernization.rs`

## Screenshot Audit Notes
The full screenshot-by-screenshot audit is already complete. Retained decisions:
- Main Settings: compact two-column structure is good; modernize shared controls/hierarchy, not the footprint.
- Chat Overlay / Colors / Speech / Tabs / Sounds / Network / Blocked Users: keep group density and tuned positions; focus on consistency and state clarity.
- Blocked Users: empty/destructive actions should not look available when they cannot do anything; implemented via native enabled/disabled state.
- DPS Settings 620×420 and Mechanics Settings 720×430 remain fixed and compact.
- Event Tracker editor remains 720×440 and staged Apply; disabled editor controls stay native rather than being replaced by heavy custom widgets.
- Benchmark: small dialog should share the app theme; implemented.
- Chat Overlay: preserve configured density, manual-scroll reading position and long translation wrapping.
- DPS/raid meter: preserve 20-player density, class-colored rows, self outline, flexible identity and existing responsive priorities.
- Entity detail tables: keep dense 30 px rows but remove disconnected legacy gray surfaces; implemented.
- Tray menu: native Windows menu semantics are intentional.
- Offline archive pages remain browser content and are outside the native runtime modernization pass.

## Verification Already Done
- Source-level audit of current branch and generated-source chain.
- Replacement anchors for the new final UI stage were tested against the exact generated source artifact from CI #347 before committing; all expected counts matched.
- Windows Rust CI #347 passed on `356b5410a2e44c8db8b7aa8b16835eb5c21e1080`: unit/regression tests, Clippy, native release build, smoke test, size budget and artifact packaging all succeeded.
- New coherent UI checkpoint `e7286497c9f3a0f546091a4a2bed9c26b69c3905` triggered Windows Rust CI #348 (`34698184991`). At this handoff update it is still in progress; do not repeatedly poll it.

## Known Limitations / Follow-up
- No live Windows screenshot/DPI visual pass has been performed in this environment. Do not claim one.
- Native disabled checkbox/edit glyphs in Event Tracker may still vary by Windows theme. Do not replace native controls merely for cosmetics unless a Windows visual pass proves a real defect.
- The historical generated-source patch chain remains architecture debt. This UI task intentionally preserves it rather than risking an unrelated refactor.
- Pixel/One UI “softness” is implemented through tone, hierarchy and control states; do not add expensive blur/acrylic or inflate geometry just to chase more rounding.

## Exact Next Steps
1. Inspect Windows Rust CI #348 **once after it finishes**. If it failed, read the useful failing step/log, batch-fix the related issue, and let the next code push rerun CI. Do not poll repeatedly.
2. If #348 is green, inspect PR #119 final diff/status and leave it open for review. Do not merge or release unless explicitly requested.
3. If a later real Windows screenshot/runtime pass exposes a concrete remaining visual defect, make a narrow correction without changing fixed dimensions, final row heights, responsive hide priorities or Chat scroll semantics.

## Do Not Revisit Unless Broken
- Do not redo the 27-screenshot audit.
- Do not alter protocol/telemetry/archive formats.
- Do not increase fixed/minimum sizes for aesthetics.
- Do not increase DPS row height or reduce visible information at equivalent window sizes.
- Do not replace compact desktop spacing with mobile Material/One UI spacing.
- Do not undo current Chat scroll/translation fixes.
- Do not replace the native tray/menu behavior with a custom oversized menu.

## Final Cleanup Rule
When final validation is green, keep this handoff long enough for review/continuation. Record any Windows-only visual limitation precisely rather than claiming testing that did not occur.