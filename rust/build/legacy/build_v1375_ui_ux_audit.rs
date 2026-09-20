// September 2026 UI/UX implementation stage. Earlier builders run first;
// modify the checked-in generator, never a transient OUT_DIR file by hand.
use std::{env, fs, path::PathBuf};
mod previous {
    include!("build_v1374_benchmark_arm_boundary.rs");
    pub fn run() { main(); }
}
fn once(source: &mut String, old: &str, new: &str, label: &str) {
    assert_eq!(source.matches(old).count(), 1, "UI audit {label}: generated anchor changed");
    *source = source.replacen(old, new, 1);
}
fn section(source: &mut String, start: &str, end: &str, old: &str, new: &str, label: &str) {
    assert_eq!(source.matches(start).count(), 1, "UI audit {label}: section missing or ambiguous");
    let begin = source.find(start).unwrap();
    let finish = begin + source[begin..].find(end).expect("UI audit section end");
    let mut fragment = source[begin..finish].to_owned();
    once(&mut fragment, old, new, label);
    source.replace_range(begin..finish, &fragment);
}
fn main() {
    previous::run();
    let path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"))
        .join("feature_overlays_v170_fixed.rs");
    let mut src = fs::read_to_string(&path).expect("generated native overlay").replace("\r\n", "\n");

    // S01/I03: header and primary rows use the same font role and minimum
    // 24-logical-unit controls. Copy and Reset remain icon-only.
    section(&mut src, "fn reference_button_w(", "fn reference_button_gap(",
        "if width < 340 { 20 } else if width < 440 { 24 } else { 28 }",
        "if width < 440 { 24 } else { 28 }", "minimum toolbar targets");
    section(&mut src, "unsafe fn paint_toolbar(", "unsafe fn reference_set_layout(",
        "let old = SelectObject(hdc, dps_cached_font(100, true));",
        "let old = SelectObject(hdc, dps_primary_font(state));", "unified header type");

    // S02: preserve manually established complete-row capacity during a size
    // change, including a reserved self row. Raid retains its fixed 10 rows.
    section(&mut src, "unsafe fn set_overlay_scale(", "unsafe fn change_overlay_scale(",
        "rescale_px((source.bottom-source.top).max(1),old,new).max(overlay_min_height_mode(overlay.kind,new,overlay.compact_mode))",
        "audit_height_preserving_rows(overlay,source,old,new).max(overlay_min_height_mode(overlay.kind,new,overlay.compact_mode))",
        "preserve visible row capacity");

    // S03/M07: prioritise identity over optional Imagine artwork.
    section(&mut src, "unsafe fn dps_row_layout_responsive(", "unsafe fn paint_dps_secondary_colored(",
        "let reserve_badges = show_imagines && effective >= 480;",
        "let reserve_badges = show_imagines && effective >= 640;",
        "identity-first column budget");
    section(&mut src, "unsafe fn paint_reference_headers(", "unsafe fn paint_dps(",
        "&& dps_effective_width(content.right - content.left, scale) >= 480",
        "&& dps_effective_width(content.right - content.left, scale) >= 640",
        "optional icon header matches rows");

    // C02/C03: improve meaningful secondary labels, restrain metric bar.
    section(&mut src, "unsafe fn paint_reference_headers(", "unsafe fn paint_dps(",
        "SetTextColor(hdc, rgb(166, 177, 195));",
        "SetTextColor(hdc, rgb(190, 200, 214));", "secondary column labels");
    section(&mut src, "unsafe fn paint_dps_secondary_colored(", "fn reference_raid_columns(",
        "SetTextColor(hdc, rgb(166, 177, 195));",
        "SetTextColor(hdc, rgb(190, 200, 214));", "secondary specialization copy");
    section(&mut src, "unsafe fn paint_reference_normal_rows(", "unsafe fn paint_compact_player(",
        "let bar_top = (bar_bottom - 3).max(r.top + 3);",
        "let bar_top = (bar_bottom - 2).max(r.top + 2);", "restrained damage bar");

    // C04/M02: non-colour YOU cue on all layouts, divider only for separately
    // pinned off-screen rank (original true rank remains unchanged).
    section(&mut src, "unsafe fn paint_reference_normal_rows(", "unsafe fn paint_compact_player(",
        "        let base = text_on(bg);",
        "        let base = text_on(bg);\n        let display_name = if row.is_local { format!(\"YOU · {}\", row.name) } else { row.name.clone() };\n        if row.is_local && screen_i > 0 && rank > state.scroll + screen_i + 1 {\n            fill(hdc, &RECT { left: r.left + 3, top: y - 2, right: r.right - 3, bottom: y - 1 }, rgb(82, 97, 106));\n        }", "pinned divider and self cue");
    section(&mut src, "unsafe fn paint_reference_normal_rows(", "unsafe fn paint_compact_player(",
        "            &row.name,", "            &display_name,", "normal self label");
    section(&mut src, "unsafe fn paint_compact_player(", "unsafe fn paint_compact_raid_rows(",
        "        &row.name,", "        &if row.is_local { format!(\"YOU · {}\", row.name) } else { row.name.clone() },",
        "compact self label");
    section(&mut src, "unsafe fn paint_raid_player(", "\n}\n",
        "        &row.name,", "        &if row.is_local { format!(\"YOU · {}\", row.name) } else { row.name.clone() },",
        "raid self label");

    // M01: history cannot be mistaken for a live encounter.
    section(&mut src, "unsafe fn paint_reference_encounter(", "unsafe fn paint_mode_tab(",
        "    SetTextColor(hdc, rgb(241, 243, 247));\n    draw(\n        hdc,\n        &title,",
        "    let title = if state.history_index.is_some() { format!(\"HISTORY · {title}\") } else { title };\n    SetTextColor(hdc, rgb(241, 243, 247));\n    draw(\n        hdc,\n        &title,", "history state title");

    // M03: rank is based on selected-mode total, not active rate.
    section(&mut src, "unsafe fn paint_reference_raid_headers(", "#[cfg(test)]\nmod reference_meter_audit_tests",
        "(\"#\", col.left + 4, col.left + 24, 0)",
        "(\"T#\", col.left + 4, col.left + 24, 0)", "raid total rank heading");
    section(&mut src, "unsafe fn paint_reference_headers(", "unsafe fn paint_dps(",
        "(\"#\", r.left + 3, r.left + 24, 0)",
        "(\"T#\", r.left + 3, r.left + 24, 0)", "compact total rank heading");
    section(&mut src, "unsafe fn overlay_help(", "fn reference_meter_rate_label(",
        "Some(format!(\"{} • click to inspect\", dps_identity(&row)))",
        "Some(format!(\"{} · Ranked by {} · click to inspect\", dps_identity(&row), reference_meter_total_label(state.sort_mode)))",
        "rank explanation");

    // I04/I06: clarify layout/action menu and distinguish collapse from hide.
    section(&mut src, "fn toolbar_action_help(", "unsafe fn reference_line(",
        "ToolbarAction::More => \"Meter layout and actions\",",
        "ToolbarAction::More => \"Choose layout; history and settings are below the divider\",",
        "menu grouping tooltip");
    section(&mut src, "unsafe fn overlay_help(", "fn reference_meter_rate_label(",
        "[\"Open overlay settings\", \"Collapse overlay\", \"Close overlay\"]",
        "[\"Open overlay settings\", \"Collapse to restore tab (click tab to expand)\", \"Hide meter (restore from tray)\"]",
        "recovery tooltip");
    once(&mut src,
        "const REFERENCE_METER_BG: u32 = 0x001B1715; // Win32 COLORREF: #15171B",
        "const REFERENCE_METER_BG: u32 = 0x00221915; // Win32 COLORREF: #151922",
        "dark neutral foundation");

    src.push_str(r#"
// Preserve visible rows, not bitmap dimensions. Work-area clamping is applied
// by the existing window scale setter after this requested height is computed.
fn audit_height_preserving_rows(state: &State, source: RECT, old: i32, new: i32) -> i32 {
    let previous = (source.bottom - source.top).max(1);
    if state.kind != Kind::Dps { return rescale_px(previous, old, new); }
    let logical = physical_extent_to_logical(previous, old);
    let previous_pitch = dps_row_h_for(state, old).max(1);
    let rows = ((logical - dps_rows_top_for(state)).max(0) / previous_pitch).clamp(2, 40);
    let new_pitch = dps_row_h_for(state, new).max(1);
    scale_px(dps_rows_top_for(state).saturating_add(rows.saturating_mul(new_pitch)).saturating_add(8), new)
}
#[cfg(test)]
mod september_ui_audit_regression {
    use super::*;
    #[test]
    fn controls_are_at_least_24_logical_units() {
        for width in [300, 340, 420, 620, 900] {
            assert!(reference_button_w(width) >= 24);
            for index in 0..3 {
                let r = reference_system_rect(width,index);
                assert!(r.right - r.left >= 24 && r.bottom - r.top >= 24);
            }
        }
    }
    #[test]
    fn five_rows_fit_after_scale_roundtrip() {
        for percent in [60,70,80,90,100] {
            let logical = dps_rows_top() + 5 * dps_row_h(percent) + 8;
            let physical = scale_px(logical,percent);
            assert!(physical_extent_to_logical(physical,percent) + 1 >= logical);
        }
    }
}
"#);
    fs::write(path, src).expect("write September audit native renderer");
    println!("cargo:rerun-if-changed=build/legacy/build_v1375_ui_ux_audit.rs");
}
