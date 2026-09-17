use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1347_video_spacing_badge_fix.rs");
    pub fn run() { main(); }
}

fn main() {
    previous::run();
    let path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"))
        .join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("generated meter source");
    let from = "    let status_w = if row.is_dead && dps_effective_width(r.right - r.left, scale) >= 280 {\n        dps_adaptive_logical_px(94, scale)\n    } else {\n        0\n    };";
    let to = "    let status_w = reference_compact_status_width(r.right - r.left, scale, total_w > 0, row.is_dead);";
    assert_eq!(source.matches(from).count(), 1, "compact status sizing anchor");
    source = source.replacen(from, to, 1);
    let from = "        \"Imagines\",\n            RECT {";
    let to = "        if badge_header_span >= dps_text_width(hdc, \"Imagines\", 65) + 2 { \"Imagines\" } else { \"Img\" },\n            RECT {";
    assert_eq!(source.matches(from).count(), 1, "responsive Imagine heading anchor");
    source = source.replacen(from, to, 1);
    source.push_str(r#"
fn reference_compact_status_width(width: i32, scale: i32, show_total: bool, dead: bool) -> i32 {
    if !dead { return 0; }
    let gap = dps_adaptive_logical_px(3, scale);
    let status = dps_adaptive_logical_px(94, scale);
    let reserve = 4 + dps_adaptive_logical_px(24, scale)
        + dps_adaptive_logical_px(64, scale) + gap
        + if show_total { dps_adaptive_logical_px(70, scale) + gap } else { 0 }
        + status + gap + dps_adaptive_logical_px(88, scale);
    if width >= reserve { status } else { 0 }
}
#[cfg(test)]
mod video_status_regression_tests {
    use super::*;
    #[test]
    fn compact_name_does_not_collapse_under_dead_status() {
        assert_eq!(reference_compact_status_width(286, 100, true, true), 0);
        assert_eq!(reference_compact_status_width(406, 100, true, true), 94);
    }
}
"#);
    fs::write(path, source).expect("write compact status fit");
    println!("cargo:rerun-if-changed=build/legacy/build_v1348_video_spacing_status.rs");
}
