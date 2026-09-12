use std::{env, fs, path::{Path, PathBuf}};

mod prior {
    include!("build_v1260_encounter_archive.rs");
    pub fn run() { main(); }
}

fn replace_between(source: &mut String, start: &str, end: &str, replacement: &str, label: &str) {
    let start_count = source.matches(start).count();
    assert_eq!(start_count, 1, "v1.27 patch {label} start expected one match, found {start_count}");
    let begin = source.find(start).expect("v1.27 start anchor");
    let rel_end = source[begin..].find(end)
        .unwrap_or_else(|| panic!("v1.27 patch {label} end anchor missing"));
    source.replace_range(begin..begin + rel_end, replacement);
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.27 patch {label} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn patch_overlay(out: &Path) {
    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read v1.26 generated overlay").replace("\r\n", "\n");

    replace_between(
        &mut source,
        "fn toolbar_action_rects(right:i32)->[(RECT,&'static str);7]{",
        "unsafe fn resize_hit_test",
        r#"fn toolbar_action_rects(right:i32)->[(RECT,&'static str);7]{let compact=right<760;let mut x=right-BUTTON_W*3-5;let mut take=|wide:i32,narrow:i32,full:&'static str,short:&'static str|{let w=if compact{narrow}else{wide};let r=RECT{left:x-w,top:5,right:x,bottom:29};x-=w+4;(r,if compact{short}else{full})};let reset=take(52,28,"Reset","R");let benchmark=take(76,28,"Benchmark","B");let copy=take(48,28,"Copy","C");let newer=take(28,24,">",">");let live=take(46,28,"Live","L");let older=take(28,24,"<","<");let raid=take(44,28,"Raid","R");[raid,older,live,newer,copy,benchmark,reset]}
"#,
        "responsive meter actions",
    );

    replace_between(
        &mut source,
        "unsafe fn paint_toolbar(hdc:HDC,rc:RECT,state:&State){",
        "fn toolbar_title(state:&State)->String{",
        r#"unsafe fn paint_toolbar(hdc:HDC,rc:RECT,state:&State){let toolbar=RECT{left:0,top:0,right:rc.right,bottom:TOOLBAR_H};fill(hdc,&toolbar,rgb(20,27,33));fill(hdc,&RECT{left:0,top:TOOLBAR_H-2,right:rc.right,bottom:TOOLBAR_H},rgb(66,211,190));let title_right=if state.kind==Kind::Dps{toolbar_action_rects(rc.right)[0].0.left-5}else{rc.right-BUTTON_W*3-5};SetTextColor(hdc,rgb(235,242,245));let title=if state.kind==Kind::Dps{"Meter".to_string()}else{toolbar_title(state)};draw(hdc,&title,RECT{left:10,top:0,right:title_right.max(54),bottom:TOOLBAR_H},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);if state.kind==Kind::Dps{let raid_active=state.features.read().map(|f|!f.meter.party_only).unwrap_or(true);for(index,(r,label))in toolbar_action_rects(rc.right).iter().enumerate(){let active=(index==0&&raid_active)||(index==2&&state.history_index.is_none());paint_tab(hdc,r.left,r.top,r.right-r.left,label,active);}}SetTextColor(hdc,rgb(170,187,198));draw(hdc,"S",RECT{left:rc.right-BUTTON_W*3,top:0,right:rc.right-BUTTON_W*2,bottom:TOOLBAR_H},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);draw(hdc,"◀",RECT{left:rc.right-BUTTON_W*2,top:0,right:rc.right-BUTTON_W,bottom:TOOLBAR_H},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);draw(hdc,"×",RECT{left:rc.right-BUTTON_W,top:0,right:rc.right,bottom:TOOLBAR_H},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}
"#,
        "compact Meter toolbar",
    );

    replace_between(
        &mut source,
        "unsafe fn on_click(hwnd:HWND,state:&mut State,lparam:LPARAM){",
        "unsafe fn on_wheel",
        r#"unsafe fn on_click(hwnd:HWND,state:&mut State,lparam:LPARAM){crate::ui::SetFocus(hwnd);if state.collapsed{expand_state(hwnd,state);return;}let x=lo_signed(lparam);let y=hi_signed(lparam);let mut rc:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rc);if y<TOOLBAR_H{if x>=rc.right-BUTTON_W{PostMessageW(state.main_hwnd,0x0111,if state.kind==Kind::Dps{CMD_HIDE_DPS as usize}else{CMD_HIDE_MECHANICS as usize},0);}else if x>=rc.right-BUTTON_W*2{collapse(hwnd,state);}else if x>=rc.right-BUTTON_W*3{open_feature_settings(hwnd,state);}else if state.kind==Kind::Dps{for(index,(r,_))in toolbar_action_rects(rc.right).iter().enumerate(){if x>=r.left&&x<r.right&&y>=r.top&&y<r.bottom{match index{0=>set_raid_scope(state),1=>history_older(state),2=>{state.history_index=None;state.scroll=0;},3=>history_newer(state),4=>copy_view(state),5=>crate::benchmark_ui::show(hwnd),6=>{if state.history_index.is_none(){archive_live_snapshot(state);crate::telemetry::request_manual_reset();state.dps=DpsSnapshot::default();state.scroll=0;}else{state.history_index=None;state.scroll=0;}},_=>{}}refresh_view_detail(state);clamp_scroll(hwnd,state);InvalidateRect(hwnd,null(),0);return;}}drag_window(hwnd);}else{drag_window(hwnd);}return;}if state.kind==Kind::Dps&&y<dps_rows_top(){let tabs_top=TOOLBAR_H+35;if y>=tabs_top&&y<tabs_top+23{match x{8..=78=>state.sort_mode=SortMode::Damage,83..=148=>state.sort_mode=SortMode::Heal,153..=218=>state.sort_mode=SortMode::Tank,_=>{}}state.scroll=0;refresh_view_detail(state);clamp_scroll(hwnd,state);InvalidateRect(hwnd,null(),0);}return;}if state.kind==Kind::Dps{if let Some(row)=dps_row_at(hwnd,state,y){open_detail(hwnd,state,row);}}}
fn set_raid_scope(state:&mut State){let snapshot=if let Ok(mut features)=state.features.write(){if !features.meter.party_only{return;}features.meter.party_only=false;Some(features.clone())}else{None};if let Some(settings)=snapshot{if let Err(err)=crate::feature_settings::save(&state.paths,&settings){crate::logging::write(format!("meter: save Raid scope failed: {err}"));}}state.scroll=0;}
"#,
        "benchmark and Raid toolbar routing",
    );

    replace_once(
        &mut source,
        "match index{0=>\"Older encounter\",1=>\"Return to live encounter\",2=>\"Newer encounter\",3=>\"Export selected encounter as CSV\",4=>\"Export selected encounter as JSON\",5=>\"Copy selected encounter summary\",6=>\"Reset the live encounter\",_=>\"Combat action\"}",
        "match index{0=>\"Show raid/all contributors\",1=>\"Older encounter\",2=>\"Return to live encounter\",3=>\"Newer encounter\",4=>\"Copy selected encounter summary\",5=>\"Set up a timed benchmark\",6=>\"Reset the live encounter\",_=>\"Combat action\"}",
        "toolbar help text",
    );

    fs::write(path, source).expect("write v1.27 generated overlay");
}

fn main() {
    prior::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_overlay(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1270_benchmark_history.rs");
    println!("cargo:rerun-if-changed=src/benchmark_ui.rs");
    println!("cargo:rerun-if-changed=src/encounter_archive_v1270.rs");
    println!("cargo:rerun-if-changed=src/encounter_context_v1270.rs");
    println!("cargo:rerun-if-changed=src/telemetry_v1270.rs");
}
