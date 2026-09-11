use std::{env,fs,path::{Path,PathBuf}};

mod prior { include!("build_v1190.rs"); pub fn run(){main();} }

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){let count=source.matches(from).count();assert_eq!(count,1,"build_v1190a patch {label} expected one match, found {count}");*source=source.replacen(from,to,1);}
fn insert_before_once(source:&mut String,anchor:&str,insertion:&str,label:&str){let count=source.matches(anchor).count();assert_eq!(count,1,"build_v1190a patch {label} expected one anchor, found {count}");let at=source.find(anchor).expect("build_v1190a insertion anchor");source.insert_str(at,insertion);}

fn patch_feature_overlay(out:&Path){let path=out.join("feature_overlays_v170_fixed.rs");let mut source=fs::read_to_string(&path).expect("read generated overlays").replace("\r\n","\n");
replace_once(&mut source,r###"unsafe fn on_click(hwnd:HWND,state:&mut State,lparam:LPARAM){
crate::ui::SetFocus(hwnd);if state.collapsed{expand_state(hwnd,state);return;}
let x=physical_to_logical(lo_signed(lparam),state.scale_percent);let y=physical_to_logical(hi_signed(lparam),state.scale_percent);let rc=logical_client_rect(hwnd,state.scale_percent);
if y<TOOLBAR_H{
    if x>=rc.right-BUTTON_W{PostMessageW(state.main_hwnd,0x0111,if state.kind==Kind::Dps{CMD_HIDE_DPS as usize}else{CMD_HIDE_MECHANICS as usize},0);}
    else if x>=rc.right-BUTTON_W*2{collapse(hwnd,state);}
    else if x>=rc.right-BUTTON_W*3{open_feature_settings(hwnd,state);}
    else if state.kind==Kind::Dps{
        for(index,(r,_))in toolbar_action_rects(rc.right).iter().enumerate(){if x>=r.left&&x<r.right&&y>=r.top&&y<r.bottom{match index{0=>history_older(state),1=>{state.history_index=None;state.scroll=0;},2=>history_newer(state),3=>copy_view_image(hwnd,state),4=>{if state.history_index.is_none(){archive_live_snapshot(state);crate::telemetry::request_manual_reset();state.dps=DpsSnapshot::default();state.scroll=0;}else{state.history_index=None;state.scroll=0;}},_=>{}}refresh_view_detail(state);clamp_scroll(hwnd,state);InvalidateRect(hwnd,null(),0);return;}}
        drag_window(hwnd);
    }else{drag_window(hwnd);}
    return;
}
if state.kind==Kind::Dps&&y<dps_rows_top(){let tabs_top=TOOLBAR_H+35;if y>=tabs_top&&y<tabs_top+23{match x{8..=78=>state.sort_mode=SortMode::Damage,83..=148=>state.sort_mode=SortMode::Heal,153..=218=>state.sort_mode=SortMode::Tank,_=>{}}state.scroll=0;refresh_view_detail(state);clamp_scroll(hwnd,state);InvalidateRect(hwnd,null(),0);}return;}
if state.kind==Kind::Dps{if let Some(row)=dps_row_at(hwnd,state,y){open_detail(hwnd,state,row);}}
}
"###,r###"unsafe fn on_click(hwnd:HWND,state:&mut State,lparam:LPARAM){
crate::ui::SetFocus(hwnd);if state.collapsed{expand_state(hwnd,state);return;}let x=physical_to_logical(lo_signed(lparam),state.scale_percent);let y=physical_to_logical(hi_signed(lparam),state.scale_percent);let rc=logical_client_rect(hwnd,state.scale_percent);
if y<TOOLBAR_H{let button=if state.kind==Kind::Dps{dps_toolbar_button_w(rc.right,dps_layout_scale(state))}else{BUTTON_W};if x>=rc.right-button{PostMessageW(state.main_hwnd,0x0111,if state.kind==Kind::Dps{CMD_HIDE_DPS as usize}else{CMD_HIDE_MECHANICS as usize},0);}else if x>=rc.right-button*2{collapse(hwnd,state);}else if x>=rc.right-button*3{open_feature_settings(hwnd,state);}else if state.kind==Kind::Dps{for(index,(r,_))in toolbar_action_rects_responsive(rc.right,dps_layout_scale(state)).iter().enumerate(){if x>=r.left&&x<r.right&&y>=r.top&&y<r.bottom{match index{0=>history_older(state),1=>{state.history_index=None;state.scroll=0;},2=>history_newer(state),3=>copy_view_image(hwnd,state),4=>{if state.history_index.is_none(){archive_live_snapshot(state);crate::telemetry::request_manual_reset();state.dps=DpsSnapshot::default();state.scroll=0;}else{state.history_index=None;state.scroll=0;}},_=>{}}refresh_view_detail(state);clamp_scroll(hwnd,state);InvalidateRect(hwnd,null(),0);return;}}drag_window(hwnd);}else{drag_window(hwnd);}return;}
if state.kind==Kind::Dps&&y<dps_rows_top(){let tabs_top=TOOLBAR_H+35;if y>=tabs_top&&y<tabs_top+23{match x{8..=78=>state.sort_mode=SortMode::Damage,83..=148=>state.sort_mode=SortMode::Heal,153..=218=>state.sort_mode=SortMode::Tank,_=>{}}state.scroll=0;refresh_view_detail(state);clamp_scroll(hwnd,state);InvalidateRect(hwnd,null(),0);}return;}if state.kind==Kind::Dps{if let Some(row)=dps_row_at(hwnd,state,y){open_detail(hwnd,state,row);}}
}
"###,"responsive click layout");
replace_once(&mut source,r###"unsafe fn paint_toolbar(hdc:HDC,rc:RECT,state:&State){
    let toolbar=RECT{left:0,top:0,right:rc.right,bottom:TOOLBAR_H};
    fill(hdc,&toolbar,crate::ui_theme::RAISED);
    fill(hdc,&RECT{left:0,top:TOOLBAR_H-2,right:rc.right,bottom:TOOLBAR_H},crate::ui_theme::ACCENT);
    let icon_left=rc.right-BUTTON_W*3;
    let title_right=if state.kind==Kind::Dps{toolbar_action_rects(rc.right)[0].0.left-5}else{icon_left-5};
    SetTextColor(hdc,crate::ui_theme::TEXT);
    draw(hdc,&toolbar_title(state),RECT{left:10,top:0,right:title_right.max(80),bottom:TOOLBAR_H},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);
    if state.kind==Kind::Dps{
        for(index,(r,label))in toolbar_action_rects(rc.right).iter().enumerate(){
            let active=index==1&&state.history_index.is_none();
            let hovered=state.hover_y<TOOLBAR_H&&state.hover_x>=r.left&&state.hover_x<r.right;
            paint_toolbar_action(hdc,*r,label,active,hovered);
        }
    }
    let gear=RECT{left:rc.right-BUTTON_W*3,top:2,right:rc.right-BUTTON_W*2,bottom:TOOLBAR_H-2};
    let collapse=RECT{left:rc.right-BUTTON_W*2,top:2,right:rc.right-BUTTON_W,bottom:TOOLBAR_H-2};
    let hide=RECT{left:rc.right-BUTTON_W,top:2,right:rc.right,bottom:TOOLBAR_H-2};
    for r in [&gear,&collapse,&hide]{
        let hovered=state.hover_y<TOOLBAR_H&&state.hover_x>=r.left&&state.hover_x<r.right;
        fill(hdc,r,if hovered{crate::ui_theme::SURFACE_HOVER}else{crate::ui_theme::SURFACE});
    }
    SetTextColor(hdc,crate::ui_theme::TEXT);
    draw_toolbar_symbol(hdc,"⚙",gear,COMBAT_ICON_PX);
    draw_toolbar_symbol(hdc,collapse_glyph(state),collapse,COMBAT_ICON_PX);
    draw_toolbar_symbol(hdc,"×",hide,HIDE_ICON_PX);
}
"###,r###"unsafe fn paint_toolbar(hdc:HDC,rc:RECT,state:&State){
    let toolbar=RECT{left:0,top:0,right:rc.right,bottom:TOOLBAR_H};fill(hdc,&toolbar,crate::ui_theme::RAISED);fill(hdc,&RECT{left:0,top:TOOLBAR_H-2,right:rc.right,bottom:TOOLBAR_H},crate::ui_theme::ACCENT);let scale=dps_layout_scale(state);let button=if state.kind==Kind::Dps{dps_toolbar_button_w(rc.right,scale)}else{BUTTON_W};let icon_left=rc.right-button*3;let action_rects=if state.kind==Kind::Dps{Some(toolbar_action_rects_responsive(rc.right,scale))}else{None};let title_right=action_rects.as_ref().map(|items|items[0].0.left-4).unwrap_or(icon_left-5);let old_font=if state.kind==Kind::Dps{SelectObject(hdc,dps_primary_font(state))}else{null_mut()};SetTextColor(hdc,crate::ui_theme::TEXT);draw(hdc,&toolbar_title(state),RECT{left:10,top:0,right:title_right.max(48),bottom:TOOLBAR_H},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);if let Some(items)=action_rects.as_ref(){for(index,(r,label))in items.iter().enumerate(){let active=index==1&&state.history_index.is_none();let hovered=state.hover_y<TOOLBAR_H&&state.hover_x>=r.left&&state.hover_x<r.right;paint_toolbar_action(hdc,*r,label,active,hovered);}}
    let gear=RECT{left:rc.right-button*3,top:2,right:rc.right-button*2,bottom:TOOLBAR_H-2};let collapse=RECT{left:rc.right-button*2,top:2,right:rc.right-button,bottom:TOOLBAR_H-2};let hide=RECT{left:rc.right-button,top:2,right:rc.right,bottom:TOOLBAR_H-2};for r in [&gear,&collapse,&hide]{let hovered=state.hover_y<TOOLBAR_H&&state.hover_x>=r.left&&state.hover_x<r.right;fill(hdc,r,if hovered{crate::ui_theme::SURFACE_HOVER}else{crate::ui_theme::SURFACE});}SetTextColor(hdc,crate::ui_theme::TEXT);let icon_scale=if state.kind==Kind::Dps{dps_adaptive_logical_px(COMBAT_ICON_PX,scale)}else{COMBAT_ICON_PX};let hide_scale=if state.kind==Kind::Dps{dps_adaptive_logical_px(HIDE_ICON_PX,scale)}else{HIDE_ICON_PX};draw_toolbar_symbol(hdc,"⚙",gear,icon_scale);draw_toolbar_symbol(hdc,collapse_glyph(state),collapse,icon_scale);draw_toolbar_symbol(hdc,"×",hide,hide_scale);if !old_font.is_null(){SelectObject(hdc,old_font);}
}
"###,"responsive toolbar");
replace_once(&mut source,r###"unsafe fn overlay_help(hwnd:HWND,state:&State,x:i32,y:i32)->Option<String>{
    if state.collapsed{return Some("Click, Enter or Space to expand".into());}
    let rc=logical_client_rect(hwnd,state.scale_percent);
    if y<TOOLBAR_H{
        if x>=rc.right-BUTTON_W{return Some("Hide overlay • restore from tray or Ctrl+Shift+F10".into());}
        if x>=rc.right-BUTTON_W*2{return Some("Collapse to the configured screen edge".into());}
        if x>=rc.right-BUTTON_W*3{return Some("Open overlay settings".into());}
        if state.kind==Kind::Dps{for(index,(r,_))in toolbar_action_rects(rc.right).iter().enumerate(){if x>=r.left&&x<r.right{return Some(match index{0=>"Older encounter",1=>"Return to live encounter",2=>"Newer encounter",3=>"Copy the full DPS Meter as an image",4=>"Reset the live encounter",_=>"Combat action"}.into());}}}
        return Some(toolbar_title(state));
    }
    if state.kind==Kind::Dps {
        if hover_badge_at(hwnd,state,x,y).is_some(){return None;}
        let target_top=TOOLBAR_H+4;if y>=target_top&&y<target_top+27{return Some("Target context • name/ID, live HP, Enrage / Power Seal timer and encounter time".into());}
        let tabs_top=TOOLBAR_H+35;if y>=tabs_top&&y<tabs_top+23{return Some("Damage / Heal / Tank • keyboard 1 / 2 / 3".into());}
        if let Some(row)=dps_row_at(hwnd,state,y){return Some(format!("{} • Total / Active / Share • Click to inspect",dps_identity(&row)));}
    } else {
        let (_,_,top)=mechanics_scroll_metrics(state,rc.bottom);if y>=top{let index=state.scroll+((y-top)/MECH_ROW_H)as usize;let tracker=crate::event_tracker::rows();if let Some(row)=tracker.get(index){return Some(format!("{} • {}",row.label,row.detail));}let now=now_ms();if let Some(row)=state.mechanics.rows.iter().filter(|r|r.persistent||r.expires_unix_ms<=0||r.expires_unix_ms>now).nth(index.saturating_sub(tracker.len())){return Some(format!("{} {}",row.label,row.target.as_deref().unwrap_or("")));}}
    }
    None
}
"###,r###"unsafe fn overlay_help(hwnd:HWND,state:&State,x:i32,y:i32)->Option<String>{
    if state.collapsed{return Some("Click, Enter or Space to expand".into());}let rc=logical_client_rect(hwnd,state.scale_percent);if y<TOOLBAR_H{let button=if state.kind==Kind::Dps{dps_toolbar_button_w(rc.right,dps_layout_scale(state))}else{BUTTON_W};if x>=rc.right-button{return Some("Hide overlay • restore from tray or Ctrl+Shift+F10".into());}if x>=rc.right-button*2{return Some("Collapse to the configured screen edge".into());}if x>=rc.right-button*3{return Some("Open overlay settings".into());}if state.kind==Kind::Dps{for(index,(r,_))in toolbar_action_rects_responsive(rc.right,dps_layout_scale(state)).iter().enumerate(){if x>=r.left&&x<r.right{return Some(match index{0=>"Older encounter",1=>"Return to live encounter",2=>"Newer encounter",3=>"Copy the full DPS Meter as an image",4=>"Reset the live encounter",_=>"Combat action"}.into());}}}return Some(toolbar_title(state));}
    if state.kind==Kind::Dps{if hover_badge_at(hwnd,state,x,y).is_some(){return None;}let target_top=TOOLBAR_H+4;if y>=target_top&&y<target_top+27{return Some("Target context • name/ID, live HP, Enrage / Power Seal timer and encounter time".into());}let tabs_top=TOOLBAR_H+35;if y>=tabs_top&&y<tabs_top+23{return Some("Damage / Heal / Tank • keyboard 1 / 2 / 3".into());}if let Some(row)=dps_row_at(hwnd,state,y){return Some(format!("{} • Total / Active / Share • Click to inspect",dps_identity(&row)));}}else{let(_,_,top)=mechanics_scroll_metrics(state,rc.bottom);if y>=top{let index=state.scroll+((y-top)/MECH_ROW_H)as usize;let tracker=crate::event_tracker::rows();if let Some(row)=tracker.get(index){return Some(format!("{} • {}",row.label,row.detail));}let now=now_ms();if let Some(row)=state.mechanics.rows.iter().filter(|r|r.persistent||r.expires_unix_ms<=0||r.expires_unix_ms>now).nth(index.saturating_sub(tracker.len())){return Some(format!("{} {}",row.label,row.target.as_deref().unwrap_or("")));}}}None
}
"###,"responsive overlay help");
fs::write(path,source).expect("write patched overlays");}

fn main(){prior::run();let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));patch_feature_overlay(&out);println!("cargo:rerun-if-changed=build/legacy/build_v1190a.rs");}
