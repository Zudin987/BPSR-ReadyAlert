# Season 4 future-proofing

This is the compatibility contract for the ReadyAlert development-freeze build.

## Season Strength

ReadyAlert treats the seasonal player stat as the generic protocol concept **Season Strength**, not a Season 3-specific display name.

- `11440` / `0x2CB0` - `AttrSeasonStrength` - preferred final value.
- `11441` / `0x2CB1` - `AttrSeasonStrengthTotal` - fallback when the final value has not been synchronized.
- `11442`-`11444` are component/modifier fields and are deliberately **not** added together by ReadyAlert.

The serialized `DpsRow.illusion_break` field is intentionally retained. Old encounter-history files already use that field name, so renaming it would create unnecessary compatibility risk. Its runtime meaning is now generic Season Strength.

## Generic Boss DBM scene events

The freeze build observes the game's `SyncSceneEvents` stream (`0x08`) in addition to the existing buff/skill mechanic sources. Boss DBM event type `29` is decoded using the packet's skill-effect ID, duration and insertion slot.

This means a future dungeon or raid does not need to be compiled into ReadyAlert before its game-provided DBM timer can appear. Name resolution is:

1. runtime mechanic override;
2. known full skill-effect ID;
3. known base skill ID (`effect_id / 100`);
4. numeric fallback such as `Boss mechanic #12345601`.

The duration supplied by the game packet is authoritative unless a runtime mechanic override explicitly replaces it.

`NoticeTip` scene events (type `1`) are recorded as diagnostics because the game uses them for on-screen mechanic instructions. They do not automatically become intrusive warnings unless a runtime rule maps the ID.

## Runtime mechanic overrides

A frozen EXE can add or correct mechanic behavior without recompiling. On first use ReadyAlert creates:

`%LOCALAPPDATA%\BPSR-ReadyAlert\mechanics_override.tsv`

Format:

`SCENE<TAB>KIND<TAB>ID<TAB>LABEL<TAB>DURATION_MS<TAB>PRIORITY`

- `SCENE` - numeric scene ID; `0` means any scene.
- `KIND=D` - Boss DBM effect ID.
- `KIND=S` - observed combat skill ID.
- `KIND=A` - observed attribute ID.
- `KIND=N` - `NoticeTip` ID.
- `DURATION_MS=0` - use the game's DBM duration for `D`; use the safe transient default for the other kinds.
- `PRIORITY` - `0` through `3`.

Example:

`13031<TAB>D<TAB>12345601<TAB>Incoming mechanic<TAB>0<TAB>3`

Scene-specific rules take precedence over scene `0` rules. Restart ReadyAlert after editing because the override table is loaded once per process.

Observed `NoticeTip` data is written once per unique scene/ID/text combination to:

`%LOCALAPPDATA%\BPSR-ReadyAlert\unknown_mechanics.log`

This log is diagnostic only.

## Event Tracker forward compatibility

Custom Event Tracker rules support:

- Buff ID
- Skill ID
- Attribute ID

The freeze build accepts up to 64 rules. Attribute rules are change-driven: repeated synchronization of the same value does not create a new event, while a changed value refreshes the configured transient row. Existing Self / Party / Any scope behavior remains in effect.

## Season 4 encounter metadata

Known Season 4 scene names still come from the global/Season 4 name catalog, with runtime name overrides taking precedence. When the older dungeon table has no row, the freeze build supplies only metadata that is explicitly identifiable from current CN scene variants:

- Judgment in the Mirror - Unstable / Hard / Master variants.
- Desolate Court - Unstable / Hard / Extreme / Master variants.
- Divine Threshold of the Distant Sky - Unstable / Hard / Master variants.
- Above the Sky, End of Day and Night raid - Hard / Nightmare / Challenge variants.

Unknown dungeon IDs are deliberately left unset rather than guessed. The Season 4 raid family uses the same generic play-type classification as the existing seasonal raid family.

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
- Unknown mechanic names do not prevent a game-provided DBM timer from being displayed.
- Extra protobuf fields are ignored by the existing field-oriented parser unless ReadyAlert explicitly understands them.

## Limit

No static build can guarantee compatibility if the game fundamentally replaces the network service/method layout, encryption/compression, entity encoding, combat-event schema, or scene-event schema. This pass is designed to survive new Season 4 IDs, names, scenes, classes/specs, known seasonal-stat fields, and game-provided Boss DBM events without requiring a new EXE for ordinary data changes.
