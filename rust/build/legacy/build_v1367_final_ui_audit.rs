//! Final UI audit corrections. Work on generated Rust, never hand-edit OUT_DIR.
//! Kept after the approved DPS visual generator to avoid altering the meter design.
use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1366_chat_header_font_offset.rs");
    pub fn run() { main(); }
}

fn once(src: &mut String, old: &str, new: &str, issue: &str) {
    let count = src.matches(old).count();
    assert_eq!(count, 1, "final UI audit {issue}: expected one anchor, got {count}");
    *src = src.replacen(old, new, 1);
}

fn region(src: &mut String, start: &str, end: &str, f: impl FnOnce(&mut String)) {
    let a = src.find(start).unwrap_or_else(|| panic!("missing audit region {start}"));
    let b = a + src[a..].find(end).unwrap_or_else(|| panic!("missing audit region end {end}"));
    let mut body = src[a..b].to_owned();
    f(&mut body);
    src.replace_range(a..b, &body);
}

// Replace one whole creation statement based on its stable Win32 control ID.
// This also tolerates the earlier audit passes changing the displayed label.
fn field(src: &mut String, id: &str, replacement: &str) {
    let rows: Vec<&str> = src.lines().filter(|line| line.contains("field(") && line.contains(id)).collect();
    assert_eq!(rows.len(), 1, "audit {id}: expected one field");
    let old = rows[0].trim();
    assert!(old.ends_with(';'), "audit {id}: malformed field statement");
    once(src, old, replacement, id);
}

fn patch_feature_footer(path: &PathBuf) {
    let mut src = fs::read_to_string(path).expect("generated feature settings");
    // F01: the previous 594 + 92 button extended beyond the 620px popup.
    // Always work in actual *client* pixels, measuring the HWND on each resize.
    once(&mut src,
        "feature_button(hwnd,2,\"Close\",594,432,92);",
        "feature_button(hwnd,2,\"Close\",20,432,92); audit_place_dps_footer(hwnd,state.kind);",
        "F01 initial footer");
    let helper = r#"unsafe fn audit_place_dps_footer(hwnd: HWND, kind: Kind) {
    if kind != Kind::Dps { return; }
    let button = fc(hwnd, 2);
    if button.is_null() { return; }
    let mut client: RECT = std::mem::zeroed();
    let mut bounds: RECT = std::mem::zeroed();
    if GetClientRect(hwnd, &mut client) == 0 || GetWindowRect(button, &mut bounds) == 0 { return; }
    let width = (bounds.right - bounds.left).max(1);
    let height = (bounds.bottom - bounds.top).max(1);
    let inset = 20;
    let x = (client.right - width - inset).max(inset);
    let y = (client.bottom - height - inset).max(0);
    windows_sys::Win32::UI::WindowsAndMessaging::MoveWindow(button, x, y, width, height, 1);
}
"#;
    once(&mut src, "unsafe fn settings_click(", &format!("{helper}unsafe fn settings_click("), "F01 footer placement helper");
    region(&mut src, "unsafe extern \"system\" fn settings_wnd_proc(", "unsafe fn paint_feature_settings(", |proc| {
        once(proc, "match msg{", "match msg{WM_SIZE=>{if !ptr.is_null(){audit_place_dps_footer(hwnd,(*ptr).kind);}0},0x02E0=>{let result=DefWindowProcW(hwnd,msg,wparam,lparam);if !ptr.is_null(){audit_place_dps_footer(hwnd,(*ptr).kind);}result},", "F01 resize and monitor DPI");
    });
    src.push_str(r#"
#[cfg(test)] mod audit_footer_geometry_tests {
    #[test] fn close_button_fits_standard_and_reduced_clients() {
        for width in [420, 560, 600, 620, 720, 900] {
            let button_width = 92;
            let x = (width - button_width - 20).max(20);
            assert!(x >= 20 && x + button_width + 20 <= width);
        }
    }
}
"#);
    fs::write(path, src).expect("write F01 footer");
}

fn patch_chat_settings(path: &PathBuf) {
    let mut src = fs::read_to_string(path).expect("generated chat settings");
    // F02: use the same two measured columns and 30px horizontal gutter for
    // username/volume. Labels and edits now share rows without intersecting.
    region(&mut src, "unsafe fn build_speech(", "unsafe fn build_tabs(", |body| {
        field(body, "ID_TTS_USERNAME", "field(hwnd, state, PAGE_SPEECH, \"Own username override (optional)\", ID_TTS_USERNAME, 184, 302, 410);");
        field(body, "ID_TTS_VOLUME", "field(hwnd, state, PAGE_SPEECH, \"TTS volume (%)\", ID_TTS_VOLUME, 624, 302, 140);");
        once(body, "ID_HIDE_RICH, \"Hide emoji-only and linked-item noise\", 184, 424", "ID_HIDE_RICH, \"Hide emoji-only and linked-item noise\", 184, 392", "F02 checkbox clearance");
        once(body, "ID_TEST_TTS, \"Test Google English TTS\", 184, 470", "ID_TEST_TTS, \"Test Google English TTS\", 184, 444", "F02 test button clearance");
        once(body, "184, 522, 610, 54", "184, 492, 610, 68", "F02 speech helper wrapping");
    });
    // F03: consistent 30px column gutter and 14px vertical group spacing.
    region(&mut src, "unsafe fn build_overlay(", "unsafe fn build_colors(", |body| {
        field(body,"ID_CLICK_HOTKEY","field(hwnd,state,PAGE_OVERLAY,\"Recovery hotkey\",ID_CLICK_HOTKEY,184,270,280);");
        field(body,"ID_FONT_FAMILY","field(hwnd,state,PAGE_OVERLAY,\"Font family\",ID_FONT_FAMILY,494,270,290);");
        field(body,"ID_WINDOW_OPACITY","field(hwnd,state,PAGE_OVERLAY,\"Window opacity (%)\",ID_WINDOW_OPACITY,184,342,110);");
        field(body,"ID_FONT_SIZE","field(hwnd,state,PAGE_OVERLAY,\"Font size (pt, 8-24)\",ID_FONT_SIZE,494,342,110);
");
        field(body,"ID_MAX_HISTORY","field(hwnd,state,PAGE_OVERLAY,\"Max history (10-500)\",ID_MAX_HISTORY,184,414,130);");
        once(body,"\"Collapse edge\",484,390,160,22", "\"Collapse edge\",494,414,180,22", "F03 collapse label");
        once(body,"ID_COLLAPSE_SIDE,484,414,180,180", "ID_COLLAPSE_SIDE,494,438,200,180", "F03 collapse control");
        once(body,"184,460,610,58", "184,494,610,76", "F03 helper clearance");
    });
    // F07: editable expressions get extra width; values are not ellipsized.
    // Users can scroll native ES_AUTOHSCROLL edits and Ctrl+A/Ctrl+C complete paths.
    region(&mut src,"unsafe fn build_sounds(","unsafe fn build_network(",|body| {
        field(body,"ID_PRIVATE_SOUND_PATH","field(hwnd,state,PAGE_SOUNDS,\"Private sound path\",ID_PRIVATE_SOUND_PATH,184,132,600);");
        field(body,"ID_RULE1_MATCH","field(hwnd,state,PAGE_SOUNDS,\"Match (OR / AND / regex)\",ID_RULE1_MATCH,204,278,310);");
        field(body,"ID_RULE1_PATH","field(hwnd,state,PAGE_SOUNDS,\"Sound path\",ID_RULE1_PATH,534,278,250);");
        field(body,"ID_RULE2_MATCH","field(hwnd,state,PAGE_SOUNDS,\"Match (OR / AND / regex)\",ID_RULE2_MATCH,204,368,310);");
        field(body,"ID_RULE2_PATH","field(hwnd,state,PAGE_SOUNDS,\"Sound path\",ID_RULE2_PATH,534,368,250);");
        field(body,"ID_CHAT_SOUND_VOLUME","field(hwnd,state,PAGE_SOUNDS,\"Chat sound volume (%)\",ID_CHAT_SOUND_VOLUME,184,190,130);");
        let marker="button(hwnd, state, PAGE_SOUNDS, ID_OPEN_LOGS, \"Open chat logs\", 184, 492, 130, 30);";
        once(body,marker,&format!("{marker}\n    info(hwnd,state,PAGE_SOUNDS,\"Long paths and matches remain fully editable. Use Home/End to scroll, or Ctrl+A then Ctrl+C to inspect the complete value.\",184,536,610,60);"),"F07 full-value instructions");
    });
    // F09: the final channel row, wrapping note and footer have distinct bands.
    region(&mut src,"unsafe fn build_tabs(","unsafe fn build_sounds(",|body| {
        once(body,"184, 500, 610, 58", "184, 514, 610, 74", "F09 tabs helper and footer spacing");
    });
    region(&mut src,"unsafe fn build_general(","unsafe fn build_overlay(",|body| {
        field(body,"ID_ALERT_VOLUME","field(hwnd,state,PAGE_GENERAL,\"Alert volume (%)\",ID_ALERT_VOLUME,184,246,110);");
    });
    // F08: do not advertise an unavailable destructive action.
    region(&mut src,"unsafe fn refresh_blocked_list(","unsafe fn unblock_selected(",|body| {
        once(body,"    }\n}","    }\n    EnableWindow(GetDlgItem(hwnd, ID_CLEAR_BLOCKED), (!state.working.chat.blocked_users.is_empty()) as i32);\n    EnableWindow(GetDlgItem(hwnd, ID_UNBLOCK), (list_sel(hwnd, ID_BLOCKED_LIST) >= 0) as i32);\n}","F08 blocked action state");
    });
    // The owner-drawn disabled control must not retain the destructive border.
    region(&mut src,"unsafe fn draw_settings_button(","unsafe fn try_dark_titlebar(",|body| {
        // Keep the current navigation/Apply styling, but make Clear all clearly
        // destructive only while enabled. Disabled appearance is neutral.
        let begin=body.find("    let bg=").expect("settings button bg");
        let end=begin+body[begin..].find(";\n").expect("settings button bg end")+2;
        body.replace_range(begin..end,"    let disabled = item.itemState & 0x0004 != 0;\n    let bg=if disabled{rgb(29,31,38)}else if id==ID_CLEAR_BLOCKED{rgb(73,42,43)}else if selected{rgb(30,48,51)}else if apply{rgb(38,112,103)}else{rgb(30,38,46)};\n");
        let begin=body.find("    let border=").expect("settings button border");
        let end=begin+body[begin..].find(";\n").expect("settings button border end")+2;
        body.replace_range(begin..end,"    let border=if disabled{rgb(56,59,68)}else if id==ID_CLEAR_BLOCKED{rgb(201,109,107)}else if selected||apply{rgb(66,211,190)}else{rgb(49,61,72)};\n");
        once(body,"if selected||apply{rgb(238,247,246)}else{rgb(211,220,227)}","if disabled{rgb(104,109,122)}else if selected||apply{rgb(238,247,246)}else{rgb(211,220,227)}","F08 disabled text");
    });
    fs::write(path,src).expect("write final settings layout polish");
}

fn patch_discard(path: &PathBuf) {
    let mut src=fs::read_to_string(path).expect("generated Event Tracker editor");
    // F05: standard Yes=Save, No=Discard, Cancel=Keep Editing. Make Cancel
    // the true Windows default (not only a visually highlighted control).
    once(&mut src,
        "wide(\"Unsaved rules\").as_ptr(),0x00000003|0x00000020);",
        "wide(\"Unsaved rules\").as_ptr(),0x00000003|0x00000020|0x00000200);",
        "F05 Cancel as initial default");
    once(&mut src,
        "wide(\"Save changes to Event Tracker before closing?\").as_ptr()",
        "wide(\"Save changes to Event Tracker? Yes = Save; No = Discard; Cancel = Keep Editing. Escape keeps edits.\").as_ptr()",
        "F05 explicit choices");
    fs::write(path,src).expect("write safe discard default");
}

fn main() {
    previous::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_feature_footer(&out.join("feature_overlays_v170_fixed.rs"));
    patch_chat_settings(&out.join("settings_ui_v1160_fixed.rs"));
    patch_discard(&out.join("event_tracker_ui_v1160_fixed.rs"));
    println!("cargo:rerun-if-changed=build/legacy/build_v1367_final_ui_audit.rs");
}
