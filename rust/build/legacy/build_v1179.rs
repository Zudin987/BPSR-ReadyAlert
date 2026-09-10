use std::{env, fs, path::{Path, PathBuf}};

mod prior {
    include!("build_v1178.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.17.9 patch {label} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn replace_between(source: &mut String, start: &str, end: &str, replacement: &str, label: &str) {
    let count = source.matches(start).count();
    assert_eq!(count, 1, "v1.17.9 patch {label} start expected one match, found {count}");
    let begin = source.find(start).expect("v1.17.9 start anchor");
    let rel_end = source[begin..].find(end).unwrap_or_else(|| panic!("v1.17.9 patch {label} end anchor missing"));
    source.replace_range(begin..begin + rel_end, replacement);
}

fn patch_overlay(out: &Path) {
    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .expect("read v1.17.8 overlay")
        .replace("\r\n", "\n");

    // v1.17.8's combat controls were 52px each. Match the Chat Overlay's
    // compact proportions, while giving Hide a slightly larger target.
    replace_once(
        &mut source,
        "const BUTTON_W: i32 = 52;",
        "const BUTTON_W: i32 = 52;\nconst COMBAT_GEAR_W:i32=40;\nconst COMBAT_COLLAPSE_W:i32=38;\nconst COMBAT_HIDE_W:i32=42;",
        "combat control widths",
    );

    replace_once(
        &mut source,
        "fn toolbar_action_rects(right:i32)->[(RECT,&'static str);6]{let mut x=right-BUTTON_W*3;let mut take=|w:i32,label:&'static str|{let r=RECT{left:x-w,top:5,right:x,bottom:29};x-=w;(r,label)};let reset=take(52,\"Reset\");let image=take(96,\"Copy as Image\");let text=take(88,\"Copy as Text\");let newer=take(30,\">\");let live=take(46,\"LIVE\");let older=take(30,\"<\");[older,live,newer,text,image,reset]}",
        "fn combat_controls_left(right:i32)->i32{right-COMBAT_GEAR_W-COMBAT_COLLAPSE_W-COMBAT_HIDE_W}\nfn toolbar_action_rects(right:i32)->[(RECT,&'static str);5]{let mut x=combat_controls_left(right)-6;let mut take=|w:i32,gap:i32,label:&'static str|{let r=RECT{left:x-w,top:5,right:x,bottom:29};x-=w+gap;(r,label)};let reset=take(52,6,\"Reset\");let image=take(96,12,\"Copy as Image\");let newer=take(30,3,\">\");let live=take(46,3,\"LIVE\");let older=take(30,0,\"<\");[older,live,newer,image,reset]}",
        "separate DPS toolbar groups",
    );

    replace_between(
        &mut source,
        "fn copy_view_text(state:&mut State){",
        "const CF_BITMAP_",
        "",
        "remove Copy as Text implementation",
    );

    replace_between(
        &mut source,
        "unsafe fn on_click(hwnd:HWND,state:&mut State,lparam:LPARAM){",
        "fn wheel_row_steps",
        r#"unsafe fn on_click(hwnd:HWND,state:&mut State,lparam:LPARAM){crate::ui::SetFocus(hwnd);if state.collapsed{expand_state(hwnd,state);return;}let x=lo_signed(lparam);let y=hi_signed(lparam);let mut rc:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rc);if y<TOOLBAR_H{let controls_left=combat_controls_left(rc.right);if x>=rc.right-COMBAT_HIDE_W{PostMessageW(state.main_hwnd,0x0111,if state.kind==Kind::Dps{CMD_HIDE_DPS as usize}else{CMD_HIDE_MECHANICS as usize},0);}else if x>=rc.right-COMBAT_HIDE_W-COMBAT_COLLAPSE_W{collapse(hwnd,state);}else if x>=controls_left{open_feature_settings(hwnd,state);}else if state.kind==Kind::Dps{for(index,(r,_))in toolbar_action_rects(rc.right).iter().enumerate(){if x>=r.left&&x<r.right&&y>=r.top&&y<r.bottom{match index{0=>history_older(state),1=>{state.history_index=None;state.scroll=0;},2=>history_newer(state),3=>copy_view_image(hwnd,state),4=>{if state.history_index.is_none(){archive_live_snapshot(state);crate::telemetry::request_manual_reset();state.dps=DpsSnapshot::default();state.scroll=0;}else{state.history_index=None;state.scroll=0;}},_=>{}}refresh_view_detail(state);clamp_scroll(hwnd,state);InvalidateRect(hwnd,null(),0);return;}}drag_window(hwnd);}else{drag_window(hwnd);}return;}if state.kind==Kind::Dps&&y<dps_rows_top(){let tabs_top=TOOLBAR_H+35;if y>=tabs_top&&y<tabs_top+23{match x{8..=78=>state.sort_mode=SortMode::Damage,83..=148=>state.sort_mode=SortMode::Heal,153..=218=>state.sort_mode=SortMode::Tank,_=>{}}state.scroll=0;refresh_view_detail(state);clamp_scroll(hwnd,state);InvalidateRect(hwnd,null(),0);}return;}if state.kind==Kind::Dps{if let Some(row)=dps_row_at(hwnd,state,y){open_detail(hwnd,state,row);}}}
"#,
        "remove text-copy routing and resize combat controls",
    );

    replace_between(
        &mut source,
        "unsafe fn paint_toolbar(hdc:HDC,rc:RECT,state:&State){",
        "fn overlay_header(kind:Kind)->&'static str{",
        r#"unsafe fn paint_toolbar(hdc:HDC,rc:RECT,state:&State){
    let toolbar=RECT{left:0,top:0,right:rc.right,bottom:TOOLBAR_H};
    fill(hdc,&toolbar,rgb(20,27,33));
    fill(hdc,&RECT{left:0,top:TOOLBAR_H-2,right:rc.right,bottom:TOOLBAR_H},rgb(66,211,190));
    let controls_left=combat_controls_left(rc.right);
    let title_right=if state.kind==Kind::Dps{toolbar_action_rects(rc.right)[0].0.left-5}else{controls_left-5};
    SetTextColor(hdc,rgb(235,242,245));
    draw(hdc,&toolbar_title(state),RECT{left:10,top:0,right:title_right.max(80),bottom:TOOLBAR_H},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);
    if state.kind==Kind::Dps{
        for(index,(r,label))in toolbar_action_rects(rc.right).iter().enumerate(){
            let active=index==1&&state.history_index.is_none();
            paint_tab(hdc,r.left,r.top,r.right-r.left,label,active);
        }
    }
    let gear=RECT{left:controls_left,top:2,right:controls_left+COMBAT_GEAR_W,bottom:TOOLBAR_H-2};
    let collapse=RECT{left:gear.right,top:2,right:gear.right+COMBAT_COLLAPSE_W,bottom:TOOLBAR_H-2};
    let hide=RECT{left:collapse.right,top:2,right:rc.right,bottom:TOOLBAR_H-2};
    fill(hdc,&gear,rgb(26,30,36));fill(hdc,&collapse,rgb(26,30,36));fill(hdc,&hide,rgb(26,30,36));
    SetTextColor(hdc,rgb(239,243,247));
    draw(hdc,"⚙",gear,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);
    draw(hdc,collapse_glyph(state),collapse,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);
    draw(hdc,"×",hide,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);
}
"#,
        "combat toolbar control proportions",
    );

    replace_once(
        &mut source,
        r#"        if x>=rc.right-BUTTON_W{return Some("Hide overlay • restore from tray or Ctrl+Shift+F10".into());}
        if x>=rc.right-BUTTON_W*2{return Some("Collapse to the configured screen edge".into());}
        if x>=rc.right-BUTTON_W*3{return Some("Open overlay settings".into());}"#,
        r#"        let controls_left=combat_controls_left(rc.right);
        if x>=rc.right-COMBAT_HIDE_W{return Some("Hide overlay • restore from tray or Ctrl+Shift+F10".into());}
        if x>=rc.right-COMBAT_HIDE_W-COMBAT_COLLAPSE_W{return Some("Collapse to the configured screen edge".into());}
        if x>=controls_left{return Some("Open overlay settings".into());}"#,
        "combat toolbar hover zones",
    );

    replace_once(
        &mut source,
        "match index{0=>\"Older encounter\",1=>\"Return to live encounter\",2=>\"Newer encounter\",3=>\"Copy selected encounter as text\",4=>\"Copy the full DPS Meter as an image\",5=>\"Reset the live encounter\",_=>\"Combat action\"}",
        "match index{0=>\"Older encounter\",1=>\"Return to live encounter\",2=>\"Newer encounter\",3=>\"Copy the full DPS Meter as an image\",4=>\"Reset the live encounter\",_=>\"Combat action\"}",
        "DPS toolbar help",
    );

    // Keep the v1.17.8 regression meaningful after intentionally removing text copy.
    replace_once(
        &mut source,
        "fn toolbar_has_text_and_image_copy_but_no_exports()",
        "fn toolbar_has_image_copy_but_no_text_or_exports()",
        "v1.17.8 test name",
    );
    replace_once(
        &mut source,
        "assert_eq!(labels,vec![\"<\",\"LIVE\",\">\",\"Copy as Text\",\"Copy as Image\",\"Reset\"]);",
        "assert_eq!(labels,vec![\"<\",\"LIVE\",\">\",\"Copy as Image\",\"Reset\"]);",
        "v1.17.8 toolbar expectation",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1179_toolbar_polish_tests {
    use super::*;

    #[test]
    fn copy_as_text_is_gone() {
        let labels:Vec<_>=toolbar_action_rects(900).iter().map(|(_,label)|*label).collect();
        assert_eq!(labels,vec!["<","LIVE",">","Copy as Image","Reset"]);
        assert!(!labels.contains(&"Copy as Text"));
    }

    #[test]
    fn navigation_and_actions_are_visually_separated() {
        let rects=toolbar_action_rects(900);
        assert!(rects[3].0.left-rects[2].0.right>=10);
        assert!(rects[4].0.left-rects[3].0.right>=4);
        assert!(combat_controls_left(900)-rects[4].0.right>=4);
    }

    #[test]
    fn combat_controls_match_chat_proportions() {
        assert_eq!(COMBAT_GEAR_W,40);
        assert_eq!(COMBAT_COLLAPSE_W,38);
        assert_eq!(COMBAT_HIDE_W,42);
        assert!(COMBAT_GEAR_W<BUTTON_W);
        assert!(COMBAT_COLLAPSE_W<BUTTON_W);
        assert!(COMBAT_HIDE_W<BUTTON_W);
    }
}
"#);

    fs::write(path, source).expect("write v1.17.9 toolbar polish overlay");
}

fn patch_chat_overlay(out:&Path){
    let manifest=PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let source_path=manifest.join("src/overlay_v150.rs");
    let mut source=fs::read_to_string(&source_path).expect("read chat overlay").replace("\r\n","\n");
    replace_once(
        &mut source,
        "const HIDE_WIDTH: i32 = 38;",
        "const HIDE_WIDTH: i32 = 42;",
        "slightly larger Chat Overlay hide button",
    );
    source.push_str(r#"

#[cfg(test)]
mod v1179_chat_toolbar_tests {
    use super::*;
    #[test]
    fn hide_button_is_slightly_larger() {
        assert_eq!(GEAR_WIDTH,40);
        assert_eq!(COLLAPSE_WIDTH,38);
        assert_eq!(HIDE_WIDTH,42);
    }
}
"#);
    fs::write(out.join("overlay_v150_v1179.rs"),source).expect("write v1.17.9 chat overlay");
}

fn main() {
    prior::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_overlay(&out);
    patch_chat_overlay(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1179.rs");
    println!("cargo:rerun-if-changed=src/overlay_v150.rs");
}
