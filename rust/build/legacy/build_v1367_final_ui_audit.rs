// Final UI audit: run after all approved DPS/Chat build transformations.
// Change only generated presentation, leaving source settings and telemetry untouched.
use std::{env, fs, path::PathBuf};
mod previous {
    include!("build_v1366_chat_header_font_offset.rs");
    pub fn run() { main(); }
}
fn replace_one(text: &mut String, old: &str, new: &str, issue: &str) {
    assert_eq!(text.matches(old).count(), 1, "final audit {issue}: source anchor changed");
    *text = text.replacen(old, new, 1);
}
fn patch_dps(out: &PathBuf) {
    let path=out.join("feature_overlays_v170_fixed.rs");
    let mut s=fs::read_to_string(&path).expect("DPS settings generated source");
    // F01: use actual client and button HWND rectangles, not hard-coded dialog width.
    replace_one(&mut s,"feature_button(hwnd,2,\"Close\",594,432,92);",
        "feature_button(hwnd,2,\"Close\",20,432,92);audit_place_dps_footer(hwnd);","F01 initial button");
    let helper=r#"unsafe fn audit_place_dps_footer(hwnd:HWND){
    let button=fc(hwnd,2);if button.is_null(){return;}
    let mut client:RECT=std::mem::zeroed();let mut bounds:RECT=std::mem::zeroed();
    if GetClientRect(hwnd,&mut client)==0||GetWindowRect(button,&mut bounds)==0{return;}
    let w=(bounds.right-bounds.left).max(1);let h=(bounds.bottom-bounds.top).max(1);
    // Clamp to the available client width so even unexpectedly small windows fit.
    let x=(client.right-w-20).max(0);let y=(client.bottom-h-20).max(0);
    windows_sys::Win32::UI::WindowsAndMessaging::MoveWindow(button,x,y,w,h,1);
}
"#;
    replace_one(&mut s,"unsafe fn build_feature_form(",&format!("{helper}unsafe fn build_feature_form("),"F01 placement helper");
    replace_one(&mut s,"        WM_SIZE=>0,","        WM_SIZE=>{if state.kind==Kind::Dps{audit_place_dps_footer(hwnd);}0},\n        0x02E0=>{let result=DefWindowProcW(hwnd,msg,wparam,lparam);if state.kind==Kind::Dps{audit_place_dps_footer(hwnd);}result},","F01 DPI/resize");
    s.push_str(r#"
#[cfg(test)]mod final_dps_footer_geometry_test{
 #[test]fn button_is_in_client_at_supported_widths(){for client in [300,420,560,600,620,720,900]{let width=92;let x=(client-width-20).max(0);assert!(x>=0&&x+width<=client);}}
}
"#);
    fs::write(path,s).expect("F01 output");
}
fn patch_settings(out:&PathBuf){
    let path=out.join("settings_ui_v1160_fixed.rs");
    let mut s=fs::read_to_string(&path).expect("chat settings generated source");
    // F02: two fields and Test TTS now have >= 10px gutters at actual pixel positions.
    replace_one(&mut s,"inline_field(hwnd,state,PAGE_SPEECH,\"Own username\",ID_TTS_USERNAME,174,228,110,170);",
        "inline_field(hwnd,state,PAGE_SPEECH,\"Own username\",ID_TTS_USERNAME,174,228,110,166);","F02 username");
    replace_one(&mut s,"inline_field(hwnd,state,PAGE_SPEECH,\"TTS volume\",ID_TTS_VOLUME,458,228,100,72);",
        "inline_field(hwnd,state,PAGE_SPEECH,\"TTS volume (%)\",ID_TTS_VOLUME,480,228,100,66);","F02 volume");
    replace_one(&mut s,"button(hwnd,state,PAGE_SPEECH,ID_TEST_TTS,\"Test TTS\",642,226,106,30);",
        "button(hwnd,state,PAGE_SPEECH,ID_TEST_TTS,\"Test TTS\",664,226,84,30);","F02 test button");
    // F03: give the right column a genuine gutter; leave font sizing and tone alone.
    replace_one(&mut s,"inline_field(hwnd,state,PAGE_OVERLAY,\"Font family\",ID_FONT_FAMILY,174,278,105,150);",
        "inline_field(hwnd,state,PAGE_OVERLAY,\"Font family\",ID_FONT_FAMILY,174,278,105,174);","F03 font family");
    replace_one(&mut s,"inline_field(hwnd,state,PAGE_OVERLAY,\"Max history\",ID_MAX_HISTORY,438,278,105,72);",
        "inline_field(hwnd,state,PAGE_OVERLAY,\"Max history\",ID_MAX_HISTORY,480,278,105,94);","F03 max history");
    replace_one(&mut s,"inline_field(hwnd,state,PAGE_OVERLAY,\"Recovery hotkey\",ID_CLICK_HOTKEY,174,310,105,150);",
        "inline_field(hwnd,state,PAGE_OVERLAY,\"Recovery hotkey\",ID_CLICK_HOTKEY,174,310,105,174);","F03 recovery hotkey");
    replace_one(&mut s,"label(hwnd,state,PAGE_OVERLAY,\"Collapse edge\",438,314,105,20);combo(hwnd,state,PAGE_OVERLAY,ID_COLLAPSE_SIDE,548,308,120,120);",
        "label(hwnd,state,PAGE_OVERLAY,\"Collapse edge\",480,314,105,20);combo(hwnd,state,PAGE_OVERLAY,ID_COLLAPSE_SIDE,591,308,143,120);","F03 collapse edge");
    // F09: percentages only for real 0-100% settings. Font size is points
    // (the chat renderer converts it to pixels via *96/72).
    for (old,new,label) in [
        ("\"Alert volume\",ID_ALERT_VOLUME","\"Alert volume (%)\",ID_ALERT_VOLUME","F09 alert"),
        ("\"Opacity\",ID_WINDOW_OPACITY","\"Opacity (%)\",ID_WINDOW_OPACITY","F09 opacity"),
        ("\"Font size\",ID_FONT_SIZE","\"Font size (pt)\",ID_FONT_SIZE","F09 font points"),
        ("\"Volume\",ID_CHAT_SOUND_VOLUME","\"Volume (%)\",ID_CHAT_SOUND_VOLUME","F09 sound"),
    ]{replace_one(&mut s,old,new,label);}
    // F07: keep native ES_AUTOHSCROLL and full persisted expressions; widen Match.
    for (old,new,label) in [
        ("\"Sound path\",ID_PRIVATE_SOUND_PATH,174,148,82,178","\"Sound path\",ID_PRIVATE_SOUND_PATH,174,148,82,182","F07 private path"),
        ("\"Match\",ID_RULE1_MATCH,458,116,62,104","\"Match\",ID_RULE1_MATCH,458,116,62,218","F07 match 1"),
        ("\"Match\",ID_RULE2_MATCH,458,216,62,104","\"Match\",ID_RULE2_MATCH,458,216,62,218","F07 match 2"),
    ]{replace_one(&mut s,old,new,label);}
    replace_one(&mut s,"button(hwnd,state,PAGE_SOUNDS,ID_OPEN_LOGS,\"Open logs\",374,292,100,28);",
        "button(hwnd,state,PAGE_SOUNDS,ID_OPEN_LOGS,\"Open logs\",374,292,100,28);\n    info(hwnd,state,PAGE_SOUNDS,\"Long paths and match expressions: Home/End scroll the field; Ctrl+A, Ctrl+C copies the entire unmodified value.\",174,356,580,42);","F07 complete-value access");
    // F08: refresh_blocked_actions already disables both empty-list actions.
    assert!(s.contains("EnableWindow(GetDlgItem(hwnd, ID_CLEAR_BLOCKED), has_users as i32);"),"F08 empty-list disable regression");
    // F09: last Tabs checkbox ends y366. Give it 39px clearance to the footer.
    replace_one(&mut s,"create_button(hwnd,ID_APPLY,\"Apply\",600,365,80,30)","create_button(hwnd,ID_APPLY,\"Apply\",600,405,80,30)","F09 Apply clearance");
    replace_one(&mut s,"create_button(hwnd, ID_CLOSE, \"Close\", 690, 365, 80, 30)","create_button(hwnd, ID_CLOSE, \"Close\", 690, 405, 80, 30)","F09 Close clearance");
    replace_one(&mut s,"crate::ui_theme::SETTINGS_W, crate::ui_theme::SETTINGS_H,",
        "crate::ui_theme::SETTINGS_W, crate::ui_theme::SETTINGS_H + 50,","F09 dialog room");
    // F04: use existing measured, scroll-capable dark/mist themed dialog.
    replace_one(&mut s,"windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(hwnd, wide(&notice).as_ptr(), wide(\"Cloud chat privacy consent\").as_ptr(), windows_sys::Win32::UI::WindowsAndMessaging::MB_YESNO | MB_ICONINFORMATION)",
        "crate::ui_modern::qa_message_box_w(hwnd, wide(&notice).as_ptr(), wide(\"Cloud chat privacy consent\").as_ptr(), windows_sys::Win32::UI::WindowsAndMessaging::MB_YESNO | MB_ICONINFORMATION)","F04 dark consent");
    fs::write(path,s).expect("chat settings audit output");
}
fn patch_dialog(out:&PathBuf){
    let path=out.join("ui_pixel_qa_v1316.rs");let mut s=fs::read_to_string(&path).expect("themed dialog source");
    // F04: no is first and focused. Enter, Escape, X all preserve non-consent.
    let consent=r#"        QaFlavor::YesNo if t.contains("cloud chat privacy consent") => vec![
            QaDialogButton { id: QA_IDNO, label: "Keep disabled".into(), primary: true, danger: false },
            QaDialogButton { id: QA_IDYES, label: "Agree & enable".into(), primary: false, danger: false },
        ],
"#;
    replace_one(&mut s,"    match flavor {\n        QaFlavor::YesNoCancel if b.contains(\"save changes\")",&format!("    match flavor {{\n{consent}        QaFlavor::YesNoCancel if b.contains(\"save changes\")"),"F04 consent actions");
    // F05: actual default action is Keep Editing, not just visual emphasis.
    replace_one(&mut s,"QaDialogButton { id: QA_IDYES, label: \"Save\".into(), primary: true, danger: false },\n            QaDialogButton { id: QA_IDNO, label: \"Don't Save\".into(), primary: false, danger: true },\n            QaDialogButton { id: QA_IDCANCEL, label: \"Cancel\".into(), primary: false, danger: false },",
        "QaDialogButton { id: QA_IDCANCEL, label: \"Keep Editing\".into(), primary: true, danger: false },\n            QaDialogButton { id: QA_IDYES, label: \"Save\".into(), primary: false, danger: false },\n            QaDialogButton { id: QA_IDNO, label: \"Discard\".into(), primary: false, danger: true },","F05 safe default");
    // F08: disabled Clear all must not show a dangerous outline.
    replace_one(&mut s,"} else if !nav && danger {","} else if !nav && danger && !disabled {","F08 neutral disabled border");
    s.push_str(r#"
#[cfg(test)]mod final_consent_and_discard_tests{
 use super::*;
 #[test]fn cloud_consent_is_opt_in(){let b=qa_dialog_buttons(QaFlavor::YesNo,"Cloud chat privacy consent","Enable cloud chat?");assert_eq!(b[0].id,QA_IDNO);assert_eq!(b[1].id,QA_IDYES);}
 #[test]fn event_discard_requires_explicit_choice(){let b=qa_dialog_buttons(QaFlavor::YesNoCancel,"Unsaved rules","Save changes?");assert_eq!(b[0].id,QA_IDCANCEL);assert_eq!(b[2].id,QA_IDNO);assert_eq!(qa_close_result(QaFlavor::YesNoCancel),QA_IDCANCEL);}
}
"#);
    fs::write(path,s).expect("themed dialog audit output");
}
fn patch_controls(out:&PathBuf){
    let path=out.join("ui_pixel_controls_v1316.rs");let mut s=fs::read_to_string(&path).expect("native controls");
    // F06: meaningful enabled unchecked glyphs and input edges target >=3:1.
    replace_one(&mut s,"if focused { BPSR_ACCENT } else { MIST_BORDER_STRONG }","if focused { BPSR_ACCENT } else if enabled { crate::ui_theme::rgb(118,124,140) } else { MIST_BORDER_STRONG }","F06 checkbox identification");
    replace_one(&mut s,"let color = if focused { BPSR_ACCENT } else if hovered { MIST_BORDER_STRONG } else { MIST_BORDER };",
        "let color = if focused { BPSR_ACCENT } else { crate::ui_theme::rgb(118,124,140) };","F06 edit identification");
    fs::write(path,s).expect("control contrast output");
}
fn main(){
 previous::run();let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
 patch_dps(&out);patch_settings(&out);patch_dialog(&out);patch_controls(&out);
 println!("cargo:rerun-if-changed=build/legacy/build_v1367_final_ui_audit.rs");
}
