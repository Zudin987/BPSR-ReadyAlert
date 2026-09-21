// Follow-up: every overlay, collapsed handle and native settings window uses
// the shared 6px outer HWND region. Reuse the existing Win32 drawing and
// preserve inner controls, settings geometry, clicks, persistence and DPI.
use std::{env, fs, path::PathBuf};
mod previous { include!("build_v1392_unified_overlay_frame.rs"); pub fn run() { main(); } }
fn exactly_once(text: &mut String, old: &str, new: &str, label: &str) {
    assert_eq!(text.matches(old).count(), 1, "six-pixel windows: {label} expected one anchor");
    *text = text.replacen(old, new, 1);
}
fn in_section(text: &mut String, start: &str, end: &str, old: &str, new: &str, label: &str) {
    assert_eq!(text.matches(start).count(), 1, "six-pixel windows: {label} ambiguous start");
    let a = text.find(start).unwrap();
    let b = a + text[a..].find(end).expect("six-pixel windows: missing end");
    let mut part = text[a..b].to_owned();
    exactly_once(&mut part, old, new, label);
    text.replace_range(a..b, &part);
}
fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));

    // The meter and tracker already paint identical collapsed text and a
    // rounded frame from v1392. Their radius is now the shared 6px constant.
    let meter_path = out.join("feature_overlays_v170_fixed.rs");
    let mut meter = fs::read_to_string(&meter_path).expect("native DPS/tracker");
    in_section(&mut meter, "unsafe fn open_feature_settings(", "unsafe fn feature_control(",
        "crate::ui_modern::finish_native_window(hwnd);ShowWindow(hwnd,SW_SHOW);",
        "crate::ui_modern::finish_native_window(hwnd);crate::ui_modern::apply_overlay_frame_region(hwnd);ShowWindow(hwnd,SW_SHOW);",
        "DPS/tracker settings initial 6px shape");
    in_section(&mut meter, "unsafe extern \"system\" fn settings_wnd_proc(", "unsafe fn ",
        "WM_SIZE=>{if state.kind==Kind::Dps{audit_place_dps_footer(hwnd);}0},",
        "WM_SIZE=>{crate::ui_modern::apply_overlay_frame_region(hwnd);if state.kind==Kind::Dps{audit_place_dps_footer(hwnd);}0},",
        "DPS/tracker settings size and DPI updates");
    fs::write(&meter_path, meter).expect("write native feature settings");

    let chat_path = out.join("overlay_v150_v1181.rs");
    let mut chat = fs::read_to_string(&chat_path).expect("native chat");
    // Do not force a faded collapsed state: all three handles obey their
    // respective user opacity, and show the same accent, fill and 6px border.
    in_section(&mut chat, "pub unsafe fn apply_style(", "pub unsafe fn expand_if_collapsed(",
        "    let collapsed = state_mut(hwnd).map(|s| s.collapsed).unwrap_or(false);\n    let percent = if collapsed { settings.chat.window_opacity.clamp(25, 58) } else { settings.chat.window_opacity.clamp(25, 100) };",
        "    let percent = settings.chat.window_opacity.clamp(25, 100);",
        "collapsed chat obeys normal opacity rather than forced 58 percent");
    in_section(&mut chat, "unsafe fn paint(hwnd: HWND, state: &mut OverlayState) {", "unsafe fn draw_border(",
        "        fill(hdc, &client, crate::ui_modern::DARK_RAISED);",
        "        fill(hdc, &client, crate::ui_modern::DARK_BG);\n        draw_border(hdc, client);",
        "collapsed chat shares the meter background and visible outline");
    in_section(&mut chat, "unsafe fn paint(hwnd: HWND, state: &mut OverlayState) {", "unsafe fn draw_border(",
        "draw_text(hdc, glyph, client, crate::ui_modern::BPSR_TEXT_SECONDARY, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX, false);",
        "draw_text(hdc, glyph, client, crate::ui_modern::BPSR_ACCENT, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX, false);",
        "collapsed chat arrow uses meter/tracker accent");
    fs::write(&chat_path, chat).expect("write unified chat collapsed handle");

    // General settings and Event Tracker settings have native title bars.
    // Clip their *whole* HWND (including the caption), not just the client.
    let general_path = out.join("settings_ui_v1160_fixed.rs");
    let mut general = fs::read_to_string(&general_path).expect("general settings");
    in_section(&mut general, "pub unsafe fn show(", "unsafe extern \"system\" fn settings_wnd_proc(",
        "crate::ui_modern::finish_native_window(hwnd);",
        "crate::ui_modern::finish_native_window(hwnd);crate::ui_modern::apply_overlay_frame_region(hwnd);",
        "general settings initial region after positioning");
    in_section(&mut general, "unsafe extern \"system\" fn settings_wnd_proc(", "unsafe fn build_ui(",
        "        WM_SIZE => 0,",
        "        WM_SIZE => { crate::ui_modern::apply_overlay_frame_region(hwnd); 0 },",
        "general settings region updates when resized");
    fs::write(&general_path, general).expect("write general settings");

    let tracker_path = out.join("event_tracker_ui_v1160_fixed.rs");
    let mut tracker = fs::read_to_string(&tracker_path).expect("event tracker settings");
    in_section(&mut tracker, "pub unsafe fn show(", "unsafe extern \"system\" fn wnd_proc(",
        "crate::ui_modern::finish_native_window(hwnd);",
        "crate::ui_modern::finish_native_window(hwnd);crate::ui_modern::apply_overlay_frame_region(hwnd);",
        "event tracker settings initial region");
    in_section(&mut tracker, "unsafe extern \"system\" fn wnd_proc(", "unsafe fn ",
        "        0x0005 => 0,",
        "        0x0005 => { crate::ui_modern::apply_overlay_frame_region(hwnd); 0 },",
        "event tracker settings resize region");
    fs::write(&tracker_path, tracker).expect("write event tracker settings");
    println!("cargo:rerun-if-changed=build/legacy/build_v1393_six_pixel_windows_and_handles.rs");
}
