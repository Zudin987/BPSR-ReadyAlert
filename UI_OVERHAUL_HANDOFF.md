# BPSR ReadyAlert UI Overhaul Handoff

## Overall Goal
Modernize the existing compact native Rust/Win32 utility using Fluent 2 for visual language, PowerToys for Settings, Task Manager for tables and Xbox Game Bar for overlays. No Electron/Tauri/web runtime or new heavy dependencies. Preserve minimum sizes, normal spacing, row density, responsive priorities and saved preferences. Exhaust reclaimable space before hiding data.

## Current Branch
`feat/native-ui-modernization`

## Base / Target
Base `main` at v1.27.2, `67b7fb27faa06c60d887b57f7eae1c2fea027ccb` (PR #118). No open PR at task start. This task requests implementation and a reviewable branch/PR; no new release number is assigned.

## Last Known Good Commit
`67b7fb27faa06c60d887b57f7eae1c2fea027ccb`: existing v1.27.2 main; baseline, not yet independently validated in this session.

## Current Working Commit
Initial audit checkpoint; see `git log -1` for current commit.

## Completed
- Read the full uploaded implementation brief.
- Individually reviewed all 27 uploaded PNGs, including 5 existing offline archive pages.
- Cloned current main and confirmed no open PR to resume.
- Identified `ui_theme.rs` as shared native theme and the assertion-guarded legacy build chain as the source of most active UI modules.

## Partially Completed
Architecture and numerical-layout audit. Do not edit legacy input anchors without generating and inspecting final outputs. Runtime modules must remain consistent with their current generated implementation.

## Not Started
Shared-control improvements, benchmark theming, targeted settings/overlay fixes, responsive verification, final Windows validation and PR.

## Important Design Decisions
- Keep existing palette identity and teal accent, class-colored full rows and dense table geometry.
- Settings fixed outer dimensions: 800 × 440; Event Tracker editor 720 × 440; DPS settings 620 × 420; Mechanics settings 720 × 430. Windows screenshots exclude invisible frame margins.
- Current app uses Windows DPI virtualization; do not switch the process to per-monitor awareness without implementing full scaling across all windows.
- Event Tracker live events are embedded in Dungeon Mechanics; there is no distinct event overlay to invent.
- Existing offline HTML archives open in the user's browser. They are not the native app's runtime.

## Protected Existing Behavior
Staged Apply in main Settings/Event Tracker; immediate feature settings; native keyboard and tray behavior; compact overlay chrome, saved bounds/opacity/collapse, class colors and meter hide order. Exact overlay constants pending generated-source inspection.

## Files Modified
- `/workspace/scratch/9183acf35e27/BPSR-ReadyAlert/UI_OVERHAUL_HANDOFF.md`: audit/continuation state.

## Responsive Layout Notes
- DPS: generated `feature_overlays_v170_fixed.rs`; tiered toolbar, right-aligned metrics, flexible identity and 20-player raid view. Record exact values before edits.
- Chat: generated `overlay_v150_v1181.rs`, wrapped by `src/overlay_v181.rs`; virtualized message rows, saved scroll state and translated text. Inspect existing fixes first.
- Mechanics: same generated feature overlay module; compact attributes/consumables/events.
- Event Tracker: fixed editor; live events share Mechanics geometry.

## Screenshot Audit Notes
All filenames are relative to `/workspace/scratch/9183acf35e27/screenshots/`.

| Screenshot | State and finding | Decision |
| --- | --- | --- |
| BPSR-ReadyAlert_niax3dCc81.png | General Settings; compact two-column groups, all nav buttons boxed; white checkboxes | Keep geometry; polish shared controls/nav/hierarchy |
| BPSR-ReadyAlert_A7KoSWFeVH.png | Chat settings; aligned fields; bright legacy dropdown arrow | Keep settings/positions; fix common control rendering |
| BPSR-ReadyAlert_UjsoQH0UW5.png | Chat colors; aligned channels, swatches/hex; dense highlights | Keep color grid and values; consistent inputs |
| BPSR-ReadyAlert_3taiftqfmV.png | Speech; disabled native label embossing; clear translation/TTS groups | Retain groups; correct disabled drawing and secondary copy |
| BPSR-ReadyAlert_nhjugxeGyY.png | Tabs; staged new tab, channel matrix close to footer | Preserve compact fit; list focus/selection and delete hierarchy |
| BPSR-ReadyAlert_cRns2y30hB.png | Sounds/logs; paths cut inside narrow fields | Preserve footprint; path affordances if safe |
| BPSR-ReadyAlert_uLx0mNiIu4.png | Network; simple two groups, adequate space | Keep layout; common style only |
| BPSR-ReadyAlert_IwYx53V1lc.png | Blocked users; empty list and enabled-looking destructive controls | Add concise empty state/appropriate disablement |
| BPSR-ReadyAlert_wRmVnx2vNI.png | DPS settings; compact matrix and native slider/combo | Preserve 620×420 footprint and immediate save |
| BPSR-ReadyAlert_0RRFLMPXaL.png | Mechanics settings; 3-column 21-attribute matrix | Preserve spacing and 8-stat maximum |
| BPSR-ReadyAlert_jkKqh2DpMQ.png | Event editor; empty rules, blank/invisible disabled edits and residual list bar | Improve empty guidance/disabled fields; preserve staged Apply |
| BPSR-ReadyAlert_FrdoXYaVr6.png | Benchmark; entirely light theme and small legacy font | Apply existing dark theme, clear Start/Cancel hierarchy; same footprint |
| BPSR-ReadyAlert_dj3VAodKyq.png | Chat normal, 7 messages; large user-selected font, alternating rows | Keep configured density; inspect follow/translation/scroll code |
| BPSR-ReadyAlert_kcUAPuUfTy.png | Mechanics; 3 stats + consumables + no-event message | Keep compact top data, use same chrome tokens; restrained empty message |
| BPSR-ReadyAlert_lZmBcZC6Z6.png | 20-player raid tank view; two columns, full-row class tint | Preserve 20 players, row density, consumable side bands |
| BPSR-ReadyAlert_b3uijZeesU.png | Same raid layout without tooltip | Keep tuned raid metrics/identity priorities |
| explorer_hIIRn2SsG5.png | Normal damage view; self pinned, 4 rows, side consumable bars | Preserve row height, self outline, external bars and flexible spec area |
| explorer_r0lg5CQVIT.png | Normal healing view + tooltip | Keep mode context; inspect tooltip truncation |
| explorer_yDulzmyIPF.png | Normal tank view + target tooltip truncation | Keep data layout; allow tooltip wrapping where needed |
| BPSR-ReadyAlert_qffASEYIPu.png | Entity skill detail; all text bold, dense 14 columns | Keep density; weight hierarchy and neutral surfaces |
| BPSR-ReadyAlert_WN8pTHFGrO.png | Entity buffs; bright native scrollbars, dense rows | Retain scrolling; theme scrollbars and soften heavy type |
| 5X8DRCwZZX.png | Native tray menu; checked items, separators and submenus | Native OS menu behavior is intentional; keep semantics |
| firefox_nqkQmZiNzc.png | Offline archive landing; two centered links | Preserve simple launcher |
| firefox_Eolmbhuvax.png | Encounter detail; fixed max-width leaves outer whitespace | Keep content; inspect responsive table use of width |
| firefox_PGzUWhTPw3.png | Unknown target detail, same archive hierarchy | Unknown source values are not a UI bug; preserve data |
| firefox_hp4SpklDHn.png | Comparison; two saved encounters | Preserve latest v1.27.2 skill comparison feature |
| firefox_ViMqYYU5YK.png | Chat archive; narrow message container amid wide empty margins | Preserve content/filter semantics; consider using available width |

## Bugs / Problems Found
Benchmark is unthemed. Shared owner-draw buttons have insufficient primary-label contrast and no reliable hover tracking. Event disabled controls show legacy embossed glyphs. Existing chat scroll/translation fixes require current generated-code verification.

## Verification Already Done
Source/screenshot audit only. No builds run. Local Rust toolchain not found yet. Existing Rust CI runs only for main pushes or PRs, so branch checkpoint pushes do not cause duplicate builds.

## Known Failures
None in source yet. No Windows runtime available at task start; do not claim live screenshot/DPI testing.

## Exact Next Steps
1. Obtain current generated source through the existing build chain; preserve all build assertions. Record exact minima, row heights, column thresholds, chat scroll/translation logic.
2. Improve `rust/src/ui_theme.rs` and `rust/src/benchmark_ui.rs`; preserve all fixed form bounds.
3. Apply targeted UI changes to directly maintained runtime modules; avoid another legacy patch stage if possible.
4. Verify layout contracts and current Windows CI once after coherent implementation; publish a PR.

## Do Not Revisit Unless Broken
All 27 screenshots have been audited; use this map instead of restarting. Keep dense row/caption/settings geometry and class palette. No protocol/telemetry/archive-format changes.

## Final Cleanup
Retain this handoff until all implementation and necessary validation are complete. Record precise limitations rather than claiming unperformed Windows visual checks. No release build polling loops.
