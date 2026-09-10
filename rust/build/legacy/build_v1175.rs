use std::{env, fs, path::{Path, PathBuf}};

mod prior {
    include!("build_v1174.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.17.5 patch {label} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn patch_telemetry(out: &Path) {
    let path = out.join("telemetry_v170_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .expect("read v1.17.4 telemetry")
        .replace("\r\n", "\n");

    // v1.17.1 carried a hard-coded regression expectation for this Food id.
    // The corrected user catalog now identifies 2032065 as S1 food level 2.
    replace_once(
        &mut source,
        r#"assert_eq!(consumable_info(2_032_065),Some((ConsumableKind::Food,"S1 lvl 1 (ATK +75, +5%)")));"#,
        r#"assert_eq!(consumable_info(2_032_065),Some((ConsumableKind::Food,"S1 lvl 2 (ATK +75, +5%)")));"#,
        "corrected Food level regression fixture",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1175_consumable_name_tests {
    use super::*;

    #[test]
    fn corrected_food_levels_follow_uploaded_catalog() {
        assert_eq!(consumable_info(2_032_012), Some((ConsumableKind::Food, "S1 lvl 1 (ATK +30)")));
        assert_eq!(consumable_info(2_032_015), Some((ConsumableKind::Food, "S1 lvl 2 (ATK +75)")));
        assert_eq!(consumable_info(2_032_017), Some((ConsumableKind::Food, "S1 lvl 3 (ATK +120)")));
        assert_eq!(consumable_info(2_032_065), Some((ConsumableKind::Food, "S1 lvl 2 (ATK +75, +5%)")));
        assert_eq!(consumable_info(2_032_067), Some((ConsumableKind::Food, "S1 lvl 3 (ATK +120, +10%)")));
        assert_eq!(consumable_info(2_032_112), Some((ConsumableKind::Food, "S2 lvl 1 (ATK +115)")));
        assert_eq!(consumable_info(2_032_114), Some((ConsumableKind::Food, "S2 lvl 2 (ATK +165)")));
        assert_eq!(consumable_info(2_032_212), Some((ConsumableKind::Food, "S3 lvl 1 (ATK +180)")));
        assert_eq!(consumable_info(2_032_214), Some((ConsumableKind::Food, "S3 lvl 2 (ATK +240)")));
    }

    #[test]
    fn serum_names_still_follow_uploaded_catalog_exactly() {
        assert_eq!(consumable_info(2_033_011), Some((ConsumableKind::Serum, "S1 lvl 1 (Fire +240)")));
        assert_eq!(consumable_info(2_033_014), Some((ConsumableKind::Serum, "S2 lvl 1 (Fire +550)")));
        assert_eq!(consumable_info(2_033_017), Some((ConsumableKind::Serum, "S3 lvl 1 (Fire +950)")));
        assert_eq!(consumable_info(2_033_293), Some((ConsumableKind::Serum, "S4 lvl 1 (Wind Res)")));
        assert_eq!(consumable_info(2_033_301), Some((ConsumableKind::Serum, "S4 lvl 3 (Wind)")));
        assert_eq!(consumable_info(2_033_383), Some((ConsumableKind::Serum, "S4 lvl 3 (MAG Boost)")));
    }
}
"#);

    fs::write(path, source).expect("write v1.17.5 telemetry");
}

fn main() {
    prior::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_telemetry(&out);
    println!("cargo:rerun-if-changed=build_v1175.rs");
    println!("cargo:rerun-if-changed=data/consumables_v1166.csv");
}
