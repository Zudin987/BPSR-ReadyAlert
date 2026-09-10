use std::{env, fs, path::{Path, PathBuf}};

mod prior {
    include!("build_v1176.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.17.8 patch {label} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn replace_between(source: &mut String, start: &str, end: &str, replacement: &str, label: &str) {
    let start_count = source.matches(start).count();
    assert_eq!(start_count, 1, "v1.17.8 patch {label} start expected one match, found {start_count}");
    let begin = source.find(start).expect("v1.17.8 start anchor");
    let rel_end = source[begin..].find(end).unwrap_or_else(|| panic!("v1.17.8 patch {label} end anchor missing"));
    source.replace_range(begin..begin + rel_end, replacement);
}

fn patch_overlay(out: &Path) {
    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .expect("read v1.17.7 overlay")
        .replace("\r\n", "\n");

    replace_once(
        &mut source,
        "sharing::{self, ExportFormat, ViewMode},",
        "sharing::{self, ViewMode},",
        "remove CSV/JSON export format from DPS overlay",
    );

    replace_once(
        &mut source,
        "dps_updated_unix_ms: i64,\nmechanics: MechanicSnapshot,",
        "dps_updated_unix_ms: i64,\nimage_render_all: bool,\nmechanics: MechanicSnapshot,",
        "full-image render state",
    );
    replace_once(
        &mut source,
        "features,dps:DpsSnapshot::default(),dps_updated_unix_ms:0,mechanics:MechanicSnapshot::default(),",
        "features,dps:DpsSnapshot::default(),dps_updated_unix_ms:0,image_render_all:false,mechanics:MechanicSnapshot::default(),",
        "full-image render state init",
    );

    replace_once(
        &mut source,
        "fn meter_regular_page_size(state:&State,physical:usize)->usize{if physical==0{return 0;}let configured=state.features.read().map(|f|f.meter.visible_rows).unwrap_or(0);if configured==0{physical}else{configured.min(physical)}}",
        "fn meter_page_size(configured:usize,physical:usize,render_all:bool)->usize{if physical==0{0}else if render_all||configured==0{physical}else{configured.min(physical)}}\nfn meter_regular_page_size(state:&State,physical:usize)->usize{let configured=state.features.read().map(|f|f.meter.visible_rows).unwrap_or(0);meter_page_size(configured,physical,state.image_render_all)}",
        "allow image renderer to bypass visible-row page limit",
    );

    replace_once(
        &mut source,
        "fn toolbar_action_rects(right:i32)->[(RECT,&'static str);7]{let mut x=right-BUTTON_W*3-5;let mut take=|w:i32,label:&'static str|{let r=RECT{left:x-w,top:5,right:x,bottom:29};x-=w+4;(r,label)};let reset=take(52,\"Reset\");let copy=take(48,\"Copy\");let json=take(44,\"JSON\");let csv=take(40,\"CSV\");let newer=take(30,\">\");let live=take(46,\"LIVE\");let older=take(30,\"<\");[older,live,newer,csv,json,copy,reset]}",
        "fn toolbar_action_rects(right:i32)->[(RECT,&'static str);6]{let mut x=right-BUTTON_W*3;let mut take=|w:i32,label:&'static str|{let r=RECT{left:x-w,top:5,right:x,bottom:29};x-=w;(r,label)};let reset=take(52,\"Reset\");let image=take(96,\"Copy as Image\");let text=take(88,\"Copy as Text\");let newer=take(30,\">\");let live=take(46,\"LIVE\");let older=take(30,\"<\");[older,live,newer,text,image,reset]}",
        "DPS sharing toolbar actions and Chat-style spacing",
    );

    replace_between(
        &mut source,
        "fn copy_view(state:&mut State){",
        "fn personal_best_text",
        r#"fn copy_view_text(state:&mut State){
    let mode=share_view_mode(state.sort_mode);
    let snapshot=view_snapshot(state).clone();
    let text=sharing::encounter_summary(&snapshot,mode);
    match sharing::copy_text(&text){
        Ok(())=>{crate::logging::write(format!("sharing: copied {} summary as text",mode.label()));set_share_notice(state,"Copied as text".into());},
        Err(err)=>{crate::logging::write(format!("sharing: text clipboard failed: {err}"));set_share_notice(state,"Copy failed".into());}
    }
}

const CF_BITMAP_:u32=2;
#[link(name="user32")]
extern "system" {
    fn GetDC(hwnd:HWND)->HDC;
    fn ReleaseDC(hwnd:HWND,hdc:HDC)->i32;
    fn OpenClipboard(hwnd:HWND)->i32;
    fn EmptyClipboard()->i32;
    fn SetClipboardData(format:u32,data:*mut c_void)->*mut c_void;
    fn CloseClipboard()->i32;
}

fn dps_image_dimensions(client_width:i32,row_count:usize)->(i32,i32){
    let rows=i32::try_from(row_count.max(1)).unwrap_or(i32::MAX/DPS_ROW_H.max(1));
    let width=client_width.max(620);
    let height=dps_rows_top().saturating_add(rows.saturating_mul(DPS_ROW_H)).saturating_add(4).max(180);
    (width,height)
}

unsafe fn copy_dps_bitmap(hwnd:HWND,state:&mut State)->Result<usize,String>{
    if state.kind!=Kind::Dps{return Err("image copy is only available for DPS Meter".into());}
    let mut client:RECT=std::mem::zeroed();
    if GetClientRect(hwnd,&mut client)==0{return Err(format!("GetClientRect failed: {}",GetLastError()));}
    let row_count=meter_rows(state).len();
    let(width,height)=dps_image_dimensions(client.right-client.left,row_count);
    let window_dc=GetDC(hwnd);
    if window_dc.is_null(){return Err(format!("GetDC failed: {}",GetLastError()));}
    let memory_dc=CreateCompatibleDC(window_dc);
    if memory_dc.is_null(){ReleaseDC(hwnd,window_dc);return Err(format!("CreateCompatibleDC failed: {}",GetLastError()));}
    let bitmap=CreateCompatibleBitmap(window_dc,width,height);
    if bitmap.is_null(){DeleteDC(memory_dc);ReleaseDC(hwnd,window_dc);return Err(format!("CreateCompatibleBitmap failed: {}",GetLastError()));}
    let old=SelectObject(memory_dc,bitmap);
    if old.is_null(){DeleteObject(bitmap);DeleteDC(memory_dc);ReleaseDC(hwnd,window_dc);return Err(format!("SelectObject(bitmap) failed: {}",GetLastError()));}

    let full=RECT{left:0,top:0,right:width,bottom:height};
    fill(memory_dc,&full,rgb(18,22,27));
    SelectObject(memory_dc,GetStockObject(DEFAULT_GUI_FONT));
    SetBkMode(memory_dc,TRANSPARENT as i32);
    let saved_scroll=state.scroll;
    let saved_render_all=state.image_render_all;
    state.scroll=0;
    state.image_render_all=true;
    paint_toolbar(memory_dc,full,state);
    paint_dps(memory_dc,full,state);
    state.image_render_all=saved_render_all;
    state.scroll=saved_scroll;

    SelectObject(memory_dc,old);
    DeleteDC(memory_dc);
    ReleaseDC(hwnd,window_dc);

    if OpenClipboard(hwnd)==0{DeleteObject(bitmap);return Err(format!("OpenClipboard failed: {}",GetLastError()));}
    let transferred=EmptyClipboard()!=0&&!SetClipboardData(CF_BITMAP_,bitmap).is_null();
    let error=GetLastError();
    CloseClipboard();
    if !transferred{DeleteObject(bitmap);return Err(format!("SetClipboardData(CF_BITMAP) failed: {error}"));}
    Ok(row_count)
}

unsafe fn copy_view_image(hwnd:HWND,state:&mut State){
    match copy_dps_bitmap(hwnd,state){
        Ok(rows)=>{crate::logging::write(format!("sharing: copied DPS Meter image with {rows} rows"));set_share_notice(state,format!("Copied image ({rows} rows)"));},
        Err(err)=>{crate::logging::write(format!("sharing: image clipboard failed: {err}"));set_share_notice(state,"Image copy failed".into());}
    }
}

"#,
        "replace CSV/JSON exports with text and full-image clipboard actions",
    );

    replace_between(
        &mut source,
        "unsafe fn on_click(hwnd:HWND,state:&mut State,lparam:LPARAM){",
        "fn wheel_row_steps",
        r#"unsafe fn on_click(hwnd:HWND,state:&mut State,lparam:LPARAM){crate::ui::SetFocus(hwnd);if state.collapsed{expand_state(hwnd,state);return;}let x=lo_signed(lparam);let y=hi_signed(lparam);let mut rc:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rc);if y<TOOLBAR_H{if x>=rc.right-BUTTON_W{PostMessageW(state.main_hwnd,0x0111,if state.kind==Kind::Dps{CMD_HIDE_DPS as usize}else{CMD_HIDE_MECHANICS as usize},0);}else if x>=rc.right-BUTTON_W*2{collapse(hwnd,state);}else if x>=rc.right-BUTTON_W*3{open_feature_settings(hwnd,state);}else if state.kind==Kind::Dps{for(index,(r,_))in toolbar_action_rects(rc.right).iter().enumerate(){if x>=r.left&&x<r.right&&y>=r.top&&y<r.bottom{match index{0=>history_older(state),1=>{state.history_index=None;state.scroll=0;},2=>history_newer(state),3=>copy_view_text(state),4=>copy_view_image(hwnd,state),5=>{if state.history_index.is_none(){archive_live_snapshot(state);crate::telemetry::request_manual_reset();state.dps=DpsSnapshot::default();state.scroll=0;}else{state.history_index=None;state.scroll=0;}},_=>{}}refresh_view_detail(state);clamp_scroll(hwnd,state);InvalidateRect(hwnd,null(),0);return;}}drag_window(hwnd);}else{drag_window(hwnd);}return;}if state.kind==Kind::Dps&&y<dps_rows_top(){let tabs_top=TOOLBAR_H+35;if y>=tabs_top&&y<tabs_top+23{match x{8..=78=>state.sort_mode=SortMode::Damage,83..=148=>state.sort_mode=SortMode::Heal,153..=218=>state.sort_mode=SortMode::Tank,_=>{}}state.scroll=0;refresh_view_detail(state);clamp_scroll(hwnd,state);InvalidateRect(hwnd,null(),0);}return;}if state.kind==Kind::Dps{if let Some(row)=dps_row_at(hwnd,state,y){open_detail(hwnd,state,row);}}}
"#,
        "DPS text/image toolbar action routing",
    );

    replace_between(
        &mut source,
        "unsafe fn paint_toolbar(hdc:HDC,rc:RECT,state:&State){",
        "fn toolbar_title(state:&State)->String{",
        r#"unsafe fn paint_toolbar(hdc:HDC,rc:RECT,state:&State){
    let toolbar=RECT{left:0,top:0,right:rc.right,bottom:TOOLBAR_H};
    fill(hdc,&toolbar,rgb(20,27,33));
    fill(hdc,&RECT{left:0,top:TOOLBAR_H-2,right:rc.right,bottom:TOOLBAR_H},rgb(66,211,190));
    let icon_left=rc.right-BUTTON_W*3;
    let title_right=if state.kind==Kind::Dps{toolbar_action_rects(rc.right)[0].0.left-5}else{icon_left-5};
    SetTextColor(hdc,rgb(235,242,245));
    draw(hdc,&toolbar_title(state),RECT{left:10,top:0,right:title_right.max(80),bottom:TOOLBAR_H},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);
    if state.kind==Kind::Dps{
        for(index,(r,label))in toolbar_action_rects(rc.right).iter().enumerate(){
            let active=index==1&&state.history_index.is_none();
            paint_tab(hdc,r.left,r.top,r.right-r.left,label,active);
        }
    }
    let gear=RECT{left:rc.right-BUTTON_W*3,top:2,right:rc.right-BUTTON_W*2,bottom:TOOLBAR_H-2};
    let collapse=RECT{left:rc.right-BUTTON_W*2,top:2,right:rc.right-BUTTON_W,bottom:TOOLBAR_H-2};
    let hide=RECT{left:rc.right-BUTTON_W,top:2,right:rc.right,bottom:TOOLBAR_H-2};
    fill(hdc,&gear,rgb(26,30,36));fill(hdc,&collapse,rgb(26,30,36));fill(hdc,&hide,rgb(26,30,36));
    SetTextColor(hdc,rgb(239,243,247));
    draw(hdc,"⚙",gear,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);
    draw(hdc,collapse_glyph(state),collapse,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);
    draw(hdc,"×",hide,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);
}
"#,
        "Chat-style settings/collapse/hide toolbar cluster",
    );

    replace_between(
        &mut source,
        "fn toolbar_title(state:&State)->String{",
        "#[derive(Clone,Copy)]struct DpsRowLayout",
        "fn overlay_header(kind:Kind)->&'static str{if kind==Kind::Mechanics{\"Tracker & Mech\"}else{\"DPSMETER\"}}\nfn toolbar_title(state:&State)->String{overlay_header(state.kind).into()}\n",
        "simple DPS and mechanics headers",
    );

    replace_once(
        &mut source,
        "Open ReadyAlert overlay settings (S)",
        "Open overlay settings",
        "settings tooltip",
    );
    replace_once(
        &mut source,
        "match index{0=>\"Older encounter\",1=>\"Return to live encounter\",2=>\"Newer encounter\",3=>\"Export selected encounter as CSV\",4=>\"Export selected encounter as JSON\",5=>\"Copy selected encounter summary\",6=>\"Reset the live encounter\",_=>\"Combat action\"}",
        "match index{0=>\"Older encounter\",1=>\"Return to live encounter\",2=>\"Newer encounter\",3=>\"Copy selected encounter as text\",4=>\"Copy the full DPS Meter as an image\",5=>\"Reset the live encounter\",_=>\"Combat action\"}",
        "DPS toolbar help without CSV/JSON",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1178_toolbar_image_tests {
    use super::*;

    #[test]
    fn headers_are_short_and_exact() {
        assert_eq!(overlay_header(Kind::Dps), "DPSMETER");
        assert_eq!(overlay_header(Kind::Mechanics), "Tracker & Mech");
    }

    #[test]
    fn toolbar_has_text_and_image_copy_but_no_exports() {
        let labels:Vec<_>=toolbar_action_rects(900).iter().map(|(_,label)|*label).collect();
        assert_eq!(labels,vec!["<","LIVE",">","Copy as Text","Copy as Image","Reset"]);
        assert!(!labels.iter().any(|label|matches!(*label,"CSV"|"JSON")));
    }

    #[test]
    fn image_render_mode_bypasses_five_row_page_limit() {
        assert_eq!(meter_page_size(5,20,false),5);
        assert_eq!(meter_page_size(5,20,true),20);
        let(_,five)=dps_image_dimensions(620,5);
        let(_,twenty)=dps_image_dimensions(620,20);
        assert_eq!(twenty-five,15*DPS_ROW_H);
    }
}
"#);

    fs::write(path, source).expect("write v1.17.8 DPS/Mechanics toolbar overlay");
}

fn main() {
    prior::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_overlay(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1178.rs");
}
