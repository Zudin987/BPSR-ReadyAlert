use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1358_rank_death_header_alignment.rs");
    pub fn run() { main(); }
}

fn once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "non-meter layout {label}: expected one anchor, found {count}");
    *source = source.replacen(from, to, 1);
}

// Replace a complete native-control statement without changing its adjacent
// state, event routing, or persistence logic.
fn control(source: &mut String, start: &str, replacement: &str, label: &str) {
    let count = source.matches(start).count();
    assert_eq!(count, 1, "non-meter control {label}: expected one anchor, found {count}");
    let begin = source.find(start).expect("control anchor");
    let end = begin + source[begin..].find(';').expect("control terminator") + 1;
    source.replace_range(begin..end, replacement);
}

fn patch_mechanics(path: &PathBuf) {
    let mut source = fs::read_to_string(path).expect("read active generated overlays");
    // v1351 moves shared opacity/scale/collapse controls down, but moves only
    // DPS fields to match. The mechanics grid still starts at y=144 beneath
    // those controls. Scope replacements to the mechanics ELSE branch: DPS
    // presentation, calculations and all its stored user options remain intact.
    let begin_anchor = "feature_label(hwnd,7020,";
    assert_eq!(source.matches(begin_anchor).count(), 1, "mechanics form start");
    let begin = source.find(begin_anchor).unwrap();
    let finish = begin + source[begin..].find("    }\n    refresh_feature_form(hwnd,state);")
        .expect("mechanics form end");
    let mut form = source[begin..finish].to_string();
    control(&mut form, "feature_label(hwnd,7020,", "feature_label(hwnd,7020,\"\",20,184,260,24);", "tracked count");
    control(&mut form, "feature_button(hwnd,7021,", "feature_button(hwnd,7021,\"Event Tracker\",534,180,150);", "tracker action");
    // Identify the section heading by its position BEFORE the rows declaration;
    // the later guidance is also a feature_label(hwnd,0,...) and must survive.
    let grid_marker = form.find("let rows=").expect("mechanics grid row declaration");
    let heading = form[..grid_marker].rfind("feature_label(hwnd,0,").expect("mechanics section heading");
    let heading_end = heading + form[heading..].find(';').unwrap() + 1;
    form.replace_range(heading..heading_end, "feature_label(hwnd,0,\"COMBAT ATTRIBUTES\",20,224,300,20);");
    control(&mut form, "feature_check(hwnd,7200+i as i32,label,",
        "feature_check(hwnd,7200+i as i32,label,20+(i/rows)as i32*224,250+(i%rows)as i32*28,216);",
        "attribute rows");
    let bottom_start = form.find("let bottom=").expect("mechanics section bottom");
    let bottom_end = bottom_start + form[bottom_start..].find(';').unwrap() + 1;
    form.replace_range(bottom_start..bottom_end, "let bottom=250+rows as i32*28;");
    control(&mut form, "feature_label(hwnd,0,\"Choose up to",
        "feature_label(hwnd,0,\"Choose up to 8 stats. Food, Serum and Event Tracker rows appear automatically. Changes save immediately.\",20,bottom+14,660,38);",
        "selection guidance");
    control(&mut form, "feature_button(hwnd,2,",
        "feature_button(hwnd,2,\"Close\",598,bottom+62,92);", "separate close footer");
    source.replace_range(begin..finish, &form);
    once(&mut source,
        "(crate::ui_theme::MECH_SETTINGS_W,crate::ui_theme::MECH_SETTINGS_H)",
        "(crate::ui_theme::MECH_SETTINGS_W,crate::ui_theme::MECH_SETTINGS_H+170)",
        "mechanics window content height");
    fs::write(path, source).expect("write isolated mechanics form geometry");
}

fn patch_event_editor(path: &PathBuf) {
    let mut source = fs::read_to_string(path).expect("read active generated Event Tracker editor");
    // Previously the 470-wide, 20-high helper shared the footer region with
    // buttons. Reserve a full-width two-line region above separate actions.
    once(&mut source,
        "const HEIGHT: i32 = crate::ui_theme::EVENT_H;",
        "const HEIGHT: i32 = crate::ui_theme::EVENT_H + 92;",
        "Event Tracker window content height");
    control(&mut source, "let note=create_static(hwnd,",
        "let note=create_static(hwnd,\"Buffs use the game timer. Skill and attribute rules use Display time.\",18,370,660,38);",
        "fully visible helper");
    control(&mut source, "create_button(hwnd,ID_APPLY,",
        "create_button(hwnd,ID_APPLY,\"Apply\",510,422,82,30);",
        "Apply below helper");
    control(&mut source, "create_button(hwnd,ID_CLOSE,",
        "create_button(hwnd,ID_CLOSE,\"Close\",600,422,82,30);",
        "Close below helper");
    // Preserve danger semantics for enabled Remove; neutral when disabled.
    once(&mut source,
        "crate::ui_theme::draw_button(item,false,id==ID_APPLY,id==ID_REMOVE,false)",
        "crate::ui_theme::draw_button(item,false,id==ID_APPLY,id==ID_REMOVE && windows_sys::Win32::UI::WindowsAndMessaging::IsWindowEnabled((*item).hwndItem)!=0,false)",
        "disabled Remove semantics");
    fs::write(path, source).expect("write isolated Event Tracker footer geometry");
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_mechanics(&out.join("feature_overlays_v170_fixed.rs"));
    patch_event_editor(&out.join("event_tracker_ui_v1160_fixed.rs"));
    println!("cargo:rerun-if-changed=build/legacy/build_v1359_non_meter_layout_fixes.rs");
}
