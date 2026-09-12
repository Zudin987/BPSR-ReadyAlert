# ReadyAlert UI Overhaul Handoff

The canonical native UI handoff for PR #120 is now:

- `BPSR_UI_OVERHAUL_HANDOFF.md` — complete native UI inventory, BPSR Mist Glass / Dark Glass design system, platform exceptions, protected behavior and completion checklist.
- `ARCHIVE_UI_HANDOFF.md` — separately scoped Encounter History/archive HTML presentation.

This file is retained only as a compatibility pointer for earlier work. The previous Pixel / Material / One UI native direction is no longer authoritative and must not be used for new or existing native ReadyAlert surfaces.

## Current branch / PR

- Branch: `feat/compact-meter-mode`
- PR: #120
- Native UI: BPSR design system from `BPSR_UI_OVERHAUL_HANDOFF.md`
- Encounter History HTML: intentionally separate archive design; do not apply the native BPSR theme to its CSS/JavaScript.

## Protected behavior

The visual overhaul must continue to preserve Compact/Raid mode logic, telemetry/protocol behavior, encounter calculations/storage, updater safety, settings persistence, Chat manual-scroll behavior, copy-as-image state, hotkeys and native lightweight Rust/Win32 architecture.

Do not merge or release solely because this handoff is complete. CI and an interactive Windows visual/DPI acceptance pass are separate gates.
