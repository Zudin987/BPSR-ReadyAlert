# Season 4 CN ID supplement

This document records the provenance and curation rules for `rust/data/game_names_season4_cn.tsv`.

## Scope

The runtime supplement contains **288 curated Season 4 mappings** that were not reliably covered by ReadyAlert's existing v1.30 supplemental catalog during the audit:

- 107 skill/action IDs (`S`)
- 13 buff/mechanic IDs (`B`)
- 38 scene/place IDs (`E`)
- 130 monster/entity IDs (`M`)

The file is deliberately small and Season-4-focused. A broad comparison against the current CN tables found 12,125 English-labelled IDs missing from the old v1.30 catalog, plus 819 Chinese-only rows, but most of that delta is old content, internal data, development records, duplicated variants, or unrelated game-data. It is not safe to import wholesale.

At runtime the v1.30/global catalog remains authoritative. The Season 4 table is loaded **fallback-only**: if the base catalog already knows an ID, the existing name wins.

## Primary source

The primary source is the actively maintained CN fork of Resonance Logs:

- `https://github.com/fudiyangjin/resonance-logs-cn`
  - `src/lib/config/SceneName.json`
  - `src/lib/config/en-US/SceneName.json`
  - `src/lib/config/MonsterIdNameType.json`
  - `src/lib/config/en-US/MonsterIdNameType.json`
  - `src/lib/config/MonsterSkillName.json`
  - `src/lib/config/en-US/MonsterSkillName.json`
  - `src/lib/config/BuffName.json`
  - `src/lib/config/en-US/BuffName.json`
  - `src/lib/config/RecountTable.json`
  - `src/lib/config/en-US/RecountTable.json`

Its dedicated Season 4 Desolate Court implementation is especially useful because it identifies IDs by mechanic role rather than only by translated table names:

- Scene `6615`
- Buffs `884609`, `884610`, `884614`, `884615`, `884616`, `884641`, `884659`, `884660`, `884661`, `884664`
- Relevant entities `4701`, `4711`, `4702`, `470131`, `884606`, `884607`, `884640`, `884642`, `884668`, `884669`, `884670`, `884671`
- Mechanic skills `470112`, `470113`, `470119`, `470125`, `470132`

## CN/community cross-check sources

The audit also searched or cross-checked the following public combat-data projects. They are useful because several share packet/game-table knowledge but expose it in different forms.

- `https://github.com/dmlgzs/StarResonanceDamageCounter` - CN packet-based damage counter; contains `tables/monster_names.json`, `tables/skill_names.json`, and `tables/skill_names_new.json`.
- `https://github.com/anying1073/StarResonanceDps` - CN DPS analyzer with CN/EN/JP/KR extracted tables. Its Recount/Bullet data independently exposes the Season 4 `340000xxxx` Boyce damage/action family and its monster data identifies Boyce.
- `https://github.com/cuteSATOU/StarResonanceDPS` - separate CN real-time DPS meter, checked for overlapping combat terminology/data.
- `https://github.com/EmiyaGm/StarResonanceDPS` - CN Star Resonance DPS implementation, checked as an additional community reference.
- `https://github.com/mxihan/StarResonanceDPS` - CN Star Resonance DPS implementation, checked as an additional community reference.
- `https://github.com/Viemean/StarResonance.DPS` - CN Star Resonance DPS implementation, checked as an additional community reference.
- `https://github.com/CKylinMC/StarResonanceDamageCounterOverlay` - companion overlay/API consumer for the CN damage-counter ecosystem.
- `https://github.com/donneeee/resonance-logs-global` - global fork used as an English-name cross-check when a global name exists.
- `https://github.com/Blue-Protocol-Source/BPSR-ZDPS` - existing global DPS/parser reference; ReadyAlert's existing/global names take precedence over this CN fallback table.

Finding a row in another meter is not enough by itself to make it runtime-visible. The final table was restricted to IDs tied to Season 4 maps, bosses, dungeon mechanics, story/map enemies, or Season 4 event content.

## Season 4 content groups covered

### Judgment in the Mirror / Boyce

Scene variants `6591`-`6594`, Boyce/clone/entity rows around `34000`-`34036` and `5100302`, plus the Boyce `3400000`-series combat actions and related transformation/story skills.

### Desolate Court / Vilda

Scene variants `6610`-`6615`, Vilda and story-route entities, the dedicated Desolate Court mechanic entities/buffs, and the `4701xx` mechanic/action family. IDs used directly by the CN minimap/mechanics implementation are included even when the generic monster-name table does not provide a useful display name.

### Divine Threshold of the Distant Sky / Lapsis

Scene variants `1901`, `1911`, `1912`, `1931`, `1932`, Lapsis phase entities (`103400`, `103500`, `103600`) and the corresponding `1035xxxx`/`1036xxxx` action families.

### Delusion and Season 4 challenge variants

- Dragon Claw Valley: `1231`-`1235`
- Dark Mist Fortress: `1341`-`1344`
- Moonlit Phantasm Wilds: `1711`, `1712`, `1722`, `1723`
- Above the Sky, End of Day and Night raid variants: `13031`-`13033`

### Montenor Valley and Season 4 story/map enemies

The base catalog already knows the main Montenor Valley scene ID, so it is not duplicated. The supplement adds uncovered Valley enemies, elites, route NPC/entity IDs, and a small set of uncovered Valley combat actions.

### Deep Sea / Wingwhale Season 4 activity content

Scene IDs `60000`-`60004`, Wingwhale Survey Areas `14001`/`14002`, and the small set of uncovered event markers/actions tied to this content.

## Translation and cleanup rules

1. Existing ReadyAlert/global names always win. CN data is fallback-only.
2. Prefer an existing global English string when it clearly names the same ID/content.
3. If the CN source has no usable English label, translate the Chinese player-facing name manually instead of exposing Chinese in the English UI.
4. Strip source-only suffixes such as `(AI)` from player-facing boss names when they do not distinguish a meaningful encounter entity.
5. Exclude explicit test, deprecated, temporary, placeholder and arena-marker rows unless the ID has a verified player-facing mechanic purpose.
6. Do not expose obviously broken machine translations. For example, two Vilda skill rows translated as `The Witcher 2` were intentionally omitted rather than guessing their exact attack names.
7. Normalize known boss spelling to the English names used by the Season 4 guide/data context: **Boyce**, **Vilda**, and **Lapsis**.
8. Keep mechanic descriptors when no canonical global skill name exists. A useful fallback such as `Vilda - Near Circle` is preferable to an unknown numeric ID.

### Manual scene translations

The following Chinese-only scene labels were translated for the supplement:

- `14001` `翼鲸调查区·一` -> `Wingwhale Survey Area I`
- `14002` `翼鲸调查区·二` -> `Wingwhale Survey Area II`

Difficulty prefixes were translated literally from the CN scene table (`困难` = Hard, `大师` = Master, `极限` = Extreme, `不稳定空间` = Unstable Space). Where an English source conflicted with the CN difficulty/content name, the CN row and known Season 4 content identity were used rather than preserving a clearly mismatched English label.

## Safety/regression expectations

Tests enforce that:

- the old v1.30 compressed catalog retains its exact historical counts and byte size;
- the merged catalog remains strictly sorted with no duplicate IDs per kind;
- representative Season 4 scenes, bosses, skills and mechanics resolve to English names;
- the Season 4 runtime supplement contains no CJK text, `(AI)` suffixes, or explicit `Deprecated` placeholders;
- unknown IDs still remain unknown rather than receiving guessed labels.
