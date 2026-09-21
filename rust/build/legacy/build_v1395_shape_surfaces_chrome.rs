// Explicit renderer-by-renderer correction: keep the native command IDs,
// layout, hit rectangles, semantics and existing generated-source chain intact.
use std::{env,fs,path::PathBuf};
mod previous{include!("build_v1394_shape_tokens.rs");pub fn run(){main();}}
fn change(s:&mut String,a:&str,b:&str,n:usize,id:&str){assert_eq!(s.matches(a).count(),n,"shape {id}: expected {n} anchors");*s=s.replace(a,b);}
fn write(path:&PathBuf,s:String){fs::write(path,s).expect("write shape-adjusted native code");}
fn main(){
    previous::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let path=out.join("feature_overlays_v170_fixed.rs");
    let mut s=fs::read_to_string(&path).expect("meter/tracker source");
    change(&mut s,"surface, 5)","surface, crate::ui_modern::SHAPE_RADIUS_CONTROL)",1,"meter toolbar selected tab");
    change(&mut s,"rgb(77, 107, 155), 5, 1)","rgb(77, 107, 155), crate::ui_modern::SHAPE_RADIUS_CONTROL, 1)",1,"meter tab outline");
    change(&mut s,"rgb(44, 50, 59), 5)","rgb(44, 50, 59), crate::ui_modern::SHAPE_RADIUS_CONTROL)",1,"meter history control");
    change(&mut s,"reference_meter_row_background(row), 5)","reference_meter_row_background(row), crate::ui_modern::SHAPE_RADIUS_CONTROL)",1,"Raid row");
    change(&mut s,"crate::ui_modern::BPSR_MUTED,3);","crate::ui_modern::BPSR_MUTED,crate::ui_modern::SHAPE_RADIUS_CONTROL);",1,"scrollbar thumb");
    change(&mut s,"crate::ui_modern::DARK_BORDER_STRONG,4,1);","crate::ui_modern::DARK_BORDER_STRONG,crate::ui_modern::SHAPE_RADIUS_CONTROL,1);",1,"manual checkbox border");
    change(&mut s,"crate::ui_modern::DARK_RAISED,4);if !checked","crate::ui_modern::DARK_RAISED,crate::ui_modern::SHAPE_RADIUS_CONTROL);if !checked",1,"manual checkbox face");
    change(&mut s,"crate::ui_modern::fill_round_rect(hdc, r, bg, 4);","crate::ui_modern::fill_round_rect(hdc, r, bg, crate::ui_modern::SHAPE_RADIUS_CONTROL);",1,"secondary toolbar control");
    change(&mut s,"crate::ui_modern::stroke_round_rect(hdc, r, color, 4, 1);","crate::ui_modern::stroke_round_rect(hdc, r, color, crate::ui_modern::SHAPE_RADIUS_CONTROL, 1);",1,"secondary toolbar outline");
    change(&mut s,"crate::telemetry::ui_audit_v1302::dark_titlebar(hwnd);crate::ui_modern::finish_native_window(hwnd);",
        "crate::ui_modern::finish_native_window(hwnd);crate::ui_modern::dark_titlebar(hwnd);",2,"details and DPS/Tracker non-client chrome");
    change(&mut s,"crate::ui_modern::finish_native_window(hwnd);crate::ui_modern::apply_overlay_frame_region(hwnd);ShowWindow(hwnd,SW_SHOW);",
        "crate::ui_modern::finish_native_window(hwnd);crate::ui_modern::dark_titlebar(hwnd);crate::ui_modern::apply_overlay_frame_region(hwnd);ShowWindow(hwnd,SW_SHOW);",1,"feature settings final caption colors");
    write(&path,s);

    let path=out.join("overlay_v150_v1181.rs");let mut s=fs::read_to_string(&path).expect("chat overlay");
    change(&mut s,"const CHAT_HEADER_RADIUS: i32 = 3;","const CHAT_HEADER_RADIUS: i32 = crate::ui_modern::SHAPE_RADIUS_CONTROL;",1,"all chat tabs toolbar and outline");
    change(&mut s,"crate::telemetry::ui_audit_v1302::dark_titlebar(hwnd);crate::ui_modern::finish_native_window(hwnd);",
        "crate::ui_modern::finish_native_window(hwnd);crate::ui_modern::dark_titlebar(hwnd);",2,"chat and child dialog chrome");
    // Old chat test codified a third radius and must now guard the new token.
    change(&mut s,"assert_eq!(CHAT_HEADER_RADIUS, 3)","assert_eq!(CHAT_HEADER_RADIUS, 2)",1,"chat radius regression test");
    write(&path,s);

    let path=out.join("ui_pixel_controls_v1316.rs");let mut s=fs::read_to_string(&path).expect("shared native controls");
    change(&mut s,"if radio { side / 2 } else { 4 }","if radio { side / 2 } else { SHAPE_RADIUS_CONTROL }",3,"checkbox all states keep circles as glyphs");
    change(&mut s,"fill_round_rect(hdc, row, if focused { MIST_SELECTED_HOVER } else { MIST_SELECTED }, 6)",
        "fill_round_rect(hdc, row, if focused { MIST_SELECTED_HOVER } else { MIST_SELECTED }, SHAPE_RADIUS_CONTROL)",1,"list selection");
    write(&path,s);

    let path=out.join("ui_pixel_qa_v1316.rs");let mut s=fs::read_to_string(&path).expect("control state painters");
    change(&mut s,"if nav { RADIUS_MEDIUM } else { RADIUS_SMALL }","RADIUS_SMALL",2,"sidebar uses control radius selected/focused");
    change(&mut s,"thumb, accent, 7","thumb, accent, SHAPE_RADIUS_CONTROL",1,"slider face");
    change(&mut s,"thumb, BPSR_ACCENT_HOVER, 7","thumb, BPSR_ACCENT_HOVER, SHAPE_RADIUS_CONTROL",1,"slider focus");
    // Notification glyph itself is art and remains unchanged; only its container
    // changes where applicable, without replacing circular icon artwork.
    write(&path,s);

    let path=out.join("settings_ui_v1160_fixed.rs");let mut s=fs::read_to_string(&path).expect("main settings");
    change(&mut s,"crate::telemetry::ui_audit_v1302::mist_titlebar(hwnd);crate::ui_modern::finish_native_window(hwnd);",
        "crate::ui_modern::finish_native_window(hwnd);crate::ui_modern::mist_titlebar(hwnd);",1,"main settings active/inactive caption setup");
    change(&mut s,"(*item).hDC,r,color,6)","(*item).hDC,r,color,crate::ui_modern::SHAPE_RADIUS_CONTROL)",1,"swatch fill");
    change(&mut s,"(*item).hDC,r,crate::ui_modern::MIST_BORDER_STRONG,6,1)",
        "(*item).hDC,r,crate::ui_modern::MIST_BORDER_STRONG,crate::ui_modern::SHAPE_RADIUS_CONTROL,1)",1,"swatch outline");
    // Original combo height was too short to show all four Collapse choices.
    change(&mut s,"ID_COLLAPSE_SIDE,591,308,143,120)","ID_COLLAPSE_SIDE,591,308,143,160)",1,"Collapse edge full four-row dropdown");
    write(&path,s);
    let path=out.join("event_tracker_ui_v1160_fixed.rs");let mut s=fs::read_to_string(&path).expect("event tracker");
    change(&mut s,"crate::telemetry::ui_audit_v1302::mist_titlebar(hwnd);crate::ui_modern::finish_native_window(hwnd);",
        "crate::ui_modern::finish_native_window(hwnd);crate::ui_modern::mist_titlebar(hwnd);",1,"event tracker caption");write(&path,s);
    let path=out.join("benchmark_ui_v1302.rs");let mut s=fs::read_to_string(&path).expect("benchmark");
    change(&mut s,"crate::telemetry::ui_audit_v1302::mist_titlebar(hwnd);crate::ui_modern::finish_native_window(hwnd);",
        "crate::ui_modern::finish_native_window(hwnd);crate::ui_modern::mist_titlebar(hwnd);crate::ui_modern::apply_overlay_frame_region(hwnd);",1,"benchmark dark caption and shell");write(&path,s);
    println!("cargo:rerun-if-changed=build/legacy/build_v1395_shape_surfaces_chrome.rs");
}
