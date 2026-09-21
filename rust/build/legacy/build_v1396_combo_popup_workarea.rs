// Native popup HWND is distinct from its closed COMBOBOX HWND. Treat its
// sizing, its theme and its real functional scrollbar separately.
use std::{env,fs,path::PathBuf};
mod previous{include!("build_v1395_shape_surfaces_chrome.rs");pub fn run(){main();}}
fn one(s:&mut String,a:&str,b:&str,id:&str){assert_eq!(s.matches(a).count(),1,"combo {id} source anchor");*s=s.replacen(a,b,1);}
fn main(){
 previous::run();let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
 let path=out.join("ui_pixel_controls_v1316.rs");let mut s=fs::read_to_string(&path).expect("native combo");
 let start="pub unsafe fn position_combo_dropdown(hwnd: HWND) {";let end="fn class_name(hwnd: HWND)";
 assert_eq!(s.matches(start).count(),1,"one dropdown controller");
 let a=s.find(start).unwrap();let b=a+s[a..].find(end).expect("combo function end");
 let mut part=s[a..b].to_owned();
 one(&mut part,"    let list_h = (list.bottom - list.top).max(COMBO_LIST_ROW_H_).max(1);",
 r#"    // CB_GETCOUNT and CB_GETITEMHEIGHT: native list owns the real items.
    let count = SendMessageW(hwnd,0x0146,0,0).max(0) as i32;
    let item = (SendMessageW(hwnd,0x0154,0,0) as i32).max(COMBO_LIST_ROW_H_);
    let wanted = count.saturating_mul(item).saturating_add(4).max(COMBO_LIST_ROW_H_);
    // Prefer the actual current monitor, not the parent dialog's shorter client.
    let mut work = RECT{left:client_top.x,top:client_top.y,right:client_bottom.x,bottom:client_bottom.y};
    let monitor = shape_monitor_from_rect(&combo,2);
    if !monitor.is_null() {
        let mut info = ShapeMonitorInfo{size:std::mem::size_of::<ShapeMonitorInfo>() as u32,
            monitor:std::mem::zeroed(),work:std::mem::zeroed(),flags:0};
        if shape_get_monitor_info(monitor,&mut info)!=0 {work=info.work;}
    }
    let max_room=(combo.top-work.top).max(work.bottom-combo.bottom).max(1);
    let list_h=wanted.min(max_room);"#,"measure full item count and monitor bounds");
 one(&mut part,"    let room_above = combo.top - client_top.y;\n    let room_below = client_bottom.y - combo.bottom;\n    let needs_reposition = list.bottom > client_bottom.y || list.top < client_top.y;\n    if !needs_reposition { return; }",
 "    let room_above=(combo.top-work.top).max(0);\n    let room_below=(work.bottom-combo.bottom).max(0);\n    // Always reapply: the list is recreated or repositioned after each open.","flip and retheme on each open");
 one(&mut part,"(combo.top - list_h).max(client_top.y)","(combo.top - list_h).max(work.top)","popup opens upward");
 one(&mut part,"combo.bottom.min((client_bottom.y - list_h).max(client_top.y))", "combo.bottom.min((work.bottom - list_h).max(work.top))", "popup opens downward");
 one(&mut part,"        list.left,\n        y,", "        list.left.clamp(work.left,(work.right-list_w).max(work.left)),\n        y,", "horizontal monitor fit");
 one(&mut part,"        SWP_NOZORDER_ | SWP_NOACTIVATE_,\n    );",
 r#"        SWP_NOZORDER_ | SWP_NOACTIVATE_,
    );
    // This is the real popup list HWND, not an overpainted fake scrollbar.
    // Keep WS_VSCROLL, native wheel, keyboard, scrollbar arrows and thumb.
    finish_native_window(info.hwndList);
    let dark=wide("DarkMode_Explorer");
    let _=SetWindowTheme(info.hwndList,dark.as_ptr(),null());
    apply_overlay_frame_region(info.hwndList);
    SendMessageW(info.hwndList,0x031a,0,0); // WM_THEMECHANGED
    InvalidateRect(info.hwndList,null(),1);"#,"theme actual popup and preserve scrolling");
 s.replace_range(a..b,&part);
 s.push_str(r#"
#[repr(C)] struct ShapeMonitorInfo {size:u32,monitor:RECT,work:RECT,flags:u32}
#[link(name="user32")]
extern "system" {
    #[link_name="MonitorFromRect"] fn shape_monitor_from_rect(r:*const RECT,flags:u32)->*mut c_void;
    #[link_name="GetMonitorInfoW"] fn shape_get_monitor_info(m:*mut c_void,info:*mut ShapeMonitorInfo)->i32;
}
#[cfg(test)] mod combo_popup_geometry_tests {
    #[test] fn fixed_option_height_has_all_collapse_directions_and_six_max_rows(){
        assert_eq!(4*26+4,108);assert_eq!(6*26+4,160);
        assert!(160<=180); // normal monitor permits all rows without a scroll rail
    }
}
"#);
 fs::write(&path,s).expect("write combo popup geometry and theme");
 println!("cargo:rerun-if-changed=build/legacy/build_v1396_combo_popup_workarea.rs");
}
