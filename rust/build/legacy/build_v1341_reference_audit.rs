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

    // Keep a minimum of two complete rows below the enlarged header even at
    // 50% scale, where text readability makes logical row height larger.
    replace_exact(&mut source,
        "fn overlay_min_height(kind:Kind,scale:i32)->i32{scale_px(if kind==Kind::Dps{200}else{260},clamp_kind_scale(kind,scale))}",
        "fn overlay_min_height(kind:Kind,scale:i32)->i32{let logical=if kind==Kind::Dps{(dps_rows_top()+2*dps_row_h(scale)+4).max(220)}else{260};scale_px(logical,clamp_kind_scale(kind,scale))}",
        1, "two-row minimum height");

    // Older regression tests asserted the previous 200px-high meter. Preserve
    // their exact scaling and width checks but change the expected heights to
    // the two-row minimum; the new audit test also checks all supported scales.
    replace_exact(&mut source, "assert_eq!(overlay_min_height(Kind::Dps,30),100)", "assert_eq!(overlay_min_height(Kind::Dps,30),129)", 1, "legacy 50% height");
    replace_exact(&mut source, "assert_eq!(overlay_min_height(Kind::Dps,100),200)", "assert_eq!(overlay_min_height(Kind::Dps,100),220)", 3, "legacy 100% height");
    replace_exact(&mut source, "assert_eq!(overlay_min_height(Kind::Dps,300),600)", "assert_eq!(overlay_min_height(Kind::Dps,300),660)", 1, "legacy 300% height");
    replace_exact(&mut source, "(150,100)", "(150,129)", 3, "legacy 50% size tuples");
    replace_exact(&mut source, "(300,200)", "(300,220)", 1, "legacy 100% size tuples");
    replace_exact(&mut source, "(900,600)", "(900,660)", 2, "legacy 300% size tuples");
    replace_exact(&mut source, "overlay_min_height_mode(Kind::Dps,100,false),200", "overlay_min_height_mode(Kind::Dps,100,false),220", 1, "normal versus compact minimum");
    replace_exact(&mut source, "fn dps_minimum_is_300_by_200_at_100_percent()", "fn dps_minimum_is_300_by_220_at_100_percent()", 1, "test name 100%");

    // Recolour real normal/raid progress paints; retain heal/tank palettes.
    replace_exact(&mut source,
        "crate::ui_modern::mix_color(crate::ui_modern::DARK_RAISED,mode_color(state.sort_mode),55)",
        "reference_meter_progress_color(state.sort_mode)", 2, "normal and raid progress bars");

    // Raid has two player columns, not the normal full-width heading.
    replace_exact(&mut source,
        "paint_reference_column_headers(hdc,hr,&layout,tier,scale,settings.meter.show_imagines);let rows=meter_rows(state);",
        "if raid_active(state){paint_reference_raid_headers(hdc,hr,state,&settings,scale);}else{paint_reference_column_headers(hdc,hr,&layout,tier,scale,settings.meter.show_imagines);}let rows=meter_rows(state);",
        1, "raid column headings");

    replace_exact(&mut source,
        "top:target_r.top,right:hp_r.left-7,bottom:target_r.bottom",
        "top:target_r.top,right:hp_r.left-7,bottom:target_r.top+21",
        1, "enrage first-line alignment");

    // Resolve the Imagine icon before the generic row help, and only compute
    // its hit rectangle once. Consumable F/S tooltips remain the first branch
    // of overlay_help before the generic player fallback.
    replace_exact(&mut source,
        "let next=overlay_help(hwnd,state,x,y).or_else(||hover_badge_at(hwnd,state,x,y));",
        "let next=hover_badge_at(hwnd,state,x,y).or_else(||overlay_help(hwnd,state,x,y));",
        1, "Imagine tooltip priority");
    replace_exact(&mut source,
        "if !state.compact_mode&&hover_badge_at(hwnd,state,x,y).is_some(){return None;}",
        "", 1, "avoid duplicate Imagine hover hit-test");
    replace_exact(&mut source,
        "if let Some(row)=dps_row_at(hwnd,state,y){return Some(if state.compact_mode",
        "if let Some(row)=if raid_active(state){raid_row_at(hwnd,state,x,y)}else{dps_row_at(hwnd,state,y)}{return Some(if state.compact_mode",
        1, "raid row hover identity");

    source.push_str(include_str!("../../src/feature_meter_reference_audit.rs"));
    source.push_str(include_str!("../../src/feature_meter_hover_qa.rs"));
    fs::write(path, source).expect("write corrected layouts, hovers and regression expectations");
    println!("cargo:rerun-if-changed=build/legacy/build_v1341_reference_audit.rs");
    println!("cargo:rerun-if-changed=src/feature_meter_reference_audit.rs");
    println!("cargo:rerun-if-changed=src/feature_meter_hover_qa.rs");
}
