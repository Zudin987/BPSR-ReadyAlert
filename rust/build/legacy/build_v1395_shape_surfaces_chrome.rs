// Explicit renderer-by-renderer transformation. Preserve behavior and original
// controls; abort generation if a changed renderer invalidates an anchor.
use std::{env,fs,path::PathBuf};
mod previous{include!("build_v1394_shape_tokens.rs");pub fn run(){main();}}
fn change(s:&mut String,a:&str,b:&str,n:usize,id:&str){assert_eq!(s.matches(a).count(),n,"shape {id}: expected {n} anchors");*s=s.replace(a,b);}
fn fix(out:&PathBuf,file:&str,mut f:impl FnMut(&mut String)){
    let path=out.join(file);let mut s=fs::read_to_string(&path).expect("generated surface");f(&mut s);fs::write(path,s).expect("write surface");
}
fn main(){
 previous::run();let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
 fix(&out,"feature_overlays_v170_fixed.rs",|s|{
    change(s,"surface, 5)","surface, crate::ui_modern::SHAPE_RADIUS_CONTROL)",1,"meter selected tab");
    change(s,"rgb(77, 107, 155), 5, 1)","rgb(77, 107, 155), crate::ui_modern::SHAPE_RADIUS_CONTROL, 1)",1,"meter tab outline");
    change(s,"rgb(44, 50, 59), 5)","rgb(44, 50, 59), crate::ui_modern::SHAPE_RADIUS_CONTROL)",1,"history control");
    change(s,"reference_meter_row_background(row), 5)","reference_meter_row_background(row), crate::ui_modern::SHAPE_RADIUS_CONTROL)",1,"Raid row");
    change(s,"crate::ui_modern::BPSR_MUTED,3);","crate::ui_modern::BPSR_MUTED,crate::ui_modern::SHAPE_RADIUS_CONTROL);",1,"scroll thumb");
    change(s,"else{crate::ui_modern::DARK_RAISED},4);","else{crate::ui_modern::DARK_RAISED},crate::ui_modern::SHAPE_RADIUS_CONTROL);",1,"manual checkbox face");
    change(s,"crate::ui_modern::DARK_BORDER_STRONG,4,1);","crate::ui_modern::DARK_BORDER_STRONG,crate::ui_modern::SHAPE_RADIUS_CONTROL,1);",1,"manual checkbox border");
    change(s,"crate::ui_modern::fill_round_rect(hdc, r, bg, 4);","crate::ui_modern::fill_round_rect(hdc, r, bg, crate::ui_modern::SHAPE_RADIUS_CONTROL);",1,"control face");
    change(s,"crate::ui_modern::stroke_round_rect(hdc, r, color, 4, 1);","crate::ui_modern::stroke_round_rect(hdc, r, color, crate::ui_modern::SHAPE_RADIUS_CONTROL, 1);",1,"control border");
    change(s,"crate::telemetry::ui_audit_v1302::dark_titlebar(hwnd);crate::ui_modern::finish_native_window(hwnd);",
        "crate::ui_modern::finish_native_window(hwnd);crate::ui_modern::dark_titlebar(hwnd);",2,"DPS/tracker settings and details chrome");
 });
 fix(&out,"overlay_v150_v1181.rs",|s|{
    change(s,"const CHAT_HEADER_RADIUS: i32 = 3;","const CHAT_HEADER_RADIUS: i32 = crate::ui_modern::SHAPE_RADIUS_CONTROL;",1,"chat controls/tabs");
    change(s,"assert_eq!(CHAT_HEADER_RADIUS,3)","assert_eq!(CHAT_HEADER_RADIUS,2)",1,"chat radius test");
    change(s,"crate::telemetry::ui_audit_v1302::dark_titlebar(hwnd);crate::ui_modern::finish_native_window(hwnd);",
        "crate::ui_modern::finish_native_window(hwnd);crate::ui_modern::dark_titlebar(hwnd);",2,"chat and auxiliary caption");
 });
 fix(&out,"ui_pixel_controls_v1316.rs",|s|{
    change(s,"if radio { side / 2 } else { 4 }","if radio { side / 2 } else { SHAPE_RADIUS_CONTROL }",3,"checkbox normal checked focus");
    change(s,"fill_round_rect(hdc, row, if focused { MIST_SELECTED_HOVER } else { MIST_SELECTED }, 6)",
        "fill_round_rect(hdc, row, if focused { MIST_SELECTED_HOVER } else { MIST_SELECTED }, SHAPE_RADIUS_CONTROL)",1,"list row selection");
 });
 fix(&out,"ui_pixel_qa_v1316.rs",|s|{
    change(s,"if nav { RADIUS_MEDIUM } else { RADIUS_SMALL }","RADIUS_SMALL",2,"sidebar hover/focus");
    change(s,"thumb, accent, 7","thumb, accent, SHAPE_RADIUS_CONTROL",1,"slider fill");
    change(s,"thumb, BPSR_ACCENT_HOVER, 7","thumb, BPSR_ACCENT_HOVER, SHAPE_RADIUS_CONTROL",1,"slider focus");
 });
 fix(&out,"settings_ui_v1160_fixed.rs",|s|{
    change(s,"crate::telemetry::ui_audit_v1302::mist_titlebar(hwnd);crate::ui_modern::finish_native_window(hwnd);",
        "crate::ui_modern::finish_native_window(hwnd);crate::ui_modern::mist_titlebar(hwnd);",1,"main settings caption after theming");
    change(s,"(*item).hDC,r,color,6)","(*item).hDC,r,color,crate::ui_modern::SHAPE_RADIUS_CONTROL)",1,"swatch face");
    change(s,"(*item).hDC,r,crate::ui_modern::MIST_BORDER_STRONG,6,1)",
        "(*item).hDC,r,crate::ui_modern::MIST_BORDER_STRONG,crate::ui_modern::SHAPE_RADIUS_CONTROL,1)",1,"swatch border");
    change(s,"ID_COLLAPSE_SIDE,591,308,143,120)","ID_COLLAPSE_SIDE,591,308,143,160)",1,"four Collapse choices");
 });
 for file in ["event_tracker_ui_v1160_fixed.rs","benchmark_ui_v1302.rs"] {
    fix(&out,file,|s|{
        change(s,"crate::telemetry::ui_audit_v1302::mist_titlebar(hwnd);crate::ui_modern::finish_native_window(hwnd);",
            "crate::ui_modern::finish_native_window(hwnd);crate::ui_modern::mist_titlebar(hwnd);",1,"event/benchmark nonclient chrome");
    });
 }
 println!("cargo:rerun-if-changed=build/legacy/build_v1395_shape_surfaces_chrome.rs");
}
