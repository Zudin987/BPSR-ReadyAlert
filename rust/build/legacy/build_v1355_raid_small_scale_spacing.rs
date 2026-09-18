use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1354_audit_font_floors.rs");
    pub fn run() { main(); }
}

fn replace_once(src: &mut String, old: &str, new: &str, label: &str) {
    let count = src.matches(old).count();
    assert_eq!(count, 1, "small-scale raid {label}: expected one generated instance, got {count}");
    *src = src.replacen(old, new, 1);
}

fn main() {
    previous::run();
    let path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"))
        .join("feature_overlays_v170_fixed.rs");
    let mut src = fs::read_to_string(&path).expect("generated overlay source");

    // Increase row pitch together with the already established small-scale
    // font floor. Paint, row hit-testing, scrolling and auto-sizing share
    // these helpers, so all four remain in sync. Preserve normal-size pitch.
    replace_once(&mut src,
        "fn dps_row_h(scale:i32)->i32{dps_adaptive_logical_px(DPS_ROW_H,scale)}",
        "fn dps_row_h(scale:i32)->i32{dps_adaptive_logical_px(if scale <= 80 { DPS_ROW_H.max(44) } else { DPS_ROW_H },scale)}",
        "normal/Raid pitch");
    replace_once(&mut src,
        "fn dps_compact_row_h(scale: i32) -> i32 {\n    dps_adaptive_logical_px(26, scale)\n}",
        "fn dps_compact_row_h(scale: i32) -> i32 {\n    dps_adaptive_logical_px(if scale <= 80 { 30 } else { 26 }, scale)\n}",
        "Compact/Compact Raid pitch");

    // The second line of each Raid player card is shared with the 40-unit
    // Food/Serum pair. Previously class/status text extended underneath it.
    // Clip the detail to a separate region rather than obscuring either item.
    replace_once(&mut src,
        "    let detail = RECT {\n        top: middle,\n        bottom: r.bottom - 4,\n        ..identity\n    };",
        "    let detail = RECT {\n        top: middle,\n        bottom: r.bottom - 4,\n        right: if settings.meter.show_consumables { identity.right.min(r.right - 48) } else { identity.right },\n        ..identity\n    };",
        "Raid secondary text and consumable separation");

    // Two taller rows now require a 135px minimum at the effective 50% scale
    // (including legacy requests for 30%). The 100% and 300% minimums stay
    // unchanged. Keep the original tests and update only their stale values.
    replace_once(&mut src,
        "assert_eq!(overlay_min_height(Kind::Dps,30),129)",
        "assert_eq!(overlay_min_height(Kind::Dps,30),135)",
        "legacy minimum height scalar test");
    replace_once(&mut src,
        "assert_eq!((overlay_min_width(Kind::Dps,50),overlay_min_height(Kind::Dps,50)),(150,129))",
        "assert_eq!((overlay_min_width(Kind::Dps,50),overlay_min_height(Kind::Dps,50)),(150,135))",
        "effective 50% minimum size test");
    let old_legacy = "assert_eq!((overlay_min_width(Kind::Dps,30),overlay_min_height(Kind::Dps,30)),(150,129))";
    let new_legacy = "assert_eq!((overlay_min_width(Kind::Dps,30),overlay_min_height(Kind::Dps,30)),(150,135))";
    assert_eq!(src.matches(old_legacy).count(), 2,
        "small-scale raid legacy 30% minimum: expected two generated tests");
    src = src.replace(old_legacy, new_legacy);

    src.push_str(r#"
#[cfg(test)]
mod audit_small_raid_spacing_tests {
    use super::*;
    #[test]
    fn reduced_scale_rows_have_two_readable_lines_and_match_shared_geometry() {
        for scale in [60, 80] {
            assert!(dps_row_h(scale) >= dps_adaptive_logical_px(44, scale));
            assert!(dps_compact_row_h(scale) >= dps_adaptive_logical_px(30, scale));
            assert!(overlay_min_height(Kind::Dps, scale)
                >= scale_px(dps_rows_top() + 2 * dps_row_h(scale) + 4, scale));
        }
        assert_eq!(overlay_min_height(Kind::Dps, 50), 135);
        assert_eq!(overlay_min_height(Kind::Dps, 30), 135);
        assert_eq!(dps_row_h(100), dps_adaptive_logical_px(DPS_ROW_H, 100));
        assert_eq!(dps_compact_row_h(100), dps_adaptive_logical_px(26, 100));
    }
}
"#);
    fs::write(path, src).expect("write reduced-scale Raid spacing");
    println!("cargo:rerun-if-changed=build/legacy/build_v1355_raid_small_scale_spacing.rs");
}
