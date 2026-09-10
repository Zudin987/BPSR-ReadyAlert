use std::{env, fs, path::{Path, PathBuf}};

mod prior {
    include!("build_v1170_winfix.rs");
    pub fn run() { main(); }
}

const TARGET_MAP_SHARDS: [&str; 5] = [
    "data/target_entities_v1171_01.csv",
    "data/target_entities_v1171_02.csv",
    "data/target_entities_v1171_03.csv",
    "data/target_entities_v1171_04.csv",
    "data/target_entities_v1171_05.csv",
];

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.17.1 patch {label} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn rust_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn exact_monster_catalog() -> String {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let mut arms = String::new();
    let mut seen = std::collections::HashSet::new();
    let mut count = 0usize;

    for relative in TARGET_MAP_SHARDS {
        let path = manifest.join(relative);
        let csv = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read {relative}: {err}"));
        for (line_no, raw) in csv.lines().enumerate().skip(1) {
            let raw = raw.trim_end_matches('\r');
            if raw.trim().is_empty() { continue; }
            let (raw_name, raw_id) = raw
                .rsplit_once(',')
                .unwrap_or_else(|| panic!("invalid target entity CSV {relative}:{}", line_no + 1));
            let id: i32 = raw_id
                .trim()
                .parse()
                .unwrap_or_else(|_| panic!("invalid monster/entity id in {relative}:{}", line_no + 1));
            assert!(seen.insert(id), "duplicate monster/entity id {id}");
            let mut name = raw_name.trim().to_string();
            if name.starts_with('"') && name.ends_with('"') && name.len() >= 2 {
                name = name[1..name.len() - 1].replace("\"\"", "\"");
            }
            assert!(!name.trim().is_empty(), "empty monster/entity name in {relative}:{}", line_no + 1);
            arms.push_str(&format!("        {id} => \"{}\",\n", rust_string(&name)));
            count += 1;
        }
    }

    assert_eq!(count, 3_479, "uploaded monster/boss/targetable entity count changed");
    assert_eq!(seen.len(), 3_479, "uploaded monster/boss/targetable entity id count changed");

    format!(
        "const EXACT_MONSTER_NAME_COUNT: usize = 3_479;\n\
fn exact_monster_name(id: i32) -> Option<&'static str> {{\n\
    Some(match id {{\n\
{arms}\
        _ => return None,\n\
    }})\n\
}}\n\n"
    )
}

fn patch_telemetry(out: &Path) {
    let path = out.join("telemetry_v170_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .expect("read generated v1.17 telemetry for exact data maps")
        .replace("\r\n", "\n");

    // ATTR_MONSTER_ID (0x0a) is already the authoritative protocol field. Map
    // that exact id to the uploaded canonical name before considering wire text.
    // If neither exists, keep the UI explicit instead of fabricating Target <id>.
    replace_once(
        &mut source,
        r#"            name: if meta.name.trim().is_empty() {
                if meta.monster_id > 0 { format!("Target {}", meta.monster_id) } else { "Target".into() }
            } else {
                meta.name.clone()
            },"#,
        r#"            name: exact_monster_name(meta.monster_id)
                .map(str::to_string)
                .or_else(|| (!meta.name.trim().is_empty()).then(|| meta.name.clone()))
                .unwrap_or_else(|| "Unknown Target".into()),"#,
        "resolve target names from exact monster id catalog",
    );

    let catalog = exact_monster_catalog();
    replace_once(
        &mut source,
        "fn monster_rule(id: i32) -> Option<(&'static str, u64, u8)> {",
        &format!("{catalog}fn monster_rule(id: i32) -> Option<(&'static str, u64, u8)> {{"),
        "insert exact monster id catalog",
    );

    // v1.16.6 regression tests intentionally check exact labels. Update those
    // expectations to the new authoritative uploaded label set.
    replace_once(
        &mut source,
        r#"assert_eq!(consumable_info(2_032_065),Some((ConsumableKind::Food,"S1 ATK +75, +5%")));"#,
        r#"assert_eq!(consumable_info(2_032_065),Some((ConsumableKind::Food,"S1 lvl 1 (ATK +75, +5%)")));"#,
        "Food exact-label regression fixture",
    );
    replace_once(
        &mut source,
        r#"assert_eq!(consumable_info(2_033_011),Some((ConsumableKind::Serum,"S1 Fire +240")));"#,
        r#"assert_eq!(consumable_info(2_033_011),Some((ConsumableKind::Serum,"S1 lvl 1 (Fire +240)")));"#,
        "Serum exact-label regression fixture",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1171_exact_mapping_tests {
    use super::*;

    #[test]
    fn uploaded_monster_catalog_has_all_exact_ids() {
        assert_eq!(EXACT_MONSTER_NAME_COUNT, 3_479);
        assert_eq!(exact_monster_name(101), Some("Dummy (Flame Orc)"));
        assert_eq!(exact_monster_name(3_000_038), Some("Celestial Flier - Resonance"));
        assert_eq!(exact_monster_name(7_700_001), Some("Goblin King"));
        assert_eq!(exact_monster_name(i32::MAX), None);
    }

    #[test]
    fn uploaded_consumable_catalog_has_exact_food_and_serum_labels() {
        assert_eq!(
            consumable_info(2_032_011),
            Some((ConsumableKind::Food, "S1 lvl 1 (ATK +15)"))
        );
        assert_eq!(
            consumable_info(2_033_383),
            Some((ConsumableKind::Serum, "S4 lvl 3 (MAG Boost)"))
        );
        assert!(consumable_info(2_010_003).is_none());
    }
}
"#);

    fs::write(path, source).expect("write v1.17.1 exact data mapping telemetry");
}

fn main() {
    prior::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_telemetry(&out);

    println!("cargo:rerun-if-changed=build_v1171.rs");
    for relative in TARGET_MAP_SHARDS {
        println!("cargo:rerun-if-changed={relative}");
    }
    println!("cargo:rerun-if-changed=data/consumables_v1166.csv");
}
