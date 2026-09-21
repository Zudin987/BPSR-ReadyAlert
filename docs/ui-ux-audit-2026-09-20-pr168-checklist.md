# 20 September 2026 UI/UX audit — PR #168 verification checklist

Source of truth: `BPSRAlert-Detailed-UI-UX-Audit-2026-09-20(1).html` (36 findings). **An unchecked finding is not verified complete.** `Partial` means some native Rust/Win32 implementation exists; it does not mean the full original acceptance criteria are satisfied. `Open` means no relevant PR implementation established. This file tracks the audit, not a release or sign-off.

## Scaling (4)
- [ ] **S01 — Partial.** Coordinate header, names, metrics, badges and controls at 100/90/80/70/60; no tiny header or clipping. Existing adaptive rows/fonts plus new v1381 toolbar-height consistency and native geometry test require final screenshots and DPI verification.
- [ ] **S02 — Partial.** Scale-only preserves complete visible-row budget when space permits; constrained view scrolls to remaining rows, never conflates a maximum row setting with visible count. v1375 preservation helper, v1376 wording and v1380 five-scale regression require native resize inspection.
- [ ] **S03 — Partial.** Long Latin/CJK names, classes and highest values never collide with badges; optional information remains in inspection and comes back after widening. v1377 verifies name/class-first planner; native stress screenshots required.
- [ ] **S04 — Partial.** UI-size label, limits, helper adjacent to setting, repeatable reset, unaffected mode/row/persistence. Label/helper changed, full min/max and persistence checks outstanding.

## Colour/contrast (4)
- [ ] **C01 — Partial.** Class-tinted whole rows identifiable on dark and bright/busy backgrounds with correct per-class palette and dominant legible primary text. 18% class blend in branch; per-class contrast and gameplay comparisons outstanding.
- [ ] **C02 — Partial.** Small class and secondary labels have measured >=4.5:1 text contrast (prefer >=7:1 primary), against actual tinted surfaces in all modes. Token brightened, numeric contrast audit outstanding.
- [ ] **C03 — Partial.** Restrained 2-logical-pixel metric bar with >=1-device-pixel visibility, stable normalized lengths and distinct damage/heal/tank meaning. Size changed; modes and zero tests outstanding.
- [ ] **C04 — Partial.** Persistent non-colour YOU cue; self, hover, keyboard focus and inspection distinguishable simultaneously and in grayscale without hiding class/bars. YOU/divider implemented; focus/hover combinations outstanding.

## Meter, history and Raid (8)
- [ ] **M01 — Partial.** Every mode clearly distinguishes history/live even at 60%, Return to live restores current encounter, layout change keeps selected history. History title and return label implemented; integration checks outstanding.
- [ ] **M02 — Partial.** Pinned self shown once, true rank preserved, divider clearly communicates rank jump; verify rank 1, 12, last, noncontributor, absent and scrolling. Divider/label present; scenarios outstanding.
- [ ] **M03 — Partial.** Ranking basis intelligible even when Total hidden; total and rate intentionally allowed to rank differently and all modes share actual documented sort. T# label/help implemented; numerical confirmation outstanding.
- [ ] **M04 — Partial.** Food/Serum accessible names, active/inactive/unknown truth, correctly aligned indicators and details. Unknown wording improved; new unknown tests, remaining state/hover visual checks outstanding.
- [ ] **M05 — Partial.** Waiting vs real zero vs unknown vs historical zero distinguishable without inventing capture failures; combat starts without losing roster. Waiting title added; lifecycle scenarios outstanding.
- [ ] **M06 — Partial.** Zero/one/two badge artwork, missing assets, multidigit counts and hover states fit row height and do not shift metrics. Name-first badge planner present; exhaustive asset/counter checks outstanding.
- [ ] **M07 — Partial.** Toggle every column and combinations at narrow/wide widths without header/data drift, clipped units or stale slots; full precision on inspection. Planner and heading checks present; combinations outstanding.
- [ ] **M08 — Open.** Two-column Raid clearly reads ranks 1–10 then 11–20 for 10/11/20/21+ people, with consistent gutter, mouse and keyboard traversal, self/empty/scroll boundaries.

## Interaction (6)
- [ ] **I01 — Partial.** All settings/modal children stay above app overlays when overlapping, on multiple monitors, without changing permanent overlay topmost setting. Transient Win32 raise added; overlap/ownership verification outstanding.
- [ ] **I02 — Open.** Tooltips at both Raid columns and screen edges avoid covering inspected result, never persist on unrelated row after scroll and stay onscreen.
- [ ] **I03 — Partial.** Real icon pointer rectangles >=24x24 logical units, no overlap at scales/DPI; Copy/Reset remain icon-only and correctly labelled. Rect minimum and native geometry regression added; DPI/hit-edge tests outstanding.
- [ ] **I04 — Partial.** Layout/action menu grouping, selected mode and discoverable history/benchmark/settings; menu placement/state survive reopens/restarts. Tooltip refined; full grouping/state checks outstanding.
- [ ] **I05 — Partial.** Only compact visible restore HWND blocks gameplay input; recovery works after monitor/resolution or scaling changes. Collapsed HWND extent capped with test; desktop input-region check outstanding.
- [ ] **I06 — Partial.** Hide/collapse/exit/click-through distinguishable and reversible via tested hotkey/menu; directions and first-time cue correct. Tooltip changed; remaining routes/cues outstanding.

## Chat (4)
- [ ] **H01 — Partial.** Duplicate laughter/emoji translations suppressed while original and meaningful short Malay/English/CJK and punctuation survive. Redundancy logic/tests added; emoji-specific tests outstanding.
- [ ] **H02 — Partial.** TTS OFF/ON/unavailable and channel scope match actual settings; no clipped labels. Visible state added and v1381 reserves full label width; channel-scope visual checks outstanding.
- [ ] **H03 — Partial.** Empty vs filtered states readable at minimum width and truthful, actionable next step, tab switching preserves scroll/messages. Short filtered copy implemented; remaining action/behavior checks outstanding.
- [ ] **H04 — Partial.** One/four/many long Latin/CJK tabs, larger font and action reservation use accessible overflow without losing selected/unread status. Existing overflow plus v1381 speech width reservation; stress/input tests outstanding.

## Tracker (4)
- [ ] **T01 — Partial.** Optional compact idle display preserves stats/timers and fixed-height preference; active transition, scroll and unknown status correct. Waiting wording changed, optional density not implemented/verified.
- [ ] **T02 — Partial.** Counter and grid enforce 0/1/8/attempted ninth without replacing saved stats; inline limit feedback, ordering/restart and long labels correct. Copy added; ninth-attempt behavior outstanding.
- [ ] **T03 — Partial.** ID/duration helper plus inline validation for blank, invalid, duplicate, unsupported and limit; valid rule add/save/disable/remove persists. Helper added, error/focus lifecycle outstanding.
- [ ] **T04 — Partial.** All tracker windows, buttons and gear entry identify Tracker & Mechanics consistently with Event Tracker as subfeature; no duplicated settings. Labels updated; entire navigation audit outstanding.

## Settings (6)
- [ ] **G01 — Partial.** Every dialog accurately labels Apply vs immediate save; Close/X/Escape/page navigation correctly manage dirty status and persistence. Explicit main-settings policy added; every-surface tests outstanding.
- [ ] **G02 — Partial.** Save & close, Discard and Keep editing each do exactly one action; validation failure stays editable and Escape/Enter defaults never discard unexpectedly. Win32 Yes/No/Cancel implementation changed; modal interaction tests outstanding.
- [ ] **G03 — Open.** Real live channel/private/keyword colour samples, inline hex validity, scoped restore default and contrast hint; pasted/short/invalid hex behavior verified.
- [ ] **G04 — Open.** Sound path Browse and volume-respecting Play/Test, full long/Unicode paths, missing/unsupported feedback without saving unrelated settings.
- [ ] **G05 — Partial.** Real parser examples plus inline validation and test-message evaluation for valid/invalid/literal/empty rules, no frozen UI or mutated saved rules. Examples changed; test-message preview outstanding.
- [ ] **G06 — Open.** Common terminology/units/spacing/reset wording for equivalent controls, explain chat font vs UI size, preserve saved settings during migration.

## Mandatory final gates (all presently unchecked)
- [ ] No stale generated anchors silently skip required behavior; inspect final compiled generated `.rs` outputs.
- [ ] Windows tests, Clippy, release build, smoke and size budget all succeed on the **final** PR head.
- [ ] Inspect native screenshots for all four layouts at 100, 90, 80, 70 and 60 percent, including clipping, row counts, truncation, counters, contrast and click rectangles.
- [ ] Verify bright/busy-game opacity, mixed DPI, resizing, history/sorting, chat/tracker/settings persistence, telemetry and all four original DPS layouts.
- [ ] Revisit each checkbox against its full original HTML `Done when:` acceptance text; leave any unverified checkbox unchecked. Keep PR draft; do not merge or release.
