use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1344_meter_header_history_ux.rs");
    pub fn run() { main(); }
}

fn once(source: &mut String, from: &str, to: &str, label: &str) {
    let found = source.matches(from).count();
    assert_eq!(found, 1, "raid/header fix {label}: expected one anchor, found {found}");
    *source = source.replacen(from, to, 1);
}
fn between(source: &mut String, from: &str, until: &str, to: &str, label: &str) {
    let found = source.matches(from).count();
    assert_eq!(found, 1, "raid/header fix {label}: expected one start, found {found}");
    let start = source.find(from).expect("start");
    let stop = start + source[start..].find(until).expect("end");
    source.replace_range(start..stop, to);
}

fn main() {
    previous::run();
    let path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"))
        .join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("feature renderer");

    // The mode label is part of the hitbox, not free-floating text. Leave room
    // for "Compact Raid ▾" (the previous 108 px button clipped/overflowed).
    between(&mut source,
        "fn reference_selector_w(state: &State, right: i32) -> i32 {",
        "fn reference_action_enabled(state: &State, action: ToolbarAction) -> bool {",
        r#"fn reference_selector_w(state: &State, _right: i32) -> i32 {
    match (state.compact_mode, raid_active(state)) {
        (true, true) => 148,
        (true, false) => 96,
        (false, true) => 76,
        _ => 96,
    }
}
fn toolbar_items(state: &State, right: i32) -> Vec<ToolbarItem> {
    if state.kind != Kind::Dps { return Vec::new(); }
    let w = reference_button_w(right);
    let gap = reference_button_gap(right);
    let selector_w = reference_selector_w(state, right);
    let selector_right = 6 + selector_w;
    let system_left = reference_system_rect(right, 0).left;
    let actions_left = system_left - gap - (2 * w + gap);
    let live_w = if right < 340 { 38 } else { 42 };
    let nav_w = 2 * w + live_w;
    let edge = if right < 340 { 1 } else { 3 };
    let rect = |left, width| RECT { left, top: 3, right: left + width, bottom: TOOLBAR_H - 3 };
    let mut items = vec![ToolbarItem {
        action: ToolbarAction::More,
        rect: rect(6, selector_w),
        label: reference_layout_label(state),
    }];
    // Keep history arrows whenever they fit; keep Live alone in the tightest
    // Compact Raid widths. All emitted controls share paint and hit rectangles.
    if actions_left - selector_right >= nav_w + 2 * edge {
        let nav_left = (right / 2 - nav_w / 2)
            .clamp(selector_right + edge, actions_left - nav_w - edge);
        items.push(ToolbarItem { action: ToolbarAction::Older, rect: rect(nav_left, w), label: "‹" });
        items.push(ToolbarItem { action: ToolbarAction::Live, rect: rect(nav_left + w, live_w), label: "Live" });
        items.push(ToolbarItem { action: ToolbarAction::Newer, rect: rect(nav_left + w + live_w, w), label: "›" });
    } else if actions_left - selector_right >= live_w + 2 * edge {
        let live_left = (right / 2 - live_w / 2)
            .clamp(selector_right + edge, actions_left - live_w - edge);
        items.push(ToolbarItem { action: ToolbarAction::Live, rect: rect(live_left, live_w), label: "Live" });
    }
    items.push(ToolbarItem { action: ToolbarAction::Copy, rect: rect(actions_left, w), label: "" });
    items.push(ToolbarItem { action: ToolbarAction::Reset, rect: rect(actions_left + w + gap, w), label: "" });
    items
}
"#,
        "bounded dynamic selector and responsive navigation");

    // Raid must mean two columns at all widths, not silently turn into Normal
    // or Compact due to an arbitrary 680/900 logical-pixel threshold.
    between(&mut source,
        "fn reference_raid_columns(state: &State, width: i32) -> bool {",
        "unsafe fn paint_reference_headers(",
        "fn reference_raid_columns(state: &State, _width: i32) -> bool { raid_active(state) }\n",
        "raid mode stays two-column");

    // Maintain workable two-column minimums on real windows. A previously
    // saved 300-450px raid window is widened on entry, with its edge anchoring
    // preserved. Keep non-raid minimums and other overlays unchanged.
    source.push_str(r#"
fn reference_raid_min_width(scale: i32, compact: bool) -> i32 {
    scale_px(if compact { 460 } else { 700 }, scale)
}
fn reference_overlay_min_width(state: &State, scale: i32) -> i32 {
    let base = overlay_min_width_mode(state.kind, scale, state.compact_mode);
    if state.kind == Kind::Dps && raid_active(state) {
        base.max(reference_raid_min_width(scale, state.compact_mode))
    } else { base }
}
fn reference_raid_full_height(state: &State) -> i32 {
    let scale = dps_layout_scale(state);
    scale_px(dps_rows_top_for(state) + RAID_ROWS_PER_COLUMN as i32 * dps_row_h_for(state, scale) + 8, state.scale_percent)
}

#[cfg(test)]
mod raid_header_regression_tests {
    use super::*;
    #[test]
    fn raid_still_has_two_columns_of_ten_and_sensible_minimum_widths() {
        assert_eq!(RAID_ROWS_PER_COLUMN, 10);
        assert_eq!(RAID_MAX_ROWS, 20);
        assert_eq!(reference_raid_min_width(100, true), 460);
        assert_eq!(reference_raid_min_width(100, false), 700);
        assert_eq!(reference_raid_min_width(150, true), 690);
    }
}
"#);
    once(&mut source,
        "overlay_min_width_mode((*ptr).kind,(*ptr).scale_percent,(*ptr).compact_mode),overlay_min_height_mode",
        "reference_overlay_min_width(&*ptr,(*ptr).scale_percent),overlay_min_height_mode",
        "drag resize min width");
    once(&mut source,
        "let min_w=overlay_min_width_mode(state.kind,state.scale_percent,state.compact_mode);let min_h=overlay_min_height_mode",
        "let min_w=reference_overlay_min_width(state,state.scale_percent);let min_h=overlay_min_height_mode",
        "expand raid min width");
    once(&mut source,
        "let target_w=rescale_px((source.right-source.left).max(1),old,new).max(overlay_min_width_mode(overlay.kind,new,overlay.compact_mode));",
        "let target_w=rescale_px((source.right-source.left).max(1),old,new).max(reference_overlay_min_width(overlay,new));",
        "scale raid min width");
    once(&mut source,
        "let min_w=overlay_min_width_mode(Kind::Dps,state.scale_percent,state.compact_mode);let min_h=overlay_min_height_mode(Kind::Dps,state.scale_percent,state.compact_mode);",
        "let min_w=reference_overlay_min_width(state,state.scale_percent);let min_h=if raid{reference_raid_full_height(state)}else{overlay_min_height_mode(Kind::Dps,state.scale_percent,state.compact_mode)};",
        "compact raid switch minimum");
    once(&mut source,
        "let target=mode_saved.or(legacy).filter(|r|raid_rect_valid(*r)).unwrap_or_else(||raid_auto_rect(hwnd,state,current));RAID_WINDOWS.with",
        "let candidate=mode_saved.or(legacy).filter(|r|raid_rect_valid(*r)).unwrap_or_else(||raid_auto_rect(hwnd,state,current));let work=crate::ui::work_area(hwnd);let ax=preferred_axis_anchor(candidate.left,candidate.right,work.left,work.right);let ay=preferred_axis_anchor(candidate.top,candidate.bottom,work.top,work.bottom);let target=anchored_scaled_rect(candidate,work,(candidate.right-candidate.left).max(reference_raid_min_width(state.scale_percent,state.compact_mode)),(candidate.bottom-candidate.top).max(reference_raid_full_height(state)),ax,ay);RAID_WINDOWS.with",
        "restore saved raid as two full columns");

    fs::write(path, source).expect("write full raid and header fit");
    println!("cargo:rerun-if-changed=build/legacy/build_v1345_narrow_header_fit.rs");
}
