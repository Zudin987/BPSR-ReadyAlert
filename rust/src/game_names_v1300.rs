#![allow(dead_code)]

use std::sync::OnceLock;

const CATALOG_PARTS: [&[u8]; 14] = [
    include_bytes!("../data/game_names_v1300.tsv.zst.001"),
    include_bytes!("../data/game_names_v1300.tsv.zst.002"),
    include_bytes!("../data/game_names_v1300.tsv.zst.003"),
    include_bytes!("../data/game_names_v1300.tsv.zst.004"),
    include_bytes!("../data/game_names_v1300.tsv.zst.005a"),
    include_bytes!("../data/game_names_v1300.tsv.zst.005b"),
    include_bytes!("../data/game_names_v1300.tsv.zst.005c"),
    include_bytes!("../data/game_names_v1300.tsv.zst.005d"),
    include_bytes!("../data/game_names_v1300.tsv.zst.006"),
    include_bytes!("../data/game_names_v1300.tsv.zst.007"),
    include_bytes!("../data/game_names_v1300.tsv.zst.008a"),
    include_bytes!("../data/game_names_v1300.tsv.zst.008b"),
    include_bytes!("../data/game_names_v1300.tsv.zst.008c"),
    include_bytes!("../data/game_names_v1300.tsv.zst.008d"),
];

const SEASON4_CN_SUPPLEMENT: &str = include_str!("../data/game_names_season4_cn.tsv");

#[derive(Default)]
struct Catalog {
    skills: Vec<(i32, &'static str)>,
    buffs: Vec<(i32, &'static str)>,
    scenes: Vec<(i32, &'static str)>,
    dungeons: Vec<(i32, &'static str)>,
    monsters: Vec<(i32, &'static str)>,
    talents: Vec<(i32, &'static str)>,
    factors: Vec<(i32, &'static str)>,
    factor_grade_items: Vec<(i32, &'static str)>,
    specs: Vec<(i32, &'static str)>,
    classes: Vec<(i32, &'static str)>,
    modifier_effects: Vec<(i32, &'static str)>,
    attributes: Vec<(i32, &'static str)>,
    objectives: Vec<(i32, &'static str)>,
    recount_rows: Vec<(i32, &'static str)>,
}

static CATALOG: OnceLock<Catalog> = OnceLock::new();

fn catalog() -> &'static Catalog {
    CATALOG.get_or_init(load_catalog)
}

fn parse_catalog_id(kind: &str, raw_id: &str) -> Option<i32> {
    if let Ok(id) = raw_id.parse::<i32>() {
        return Some(id);
    }

    // Skill ids arrive from the packet as a varint and the native telemetry
    // path intentionally stores them as i32 (`value as i32`). Some game-data
    // skill keys are unsigned/composite values wider than signed i32, so use
    // the same low-32-bit normalization here instead of silently dropping them.
    if kind == "S" {
        return raw_id.parse::<u64>().ok().map(|id| id as i32);
    }

    None
}

fn entries_for_kind<'a>(catalog: &'a mut Catalog, kind: &str) -> Option<&'a mut Vec<(i32, &'static str)>> {
    match kind {
        "S" => Some(&mut catalog.skills),
        "B" => Some(&mut catalog.buffs),
        "E" => Some(&mut catalog.scenes),
        "D" => Some(&mut catalog.dungeons),
        "M" => Some(&mut catalog.monsters),
        "T" => Some(&mut catalog.talents),
        "F" => Some(&mut catalog.factors),
        "G" => Some(&mut catalog.factor_grade_items),
        "P" => Some(&mut catalog.specs),
        "C" => Some(&mut catalog.classes),
        "X" => Some(&mut catalog.modifier_effects),
        "A" => Some(&mut catalog.attributes),
        "O" => Some(&mut catalog.objectives),
        "R" => Some(&mut catalog.recount_rows),
        _ => None,
    }
}

fn ingest_catalog_text(catalog: &mut Catalog, text: &'static str, fallback_only: bool) {
    for line in text.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.splitn(3, '\t');
        let Some(kind) = parts.next() else { continue };
        let Some(raw_id) = parts.next() else { continue };
        let Some(name) = parts.next() else { continue };
        let Some(id) = parse_catalog_id(kind, raw_id) else { continue };
        if name.is_empty() || is_excluded_development_record(kind, id) {
            continue;
        }
        let Some(entries) = entries_for_kind(catalog, kind) else { continue };
        if fallback_only && entries.iter().any(|entry| entry.0 == id) {
            continue;
        }
        entries.push((id, name));
    }
}

fn sort_catalog(catalog: &mut Catalog) {
    catalog.skills.sort_by_key(|entry| entry.0);
    catalog.buffs.sort_by_key(|entry| entry.0);
    catalog.scenes.sort_by_key(|entry| entry.0);
    catalog.dungeons.sort_by_key(|entry| entry.0);
    catalog.monsters.sort_by_key(|entry| entry.0);
    catalog.talents.sort_by_key(|entry| entry.0);
    catalog.factors.sort_by_key(|entry| entry.0);
    catalog.factor_grade_items.sort_by_key(|entry| entry.0);
    catalog.specs.sort_by_key(|entry| entry.0);
    catalog.classes.sort_by_key(|entry| entry.0);
    catalog.modifier_effects.sort_by_key(|entry| entry.0);
    catalog.attributes.sort_by_key(|entry| entry.0);
    catalog.objectives.sort_by_key(|entry| entry.0);
    catalog.recount_rows.sort_by_key(|entry| entry.0);
}

fn load_catalog_from_sources(include_season4: bool) -> Catalog {
    let compressed_len: usize = CATALOG_PARTS.iter().map(|part| part.len()).sum();
    let mut compressed = Vec::with_capacity(compressed_len);
    for part in CATALOG_PARTS {
        compressed.extend_from_slice(part);
    }
    let decoded = zstd::stream::decode_all(compressed.as_slice())
        .expect("decode embedded BPSR supplemental id catalog");
    let text = String::from_utf8(decoded).expect("supplemental id catalog is UTF-8");
    let text: &'static str = Box::leak(text.into_boxed_str());

    let mut out = Catalog::default();
    ingest_catalog_text(&mut out, text, false);
    if include_season4 {
        // CN Season 4 rows are deliberately fallback-only. Existing v1.30
        // global/ZDPS mappings always win when both catalogs know the same ID.
        ingest_catalog_text(&mut out, SEASON4_CN_SUPPLEMENT, true);
    }
    sort_catalog(&mut out);
    out
}

fn load_catalog() -> Catalog {
    load_catalog_from_sources(true)
}

fn is_excluded_development_record(kind: &str, id: i32) -> bool {
    match kind {
        "E" => matches!(id, 1399 | 5000 | 6001 | 10003 | 10004 | 10005 | 10010 | 10020 | 10021 | 10030 | 16005 | 16006),
        "D" => matches!(id, 1399 | 5000 | 6001 | 10003 | 10004 | 10005 | 10020 | 10021 | 10030 | 16005 | 16006),
        "M" => matches!(id, 101 | 102 | 105 | 106 | 1365 | 9999 | 11016 | 11017 | 11018 | 11019 | 33923 | 69010 | 69011 | 69012 | 69104 | 101273 | 210102 | 604002),
        "O" => id == 20,
        _ => false,
    }
}

fn lookup(entries: &[(i32, &'static str)], id: i32) -> Option<&'static str> {
    entries
        .binary_search_by_key(&id, |entry| entry.0)
        .ok()
        .map(|index| entries[index].1)
}

pub fn skill_name(id: i32) -> Option<&'static str> { lookup(&catalog().skills, id) }
pub fn buff_name(id: i32) -> Option<&'static str> { lookup(&catalog().buffs, id) }
pub fn scene_name(id: i32) -> Option<&'static str> { lookup(&catalog().scenes, id) }
pub fn dungeon_name(id: i32) -> Option<&'static str> { lookup(&catalog().dungeons, id) }
pub fn monster_name(id: i32) -> Option<&'static str> { lookup(&catalog().monsters, id) }
pub fn talent_name(id: i32) -> Option<&'static str> { lookup(&catalog().talents, id) }
pub fn factor_name(id: i32) -> Option<&'static str> { lookup(&catalog().factors, id) }
pub fn factor_grade_item_name(id: i32) -> Option<&'static str> { lookup(&catalog().factor_grade_items, id) }
pub fn spec_name(id: i32) -> Option<&'static str> { lookup(&catalog().specs, id) }
pub fn class_name(id: i32) -> Option<&'static str> { lookup(&catalog().classes, id) }
pub fn modifier_effect_name(id: i32) -> Option<&'static str> { lookup(&catalog().modifier_effects, id) }
pub fn attribute_name(id: i32) -> Option<&'static str> { lookup(&catalog().attributes, id) }
pub fn objective_name(id: i32) -> Option<&'static str> { lookup(&catalog().objectives, id) }
pub fn recount_name(id: i32) -> Option<&'static str> { lookup(&catalog().recount_rows, id) }

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_strictly_sorted(entries: &[(i32, &'static str)]) {
        assert!(
            entries.windows(2).all(|pair| pair[0].0 < pair[1].0),
            "supplemental catalog contains an unsorted or duplicate id"
        );
    }

    #[test]
    fn supplemental_catalog_decodes_and_resolves_representative_ids() {
        assert_eq!(skill_name(1), Some("Red Light Counter"));
        assert_eq!(skill_name(2_203_110_103_u32 as i32), Some("Lucky Strike"));
        assert_eq!(skill_name(11_007_300_102_u64 as i32), Some("Stunt! Frenzied Shot"));
        assert_eq!(buff_name(201), Some("Intellect Conversion"));
        assert_eq!(scene_name(7), Some("Asteria Plains"));
        assert_eq!(dungeon_name(1101), Some("Towering Ruin"));
        assert_eq!(monster_name(103), Some("Ignisor"));
        assert_eq!(talent_name(1), Some("Strength"));
        assert_eq!(factor_name(202101), Some("Stormblade X1"));
        assert_eq!(factor_grade_item_name(20020001), Some("Stormblade X1 - G1"));
        assert_eq!(spec_name(130), Some("Iaido"));
        assert_eq!(class_name(5), Some("Verdant Oracle"));
        assert_eq!(modifier_effect_name(1), Some("Strength Boost"));
        assert_eq!(attribute_name(10030), Some("Ability Score"));
        assert_eq!(objective_name(1033), Some("Defeat the final boss"));
        assert_eq!(recount_name(1), Some("Red Light Counter"));
    }

    #[test]
    fn season4_cn_supplement_resolves_new_content_in_english() {
        assert_eq!(scene_name(6594), Some("Master - Judgment in the Mirror"));
        assert_eq!(scene_name(6615), Some("Master - Desolate Court"));
        assert_eq!(scene_name(1932), Some("Master - Divine Threshold of the Distant Sky"));
        assert_eq!(scene_name(13033), Some("Final Battle - Above the Sky, End of Day and Night"));
        assert_eq!(scene_name(60004), Some("Finale of the Raging Waves"));
        assert_eq!(scene_name(14001), Some("Wingwhale Survey Area I"));

        assert_eq!(monster_name(34000), Some("Anti-Fantasy: Boyce"));
        assert_eq!(monster_name(103600), Some("Lapsis - Phase 3"));
        assert_eq!(monster_name(6611017), Some("Vilda"));
        assert_eq!(monster_name(73071), Some("Valley Crossbowman"));
        assert_eq!(monster_name(884640), Some("Vilda Energy Orb"));

        assert_eq!(skill_name(3400018), Some("Nine-Ring Combo"));
        assert_eq!(skill_name(10350016), Some("Lapsis 02 Celestial Admonition"));
        assert_eq!(skill_name(470112), Some("Vilda - Near Circle"));
        assert_eq!(buff_name(884614), Some("Wheel of Fate"));
    }

    #[test]
    fn season4_supplement_is_english_and_free_of_development_placeholders() {
        for line in SEASON4_CN_SUPPLEMENT.lines() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut fields = line.splitn(3, '\t');
            let _kind = fields.next().expect("kind");
            let _id = fields.next().expect("id");
            let name = fields.next().expect("name");
            assert!(!name.contains("(AI)"), "AI suffix leaked into runtime name: {name}");
            assert!(!name.to_ascii_lowercase().contains("deprecated"), "deprecated placeholder leaked: {name}");
            assert!(
                !name.chars().any(|ch| ('\u{3400}'..='\u{9fff}').contains(&ch) || ('\u{f900}'..='\u{faff}').contains(&ch)),
                "untranslated CJK leaked into runtime name: {name}"
            );
        }
    }

    #[test]
    fn explicit_development_placeholders_are_excluded() {
        assert_eq!(scene_name(5000), None);
        assert_eq!(dungeon_name(10030), None);
        assert_eq!(monster_name(11016), None);
        assert_eq!(objective_name(20), None);
        assert_eq!(monster_name(114), Some("Enemy Training Dummy"));
        assert_eq!(monster_name(116), Some("Wooden Dummy"));
    }

    #[test]
    fn base_catalog_counts_remain_stable() {
        let catalog = load_catalog_from_sources(false);
        assert_eq!(CATALOG_PARTS.iter().map(|part| part.len()).sum::<usize>(), 78_485);
        assert_eq!(catalog.skills.len(), 8_457);
        assert_eq!(catalog.buffs.len(), 1_778);
        assert_eq!(catalog.scenes.len(), 586);
        assert_eq!(catalog.dungeons.len(), 572);
        assert_eq!(catalog.monsters.len(), 3_020);
        assert_eq!(catalog.talents.len(), 648);
        assert_eq!(catalog.factors.len(), 456);
        assert_eq!(catalog.factor_grade_items.len(), 2_280);
        assert_eq!(catalog.specs.len(), 18);
        assert_eq!(catalog.classes.len(), 9);
        assert_eq!(catalog.modifier_effects.len(), 147);
        assert_eq!(catalog.attributes.len(), 161);
        assert_eq!(catalog.objectives.len(), 755);
        assert_eq!(catalog.recount_rows.len(), 344);
    }

    #[test]
    fn merged_catalog_sort_order_is_stable() {
        let catalog = catalog();
        assert_strictly_sorted(&catalog.skills);
        assert_strictly_sorted(&catalog.buffs);
        assert_strictly_sorted(&catalog.scenes);
        assert_strictly_sorted(&catalog.dungeons);
        assert_strictly_sorted(&catalog.monsters);
        assert_strictly_sorted(&catalog.talents);
        assert_strictly_sorted(&catalog.factors);
        assert_strictly_sorted(&catalog.factor_grade_items);
        assert_strictly_sorted(&catalog.specs);
        assert_strictly_sorted(&catalog.classes);
        assert_strictly_sorted(&catalog.modifier_effects);
        assert_strictly_sorted(&catalog.attributes);
        assert_strictly_sorted(&catalog.objectives);
        assert_strictly_sorted(&catalog.recount_rows);
    }

    #[test]
    fn unknown_ids_stay_unknown() {
        assert_eq!(skill_name(i32::MAX), None);
        assert_eq!(buff_name(i32::MAX), None);
        assert_eq!(monster_name(i32::MAX), None);
        assert_eq!(objective_name(i32::MAX), None);
    }
}
