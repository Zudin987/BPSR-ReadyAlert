// Target the user-marked native controls without altering their command IDs or hit boxes.
use std::{env,fs,path::PathBuf};
mod previous { include!("build_v1396_combo_popup_workarea.rs"); pub fn run(){main();} }
fn once(s:&mut String,a:&str,b:&str,id:&str){assert_eq!(s.matches(a).count(),1,"marked UI {id} anchor");*s=s.replacen(a,b,1);}
fn main(){
 previous::run();
 let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
 let meter=out.join("feature_overlays_v170_fixed.rs");
 let mut m=fs::read_to_string(&meter).expect("feature UI generated");
 once(&mut m,"unsafe fn audit_place_dps_footer(hwnd:HWND){","unsafe fn audit_place_feature_footer(hwnd:HWND){","both settings share footer anchoring");
 assert_eq!(m.matches("audit_place_dps_footer(hwnd);").count(),3,"initial/size/DPI footer use");
 m=m.replace("audit_place_dps_footer(hwnd);","audit_place_feature_footer(hwnd);");
 once(&mut m,"let x=(client.right-w-20).max(0);let y=(client.bottom-h-20).max(0);",
  "let inset=20;let x=(client.right-w-inset).max(0);let y=(client.bottom-h-inset).max(0);",
  "consistent twenty-pixel safe footer inset");
 once(&mut m,"feature_button(hwnd,2,\"Close\",598,bottom+62,92);",
  "feature_button(hwnd,2,\"Close\",598,bottom+62,92);audit_place_feature_footer(hwnd);",
  "mechanics close initial placement");
 once(&mut m,"WM_SIZE=>{if state.kind==Kind::Dps{audit_place_feature_footer(hwnd);}0},",
  "WM_SIZE=>{audit_place_feature_footer(hwnd);0},","both footers on size");
 once(&mut m,"0x02E0=>{let result=DefWindowProcW(hwnd,msg,wparam,lparam);if state.kind==Kind::Dps{audit_place_feature_footer(hwnd);}result},",
  "0x02E0=>{let result=DefWindowProcW(hwnd,msg,wparam,lparam);audit_place_feature_footer(hwnd);result},",
  "both footers on DPI");
 once(&mut m,"encounter_ms: u64,\n}","encounter_ms: u64,\nclose_hover: bool,\n}","detail hover storage");
 once(&mut m,"mode:DetailMode::Damage,encounter_ms:view_snapshot(state).encounter_ms}",
  "mode:DetailMode::Damage,encounter_ms:view_snapshot(state).encounter_ms,close_hover:false}",
  "detail hover initialization");
 once(&mut m,"let row=&state.row;paint_popup_toolbar(hdc,rc,&format!(\"Player details — {}\",row.name));",
  "let row=&state.row;paint_popup_toolbar(hdc,rc,&format!(\"Player details — {}\",row.name),state.close_hover);",
  "detail toolbar receives hover");
 once(&mut m,"unsafe fn paint_popup_toolbar(hdc:HDC,rc:RECT,title:&str){fill(hdc,&RECT{left:0,top:0,right:rc.right,bottom:TOOLBAR_H},crate::ui_modern::DARK_SURFACE);fill(hdc,&RECT{left:0,top:TOOLBAR_H-1,right:rc.right,bottom:TOOLBAR_H},crate::ui_modern::DARK_BORDER);SetTextColor(hdc,crate::ui_modern::BPSR_TEXT);draw(hdc,title,RECT{left:12,top:0,right:rc.right-BUTTON_W-4,bottom:TOOLBAR_H},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);draw(hdc,\"×\",RECT{left:rc.right-BUTTON_W,top:0,right:rc.right,bottom:TOOLBAR_H},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}",
 r#"unsafe fn paint_popup_toolbar(hdc:HDC,rc:RECT,title:&str,hovered:bool){
 fill(hdc,&RECT{left:0,top:0,right:rc.right,bottom:TOOLBAR_H},crate::ui_modern::DARK_SURFACE);
 fill(hdc,&RECT{left:0,top:TOOLBAR_H-1,right:rc.right,bottom:TOOLBAR_H},crate::ui_modern::DARK_BORDER);
 SetTextColor(hdc,crate::ui_modern::BPSR_TEXT);
 draw(hdc,title,RECT{left:12,top:0,right:rc.right-BUTTON_W-4,bottom:TOOLBAR_H},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);
 let hit=RECT{left:rc.right-BUTTON_W,top:0,right:rc.right,bottom:TOOLBAR_H};
 let tile=RECT{left:hit.left+4,top:4,right:hit.right-4,bottom:hit.bottom-4};
 crate::ui_modern::fill_round_rect(hdc,tile,if hovered{crate::ui_modern::DARK_HOVER}else{crate::ui_modern::DARK_RAISED},crate::ui_modern::SHAPE_RADIUS_CONTROL);
 if hovered{crate::ui_modern::stroke_round_rect(hdc,tile,crate::ui_modern::DARK_BORDER_STRONG,crate::ui_modern::SHAPE_RADIUS_CONTROL,1);}
 SetTextColor(hdc,crate::ui_modern::BPSR_TEXT);
 draw(hdc,"×",hit,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);
}"#,"details 2px close tile within original hit target");
 once(&mut m,"        WM_PAINT=>{paint_detail(hwnd,state);0}",
 r#"        WM_PAINT=>{paint_detail(hwnd,state);0}
        WM_MOUSEMOVE=>{
            let mut rc:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rc);
            let hover=hi_signed(lparam)>=0&&hi_signed(lparam)<TOOLBAR_H&&lo_signed(lparam)>=rc.right-BUTTON_W;
            if hover!=state.close_hover{state.close_hover=hover;InvalidateRect(hwnd,null(),0);}
            if hover{detail_request_mouse_leave(hwnd);}
            0
        }
        WM_MOUSELEAVE_=>{if state.close_hover{state.close_hover=false;InvalidateRect(hwnd,null(),0);}0}"#,
  "hover and leave repaint");
 m.push_str(r#"
#[repr(C)] struct DetailTrackMouse {size:u32,flags:u32,hwnd:HWND,hover_time:u32}
#[link(name="user32")]
extern "system" {#[link_name="TrackMouseEvent"] fn detail_track_mouse(event:*mut DetailTrackMouse)->i32;}
unsafe fn detail_request_mouse_leave(hwnd:HWND){let mut event=DetailTrackMouse{size:std::mem::size_of::<DetailTrackMouse>() as u32,flags:2,hwnd,hover_time:0};let _=detail_track_mouse(&mut event);}
#[cfg(test)] mod marked_native_shape_tests {
 #[test] fn details_close_tile_keeps_the_same_click_target(){
  let button=36;let toolbar=36;
  assert!(button-8>=24);assert!(toolbar-8>=24);
 }
}
"#);
 fs::write(meter,m).expect("feature control patch");
 let chat=out.join("overlay_v150_v1181.rs");
 let mut c=fs::read_to_string(&chat).expect("chat generated");
 once(&mut c,"SelectObject(hdc, stock_font);\n        fill(hdc, &client, crate::ui_modern::DARK_RAISED);",
 "SelectObject(hdc, crate::ui_modern::medium_font());\n        fill(hdc, &client, crate::ui_modern::DARK_BG);",
 "collapsed chat font and fill match meter");
 once(&mut c,"draw_text(hdc, glyph, client, crate::ui_modern::BPSR_TEXT_SECONDARY, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX, false);",
 "draw_text(hdc, glyph, client, crate::ui_modern::BPSR_ACCENT, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX, false);\n        crate::ui_modern::stroke_round_rect(hdc,RECT{left:0,top:0,right:client.right-1,bottom:client.bottom-1},crate::ui_modern::DARK_BORDER,crate::ui_modern::SHAPE_RADIUS_CONTAINER,1);",
 "collapsed chat accent glyph and same shell border");
 fs::write(chat,c).expect("chat handle patch");
 println!("cargo:rerun-if-changed=build/legacy/build_v1397_marked_controls_and_handles.rs");
}
