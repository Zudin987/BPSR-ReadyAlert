// Chat, DPS Meter and Tracker share one exact physical 8px outer corner.
// The same native region clips the HWND and its input boundary after every
// resize. Only the OUTER frame is affected; buttons/tabs retain their radii.
use std::{env, fs, path::PathBuf};
mod previous { include!("build_v1391_normal_imagines_intrinsic_fit.rs"); pub fn run() { main(); } }
fn exact(src: &mut String, old: &str, new: &str, label: &str) {
    assert_eq!(src.matches(old).count(), 1, "unified frame: {label} expected one anchor");
    *src = src.replacen(old, new, 1);
}
fn section(src: &mut String, start: &str, end: &str, old: &str, new: &str, label: &str) {
    assert_eq!(src.matches(start).count(), 1, "unified frame: {label} ambiguous section");
    let a=src.find(start).unwrap();
    let b=a+src[a..].find(end).expect("unified frame section end");
    let mut part=src[a..b].to_owned();
    exact(&mut part,old,new,label);
    src.replace_range(a..b,&part);
}
fn main() {
    previous::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let feature=out.join("feature_overlays_v170_fixed.rs");
    let mut meter=fs::read_to_string(&feature).expect("generated DPS/tracker");
    section(&mut meter,"unsafe fn create(instance:","struct ConsumablePopupState{",
        "apply_opacity(hwnd,layout.opacity);SetWindowPos(hwnd,HWND_TOPMOST",
        "crate::ui_modern::apply_overlay_frame_region(hwnd);apply_opacity(hwnd,layout.opacity);SetWindowPos(hwnd,HWND_TOPMOST",
        "DPS/tracker initial region after restored window dimensions");
    section(&mut meter,"unsafe extern \"system\" fn wnd_proc(","unsafe fn reference_legacy_on_click(",
        "WM_SIZE=>{if !ptr.is_null(){clamp_scroll(hwnd,&mut*ptr);sync_consumable_popup(hwnd,&mut*ptr);}InvalidateRect(hwnd,null(),0);0},",
        "WM_SIZE=>{crate::ui_modern::apply_overlay_frame_region(hwnd);if !ptr.is_null(){clamp_scroll(hwnd,&mut*ptr);sync_consumable_popup(hwnd,&mut*ptr);}InvalidateRect(hwnd,null(),0);0},",
        "DPS/tracker resize and collapse region");
    section(&mut meter,"unsafe fn paint(hwnd:HWND,state:&mut State)","unsafe fn reference_legacy_paint_toolbar(",
        "paint_hover(hdc,rc,state);}reset_overlay_dc(hdc);",
        "paint_hover(hdc,rc,state);}if rc.right>2&&rc.bottom>2{crate::ui_modern::stroke_round_rect(hdc,RECT{left:0,top:0,right:rc.right-1,bottom:rc.bottom-1},crate::ui_modern::DARK_BORDER,crate::ui_modern::overlay_frame_radius_logical(state.scale_percent),1);}reset_overlay_dc(hdc);",
        "DPS/tracker painted 8px outer outline, including collapsed strip");
    fs::write(&feature,meter).expect("write rounded DPS/tracker");

    let chat=out.join("overlay_v150_v1181.rs");
    let mut source=fs::read_to_string(&chat).expect("generated chat");
    section(&mut source,"pub unsafe fn create(","pub unsafe fn push_chat(",
        "    apply_style(hwnd, &snapshot);\n    sync_hotkey(hwnd, &mut *state_ptr, &snapshot);",
        "    apply_style(hwnd, &snapshot);\n    crate::ui_modern::apply_overlay_frame_region(hwnd);\n    sync_hotkey(hwnd, &mut *state_ptr, &snapshot);",
        "chat initial region after style");
    section(&mut source,"unsafe extern \"system\" fn overlay_wnd_proc(","unsafe fn paint(",
        "WM_SIZE => { InvalidateRect(hwnd, null(), 0); 0 }",
        "WM_SIZE => { crate::ui_modern::apply_overlay_frame_region(hwnd); InvalidateRect(hwnd, null(), 0); 0 }",
        "chat region updates with size");
    exact(&mut source,
        "unsafe fn draw_border(hdc:HDC,client:RECT){let c=crate::ui_modern::DARK_BORDER;fill(hdc,&RECT{left:0,top:0,right:client.right,bottom:1},c);fill(hdc,&RECT{left:0,top:client.bottom-1,right:client.right,bottom:client.bottom},c);fill(hdc,&RECT{left:0,top:0,right:1,bottom:client.bottom},c);fill(hdc,&RECT{left:client.right-1,top:0,right:client.right,bottom:client.bottom},c);}",
        "unsafe fn draw_border(hdc:HDC,client:RECT){if client.right>2&&client.bottom>2{crate::ui_modern::stroke_round_rect(hdc,RECT{left:0,top:0,right:client.right-1,bottom:client.bottom-1},crate::ui_modern::DARK_BORDER,crate::ui_modern::OVERLAY_FRAME_RADIUS_PX,1);}}",
        "chat draws a curved rather than rectangular border");
    fs::write(&chat,source).expect("write rounded chat");
    println!("cargo:rerun-if-changed=build/legacy/build_v1392_unified_overlay_frame.rs");
}
