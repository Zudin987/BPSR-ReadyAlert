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
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("feature renderer");

    // The selector's painted label and interactive hitbox must have the same
    // width. The legacy 108/94 px Compact Raid button clipped its text.
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
    // Keep history arrows when all three navigation buttons fit; reduce to
    // Live in a narrow header instead of overlapping the selector or icons.
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
        "bounded selector and responsive navigation");

    // Raid always means two columns: ten players per column. Window minimums
    // below ensure two columns have space rather than silently changing modes.
    between(&mut source,
        "fn reference_raid_columns(state: &State, width: i32) -> bool {",
        "unsafe fn paint_reference_headers(",
        "fn reference_raid_columns(state: &State, _width: i32) -> bool { raid_active(state) }\n",
        "raid mode stays two-column");

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

    // Preserve the complete QA suite, but correct its single outdated
    // width-gated assertion in a generated test copy (the production renderer
    // no longer width-gates two-column raid mode). The source QA file remains
    // untouched; all of its other checks still execute.
    let qa_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"))
        .join("src/ui_meter_qa_tests.rs");
    let mut qa = fs::read_to_string(&qa_path).expect("meter QA source");
    once(&mut qa,
        "let expected = width >= if compact { 680 } else { 900 };",
        "let expected = true;",
        "raid QA expectation");
    qa.push_str(r#"
#[test]
fn compact_raid_header_selector_fits_and_does_not_overlap_actions() {
    let s = state_stub(true);
    RAID_WINDOWS.with(|map| {
        map.borrow_mut().insert(raid_key(&s), RaidWindowState {
            active: true, normal: s.expanded, raid: s.expanded,
            has_raid: true, entry_compact: true,
        });
    });
    for width in [300, 420, 460, 600, 720, 1000] {
        let items = toolbar_items(&s, width);
        let selector = items.first().unwrap();
        assert_eq!(selector.label, "Compact Raid ▾");
        assert!(selector.rect.right - selector.rect.left >= 148);
        assert_eq!(reference_raid_columns(&s, width), true);
        assert!(items.last().unwrap().rect.right <= reference_system_rect(width, 0).left);
        for pair in items.windows(2) {
            assert!(pair[0].rect.right <= pair[1].rect.left,
                "header controls overlap at width {width}");
        }
    }
    RAID_WINDOWS.with(|map| { map.borrow_mut().remove(&raid_key(&s)); });
}
"#);
    let qa_generated = out.join("ui_meter_qa_tests_raid_v1345.rs");
    fs::write(&qa_generated, qa).expect("write adjusted regression suite");
    once(&mut source,
        "include!(concat!(env!(\"CARGO_MANIFEST_DIR\"),\"/src/ui_meter_qa_tests.rs\"));",
        "include!(concat!(env!(\"OUT_DIR\"),\"/ui_meter_qa_tests_raid_v1345.rs\"));",
        "use full adjusted QA suite");

    fs::write(path, source).expect("write full raid and header fit");
    println!("cargo:rerun-if-changed=build/legacy/build_v1345_narrow_header_fit.rs");
    println!("cargo:rerun-if-changed=src/ui_meter_qa_tests.rs");
}
