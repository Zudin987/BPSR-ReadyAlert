# Season 4 future-proofing

This is the compatibility contract for the ReadyAlert development-freeze build.

## Season Strength

ReadyAlert treats the seasonal player stat as the generic protocol concept **Season Strength**, not a Season 3-specific display name.

- `11440` / `0x2CB0` - `AttrSeasonStrength` - preferred final value.
- `11441` / `0x2CB1` - `AttrSeasonStrengthTotal` - fallback when the final value has not been synchronized.
- `11442`-`11444` are component/modifier fields and are deliberately **not** added together by ReadyAlert.

The serialized `DpsRow.illusion_break` field is intentionally retained. Old encounter-history files already use that field name, so renaming it would create unnecessary compatibility risk. Its runtime meaning is now generic Season Strength.

## Unknown future IDs

Unknown game-data IDs are non-fatal. Combat packets continue through the existing telemetry parser whether a name is known or not.

ReadyAlert keeps numeric identity in user-facing fallbacks where practical, for example:

- `Unknown Skill (123456)`
- `Unknown Target (123456)`
- `Unknown Scene (123456)`
- `Class 99`

Newly observed unknown skill, buff, monster, class, attribute and other catalog IDs are written once to:

`%LOCALAPPDATA%\BPSR-ReadyAlert\unknown_ids.log`

Each line is:

`unix_milliseconds<TAB>kind<TAB>id`

The log is diagnostic only and never controls whether damage/healing is counted.

## Runtime name overrides

The frozen EXE can be corrected for future content without recompiling. On first use ReadyAlert creates:

`%LOCALAPPDATA%\BPSR-ReadyAlert\game_names_override.tsv`

Format:

`KIND<TAB>ID<TAB>NAME`

Supported kinds:

- `S` skill/action
- `B` buff
- `E` scene
- `D` dungeon
- `M` monster/entity
- `T` talent
- `F` factor
- `G` factor-grade item
- `P` specialization
- `C` class
- `X` modifier effect
- `A` attribute
- `O` objective
- `R` recount row

Example:

`S<TAB>123456<TAB>Official Global Skill Name`

Runtime overrides take priority over embedded ReadyAlert/global/Season 4 fallback names. Restart ReadyAlert after editing because the override table is loaded once per process.

## What is intentionally not season-dependent

- DPS/healing/tank totals use observed network combat events, not a hardcoded seasonal damage formula.
- A known scene ID is not required for the DPS meter to function.
- A known boss name is not required for encounter damage to be counted.
- Unknown classes/specs remain displayable instead of removing the player row.
- Unknown mechanics may reduce mechanic-specific warnings, but must not stop the meter or chat overlay.
- Extra protobuf fields are ignored by the existing field-oriented parser unless ReadyAlert explicitly understands them.

## Limit

No static build can guarantee compatibility if the game fundamentally replaces the network service/method layout, encryption/compression, entity encoding, or combat-event schema. This pass is designed to survive new Season 4 IDs, names, scenes, classes/specs and the known generic seasonal-stat family without requiring a new EXE.
