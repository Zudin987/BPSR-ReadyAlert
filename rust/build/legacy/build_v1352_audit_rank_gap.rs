use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1351_audit_identity_settings.rs");
    pub fn run() { main(); }
}

fn once(src: &mut String, old: &str, new: &str, what: &str) {
    let n = src.matches(old).count();
    assert_eq!(n, 1, "audit rank gap {what}: expected one anchor, found {n}");
    *src = src.replacen(old, new, 1);
}
fn section(src: &mut String, start: &str, end: &str, old: &str, new: &str, what: &str) {
    let a = src.find(start).expect("rank-gap section start");
    let b = a + src[a..].find(end).expect("rank-gap section end");
    let mut body = src[a..b].to_owned();
    once(&mut body, old, new, what);
    src.replace_range(a..b, &body);
}

fn main() {
    previous::run();
    let path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"))
        .join("feature_overlays_v170_fixed.rs");
    let mut src = fs::read_to_string(&path).expect("generated overlay source");

    // R05: use the same adaptive rank cell + four-unit gap that the player
    // layout itself reserves. This prevents two-digit ranks touching names at
    // reduced scale and keeps heading/row anchors consistent.
    section(
        &mut src,
        "unsafe fn dps_row_layout_responsive(",
        "unsafe fn paint_dps_secondary_colored(",
        "    let name_left = r.left + 25;",
        "    let name_left = r.left + dps_adaptive_logical_px(24, scale) + dps_adaptive_logical_px(4, scale);",
        "responsive name anchor",
    );

    section(
        &mut src,
        "unsafe fn paint_reference_normal_rows(",
        "unsafe fn paint_compact_player(",
        "                right: r.left + 25,",
        "                right: r.left + dps_adaptive_logical_px(24, scale),",
        "normal rank cell",
    );

    section(
        &mut src,
        "unsafe fn paint_compact_player(",
        "unsafe fn paint_compact_raid_rows(",
        "    let status_left = (identity_right - status_w).max(r.left + rank_w + 36);",
        "    let status_left = (identity_right - status_w).max(r.left + rank_w + gap + 36);",
        "compact status floor",
    );
    section(
        &mut src,
        "unsafe fn paint_compact_player(",
        "unsafe fn paint_compact_raid_rows(",
        "            left: r.left + rank_w,",
        "            left: r.left + rank_w + gap,",
        "compact name gap",
    );

    section(
        &mut src,
        "unsafe fn paint_raid_player(",
        "mod white_header_icons",
        "            right: r.left + 24,",
        "            right: r.left + dps_adaptive_logical_px(24, scale),",
        "raid rank cell",
    );

    // Compact headings share the same adaptive rank geometry as their rows.
    section(
        &mut src,
        "unsafe fn paint_reference_headers(",
        "unsafe fn paint_dps(",
        "        for r in columns {\n            let rate_right = r.right - 4;",
        "        for r in columns {\n            let rank_w = dps_adaptive_logical_px(24, scale);\n            let rank_gap = dps_adaptive_logical_px(4, scale);\n            let rate_right = r.right - 4;",
        "compact header rank metrics",
    );
    section(
        &mut src,
        "unsafe fn paint_reference_headers(",
        "unsafe fn paint_dps(",
        "                (\"#\", r.left + 3, r.left + 24, 0),\n                (\"Player\", r.left + 24, name_right, 0),",
        "                (\"#\", r.left + 3, r.left + rank_w, DT_RIGHT),\n                (\"Player\", r.left + rank_w + rank_gap, name_right, 0),",
        "compact header rank/name anchors",
    );

    src.push_str(r#"
#[cfg(test)]
mod audit_rank_gap_tests {
    use super::*;
    #[test]
    fn reduced_scale_rank_cell_keeps_four_unit_gap() {
        for scale in [60, 80, 100, 150, 200] {
            let rank_w = dps_adaptive_logical_px(24, scale);
            let gap = dps_adaptive_logical_px(4, scale);
            assert!(rank_w >= 24);
            assert!(gap >= 4);
            assert!(rank_w + gap > rank_w);
        }
    }
}
"#);

    fs::write(path, src).expect("write audit rank gap refinements");
    println!("cargo:rerun-if-changed=build/legacy/build_v1352_audit_rank_gap.rs");
}
