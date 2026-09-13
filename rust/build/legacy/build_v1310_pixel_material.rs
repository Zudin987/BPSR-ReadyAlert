use std::{env, fs, path::{Path, PathBuf}};

mod previous {
    include!("build_v1300_idmap.rs");
    pub fn run() { main(); }
}

fn replace_if_present(source: &mut String, from: &str, to: &str) {
    if source.contains(from) { *source = source.replace(from, to); }
}

fn pixel_tokens(source: &mut String) {
    // Longest names first so partial token names never pre-empt a replacement.
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

    // Legacy chrome literals that survived earlier versioned patches. Do not
    // touch class/metric colors here: those are semantic data, not chrome.
    for (from, to) in [
        ("rgb(14,23,29)", "crate::ui_modern::DARK_BG"),
        ("rgb(14, 23, 29)", "crate::ui_modern::DARK_BG"),
        ("rgb(19,25,30)", "crate::ui_modern::DARK_SURFACE"),
        ("rgb(19, 25, 30)", "crate::ui_modern::DARK_SURFACE"),
        ("rgb(20,24,29)", "crate::ui_modern::DARK_SURFACE"),
        ("rgb(20, 24, 29)", "crate::ui_modern::DARK_SURFACE"),
        ("rgb(24,32,38)", "crate::ui_modern::DARK_RAISED"),
        ("rgb(24, 32, 38)", "crate::ui_modern::DARK_RAISED"),
        ("rgb(31,43,52)", "crate::ui_modern::DARK_HOVER"),
        ("rgb(31, 43, 52)", "crate::ui_modern::DARK_HOVER"),
        ("rgb(40,47,56)", "crate::ui_modern::DARK_BORDER"),
        ("rgb(40, 47, 56)", "crate::ui_modern::DARK_BORDER"),
        ("rgb(62,67,74)", "crate::ui_modern::DARK_BORDER_STRONG"),
        ("rgb(62, 67, 74)", "crate::ui_modern::DARK_BORDER_STRONG"),
    ] { replace_if_present(source, from, to); }
}

fn patch_configuration_surface(source: &mut String) {
    // Feature-overlay Settings is still a configuration window even though it
    // lives in the overlay module. Keep its exact geometry/controls and only
    // swap its paint family.
    replace_if_present(source,
        "background:CreateSolidBrush(crate::ui_theme::BG)",
        "background:CreateSolidBrush(crate::ui_modern::MIST_BG)");
    replace_if_present(source,
        "background:CreateSolidBrush(crate::ui_modern::DARK_BG)",
        "background:CreateSolidBrush(crate::ui_modern::MIST_BG)");
    replace_if_present(source,
        "state.settings_hwnd=hwnd;crate::ui::fit_window(hwnd,parent,true);crate::ui_theme::dark_titlebar(hwnd);",
        "state.settings_hwnd=hwnd;crate::ui::fit_window(hwnd,parent,true);crate::ui_modern::mist_titlebar(hwnd);");
    replace_if_present(source,
        "SetBkColor(hdc,crate::ui_theme::BG);let warning=lparam as HWND==fc(hwnd,7009)&&settings_scale_is_limited(state,settings_overlay_scale(state));SetTextColor(hdc,if warning{crate::ui_theme::WARNING}else{crate::ui_theme::TEXT});",
        "SetBkColor(hdc,crate::ui_modern::MIST_BG);let warning=lparam as HWND==fc(hwnd,7009)&&settings_scale_is_limited(state,settings_overlay_scale(state));SetTextColor(hdc,if warning{crate::ui_modern::BPSR_WARNING}else{crate::ui_modern::BPSR_TEXT});");
}

fn patch_dense_overlay_components(source: &mut String) {
    replace_if_present(source,
        r#"unsafe fn paint_popup_toolbar(hdc:HDC,rc:RECT,title:&str){fill(hdc,&RECT{left:0,top:0,right:rc.right,bottom:TOOLBAR_H},crate::ui_theme::SIDEBAR);fill(hdc,&RECT{left:0,top:TOOLBAR_H-2,right:rc.right,bottom:TOOLBAR_H},crate::ui_theme::ACCENT);SetTextColor(hdc,crate::ui_theme::TEXT);draw(hdc,title,RECT{left:10,top:0,right:rc.right-BUTTON_W-4,bottom:TOOLBAR_H},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);draw(hdc,"×",RECT{left:rc.right-BUTTON_W,top:0,right:rc.right,bottom:TOOLBAR_H},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}"#,
        r#"unsafe fn paint_popup_toolbar(hdc:HDC,rc:RECT,title:&str){fill(hdc,&RECT{left:0,top:0,right:rc.right,bottom:TOOLBAR_H},crate::ui_modern::DARK_SURFACE);fill(hdc,&RECT{left:0,top:TOOLBAR_H-1,right:rc.right,bottom:TOOLBAR_H},crate::ui_modern::DARK_BORDER);SetTextColor(hdc,crate::ui_modern::BPSR_TEXT);draw(hdc,title,RECT{left:12,top:0,right:rc.right-BUTTON_W-4,bottom:TOOLBAR_H},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);draw(hdc,"×",RECT{left:rc.right-BUTTON_W,top:0,right:rc.right,bottom:TOOLBAR_H},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}"#);

    replace_if_present(source,
        r#"unsafe fn paint_mode_tab(hdc:HDC,x:i32,y:i32,width:i32,text:&str,mode:SortMode,active:bool){let r=RECT{left:x,top:y,right:x+width,bottom:y+23};fill(hdc,&r,crate::ui_theme::SURFACE);if active{fill(hdc,&RECT{left:r.left,top:r.bottom-3,right:r.right,bottom:r.bottom},mode_color(mode));}SetTextColor(hdc,if active{crate::ui_theme::TEXT}else{crate::ui_theme::TEXT_SECONDARY});draw(hdc,text,r,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}"#,
        r#"unsafe fn paint_mode_tab(hdc:HDC,x:i32,y:i32,width:i32,text:&str,mode:SortMode,active:bool){let r=RECT{left:x,top:y,right:x+width,bottom:y+23};crate::ui_modern::fill_round_rect(hdc,r,if active{crate::ui_modern::SELECTED_SURFACE}else{crate::ui_modern::DARK_SURFACE},crate::ui_modern::RADIUS_SMALL);if active{crate::ui_modern::fill_round_rect(hdc,RECT{left:r.left+8,top:r.bottom-3,right:r.right-8,bottom:r.bottom},mode_color(mode),2);}SetTextColor(hdc,if active{crate::ui_modern::BPSR_TEXT}else{crate::ui_modern::BPSR_TEXT_SECONDARY});draw(hdc,text,r,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}"#);

    replace_if_present(source,
        r#"unsafe fn paint_check(hdc:HDC,x:i32,y:i32,label:&str,checked:bool,right:i32){let box_r=RECT{left:x,top:y+3,right:x+18,bottom:y+21};fill(hdc,&box_r,if checked{rgb(66,211,190)}else{crate::ui_modern::DARK_BORDER});if checked{SetTextColor(hdc,rgb(255,255,255));draw(hdc,"✓",box_r,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}SetTextColor(hdc,crate::ui_modern::BPSR_TEXT_SECONDARY);draw(hdc,label,RECT{left:x+26,top:y,right:right-10,bottom:y+25},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}"#,
        r#"unsafe fn paint_check(hdc:HDC,x:i32,y:i32,label:&str,checked:bool,right:i32){let box_r=RECT{left:x,top:y+3,right:x+18,bottom:y+21};crate::ui_modern::fill_round_rect(hdc,box_r,if checked{crate::ui_modern::BPSR_ACCENT}else{crate::ui_modern::DARK_RAISED},4);crate::ui_modern::stroke_round_rect(hdc,box_r,if checked{crate::ui_modern::BPSR_ACCENT}else{crate::ui_modern::DARK_BORDER_STRONG},4,1);if checked{SetTextColor(hdc,crate::ui_modern::BPSR_ACCENT_TEXT);draw(hdc,"✓",box_r,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}SetTextColor(hdc,crate::ui_modern::BPSR_TEXT_SECONDARY);draw(hdc,label,RECT{left:x+26,top:y,right:right-10,bottom:y+25},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}"#);
}

fn patch_file(out: &Path, name: &str, config_surface: bool, dense_components: bool) {
    let path = out.join(name);
    if !path.exists() { return; }
    let mut source = fs::read_to_string(&path).unwrap_or_else(|_| panic!("read {name} for Pixel/Material finish")).replace("\r\n", "\n");
    if config_surface { patch_configuration_surface(&mut source); }
    if dense_components { patch_dense_overlay_components(&mut source); }
    pixel_tokens(&mut source);
    fs::write(path, source).unwrap_or_else(|_| panic!("write {name} for Pixel/Material finish"));
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_file(&out, "settings_ui_v1160_fixed.rs", false, false);
    patch_file(&out, "event_tracker_ui_v1160_fixed.rs", false, false);
    patch_file(&out, "overlay_v150_v1181.rs", false, false);
    patch_file(&out, "feature_overlays_v170_fixed.rs", true, true);
    patch_file(&out, "win_v182_fixed.rs", false, false);
    patch_file(&out, "updater_v1241.rs", false, false);
    println!("cargo:rerun-if-changed=build/legacy/build_v1310_pixel_material.rs");
    println!("cargo:rerun-if-changed=src/ui_modern.rs");
}
