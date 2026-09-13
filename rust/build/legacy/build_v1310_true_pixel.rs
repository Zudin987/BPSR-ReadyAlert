use std::{env, fs, path::{Path, PathBuf}};

mod previous {
    include!("build_v1302_ui_audit.rs");
    pub fn run() { main(); }
}

fn replace_if_present(source: &mut String, from: &str, to: &str) {
    if source.contains(from) { *source = source.replace(from, to); }
}

fn map_dark_fallback_tokens(source: &mut String) {
    // Longest names first.  These are presentation tokens only; semantic combat/class
    // colors are intentionally not remapped.
    for (from, to) in [
        ("crate::ui_theme::BORDER_STRONG", "crate::ui_modern::DARK_BORDER_STRONG"),
        ("crate::ui_theme::TEXT_SECONDARY", "crate::ui_modern::BPSR_TEXT_SECONDARY"),
        ("crate::ui_theme::TEXT_DISABLED", "crate::ui_modern::BPSR_DISABLED"),
        ("crate::ui_theme::SURFACE_PRESSED", "crate::ui_modern::DARK_PRESSED"),
        ("crate::ui_theme::SURFACE_HOVER", "crate::ui_modern::DARK_HOVER"),
        ("crate::ui_theme::ACCENT_PRESSED", "crate::ui_modern::BPSR_ACCENT_PRESSED"),
        ("crate::ui_theme::ACCENT_HOVER", "crate::ui_modern::BPSR_ACCENT_HOVER"),
        ("crate::ui_theme::ACCENT_TEXT", "crate::ui_modern::BPSR_ACCENT_TEXT"),
        ("crate::ui_theme::DANGER_HOVER", "crate::ui_modern::BPSR_DANGER_HOVER"),
        ("crate::ui_theme::RANK_TEXT", "crate::ui_modern::BPSR_TEXT_SECONDARY"),
        ("crate::ui_theme::SIDEBAR", "crate::ui_modern::DARK_SURFACE"),
        ("crate::ui_theme::SURFACE", "crate::ui_modern::DARK_SURFACE"),
        ("crate::ui_theme::RAISED", "crate::ui_modern::DARK_RAISED"),
        ("crate::ui_theme::INPUT", "crate::ui_modern::DARK_INPUT"),
        ("crate::ui_theme::BORDER", "crate::ui_modern::DARK_BORDER"),
        ("crate::ui_theme::MUTED", "crate::ui_modern::BPSR_MUTED"),
        ("crate::ui_theme::WARNING", "crate::ui_modern::BPSR_WARNING"),
        ("crate::ui_theme::CRITICAL", "crate::ui_modern::BPSR_DANGER"),
        ("crate::ui_theme::DANGER", "crate::ui_modern::BPSR_DANGER"),
        ("crate::ui_theme::ACCENT", "crate::ui_modern::BPSR_ACCENT"),
        ("crate::ui_theme::TEXT", "crate::ui_modern::BPSR_TEXT"),
        ("crate::ui_theme::BG", "crate::ui_modern::DARK_BG"),
    ] { replace_if_present(source, from, to); }
}

fn map_mist_fallback_tokens(source: &mut String) {
    for (from, to) in [
        ("crate::ui_theme::BORDER_STRONG", "crate::ui_modern::MIST_BORDER_STRONG"),
        ("crate::ui_theme::TEXT_SECONDARY", "crate::ui_modern::BPSR_TEXT_SECONDARY"),
        ("crate::ui_theme::TEXT_DISABLED", "crate::ui_modern::BPSR_DISABLED"),
        ("crate::ui_theme::SURFACE_PRESSED", "crate::ui_modern::MIST_PRESSED"),
        ("crate::ui_theme::SURFACE_HOVER", "crate::ui_modern::MIST_HOVER"),
        ("crate::ui_theme::ACCENT_PRESSED", "crate::ui_modern::BPSR_ACCENT_PRESSED"),
        ("crate::ui_theme::ACCENT_HOVER", "crate::ui_modern::BPSR_ACCENT_HOVER"),
        ("crate::ui_theme::ACCENT_TEXT", "crate::ui_modern::BPSR_ACCENT_TEXT"),
        ("crate::ui_theme::SIDEBAR", "crate::ui_modern::MIST_SIDEBAR"),
        ("crate::ui_theme::SURFACE", "crate::ui_modern::MIST_SURFACE"),
        ("crate::ui_theme::RAISED", "crate::ui_modern::MIST_RAISED"),
        ("crate::ui_theme::INPUT", "crate::ui_modern::MIST_INPUT"),
        ("crate::ui_theme::BORDER", "crate::ui_modern::MIST_BORDER"),
        ("crate::ui_theme::MUTED", "crate::ui_modern::BPSR_MUTED"),
        ("crate::ui_theme::WARNING", "crate::ui_modern::BPSR_WARNING"),
        ("crate::ui_theme::DANGER", "crate::ui_modern::BPSR_DANGER"),
        ("crate::ui_theme::ACCENT", "crate::ui_modern::BPSR_ACCENT"),
        ("crate::ui_theme::TEXT", "crate::ui_modern::BPSR_TEXT"),
        ("crate::ui_theme::BG", "crate::ui_modern::MIST_BG"),
    ] { replace_if_present(source, from, to); }
}

fn patch_feature_components(source: &mut String) {
    // Detail tabs: selected state is a tonal container. Inactive tabs have no hard
    // outline, matching the surface-driven Pixel hierarchy.
    replace_if_present(source,
        r#"unsafe fn paint_tab(hdc:HDC,x:i32,y:i32,width:i32,text:&str,active:bool){let r=RECT{left:x,top:y,right:x+width,bottom:y+23};crate::ui_modern::fill_round_rect(hdc,r,if active{crate::ui_modern::SELECTED_SURFACE}else{crate::ui_modern::DARK_SURFACE},crate::ui_modern::RADIUS_SMALL);crate::ui_modern::stroke_round_rect(hdc,r,if active{crate::ui_modern::BPSR_ACCENT}else{crate::ui_modern::DARK_BORDER},crate::ui_modern::RADIUS_SMALL,1);SetTextColor(hdc,if active{crate::ui_modern::BPSR_TEXT}else{crate::ui_modern::BPSR_TEXT_SECONDARY});draw(hdc,text,r,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}"#,
        r#"unsafe fn paint_tab(hdc:HDC,x:i32,y:i32,width:i32,text:&str,active:bool){let r=RECT{left:x,top:y,right:x+width,bottom:y+23};crate::ui_modern::fill_round_rect(hdc,r,if active{crate::ui_modern::SELECTED_SURFACE}else{crate::ui_modern::DARK_SURFACE},crate::ui_modern::RADIUS_SMALL);SetTextColor(hdc,if active{crate::ui_modern::BPSR_TEXT}else{crate::ui_modern::BPSR_TEXT_SECONDARY});draw(hdc,text,r,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}"#);

    // Dense hand-painted checks used by the overlay settings should visually match
    // native checkbox controls without changing their hit boxes or toggling logic.
    replace_if_present(source,
        r#"unsafe fn paint_check(hdc:HDC,x:i32,y:i32,label:&str,checked:bool,right:i32){let box_r=RECT{left:x,top:y+3,right:x+18,bottom:y+21};fill(hdc,&box_r,if checked{rgb(66,211,190)}else{crate::ui_modern::DARK_BORDER});if checked{SetTextColor(hdc,rgb(255,255,255));draw(hdc,"✓",box_r,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}SetTextColor(hdc,crate::ui_modern::BPSR_TEXT_SECONDARY);draw(hdc,label,RECT{left:x+26,top:y,right:right-10,bottom:y+25},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}"#,
        r#"unsafe fn paint_check(hdc:HDC,x:i32,y:i32,label:&str,checked:bool,right:i32){let box_r=RECT{left:x,top:y+3,right:x+18,bottom:y+21};crate::ui_modern::fill_round_rect(hdc,box_r,if checked{crate::ui_modern::BPSR_ACCENT}else{crate::ui_modern::DARK_RAISED},5);if !checked{crate::ui_modern::stroke_round_rect(hdc,box_r,crate::ui_modern::DARK_BORDER_STRONG,5,1);}if checked{SetTextColor(hdc,crate::ui_modern::BPSR_ACCENT_TEXT);draw(hdc,"✓",box_r,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}SetTextColor(hdc,crate::ui_modern::BPSR_TEXT_SECONDARY);draw(hdc,label,RECT{left:x+26,top:y,right:right-10,bottom:y+25},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}"#);

    // Popup toolbars lose the old saturated accent rule; hierarchy comes from the
    // title/surface instead.
    replace_if_present(source,
        r#"unsafe fn paint_popup_toolbar(hdc:HDC,rc:RECT,title:&str){fill(hdc,&RECT{left:0,top:0,right:rc.right,bottom:TOOLBAR_H},crate::ui_theme::SIDEBAR);fill(hdc,&RECT{left:0,top:TOOLBAR_H-2,right:rc.right,bottom:TOOLBAR_H},crate::ui_theme::ACCENT);SetTextColor(hdc,crate::ui_theme::TEXT);draw(hdc,title,RECT{left:10,top:0,right:rc.right-BUTTON_W-4,bottom:TOOLBAR_H},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);draw(hdc,"×",RECT{left:rc.right-BUTTON_W,top:0,right:rc.right,bottom:TOOLBAR_H},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}"#,
        r#"unsafe fn paint_popup_toolbar(hdc:HDC,rc:RECT,title:&str){fill(hdc,&RECT{left:0,top:0,right:rc.right,bottom:TOOLBAR_H},crate::ui_modern::DARK_SURFACE);fill(hdc,&RECT{left:0,top:TOOLBAR_H-1,right:rc.right,bottom:TOOLBAR_H},crate::ui_modern::DARK_BORDER);SetTextColor(hdc,crate::ui_modern::BPSR_TEXT);draw(hdc,title,RECT{left:12,top:0,right:rc.right-BUTTON_W-4,bottom:TOOLBAR_H},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);draw(hdc,"×",RECT{left:rc.right-BUTTON_W,top:0,right:rc.right,bottom:TOOLBAR_H},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}"#);
}

fn patch_chat_components(source: &mut String) {
    // Toolbar actions are compact tonal buttons; remove the always-visible outline.
    replace_if_present(source,
        r#"unsafe fn draw_toolbar_button(hdc: HDC, rect: RECT, text: &str, back: u32, fore: u32) {
    crate::ui_modern::fill_round_rect(hdc, rect, back, crate::ui_modern::RADIUS_SMALL);
    crate::ui_modern::stroke_round_rect(hdc, rect, crate::ui_modern::DARK_BORDER, crate::ui_modern::RADIUS_SMALL, 1);
    draw_text(hdc, text, rect, fore, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX, false);
}"#,
        r#"unsafe fn draw_toolbar_button(hdc: HDC, rect: RECT, text: &str, back: u32, fore: u32) {
    crate::ui_modern::fill_round_rect(hdc, rect, back, crate::ui_modern::RADIUS_SMALL);
    draw_text(hdc, text, rect, fore, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX, false);
}"#);
    replace_if_present(source,
        r#"unsafe fn draw_toolbar_button_sized(hdc:HDC,rect:RECT,text:&str,back:u32,fore:u32,pixel_height:i32){
    crate::ui_modern::fill_round_rect(hdc,rect,back,crate::ui_modern::RADIUS_SMALL);crate::ui_modern::stroke_round_rect(hdc,rect,crate::ui_modern::DARK_BORDER,crate::ui_modern::RADIUS_SMALL,1);"#,
        r#"unsafe fn draw_toolbar_button_sized(hdc:HDC,rect:RECT,text:&str,back:u32,fore:u32,pixel_height:i32){
    crate::ui_modern::fill_round_rect(hdc,rect,back,crate::ui_modern::RADIUS_SMALL);"#);
}

fn patch_file(out: &Path, name: &str, mist: bool, feature: bool, chat: bool) {
    let path = out.join(name);
    if !path.exists() { return; }
    let mut source = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {name} for true Pixel finish: {e}")).replace("\r\n", "\n");
    if feature { patch_feature_components(&mut source); }
    if chat { patch_chat_components(&mut source); }
    if mist { map_mist_fallback_tokens(&mut source); } else { map_dark_fallback_tokens(&mut source); }
    fs::write(path, source).unwrap_or_else(|e| panic!("write {name} for true Pixel finish: {e}"));
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_file(&out, "settings_ui_v1160_fixed.rs", true, false, false);
    patch_file(&out, "event_tracker_ui_v1160_fixed.rs", true, false, false);
    patch_file(&out, "feature_overlays_v170_fixed.rs", false, true, false);
    patch_file(&out, "overlay_v150_v1181.rs", false, false, true);
    patch_file(&out, "win_v182_fixed.rs", false, false, false);
    patch_file(&out, "updater_v1241.rs", false, false, false);
    println!("cargo:rerun-if-changed=build/legacy/build_v1310_true_pixel.rs");
    println!("cargo:rerun-if-changed=src/ui_pixel_core.rs");
    println!("cargo:rerun-if-changed=src/ui_pixel_controls.rs");
    println!("cargo:rerun-if-changed=src/ui_pixel_dialog.rs");
}
