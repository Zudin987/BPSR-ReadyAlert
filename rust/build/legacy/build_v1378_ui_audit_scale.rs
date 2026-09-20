// The 60%-UI audit found that the toolbar shrank while rows and badges
// effectively stayed close to their 100% size. Keep minimum legible glyphs,
// but stop applying a second, unrelated 80%-at-60% geometry multiplier.
use std::{env, fs, path::PathBuf};
mod previous {
    include!("build_v1377_ui_audit_verified.rs");
    pub fn run() { main(); }
}
fn checked(src: &mut String, before: &str, after: &str, label: &str) {
    assert_eq!(src.matches(before).count(), 1, "S01 {label}: expected one generated source location");
    *src = src.replacen(before, after, 1);
    println!("cargo:warning=UI audit S01: verified {label}");
}
fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut src = fs::read_to_string(&path).expect("meter generated source");
    // A single geometric scale: physical row/icon sizes now follow the selected
    // percent just like the toolbar. Font creation retains separate 11/12-pixel
    // legibility floors; row heights account for both lines where applicable.
    checked(&mut src,
        "fn dps_readability_percent(scale:i32)->i32{let s=clamp_kind_scale(Kind::Dps,scale);if s<100{(s+100)/2}else{s}}",
        "fn dps_readability_percent(scale:i32)->i32{clamp_kind_scale(Kind::Dps,scale)}",
        "one geometry factor");
    checked(&mut src,
        "fn dps_row_h(scale:i32)->i32{dps_adaptive_logical_px(if scale <= 80 { DPS_ROW_H.max(44) } else { DPS_ROW_H },scale)}",
        "fn dps_row_h(scale:i32)->i32{dps_adaptive_logical_px(DPS_ROW_H,scale).max(dps_readable_font_height(scale,true)+dps_readable_font_height(scale,false)+8)}",
        "two-line row legibility");
    checked(&mut src,
        "dps_adaptive_logical_px(if scale <= 80 { 30 } else { 26 }, scale)\n}",
        "dps_adaptive_logical_px(if scale <= 80 { 30 } else { 26 }, scale).max(dps_readable_font_height(scale,true)+6)\n}",
        "compact row minimum font clearance");
    // Update regression expectations to the new, intentional scaling policy.
    checked(&mut src,
        "fn downscale_readability_is_softened_but_upscale_stays_linear(){\n        assert_eq!(dps_readability_percent(50),75);\n        assert_eq!(dps_readability_percent(60),80);\n        assert_eq!(dps_readability_percent(70),85);\n        assert_eq!(dps_readability_percent(80),90);\n        assert_eq!(dps_readability_percent(90),95);",
        "fn geometry_scales_with_the_shell_but_font_floors_remain(){\n        assert_eq!(dps_readability_percent(50),50);\n        assert_eq!(dps_readability_percent(60),60);\n        assert_eq!(dps_readability_percent(70),70);\n        assert_eq!(dps_readability_percent(80),80);\n        assert_eq!(dps_readability_percent(90),90);",
        "scale matrix expectation");
    checked(&mut src,
        "fn low_scale_effective_width_accounts_for_readability_floor(){\n        assert!(dps_effective_width(500,50)<500);\n        assert!(dps_row_h(50)>DPS_ROW_H);\n        assert!(dps_badge_w(50)>BADGE_W);",
        "fn low_scale_geometry_is_coordinated_with_readable_type(){\n        assert_eq!(dps_effective_width(500,50),500);\n        assert!(dps_row_h(50)>DPS_ROW_H);\n        assert_eq!(dps_badge_w(50),BADGE_W);",
        "row and icon scale regression");
    checked(&mut src,
        "assert_eq!(dps_readability_percent(30),75);",
        "assert_eq!(dps_readability_percent(30),50);",
        "persisted minimum scale regression");
    src.push_str(r#"
#[cfg(test)]
mod september_coordinated_scale_tests {
    use super::*;
    #[test]
    fn minimized_content_and_toolbar_use_a_coherent_physical_pitch() {
        for scale in [50, 60, 70, 80, 90, 100] {
            let row = scale_px(dps_row_h(scale), scale);
            let toolbar = scale_px(TOOLBAR_H, scale);
            assert!(row >= scale_px(dps_readable_font_height(scale, true), scale)
                + scale_px(dps_readable_font_height(scale, false), scale),
                "two text lines must fit at {scale}%");
            assert!(row <= toolbar + 10,
                "meter rows must not stay near full size under a tiny toolbar at {scale}%");
            assert!(scale_px(dps_badge_w(scale), scale) <= scale_px(BADGE_W, 100),
                "optional image icons must not grow as the UI shrinks");
        }
    }
}
"#);
    fs::write(path, src).expect("write coordinated-scale source");
    println!("cargo:rerun-if-changed=build/legacy/build_v1378_ui_audit_scale.rs");
}
