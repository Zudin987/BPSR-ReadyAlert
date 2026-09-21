// September 2026 audit: strictly checked native generated-source transformations.
// The retired S03/M07 literal anchors were superseded by the measured planner
// in v1377, whose exact final output is asserted again by v1380. Do not silently
// skip any remaining required transformation: a changed upstream builder must
// fail CI until its native patch is reviewed and updated.
use std::{env, fs, path::PathBuf};
mod previous {
    include!("build_v1374_benchmark_arm_boundary.rs");
    pub fn run() { main(); }
}
fn once(source: &mut String, old: &str, new: &str, label: &str) {
    let count = source.matches(old).count();
    assert_eq!(count, 1, "September UI audit {label}: expected exactly one native source anchor; found {count}");
    *source = source.replacen(old, new, 1);
    assert!(source.contains(new), "September UI audit {label}: missing compiled replacement");
    println!("cargo:warning=September UI audit {label}: verified one generated-source edit");
}
fn within(source: &mut String, start: &str, end: &str, old: &str, new: &str, label: &str) {
    let starts = source.matches(start).count();
    assert_eq!(starts, 1, "September UI audit {label}: expected exactly one native section; found {starts}");
    let begin = source.find(start).expect("verified native section start");
    let finish = begin + source[begin..].find(end).unwrap_or_else(|| panic!("September UI audit {label}: section end absent"));
    let mut piece = source[begin..finish].to_owned();
    once(&mut piece, old, new, label);
    source.replace_range(begin..finish, &piece);
}
fn main() {
    previous::run();
    let path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("feature_overlays_v170_fixed.rs");
    let mut src = fs::read_to_string(&path).expect("generated native overlay").replace("\r\n", "\n");

    // S01/I03: toolbar font and actual 24-unit pointer targets.
    within(&mut src, "fn reference_button_w(", "fn reference_button_gap(",
        "if width < 340 { 20 } else if width < 440 { 24 } else { 28 }",
        "if width < 440 { 24 } else { 28 }", "minimum toolbar target");
    within(&mut src, "unsafe fn paint_toolbar(", "unsafe fn reference_set_layout(",
        "let old = SelectObject(hdc, dps_cached_font(100, true));",
        "let old = SelectObject(hdc, dps_primary_font(state));", "unified primary font role");

    // S02: preserve complete visible rows when UI-size alone changes.
    within(&mut src, "unsafe fn set_overlay_scale(", "unsafe fn change_overlay_scale(",
        "rescale_px((source.bottom-source.top).max(1),old,new).max(overlay_min_height_mode(overlay.kind,new,overlay.compact_mode))",
        "audit_height_preserving_rows(overlay,source,old,new).max(overlay_min_height_mode(overlay.kind,new,overlay.compact_mode))",
        "preserve complete-row viewport");

    // S03/M07 obsolete thresholds intentionally removed. The final name-first
    // layout and matching header must be present (v1377/v1380 hard assertions).

    // C02/C03: bright secondary information and restrained metric bars.
    within(&mut src, "unsafe fn paint_reference_headers(", "unsafe fn paint_dps(",
        "SetTextColor(hdc, rgb(166, 177, 195));",
        "SetTextColor(hdc, rgb(190, 200, 214));", "column contrast");
    within(&mut src, "unsafe fn paint_dps_secondary_colored(", "fn reference_raid_columns(",
        "SetTextColor(hdc, rgb(166, 177, 195));",
        "SetTextColor(hdc, rgb(190, 200, 214));", "class secondary contrast");
    within(&mut src, "unsafe fn paint_reference_normal_rows(", "unsafe fn paint_compact_player(",
        "let bar_top = (bar_bottom - 3).max(r.top + 3);",
        "let bar_top = (bar_bottom - 2).max(r.top + 2);", "two-unit progress bar");

    // C04/M02: paint-only owner marker, stable self rank and divider.
    for (start, end, label) in [
        ("unsafe fn paint_reference_normal_rows(", "unsafe fn paint_compact_player(", "normal self marker"),
        ("unsafe fn paint_compact_player(", "unsafe fn paint_compact_raid_rows(", "compact self marker"),
        ("unsafe fn paint_raid_player(", "\n}\n", "raid self marker"),
    ] {
        within(&mut src, start, end,
            "&reference_player_display_name(row),",
            "&if row.is_local { format!(\"YOU · {}\", reference_player_display_name(row)) } else { reference_player_display_name(row) },",
            label);
    }
    within(&mut src, "unsafe fn paint_reference_normal_rows(", "unsafe fn paint_compact_player(",
        "        let base = text_on(bg);",
        "        let base = text_on(bg);\n        if row.is_local && screen_i > 0 && rank > state.scroll.saturating_add(screen_i + 1) {\n            fill(hdc, &RECT { left: r.left + 3, top: y - 2, right: r.right - 3, bottom: y - 1 }, rgb(82, 97, 106));\n        }",
        "separate pinned self from ranking");

    // M01: visible HISTORY prefix while retaining encounter title.
    within(&mut src, "unsafe fn paint_reference_encounter(", "unsafe fn paint_mode_tab(",
        "    SetTextColor(hdc, rgb(241, 243, 247));\n    draw(hdc, &title, RECT",
        "    let title = if state.history_index.is_some() { format!(\"HISTORY · {title}\") } else { title };\n    SetTextColor(hdc, rgb(241, 243, 247));\n    draw(hdc, &title, RECT",
        "historical title");

    // M03: rank is sorted by total contribution, not active rate.
    within(&mut src, "unsafe fn paint_reference_raid_headers(", "#[cfg(test)]\nmod reference_meter_audit_tests",
        "(\"#\", col.left + 4, col.left + 24, 0)",
        "(\"T#\", col.left + 4, col.left + 24, 0)", "raid total-ranking heading");
    within(&mut src, "unsafe fn paint_reference_headers(", "unsafe fn paint_dps(",
        "(\"#\", r.left + 3, r.left + rank_w, DT_RIGHT)",
        "(\"T#\", r.left + 3, r.left + rank_w, DT_RIGHT)", "compact total-ranking heading");
    within(&mut src, "unsafe fn overlay_help(", "fn reference_meter_rate_label(",
        "Some(format!(\"{} • click to inspect\", dps_identity(&row)))",
        "Some(format!(\"{} · Ranked by {} · click to inspect\", dps_identity(&row), reference_meter_total_label(state.sort_mode)))",
        "rank basis help");

    // I04/I06: layout menu and reversible hide/collapse semantics.
    within(&mut src, "fn toolbar_action_help(", "unsafe fn reference_line(",
        "ToolbarAction::More => \"Meter layout and actions\",",
        "ToolbarAction::More => \"Choose layout; history and settings are below the divider\",",
        "dropdown explanation");
    within(&mut src, "unsafe fn overlay_help(", "fn reference_meter_rate_label(",
        "[\"Open overlay settings\", \"Collapse overlay\", \"Close overlay\"]",
        "[\"Open overlay settings\", \"Collapse to restore tab (click tab to expand)\", \"Hide meter (restore from tray)\"]",
        "collapse recovery explanation");
    once(&mut src,
        "const REFERENCE_METER_BG: u32 = 0x001B1715; // Win32 COLORREF: #15171B",
        "const REFERENCE_METER_BG: u32 = 0x00221915; // Win32 COLORREF: #151922",
        "neutral overlay surface");

    src.push_str(r#"
fn audit_height_preserving_rows(state: &State, source: RECT, old: i32, new: i32) -> i32 {
    let previous = (source.bottom - source.top).max(1);
    if state.kind != Kind::Dps { return rescale_px(previous, old, new); }
    let logical = physical_extent_to_logical(previous, old);
    let old_pitch = dps_row_h_for(state, old).max(1);
    let rows = ((logical - dps_rows_top_for(state)).max(0) / old_pitch).clamp(2, 40);
    let pitch = dps_row_h_for(state, new).max(1);
    scale_px(dps_rows_top_for(state).saturating_add(rows.saturating_mul(pitch)).saturating_add(8), new)
}
#[cfg(test)]
mod september_ui_audit_regressions {
    use super::*;
    #[test]
    fn pointer_cells_never_fall_below_24_logical_units() {
        for width in [300,340,420,620,900] {
            assert!(reference_button_w(width) >= 24);
            for index in 0..3 {
                let r = reference_system_rect(width,index);
                assert!(r.right-r.left >= 24 && r.bottom-r.top >= 24);
            }
        }
    }
    #[test]
    fn a_five_row_layout_survives_scaled_coordinate_rounding() {
        for scale in [60,70,80,90,100] {
            let needed = dps_rows_top() + 5*dps_row_h(scale) + 8;
            assert!(physical_extent_to_logical(scale_px(needed,scale),scale) + 1 >= needed);
        }
    }
}
"#);
    fs::write(path,src).expect("write September audit generated overlay");
    println!("cargo:rerun-if-changed=build/legacy/build_v1375_ui_ux_audit.rs");
}
