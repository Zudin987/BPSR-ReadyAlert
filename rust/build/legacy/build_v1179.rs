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

fn remove_between(source: &mut String, start: &str, end: &str, label: &str) {
    let begin = source.find(start).unwrap_or_else(|| panic!("v1.17.9 patch {label} start missing"));
    let rel_end = source[begin..].find(end).unwrap_or_else(|| panic!("v1.17.9 patch {label} end missing"));
    source.replace_range(begin..begin + rel_end, "");
}

fn patch_feature_overlay(out: &Path) {
    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read v1.17.8 feature overlay").replace("\r\n", "\n");

    replace_once(
        &mut source,
        "fn toolbar_action_rects(right:i32)->[(RECT,&'static str);6]{let mut x=right-BUTTON_W*3;let mut take=|w:i32,label:&'static str|{let r=RECT{left:x-w,top:5,right:x,bottom:29};x-=w;(r,label)};let reset=take(52,\"Reset\");let image=take(96,\"Copy as Image\");let text=take(88,\"Copy as Text\");let newer=take(30,\">\");let live=take(46,\"LIVE\");let older=take(30,\"<\");[older,live,newer,text,image,reset]}",
        "fn toolbar_action_rects(right:i32)->[(RECT,&'static str);5]{let mut x=right-BUTTON_W*3;let mut take=|w:i32,label:&'static str|{let r=RECT{left:x-w,top:5,right:x,bottom:29};x-=w;(r,label)};let reset=take(52,\"Reset\");let image=take(96,\"Copy as Image\");x-=12;let newer=take(30,\">\");let live=take(46,\"LIVE\");let older=take(30,\"<\");[older,live,newer,image,reset]}",
        "remove Copy as Text and separate navigation/share groups",
    );

    remove_between(&mut source, "fn copy_view_text(state:&mut State){", "const CF_BITMAP_:u32=2;", "Copy as Text function");

    replace_once(
        &mut source,
        "3=>copy_view_text(state),4=>copy_view_image(hwnd,state),5=>{if state.history_index.is_none(){archive_live_snapshot(state);crate::telemetry::request_manual_reset();state.dps=DpsSnapshot::default();state.scroll=0;}else{state.history_index=None;state.scroll=0;}},",
        "3=>copy_view_image(hwnd,state),4=>{if state.history_index.is_none(){archive_live_snapshot(state);crate::telemetry::request_manual_reset();state.dps=DpsSnapshot::default();state.scroll=0;}else{state.history_index=None;state.scroll=0;}},",
        "DPS click routing without text copy",
    );

    replace_once(
        &mut source,
        "match index{0=>\"Older encounter\",1=>\"Return to live encounter\",2=>\"Newer encounter\",3=>\"Copy selected encounter as text\",4=>\"Copy the full DPS Meter as an image\",5=>\"Reset the live encounter\",_=>\"Combat action\"}",
        "match index{0=>\"Older encounter\",1=>\"Return to live encounter\",2=>\"Newer encounter\",3=>\"Copy the full DPS Meter as an image\",4=>\"Reset the live encounter\",_=>\"Combat action\"}",
        "DPS toolbar help",
    );

    replace_once(
        &mut source,
        "    draw(hdc,\"⚙\",gear,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);\n    draw(hdc,collapse_glyph(state),collapse,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);\n    draw(hdc,\"×\",hide,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);",
        "    draw_toolbar_symbol(hdc,\"⚙\",gear,COMBAT_ICON_PX);\n    draw_toolbar_symbol(hdc,collapse_glyph(state),collapse,COMBAT_ICON_PX);\n    draw_toolbar_symbol(hdc,\"×\",hide,HIDE_ICON_PX);",
        "combat icon sizing",
    );

    replace_once(
        &mut source,
        "fn toolbar_title(state:&State)->String{overlay_header(state.kind).into()}",
        r#"const COMBAT_ICON_PX:i32=12;
const HIDE_ICON_PX:i32=15;
unsafe fn draw_toolbar_symbol(hdc:HDC,text:&str,rect:RECT,pixel_height:i32){
    let face=wide("Segoe UI Symbol");
    let font=windows_sys::Win32::Graphics::Gdi::CreateFontW(-pixel_height,0,0,0,400,0,0,0,1,0,0,5,0,face.as_ptr());
    if font.is_null(){draw(hdc,text,rect,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);return;}
    let old=SelectObject(hdc,font);
    draw(hdc,text,rect,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);
    SelectObject(hdc,old);
    DeleteObject(font);
}
fn toolbar_title(state:&State)->String{overlay_header(state.kind).into()}"#,
        "combat toolbar symbol fonts",
    );

    replace_once(
        &mut source,
        "fn toolbar_has_text_and_image_copy_but_no_exports() {\n        let labels:Vec<_>=toolbar_action_rects(900).iter().map(|(_,label)|*label).collect();\n        assert_eq!(labels,vec![\"<\",\"LIVE\",\">\",\"Copy as Text\",\"Copy as Image\",\"Reset\"]);\n        assert!(!labels.iter().any(|label|matches!(*label,\"CSV\"|\"JSON\")));\n    }",
        "fn toolbar_has_image_copy_only() {\n        let actions=toolbar_action_rects(900);\n        let labels:Vec<_>=actions.iter().map(|(_,label)|*label).collect();\n        assert_eq!(labels,vec![\"<\",\"LIVE\",\">\",\"Copy as Image\",\"Reset\"]);\n        assert_eq!(actions[3].0.left-actions[2].0.right,12);\n        assert!(!labels.iter().any(|label|matches!(*label,\"CSV\"|\"JSON\"|\"Copy as Text\")));\n    }",
        "update v1.17.8 toolbar regression",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1179_toolbar_polish_tests {
    use super::*;

    #[test]
    fn combat_toolbar_icons_use_smaller_controls_and_larger_hide_glyph() {
        assert_eq!(COMBAT_ICON_PX,12);
        assert_eq!(HIDE_ICON_PX,15);
        assert!(HIDE_ICON_PX>COMBAT_ICON_PX);
    }

    #[test]
    fn navigation_is_visually_separate_from_image_actions() {
        let actions=toolbar_action_rects(700);
        assert_eq!(actions[3].0.left-actions[2].0.right,12);
    }
}
"#);

    fs::write(path, source).expect("write v1.17.9 feature overlay");
}

fn patch_chat_overlay(out: &Path) {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let input = manifest.join("src/overlay_v150.rs");
    let mut source = fs::read_to_string(&input).expect("read chat overlay source").replace("\r\n", "\n");

    replace_once(
        &mut source,
        "    draw_toolbar_button(hdc, actions.hide, \"×\", rgb(26,30,36), rgb(239,243,247));",
        "    draw_toolbar_button_sized(hdc, actions.hide, \"×\", rgb(26,30,36), rgb(239,243,247), 15);",
        "larger chat hide glyph",
    );
    replace_once(
        &mut source,
        "unsafe fn draw_toolbar_button(hdc: HDC, rect: RECT, text: &str, back: u32, fore: u32) {\n    fill(hdc, &rect, back);\n    draw_text(hdc, text, rect, fore, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX, false);\n}",
        r#"unsafe fn draw_toolbar_button(hdc: HDC, rect: RECT, text: &str, back: u32, fore: u32) {
    fill(hdc, &rect, back);
    draw_text(hdc, text, rect, fore, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX, false);
}

unsafe fn draw_toolbar_button_sized(hdc:HDC,rect:RECT,text:&str,back:u32,fore:u32,pixel_height:i32){
    fill(hdc,&rect,back);
    let face=wide("Segoe UI Symbol");
    let font=windows_sys::Win32::Graphics::Gdi::CreateFontW(-pixel_height,0,0,0,400,0,0,0,1,0,0,5,0,face.as_ptr());
    if font.is_null(){draw_text(hdc,text,rect,fore,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX,false);return;}
    let old=SelectObject(hdc,font);
    draw_text(hdc,text,rect,fore,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX,false);
    SelectObject(hdc,old);
    DeleteObject(font);
}"#,
        "chat sized toolbar glyph helper",
    );

    fs::write(out.join("overlay_v150_v1179.rs"), source).expect("write v1.17.9 chat overlay source");
}

fn main() {
    prior::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_feature_overlay(&out);
    patch_chat_overlay(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1179.rs");
    println!("cargo:rerun-if-changed=src/overlay_v150.rs");
}
