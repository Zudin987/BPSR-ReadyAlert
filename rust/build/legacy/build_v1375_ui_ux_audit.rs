// Implement the September 2026 UI/UX audit against the actual generated UI.
// This is the final build stage: earlier strict presentation patches run first.
// Keep telemetry, class mappings, saved feature flags and the four meter modes.
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
    assert_eq!(source.matches(start).count(), 1, "UI audit {label}: section missing/ambiguous");
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

    // S01/I03: the toolbar and roster use the SAME cached type role, with a
    // 24-unit control floor. The old 20-unit glyph targets shrank independently
    // of the readable roster; do not change icon-only Copy or Reset semantics.
    section(&mut src, "fn reference_button_w(", "fn reference_button_gap(",
        "if width < 340 { 20 } else if width < 440 { 24 } else { 28 }",
        "if width < 440 { 24 } else { 28 }", "toolbar minimum targets");
    section(&mut src, "unsafe fn paint_toolbar(", "unsafe fn reference_set_layout(",
        "let old = SelectObject(hdc, dps_cached_font(100, true));",
        "let old = SelectObject(hdc, dps_primary_font(state));", "unified toolbar and data type");

    // S02: size changes retain the user's current complete-row viewport rather
    // than scaling the shell to half its height while fonts/badges hit floors.
    // Manual resize remains independent; raid keeps its established 10-row
    // fixed-height contract, and screen work-area clamping happens afterwards.
    section(&mut src, "unsafe fn set_overlay_scale(", "unsafe fn change_overlay_scale(",
        "rescale_px((source.bottom-source.top).max(1),old,new).max(overlay_min_height_mode(overlay.kind,new,overlay.compact_mode))",
        "audit_height_preserving_rows(overlay,source,old,new).max(overlay_min_height_mode(overlay.kind,new,overlay.compact_mode))",
        "preserve normal and compact row viewport");

    // S03/M07: protect names and class identity before reserving two intricate
    // Imagine images. Hidden optional data remains accessible in player detail.
    section(&mut src, "unsafe fn dps_row_layout_responsive(", "unsafe fn paint_dps_secondary_colored(",
        "let reserve_badges = show_imagines && effective >= 480;",
        "let reserve_badges = show_imagines && effective >= 640;",
        "prioritise name and class over optional badges");
    section(&mut src, "unsafe fn paint_reference_headers(", "unsafe fn paint_dps(",
        "&& dps_effective_width(content.right - content.left, scale) >= 480",
        "&& dps_effective_width(content.right - content.left, scale) >= 640",
        "badge header matches row visibility");

    // C02: muted-but-meaningful labels receive one consistent secondary tone.
    section(&mut src, "unsafe fn paint_reference_headers(", "unsafe fn paint_dps(",
        "SetTextColor(hdc, rgb(166, 177, 195));",
        "SetTextColor(hdc, rgb(190, 200, 214));", "column label readability");
    section(&mut src, "unsafe fn paint_dps_secondary_colored(", "fn reference_raid_columns(",
        "SetTextColor(hdc, rgb(166, 177, 195));",
        "SetTextColor(hdc, rgb(190, 200, 214));", "class secondary copy readability");

    // C03: a two-unit metric line remains legible without becoming the dominant
    // visual separator. The full row's 18%-tinted class surface carries identity.
    section(&mut src, "unsafe fn paint_reference_normal_rows(", "unsafe fn paint_compact_player(",
        "let bar_top = (bar_bottom - 3).max(r.top + 3);",
        "let bar_top = (bar_bottom - 2).max(r.top + 2);", "restrained normal metric bar");

    // C04/M02: non-colour YOU identification in ALL modes; show a quiet divider
    // only when an off-screen true-rank player is separately pinned at the end.
    section(&mut src, "unsafe fn paint_reference_normal_rows(", "unsafe fn paint_compact_player(",
        "        let base = text_on(bg);",
        "        let base = text_on(bg);\n        let display_name = if row.is_local { format!(\"YOU · {}\", row.name) } else { row.name.clone() };\n        if row.is_local && screen_i > 0 && rank > state.scroll + screen_i + 1 {\n            fill(hdc, &RECT { left: r.left + 3, top: y - 2, right: r.right - 3, bottom: y - 1 }, rgb(82, 97, 106));\n        }",
        "self cue and pinned divider");
    section(&mut src, "unsafe fn paint_reference_normal_rows(", "unsafe fn paint_compact_player(",
        "            &row.name,",
        "            &display_name,", "normal self label");
    section(&mut src, "unsafe fn paint_compact_player(", "unsafe fn paint_compact_raid_rows(",
        "        &row.name,",
        "        &if row.is_local { format!(\"YOU · {}\", row.name) } else { row.name.clone() },",
        "compact self label");
    section(&mut src, "unsafe fn paint_raid_player(", "fn overlay_min_height_mode(",
        "        &row.name,",
        "        &if row.is_local { format!(\"YOU · {}\", row.name) } else { row.name.clone() },",
        "raid self label");

    // M01: make historical state impossible to confuse with a current readout.
    // Live remains a dedicated return action; title keeps one encounter row.
    section(&mut src, "unsafe fn paint_reference_encounter(", "unsafe fn paint_mode_tab(",
        "    SetTextColor(hdc, rgb(241, 243, 247));\n    draw(\n        hdc,\n        &title,",
        "    let title = if state.history_index.is_some() { format!(\"HISTORY · {title}\") } else { title };\n    SetTextColor(hdc, rgb(241, 243, 247));\n    draw(\n        hdc,\n        &title,",
        "history not live title");

    // M03: the list is ranked by selected-mode total, NOT the active rate.
    // Narrow Raid may hide totals, so its # heading must signal that basis.
    section(&mut src, "unsafe fn paint_reference_raid_headers(", "#[cfg(test)]\nmod reference_meter_audit_tests", 
        "(\"#\", col.left + 4, col.left + 24, 0)",
        "(\"T#\", col.left + 4, col.left + 24, 0)", "raid total-rank label");
    section(&mut src, "unsafe fn paint_reference_headers(", "unsafe fn paint_dps(",
        "(\"#\", r.left + 3, r.left + 24, 0)",
        "(\"T#\", r.left + 3, r.left + 24, 0)", "compact total-rank label");
    section(&mut src, "unsafe fn overlay_help(", "fn reference_meter_rate_label(",
        "Some(format!(\"{} • click to inspect\", dps_identity(&row)))",
        "Some(format!(\"{} · Ranked by {} · click to inspect\", dps_identity(&row), reference_meter_total_label(state.sort_mode)))",
        "explain rank basis in hover detail");

    // M04/I06: precise recovery/action help, including an explicit unit key.
    section(&mut src, "fn toolbar_action_help(", "unsafe fn reference_line(",
        "ToolbarAction::More => \"Meter layout and actions\",",
        "ToolbarAction::More => \"Choose layout; history and settings are below the divider\",",
        "grouped menu description");
    section(&mut src, "unsafe fn overlay_help(", "fn reference_meter_rate_label(",
        "[\"Open overlay settings\", \"Collapse overlay\", \"Close overlay\"]",
        "[\"Open overlay settings\", \"Collapse to restore tab (click tab to expand)\", \"Hide meter (restore from tray)\"]",
        "collapse versus hide help");

    // Colour foundation: background hue stays neutral; full-width row colour
    // lives in feature_meter_reference_theme.rs and preserves class mapping.
    once(&mut src,
        "const REFERENCE_METER_BG: u32 = 0x001B1715; // Win32 COLORREF: #15171B",
        "const REFERENCE_METER_BG: u32 = 0x00221915; // Win32 COLORREF: #151922",
        "neutral overlay surface");

    src.push_str(r#"
// A UI-size adjustment preserves row capacity while text and icons have their
// readability floors. This helper is also used for manually chosen heights;
// neither setting values nor encounter order are changed by scaling.
fn audit_height_preserving_rows(state: &State, source: RECT, old: i32, new: i32) -> i32 {
    let previous = (source.bottom - source.top).max(1);
    if state.kind != Kind::Dps { return rescale_px(previous, old, new); }
    let logical = physical_extent_to_logical(previous, old);
    let previous_pitch = dps_row_h_for(state, old).max(1);
    let rows = ((logical - dps_rows_top_for(state)).max(0) / previous_pitch).clamp(2, 40);
    let next_pitch = dps_row_h_for(state, new).max(1);
    let required = dps_rows_top_for(state)
        .saturating_add(rows.saturating_mul(next_pitch))
        .saturating_add(8);
    scale_px(required, new)
}
#[cfg(test)]
mod september_ui_audit_regression {
    use super::*;
    #[test]
    fn control_targets_do_not_shrink_below_24_logical_units() {
        for width in [300, 340, 420, 620, 900] {
            assert!(reference_button_w(width) >= 24);
            for index in 0..3 {
                let r = reference_system_rect(width,index);
                assert!(r.right - r.left >= 24 && r.bottom - r.top >= 24);
            }
        }
    }
    #[test]
    fn scaling_keeps_complete_row_budget_in_normal_and_compact() {
        for compact in [false, true] {
            let mut state = State::test_default_dps();
            state.compact_mode = compact;
            for old in [60, 70, 80, 90, 100] {
                let pitch = dps_row_h_for(&state, old);
                let h = scale_px(dps_rows_top_for(&state) + 5*pitch + 8, old);
                for new in [60, 70, 80, 90, 100] {
                    let rect = RECT {left:0,top:0,right:600,bottom:h};
                    let next = audit_height_preserving_rows(&state,rect,old,new);
                    assert!(physical_extent_to_logical(next,new) >= dps_rows_top_for(&state) + 5*dps_row_h_for(&state,new), "{old} -> {new}, compact={compact}");
                }
            }
        }
    }
}
"#);
    fs::write(path, src).expect("write September audit generated renderer");
    println!("cargo:rerun-if-changed=build/legacy/build_v1375_ui_ux_audit.rs");
}
