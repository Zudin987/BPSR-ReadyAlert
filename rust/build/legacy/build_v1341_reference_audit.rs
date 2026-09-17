use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1340_reference_hitboxes.rs");
    pub fn run() { main(); }
}

fn replace_exact(source: &mut String, from: &str, to: &str, expected: usize, label: &str) {
    let found = source.matches(from).count();
    assert_eq!(found, expected, "meter audit: {label} changed ({found}, expected {expected})");
    *source = source.replace(from, to);
}

fn main() {
    previous::run();
    let path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"))
        .join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read generated audited meter");

    // A 200px minimum leaves only one row under the taller header.
    // At 50% scale, adaptive rows are taller in logical pixels; derive the
    // minimum from the scaled row height rather than hardcoding 220.
    replace_exact(&mut source,
        "fn overlay_min_height(kind:Kind,scale:i32)->i32{scale_px(if kind==Kind::Dps{200}else{260},clamp_kind_scale(kind,scale))}",
        "fn overlay_min_height(kind:Kind,scale:i32)->i32{let logical=if kind==Kind::Dps{(dps_rows_top()+2*dps_row_h(scale)+4).max(220)}else{260};scale_px(logical,clamp_kind_scale(kind,scale))}",
        1, "two-row minimum height");

    // Recolour the real paint-time progress path, not unused colour literals.
    // Healing and tanking retain their own mode-specific palette.
    replace_exact(&mut source,
        "crate::ui_modern::mix_color(crate::ui_modern::DARK_RAISED,mode_color(state.sort_mode),55)",
        "reference_meter_progress_color(state.sort_mode)", 2, "normal and raid progress bars");

    // Raid has two player columns, so the normal full-width header is wrong.
    replace_exact(&mut source,
        "paint_reference_column_headers(hdc,hr,&layout,tier,scale,settings.meter.show_imagines);let rows=meter_rows(state);",
        "if raid_active(state){paint_reference_raid_headers(hdc,hr,state,&settings,scale);}else{paint_reference_column_headers(hdc,hr,&layout,tier,scale,settings.meter.show_imagines);}let rows=meter_rows(state);",
        1, "raid column headings");

    // Prevent the enrage text box from spilling into the second target line.
    replace_exact(&mut source,
        "top:target_r.top,right:hp_r.left-7,bottom:target_r.bottom",
        "top:target_r.top,right:hp_r.left-7,bottom:target_r.top+21",
        1, "enrage first-line alignment");

    // Give Imagine icons priority over generic row help. F/S help remains
    // first inside overlay_help, before any fallback generic player tooltip.
    replace_exact(&mut source,
        "let next=overlay_help(hwnd,state,x,y).or_else(||hover_badge_at(hwnd,state,x,y));",
        "let next=hover_badge_at(hwnd,state,x,y).or_else(||overlay_help(hwnd,state,x,y));",
        1, "Imagine tooltip priority");
    replace_exact(&mut source,
        "if !state.compact_mode&&hover_badge_at(hwnd,state,x,y).is_some(){return None;}",
        "", 1, "avoid duplicate Imagine hover hit-test");
    // In raid, generic hover must resolve left/right columns, not whichever
    // row happens to be at the same y in normal mode.
    replace_exact(&mut source,
        "if let Some(row)=dps_row_at(hwnd,state,y){return Some(if state.compact_mode",
        "if let Some(row)=if raid_active(state){raid_row_at(hwnd,state,x,y)}else{dps_row_at(hwnd,state,y)}{return Some(if state.compact_mode",
        1, "raid row hover identity");

    source.push_str(include_str!("../../src/feature_meter_reference_audit.rs"));
    fs::write(path, source).expect("write corrected layouts and hover behavior");
    println!("cargo:rerun-if-changed=build/legacy/build_v1341_reference_audit.rs");
    println!("cargo:rerun-if-changed=src/feature_meter_reference_audit.rs");
}
