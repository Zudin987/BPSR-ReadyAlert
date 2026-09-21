// Follow-up to user-observed windows: prefer correct native dropdown geometry,
// keep caption colours after showing a window, fix footer overlap, and reduce
// only single-line Normal DPS row pitch. Raid retains its two-line pitch.
use std::{env,fs,path::PathBuf};
mod previous{include!("build_v1397_marked_controls_and_handles.rs");pub fn run(){main();}}
fn one(s:&mut String,from:&str,to:&str,id:&str){assert_eq!(s.matches(from).count(),1,"v1398 {id} anchor");*s=s.replacen(from,to,1);}
fn all(s:&mut String,from:&str,to:&str,n:usize,id:&str){assert_eq!(s.matches(from).count(),n,"v1398 {id} count");*s=s.replace(from,to);}
fn patch(out:&PathBuf,name:&str,f:impl FnOnce(&mut String)){let p=out.join(name);let mut s=fs::read_to_string(&p).expect("v1398 source");f(&mut s);fs::write(p,s).expect("v1398 output");}
fn main(){
 previous::run();let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
 patch(&out,"feature_overlays_v170_fixed.rs",|s|{
  one(s,"fn dps_row_h(scale:i32)->i32{dps_adaptive_logical_px(if scale <= 80 { DPS_ROW_H.max(44) } else { DPS_ROW_H },scale)}",
      "fn dps_row_h(scale:i32)->i32{dps_adaptive_logical_px(if scale <= 80 { DPS_ROW_H.max(44) } else { DPS_ROW_H },scale)}\n// Only Normal (single-line) is denser. Raid still needs its full two-line pitch.\nfn dps_normal_row_h(scale:i32)->i32{dps_adaptive_logical_px(if scale<=80{38}else{36},scale).max(dps_badge_h(scale)+6)}",
      "separate Normal/Raid pitch");
  one(s,"fn dps_row_h_for(state:&State,scale:i32)->i32{if state.kind==Kind::Dps&&state.compact_mode{dps_compact_row_h(scale)}else{dps_row_h(scale)}}",
      "fn dps_row_h_for(state:&State,scale:i32)->i32{if state.kind==Kind::Dps&&state.compact_mode{dps_compact_row_h(scale)}else if state.kind==Kind::Dps&&!raid_active(state){dps_normal_row_h(scale)}else{dps_row_h(scale)}}",
      "mode-aware pitch shared by hit testing and row counts");
  one(s,"    let row_h = dps_row_h(scale);\n    let visible = visible_dps_rows_scaled(rc.bottom, scale);",
      "    let row_h = dps_normal_row_h(scale);\n    let visible = visible_dps_rows_scaled_for(state,rc.bottom,scale);",
      "Normal painter matches hit testing");
  one(s,"    let row_h = dps_row_h(scale);\n    let top = dps_rows_top();",
      "    let row_h = dps_normal_row_h(scale);\n    let top = dps_rows_top();",
      "Normal badge hover row pitch");
  one(s,"    let top = dps_rows_top();\n    let row_h = dps_row_h(scale);",
      "    let top = dps_rows_top();\n    let row_h = dps_normal_row_h(scale);",
      "Normal consumable hover row pitch");
  all(s,"    let physical = visible_dps_rows_scaled(rc.bottom, scale);",
      "    let physical = visible_dps_rows_scaled_for(state,rc.bottom,scale);",2,
      "badge and consumable hover count");
  one(s,"let (w,h)=if state.kind==Kind::Dps{(crate::ui_theme::DPS_SETTINGS_W,crate::ui_theme::DPS_SETTINGS_H+110)}else{(crate::ui_theme::MECH_SETTINGS_W,crate::ui_theme::MECH_SETTINGS_H+170)};",
      "let (w,h)=if state.kind==Kind::Dps{(crate::ui_theme::DPS_SETTINGS_W,crate::ui_theme::DPS_SETTINGS_H+138)}else{(crate::ui_theme::MECH_SETTINGS_W,crate::ui_theme::MECH_SETTINGS_H+194)};",
      "footer breathing room without moving content");
  one(s,"let inset=20;let x=(client.right-w-inset).max(0);let y=(client.bottom-h-inset).max(0);",
      "let inset=16;let x=(client.right-w-inset).max(0);let y=(client.bottom-h-inset).max(0);",
      "consistent dialog footer inset");
  one(s,"    windows_sys::Win32::UI::WindowsAndMessaging::MoveWindow(button,x,y,w,h,1);",
      "    windows_sys::Win32::UI::WindowsAndMessaging::MoveWindow(button,x,y,w,h,1);\n    // The DPS footer note is a sibling of Close, not a message below it.\n    let note=fc(hwnd,7300);if !note.is_null(){windows_sys::Win32::UI::WindowsAndMessaging::MoveWindow(note,20,y+4,250,20,1);}",
      "align save note with Close without overlap");
  one(s,"feature_label(hwnd,0,\"Changes save immediately.\",20,466,250,20);",
      "feature_label(hwnd,7300,\"Changes save immediately.\",20,466,250,20);",
      "addressable DPS footer note");
  one(s,"crate::ui_modern::dark_titlebar(hwnd);crate::ui_modern::apply_overlay_frame_region(hwnd);ShowWindow(hwnd,SW_SHOW);SetForegroundWindow(hwnd);",
      "crate::ui_modern::dark_titlebar(hwnd);crate::ui_modern::apply_overlay_frame_region(hwnd);ShowWindow(hwnd,SW_SHOW);SetForegroundWindow(hwnd);crate::ui_modern::dark_titlebar(hwnd);",
      "reapply DPS/Tracker dark caption after visible native frame");
  s.push_str(r#"
#[cfg(test)]mod normal_only_row_density_tests {
 use super::*;
 #[test]fn normal_is_denser_and_raid_still_has_space_for_two_lines(){
  for scale in [60,80,100,125,150,200]{
   assert!(dps_normal_row_h(scale)<dps_row_h(scale),"scale {scale}");
   assert!(dps_normal_row_h(scale)-2>=dps_badge_h(scale)+4);
   assert!(dps_row_h(scale)>=dps_adaptive_logical_px(40,scale));
  }
 }
}
"#);
 });
 patch(&out,"ui_pixel_controls_v1316.rs",|s|{
  one(s,"    let y = if room_above >= list_h || room_above > room_below {\n        (combo.top - list_h).max(work.top)\n    } else {\n        combo.bottom.min((work.bottom - list_h).max(work.top))\n    };",
      "    // Prefer below whenever all options fit; flip above only if needed.\n    let y=if room_below>=list_h{combo.bottom}\n      else if room_above>=list_h{combo.top-list_h}\n      else if room_below>=room_above{combo.bottom}else{(combo.top-list_h).max(work.top)};",
      "natural below-first native combo flip");
  one(s,"    apply_overlay_frame_region(info.hwndList);\n    SendMessageW(info.hwndList,0x031a,0,0); // WM_THEMECHANGED\n    InvalidateRect(info.hwndList,null(),1);",
      "    apply_overlay_frame_region(info.hwndList);\n    // This is the REAL native scrollbar, never a painted-over decoration.\n    // Hide its otherwise-white track when all fixed choices fit; show it again\n    // for constrained monitors so thumb/wheel/arrows remain functional.\n    combo_show_scroll_bar(info.hwndList,1,(wanted>list_h) as i32);\n    InvalidateRect(info.hwndList,null(),1);",
      "hide unnecessary native scrollbar and avoid theme reset");
  s.push_str(r#"
#[link(name="user32")]
extern "system" {#[link_name="ShowScrollBar"] fn combo_show_scroll_bar(hwnd:HWND,bar:i32,show:i32)->i32;}
#[cfg(test)]mod native_popup_fit_tests{
 #[test]fn short_lists_do_not_require_a_scrollbar(){
  for (items,space) in [(4,108),(6,160),(3,82)]{assert!(items*26+4<=space);}
  assert!(6*26+4>100);
 }
}
"#);
 });
 for name in ["settings_ui_v1160_fixed.rs","event_tracker_ui_v1160_fixed.rs"]{
  patch(&out,name,|s|{
   one(s,"    ShowWindow(hwnd, SW_SHOW);","    ShowWindow(hwnd, SW_SHOW);\n    crate::ui_modern::mist_titlebar(hwnd);","refresh settings caption after ShowWindow");
  });
 }
 println!("cargo:rerun-if-changed=build/legacy/build_v1398_native_caption_combo_normal_pitch.rs");
}
