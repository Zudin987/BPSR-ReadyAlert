// Keep the previous checked-in generator intact; emit a shape-aware Pixel core
// and legacy modal into OUT_DIR so all consumers share the same shape contract.
use std::{env,fs,path::PathBuf};
mod previous { include!("build_v1393_six_pixel_windows_and_handles.rs"); pub fn run(){main();} }
fn once(s:&mut String,from:&str,to:&str,id:&str){assert_eq!(s.matches(from).count(),1,"shape core {id} anchor");*s=s.replacen(from,to,1);}
fn main(){
 previous::run();
 let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
 let base=PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest"));
 // Windows Git checkout can use CRLF even when the GitHub blob uses LF.
 let mut src=fs::read_to_string(base.join("src/ui_pixel_core.rs")).expect("Pixel core").replace("\r\n","\n");
 once(&mut src,"pub const RADIUS_SMALL: i32 = 8;\npub const RADIUS_MEDIUM: i32 = 12;\npub const RADIUS_LARGE: i32 = 16;\npub const RADIUS_DIALOG: i32 = 20;",
 "pub const SHAPE_RADIUS_CONTROL: i32 = 2;\npub const SHAPE_RADIUS_CONTAINER: i32 = 4;\npub const RADIUS_SMALL: i32 = SHAPE_RADIUS_CONTROL;\npub const RADIUS_MEDIUM: i32 = SHAPE_RADIUS_CONTAINER;\npub const RADIUS_LARGE: i32 = SHAPE_RADIUS_CONTAINER;\npub const RADIUS_DIALOG: i32 = SHAPE_RADIUS_CONTAINER;","two fixed shape tokens");
 assert_eq!(src.matches("let diameter = radius.max(1) * 2;").count(),2,"fill and stroke radii");
 src=src.replace("let diameter = radius.max(1) * 2;","let diameter = (shape_mapped_radius(hdc,radius)*2).min(rect.right-rect.left).min(rect.bottom-rect.top).max(1);");
 // This preference is not an arbitrary corner radius; the HWND region below
 // owns exact physical clipping. Avoid layering the OS's 8px rounding on top.
 if src.contains("let rounded: i32 = 2; // DWMWCP_ROUND") {
   once(&mut src,"let rounded: i32 = 2; // DWMWCP_ROUND","let rounded: i32 = 1; // DWMWCP_DONOTROUND; HWND region owns corners","OS corner preference");
 }
 src.push_str(r#"
#[repr(C)] struct ShapeExtent { cx:i32,cy:i32 }
#[link(name="gdi32")]
extern "system" {
 #[link_name="GetDeviceCaps"] fn shape_get_device_caps(hdc:HDC,index:i32)->i32;
 #[link_name="GetMapMode"] fn shape_get_map_mode(hdc:HDC)->i32;
 #[link_name="GetWindowExtEx"] fn shape_get_window_ext(hdc:HDC,size:*mut ShapeExtent)->i32;
 #[link_name="GetViewportExtEx"] fn shape_get_viewport_ext(hdc:HDC,size:*mut ShapeExtent)->i32;
}
fn shape_mapped_radius(hdc:HDC,token:i32)->i32 { unsafe {
 let dpi=shape_get_device_caps(hdc,88).max(96);
 let physical=((token.max(1)*dpi+48)/96).max(1);
 if shape_get_map_mode(hdc)==8 {
  let mut window=ShapeExtent{cx:0,cy:0};let mut viewport=ShapeExtent{cx:0,cy:0};
  if shape_get_window_ext(hdc,&mut window)!=0 && shape_get_viewport_ext(hdc,&mut viewport)!=0 && viewport.cx!=0 {
   return ((physical*window.cx.abs()+viewport.cx.abs()/2)/viewport.cx.abs()).max(1);
  }
 }
 physical
} }
#[cfg(test)] mod two_shape_token_tests {
 use super::*;
 #[test] fn controls_and_containers_are_the_only_tokens(){
  assert_eq!((SHAPE_RADIUS_CONTROL,SHAPE_RADIUS_CONTAINER),(2,4));
  assert_eq!((RADIUS_SMALL,RADIUS_MEDIUM,RADIUS_LARGE,RADIUS_DIALOG),(2,4,4,4));
 }
}
"#);
 fs::write(out.join("ui_pixel_core_shape_v1394.rs"),src).expect("shape core output");
 let mut dialog=fs::read_to_string(base.join("src/ui_pixel_dialog.rs")).expect("pixel dialog").replace("\r\n","\n");
 once(&mut dialog,"(8, 12, 16, 20)","(2, 4, 4, 4)","dialog test");
 once(&mut dialog,"(*ptr).kind_color, 5)","(*ptr).kind_color, RADIUS_SMALL)","modal icon container");
 once(&mut dialog,"crate::ui::fit_window(hwnd, owner, true);",
  "crate::ui::fit_window(hwnd, owner, true);\n    apply_overlay_frame_region(hwnd);\n    mist_titlebar(hwnd);","modal window region and caption");
 fs::write(out.join("ui_pixel_dialog_shape_v1394.rs"),dialog).expect("shape modal");
 println!("cargo:rerun-if-changed=build/legacy/build_v1394_shape_tokens.rs");
 println!("cargo:rerun-if-changed=src/ui_pixel_core.rs");
 println!("cargo:rerun-if-changed=src/ui_pixel_dialog.rs");
}
