use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1353_audit_raid_hit_alignment.rs");
    pub fn run() { main(); }
}

fn main() {
    previous::run();
    let path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"))
        .join("feature_overlays_v170_fixed.rs");
    let mut src = fs::read_to_string(&path).expect("generated overlay source");
    let old = r#"    let base = if bold { 14 } else { 12 };
    let height = dps_adaptive_logical_px(base, scale);
    let face = wide("Segoe UI Variable Text");"#;
    let new = r#"    let height = dps_readable_font_height(scale, bold);
    let face = wide("Segoe UI Variable Text");"#;
    assert_eq!(src.matches(old).count(), 1, "DPS font sizing anchor changed");
    src = src.replacen(old, new, 1);
    let anchor = "unsafe fn dps_cached_font(scale: i32, bold: bool) -> HFONT {";
    assert_eq!(src.matches(anchor).count(), 1, "DPS font function anchor changed");
    src = src.replacen(anchor, r#"// Keep 13px primary and 12px secondary em floors at 96 DPI while the
// overlay is reduced. This is logical GDI geometry before the app transform;
// monitor DPI is handled by the existing Windows DPI path. Larger scales stay.
fn dps_readable_font_height(scale: i32, bold: bool) -> i32 {
    let app_scale = clamp_kind_scale(Kind::Dps, scale).max(1);
    let base = if bold { 14 } else { 12 };
    let physical_floor = if bold { 13 } else { 12 };
    let logical_floor = (physical_floor * 100 + app_scale - 1) / app_scale;
    dps_adaptive_logical_px(base, scale).max(logical_floor)
}

unsafe fn dps_cached_font(scale: i32, bold: bool) -> HFONT {"#, 1);
    src.push_str(r#"
#[cfg(test)]
mod audit_readable_font_floor_tests {
    use super::*;
    #[test]
    fn primary_and_secondary_ems_meet_96_dpi_audit_floors() {
        for scale in [50, 60, 65, 75, 80, 100, 125, 150, 175, 200] {
            for (bold, floor) in [(true, 13), (false, 12)] {
                let height = dps_readable_font_height(scale, bold);
                let app = clamp_kind_scale(Kind::Dps, scale).max(1);
                assert!(height * app >= floor * 100, "{scale}% bold={bold}");
                assert!(height >= dps_adaptive_logical_px(if bold { 14 } else { 12 }, scale));
            }
        }
    }
}
"#);
    fs::write(path, src).expect("write DPS font floor audit");
    println!("cargo:rerun-if-changed=build/legacy/build_v1354_audit_font_floors.rs");
}
