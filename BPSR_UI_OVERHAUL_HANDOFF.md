# BPSR ReadyAlert Native UI Overhaul Handoff

This document is the completion checklist for the application-wide native UI redesign on PR #120 / `feat/compact-meter-mode`.

## Scope

The redesign covers every discovered ReadyAlert-native Rust/Win32 surface, including rare and error states. The visual direction is now based on **Blue Protocol: Star Resonance** rather than Pixel OS, One UI, Material 3, Fluent, or generic Win32.

The application remains lightweight native Rust + Win32/GDI. No Electron, Tauri, WebView runtime, React, or heavy GUI framework is introduced.

The major visual exclusion is the separately designed archive/Encounter History HTML presentation. Native controls that open or manage archives remain in scope.

## BPSR design system

### MIST GLASS

Used for configuration and utility surfaces:

- Settings and all Settings pages
- Event Tracker editor
- DPS / Mechanics settings popups
- Benchmark dialog
- native confirmation / warning / error / information dialogs
- updater prompts and update-result dialogs

Characteristics:

- cool mist blue-gray window surface
- darker left/navigation rail
- thin pale blue-gray borders
- restrained 3 / 4 / 6 px corner system
- icy cyan selection/focus accent
- pale white primary text and cool-gray secondary text
- no large Material pills
- no One UI card language
- subtle one-pixel/two-pixel GDI shadow separation instead of fake heavy blur

### DARK GLASS

Used for live/in-game surfaces:

- DPS Meter
- Raid DPS Meter
- Compact DPS Meter
- Raid + Compact DPS Meter
- Chat Overlay
- Dungeon Mechanics
- DPS/player/skill detail surfaces
- collapsed overlay handles
- overlay quick actions, tabs and transient helper surfaces

Characteristics:

- near-black/navy glass base
- low-radius clipped panels
- thin cool borders
- icy cyan active state
- semantic class/combat colors preserved as data meaning, not used as general chrome
- compact BPSR-like hierarchy rather than generic desktop cards

## Native UI Inventory

Statuses below are the completion gate. There are no unexplained `NOT STARTED` or `IN PROGRESS` items.

### Application host / shell integration

- Hidden main/tray host window — **COMPLETE** — no user-facing canvas; host behavior unchanged.
- System tray icon + tooltip — **COMPLETE** — shell-owned presentation; text/status integration retained.
- Desktop notification / shell balloon notice — **COMPLETE** — shell-owned presentation intentionally retained for Windows notification reliability.
- Tray root context menu — **COMPLETE** — audited; remains system-rendered because `TrackPopupMenu` is the stable Explorer notification-area path.
- Tray `Network adapter` submenu — **COMPLETE** — same platform-rendered exception.
- Tray `Alert volume` submenu — **COMPLETE** — same platform-rendered exception.
- Tray checked/unchecked/disabled menu states — **COMPLETE** — platform-owned state rendering, semantic state preserved.

### Settings / MIST GLASS

- Settings window chrome/title treatment — **COMPLETE**.
- Settings navigation rail + selected/hover/focus states — **COMPLETE**.
- Settings / General alerts — **COMPLETE**.
- Settings / Chat overlay — **COMPLETE**.
- Settings / Chat colors — **COMPLETE**.
- Settings / Speech & translation — **COMPLETE**.
- Settings / Tabs & filters — **COMPLETE**.
- Settings / Sounds & logs — **COMPLETE**.
- Settings / Network & integration — **COMPLETE**.
- Settings / Blocked users — **COMPLETE**.
- Settings Apply / Close bottom actions — **COMPLETE**.
- Settings buttons / checkboxes / toggles — **COMPLETE**.
- Settings edits / numeric inputs / text inputs — **COMPLETE**.
- Settings dropdowns / combo popup surface — **COMPLETE**.
- Settings lists / selected rows / empty lists — **COMPLETE**.
- Settings disabled action states — **COMPLETE**.
- Settings hover / pressed / keyboard focus states — **COMPLETE**.
- Settings scrolling / minimum-work-area state — **COMPLETE** — Windows scrollbar mechanics retained; surrounding surface and controls use the BPSR system.
- Blocked-user selection / Unblock / Clear-all enablement — **COMPLETE**.
- Tabs list selection / Add / Delete enablement — **COMPLETE**.
- Color values / highlight color fields — **COMPLETE** — current product uses validated `#RRGGBB` fields rather than a custom native color-picker window.
- Settings paths / advanced JSON / ReadyAlert folder actions — **COMPLETE** — no custom file chooser exists in this flow; Explorer/file opening remains shell-owned.

### Event Tracker / MIST GLASS

- Custom Event Tracker window/chrome — **COMPLETE**.
- Global enable + max-row controls — **COMPLETE**.
- Rules list + selected / empty states — **COMPLETE**.
- Add / Remove rule actions — **COMPLETE**.
- Rule enabled state — **COMPLETE**.
- Type dropdown — **COMPLETE**.
- Numeric event ID field — **COMPLETE**.
- Label field — **COMPLETE**.
- Scope dropdown — **COMPLETE**.
- Hold/display-seconds field — **COMPLETE**.
- Apply / Close actions — **COMPLETE**.
- Invalid-input / error modal paths — **COMPLETE** — routed through BPSR utility modal treatment.
- Disabled / hover / pressed / focus states — **COMPLETE**.

### DPS Meter / DARK GLASS

- DPS Meter / Normal — **COMPLETE**.
- DPS Meter / Raid — **COMPLETE**.
- DPS Meter / Compact — **COMPLETE**.
- DPS Meter / Raid + Compact — **COMPLETE**.
- DPS collapsed handle — **COMPLETE**.
- DPS toolbar shell — **COMPLETE**.
- `Live` state — **COMPLETE** — semantic highlight remains tied to live state, not button position.
- `Raid` state — **COMPLETE** — semantic highlight remains tied to raid state.
- `Compact` / `C` state — **COMPLETE** — permanent Compact toggle retained.
- Previous / next (`<` / `>`) actions — **COMPLETE** — never inherit neighboring selected state.
- Reset / settings / close / quick actions — **COMPLETE**.
- Damage / Heal / Tank tabs — **COMPLETE**.
- Target / HP summary — **COMPLETE**.
- Wrapped target hover tooltip — **COMPLETE**.
- Waiting/no-combat empty state — **COMPLETE**.
- Player rows / class-spec identity color — **COMPLETE**.
- Local-player emphasis — **COMPLETE**.
- Dead-player visual state — **COMPLETE**.
- Imagine badges in modes where they are allowed — **COMPLETE**.
- Compact hidden-field contract (Ability Score, Illusion Score, Imagine/build detail, extra chrome) — **COMPLETE**.
- Raid two-column layout — **COMPLETE**.
- Scroll indicator / wheel state — **COMPLETE**.
- Copy -> As Image / Normal — **COMPLETE**.
- Copy -> As Image / Raid — **COMPLETE**.
- Copy -> As Image / Compact — **COMPLETE**.
- Copy -> As Image / Raid + Compact — **COMPLETE**.
- Encounter History native opener/action — **COMPLETE**.

### DPS detail / DARK GLASS

- Player/entity detail window — **COMPLETE**.
- Skill distribution table — **COMPLETE**.
- Detail header — **COMPLETE**.
- Detail selected tabs / section controls — **COMPLETE**.
- Empty skill state — **COMPLETE**.
- Detail scrolling — **COMPLETE**.
- Close / drag interactions — **COMPLETE**.

### DPS / Mechanics configuration / MIST GLASS

- DPS Meter Settings popup — **COMPLETE**.
- DPS opacity controls — **COMPLETE**.
- DPS collapse-side control — **COMPLETE**.
- DPS field visibility controls — **COMPLETE**.
- Dungeon Mechanics Settings popup — **COMPLETE**.
- Mechanics opacity controls — **COMPLETE**.
- Mechanics collapse-side control — **COMPLETE**.
- Mechanics tracked-attribute selection states — **COMPLETE**.

### Dungeon Mechanics / DARK GLASS

- Dungeon Mechanics overlay — **COMPLETE**.
- Mechanics collapsed handle — **COMPLETE**.
- Mechanics toolbar / quick actions — **COMPLETE**.
- Attribute strip — **COMPLETE**.
- Food / serum status panel — **COMPLETE**.
- Mechanic rows / zebra hierarchy — **COMPLETE**.
- High-priority warning state — **COMPLETE**.
- LIVE / countdown states — **COMPLETE**.
- No-active-mechanic empty state — **COMPLETE**.
- Mechanics scroll indicator — **COMPLETE**.

### Chat Overlay / DARK GLASS

- Chat Overlay normal mode — **COMPLETE**.
- Chat compact-message layout — **COMPLETE**.
- Chat collapsed edge handle — **COMPLETE**.
- Chat toolbar — **COMPLETE**.
- Chat tabs / selected tab — **COMPLETE**.
- Tab overflow action — **COMPLETE**.
- `+ Tab` action — **COMPLETE**.
- TTS enabled / muted / unavailable semantic states — **COMPLETE**.
- Settings / collapse / hide actions — **COMPLETE**.
- Message rows / zebra / channel color band — **COMPLETE**.
- Private/highlight row states — **COMPLETE**.
- Translation row treatment — **COMPLETE**.
- Waiting-for-chat empty state — **COMPLETE**.
- No-messages-in-tab empty state — **COMPLETE**.
- Manual-scroll reading state — **COMPLETE**.
- Unseen/new-message back-to-latest pill — **COMPLETE**.
- Resize-grip interaction region — **COMPLETE** — invisible hit-region behavior preserved.
- Click-through recovery / keyboard state — **COMPLETE**.
- Chat tab/actions right-click menu — **COMPLETE** — audited; remains system-rendered `TrackPopupMenu` for native stability.
- Chat per-message player context menu — **COMPLETE** — audited; same system-rendered exception.
- Block-player action / resulting blocked-user state — **COMPLETE**.

### Benchmark / utility dialogs / MIST GLASS

- Benchmark dialog — **COMPLETE** — fixed 440 x 285 footprint preserved.
- Benchmark text inputs — **COMPLETE**.
- Benchmark Start / Cancel actions — **COMPLETE**.
- Benchmark invalid-duration warning — **COMPLETE** — BPSR utility modal path.
- Generic information dialog path — **COMPLETE**.
- Generic warning dialog path — **COMPLETE**.
- Generic error dialog path — **COMPLETE**.
- Generic Yes / Later confirmation path — **COMPLETE**.
- Startup single-instance notice — **COMPLETE**.
- Npcap-required / capture-start error notice — **COMPLETE**.
- Network-adapter failure notice — **COMPLETE**.

### Updater / MIST GLASS

- Update-check-busy dialog — **COMPLETE**.
- Up-to-date dialog — **COMPLETE**.
- Update-available confirmation — **COMPLETE**.
- Update download/check failure dialog — **COMPLETE**.
- Update install/helper failure dialog — **COMPLETE**.
- Update restart confirmation state — **COMPLETE**.
- Updater progress window — not present in the current native codebase; download remains background work followed by a modal decision/result path.

### Archive/native boundary

- Native `Open local archives` action — **COMPLETE**.
- Native Encounter History open/manage action — **COMPLETE**.
- Encounter History HTML presentation — **INTENTIONALLY EXCLUDED** — separately designed browser/offline archive surface.
- Encounter History HTML CSS — **INTENTIONALLY EXCLUDED**.
- Encounter History HTML JavaScript — **INTENTIONALLY EXCLUDED**.
- Archive HTML visual theme / archive hub HTML presentation — **INTENTIONALLY EXCLUDED**.

### Native surfaces not found in the authoritative code sweep

No separate native first-run wizard, onboarding window, About dialog, changelog/release-notes window, custom log viewer, custom debug panel, custom network-debug window, custom file/folder picker, or custom updater-progress window is currently registered/created by the native ReadyAlert code. If one is added later, it must select MIST GLASS or DARK GLASS explicitly and be added to this inventory.

## Platform-rendered exceptions

The following remain Windows/shell rendered after audit because replacing them would trade native reliability for cosmetic imitation:

- Explorer notification-area popup menus and nested tray submenus (`TrackPopupMenu`)
- Chat right-click popup menus (`TrackPopupMenu`)
- Windows shell desktop notification/balloon presentation
- Explorer/file opening launched by ShellExecute
- native scrollbar mechanics used by scrollable configuration forms

These are not forgotten legacy UI. They are documented platform boundaries. Their surrounding ReadyAlert windows, controls, labels and state semantics use the BPSR system.

## Protected behavior / density

The BPSR visual pass must not change telemetry, protocol parsing, DPS calculations, encounter storage, updater safety, settings persistence, hotkeys, archive meaning, chat manual-scroll behavior, or Compact/Raid algorithms.

Protected layout contracts remain:

- Settings: 800 x 440 target budget
- Event Tracker: 720 x 440 target budget
- DPS Settings: 620 x 420 target budget
- Dungeon Mechanics Settings: 720 x 430 target budget
- Benchmark: 440 x 285
- Normal DPS row: about 33 px
- Compact DPS row: about 24 px
- Compact minimum: 300 x 140 logical px
- Compact non-Raid default width: 420 logical px
- Compact Raid default width: 760 logical px
- Compact Raid: two columns, ranks 1-10 / 11-20
- Chat minimum: 360 x 180
- Mechanics minimum: 400 x 220

## Implementation points

- `rust/src/ui_modern.rs` now owns the two BPSR surface families, low-radius geometry, cool-cyan focus/selection states, Mist titlebar treatment, dark overlay helpers, control hover/focus handling and the synchronous BPSR native utility modal.
- `rust/build/legacy/build_v1290_native_finish.rs` remains the final generated-source stage after Compact stage1..stage7 and propagates the BPSR system across Settings, Event Tracker, DPS/Mechanics, Chat, generic dialogs and updater dialogs.
- `rust/src/benchmark_ui.rs` uses the Mist Glass controls and BPSR utility warning path directly.
- Historical modernization stages remain earlier in the build chain only to preserve stable generated-source behavior/anchors. The final BPSR stage is authoritative for presentation.

## Completion test

Repository discovery and the inventory above are the completion checklist. There are no remaining `NOT STARTED` or `IN PROGRESS` native surfaces in this handoff.

A real interactive Windows screenshot/DPI review is still a manual acceptance step; CI/build success must not be described as visual proof. If a manual pass finds a visual mismatch, update the relevant inventory item and fix it on this same PR rather than starting another visual branch.
