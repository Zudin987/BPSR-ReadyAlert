use std::{env, fs, path::{Path, PathBuf}};

mod previous {
    include!("build_pixel_phase2.rs");
    pub fn run() { main(); }
}

fn replace_required(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "Pixel phase 3 target {label:?} expected once, found {count}");
    *source = source.replacen(from, to, 1);
}

fn patch_feature_final(source: &mut String) {
    // The old full-width cyan rule still made the meter read like a themed utility.
    // Keep toolbar separation tonal and let selected/hovered actions carry state.
    replace_required(
        source,
        "let toolbar=RECT{left:0,top:0,right:rc.right,bottom:TOOLBAR_H};fill(hdc,&toolbar,crate::ui_modern::DARK_RAISED);fill(hdc,&RECT{left:0,top:TOOLBAR_H-2,right:rc.right,bottom:TOOLBAR_H},crate::ui_modern::BPSR_ACCENT);let scale=",
        "let toolbar=RECT{left:0,top:0,right:rc.right,bottom:TOOLBAR_H};fill(hdc,&toolbar,crate::ui_modern::DARK_RAISED);fill(hdc,&RECT{left:0,top:TOOLBAR_H-1,right:rc.right,bottom:TOOLBAR_H},crate::ui_modern::DARK_BORDER);let scale=",
        "meter toolbar separator",
    );

    // System toolbar actions are icon buttons: quiet at rest, tonal on hover. Their
    // hit rectangles and command routing are untouched.
    replace_required(
        source,
        "if state.kind==Kind::Dps&&state.compact_mode{let hide=RECT{left:rc.right-button,top:2,right:rc.right,bottom:TOOLBAR_H-2};let hovered=state.hover_y<TOOLBAR_H&&state.hover_x>=hide.left&&state.hover_x<hide.right;fill(hdc,&hide,if hovered{crate::ui_modern::DARK_HOVER}else{crate::ui_modern::DARK_SURFACE});SetTextColor",
        "if state.kind==Kind::Dps&&state.compact_mode{let hide=RECT{left:rc.right-button,top:2,right:rc.right,bottom:TOOLBAR_H-2};let hovered=state.hover_y<TOOLBAR_H&&state.hover_x>=hide.left&&state.hover_x<hide.right;if hovered{crate::ui_modern::fill_round_rect(hdc,RECT{left:hide.left+2,top:hide.top+1,right:hide.right-2,bottom:hide.bottom-1},crate::ui_modern::DARK_HOVER,crate::ui_modern::RADIUS_SMALL);}SetTextColor",
        "compact hide icon surface",
    );
    replace_required(
        source,
        "for r in [&gear,&collapse,&hide]{let hovered=state.hover_y<TOOLBAR_H&&state.hover_x>=r.left&&state.hover_x<r.right;fill(hdc,r,if hovered{crate::ui_modern::DARK_HOVER}else{crate::ui_modern::DARK_SURFACE});}",
        "for r in [&gear,&collapse,&hide]{let hovered=state.hover_y<TOOLBAR_H&&state.hover_x>=r.left&&state.hover_x<r.right;if hovered{crate::ui_modern::fill_round_rect(hdc,RECT{left:r.left+2,top:r.top+1,right:r.right-2,bottom:r.bottom-1},crate::ui_modern::DARK_HOVER,crate::ui_modern::RADIUS_SMALL);}}",
        "system icon hover surfaces",
    );
    replace_required(
        source,
        "let back=if active{crate::ui_modern::BPSR_ACCENT}else if hovered{crate::ui_modern::DARK_HOVER}else{crate::ui_modern::DARK_SURFACE};",
        "let back=if active{crate::ui_modern::BPSR_ACCENT}else if hovered{crate::ui_modern::DARK_HOVER}else{crate::ui_modern::DARK_RAISED};",
        "toolbar action resting surface",
    );

    // Entity Inspector is intentionally dense, but its identity/summary cards should
    // use the same shape hierarchy as the rest of the redesign rather than square slabs.
    replace_required(
        source,
        "unsafe fn detail_card(hdc:HDC,r:RECT,title:&str,lines:&[String]){fill(hdc,&r,crate::ui_modern::DARK_SURFACE);fill(hdc,&RECT{left:r.left,top:r.top,right:r.left+3,bottom:r.bottom},crate::ui_modern::BPSR_ACCENT);",
        "unsafe fn detail_card(hdc:HDC,r:RECT,title:&str,lines:&[String]){crate::ui_modern::fill_round_rect(hdc,r,crate::ui_modern::DARK_SURFACE,crate::ui_modern::RADIUS_MEDIUM);crate::ui_modern::fill_round_rect(hdc,RECT{left:r.left+4,top:r.top+8,right:r.left+7,bottom:r.bottom-8},crate::ui_modern::BPSR_ACCENT,2);",
        "detail cards",
    );
    replace_required(
        source,
        "let identity=RECT{left:8,top:TOOLBAR_H+7,right:rc.right-8,bottom:TOOLBAR_H+53};fill(hdc,&identity,crate::ui_modern::DARK_SURFACE);fill(hdc,&RECT{left:identity.left,top:identity.top,right:identity.left+4,bottom:identity.bottom},crate::ui_modern::BPSR_ACCENT);",
        "let identity=RECT{left:8,top:TOOLBAR_H+7,right:rc.right-8,bottom:TOOLBAR_H+53};crate::ui_modern::fill_round_rect(hdc,identity,crate::ui_modern::DARK_SURFACE,crate::ui_modern::RADIUS_MEDIUM);crate::ui_modern::fill_round_rect(hdc,RECT{left:identity.left+4,top:identity.top+8,right:identity.left+8,bottom:identity.bottom-8},crate::ui_modern::BPSR_ACCENT,2);",
        "entity identity card",
    );
    replace_required(
        source,
        "let summary_top=cards_bottom+7;let summary=RECT{left:8,top:summary_top,right:rc.right-8,bottom:detail_tab_top()-7};fill(hdc,&summary,crate::ui_modern::DARK_SURFACE);",
        "let summary_top=cards_bottom+7;let summary=RECT{left:8,top:summary_top,right:rc.right-8,bottom:detail_tab_top()-7};crate::ui_modern::fill_round_rect(hdc,summary,crate::ui_modern::DARK_SURFACE,crate::ui_modern::RADIUS_MEDIUM);",
        "entity summary card",
    );
}

fn patch_chat_final(source: &mut String) {
    // Preserve the overlay resize affordance, but demote the old 2px hard frame to a
    // quiet 1px edge so the content surface does the visual grouping.
    replace_required(
        source,
        "unsafe fn draw_border(hdc:HDC,client:RECT){let c=crate::ui_modern::DARK_BORDER;fill(hdc,&RECT{left:0,top:0,right:client.right,bottom:2},c);fill(hdc,&RECT{left:0,top:client.bottom-2,right:client.right,bottom:client.bottom},c);fill(hdc,&RECT{left:0,top:0,right:2,bottom:client.bottom},c);fill(hdc,&RECT{left:client.right-2,top:0,right:client.right,bottom:client.bottom},c);}",
        "unsafe fn draw_border(hdc:HDC,client:RECT){let c=crate::ui_modern::DARK_BORDER;fill(hdc,&RECT{left:0,top:0,right:client.right,bottom:1},c);fill(hdc,&RECT{left:0,top:client.bottom-1,right:client.right,bottom:client.bottom},c);fill(hdc,&RECT{left:0,top:0,right:1,bottom:client.bottom},c);fill(hdc,&RECT{left:client.right-1,top:0,right:client.right,bottom:client.bottom},c);}",
        "chat outer edge",
    );
    replace_required(
        source,
        "draw_text(hdc, glyph, client, rgb(215, 223, 232),",
        "draw_text(hdc, glyph, client, crate::ui_modern::BPSR_TEXT_SECONDARY,",
        "collapsed chat glyph",
    );
    replace_required(
        source,
        "let window_back = scaled_color((15, 19, 23), snapshot.chat.background_opacity);",
        "let window_back = scaled_color((17, 19, 24), snapshot.chat.background_opacity);",
        "chat background base",
    );
    replace_required(
        source,
        "fill(hdc, &toolbar, scaled_color((26, 32, 39), snapshot.chat.toolbar_opacity));",
        "fill(hdc, &toolbar, scaled_color((32, 35, 42), snapshot.chat.toolbar_opacity));",
        "chat toolbar base",
    );
    replace_required(
        source,
        "crate::ui_modern::fill_round_rect(hdc, rect, crate::ui_modern::BPSR_ACCENT_PRESSED, crate::ui_modern::RADIUS_SMALL);\n        draw_text(hdc, &label, rect, rgb(255,255,255),",
        "crate::ui_modern::fill_round_rect(hdc, rect, crate::ui_modern::BPSR_ACCENT, crate::ui_modern::RADIUS_SMALL);\n        draw_text(hdc, &label, rect, crate::ui_modern::BPSR_ACCENT_TEXT,",
        "chat back-to-latest chip",
    );
    replace_required(
        source,
        "fn tts_visual(enabled:bool,usable:bool)->(u32,u32){if !enabled{(crate::ui_modern::DARK_SURFACE,crate::ui_modern::BPSR_TEXT_SECONDARY)}else if !usable{(rgb(75,58,29),crate::ui_modern::BPSR_WARNING)}else{(dim_chat_color(crate::ui_modern::BPSR_ACCENT,62),rgb(244,252,250))}}",
        "fn tts_visual(enabled:bool,usable:bool)->(u32,u32){if !enabled{(crate::ui_modern::DARK_SURFACE,crate::ui_modern::BPSR_TEXT_SECONDARY)}else if !usable{(crate::ui_modern::mix_color(crate::ui_modern::DARK_RAISED,crate::ui_modern::BPSR_WARNING,26),crate::ui_modern::BPSR_WARNING)}else{(crate::ui_modern::mix_color(crate::ui_modern::DARK_RAISED,crate::ui_modern::BPSR_ACCENT,28),crate::ui_modern::BPSR_ACCENT_HOVER)}}",
        "chat TTS tonal state",
    );
}

fn patch_settings_final(source: &mut String) {
    // Make the hierarchy explicit: page title > section label > field label. The
    // controls stay at exactly the same coordinates and sizes.
    replace_required(
        source,
        "let h = create_static(hwnd, text, x, y, 580, 24); crate::ui_theme::set_font(h,crate::ui_theme::FontRole::Heading); state.page_controls.push((h, page));",
        "let h = create_static(hwnd, text, x, y, 580, 24); SendMessageW(h,WM_SETFONT,crate::ui_modern::heading_font() as usize,1); state.page_controls.push((h, page));",
        "settings page title font",
    );
    replace_required(
        source,
        "unsafe fn section(hwnd:HWND,state:&mut SettingsState,page:usize,text:&str,x:i32,y:i32,w:i32){let c=create_static(hwnd,text,x,y,w,18);crate::ui_theme::set_font(c,crate::ui_theme::FontRole::Secondary);state.page_controls.push((c,page));}",
        "unsafe fn section(hwnd:HWND,state:&mut SettingsState,page:usize,text:&str,x:i32,y:i32,w:i32){let c=create_static(hwnd,text,x,y,w,18);SendMessageW(c,WM_SETFONT,crate::ui_modern::medium_font() as usize,1);state.page_controls.push((c,page));}",
        "settings section font",
    );

    // Swatches are real small surface components rather than sharp bordered squares.
    replace_required(
        source,
        "let parent=windows_sys::Win32::UI::WindowsAndMessaging::GetParent((*item).hwndItem);let edit=swatch_edit_id(id).unwrap_or(ID_HIGHLIGHT_COLOR);let color=hex_colorref(&get_text(parent,edit)).unwrap_or(crate::ui_modern::MIST_HOVER);let r=(*item).rcItem;let br=CreateSolidBrush(color);FillRect((*item).hDC,&r,br);DeleteObject(br);let border=CreateSolidBrush(crate::ui_modern::MIST_BORDER_STRONG);let top=RECT{left:r.left,top:r.top,right:r.right,bottom:r.top+1};let bottom=RECT{left:r.left,top:r.bottom-1,right:r.right,bottom:r.bottom};let left=RECT{left:r.left,top:r.top,right:r.left+1,bottom:r.bottom};let right=RECT{left:r.right-1,top:r.top,right:r.right,bottom:r.bottom};for edge in [&top,&bottom,&left,&right]{FillRect((*item).hDC,edge,border);}DeleteObject(border);1",
        "let parent=windows_sys::Win32::UI::WindowsAndMessaging::GetParent((*item).hwndItem);let edit=swatch_edit_id(id).unwrap_or(ID_HIGHLIGHT_COLOR);let color=hex_colorref(&get_text(parent,edit)).unwrap_or(crate::ui_modern::MIST_HOVER);let mut r=(*item).rcItem;r.left+=1;r.top+=1;r.right-=1;r.bottom-=1;crate::ui_modern::fill_round_rect((*item).hDC,r,color,6);crate::ui_modern::stroke_round_rect((*item).hDC,r,crate::ui_modern::MIST_BORDER_STRONG,6,1);1",
        "settings color swatches",
    );
}

fn patch_event_tracker_final(source: &mut String) {
    replace_required(
        source,
        "let title=create_static(hwnd,\"Event Tracker\",18,14,420,24);crate::ui_theme::set_font(title,crate::ui_theme::FontRole::Heading);",
        "let title=create_static(hwnd,\"Event Tracker\",18,14,420,24);SendMessageW(title,0x0030,crate::ui_modern::heading_font() as usize,1);",
        "event tracker title font",
    );
    replace_required(
        source,
        "let rules=create_static(hwnd,\"RULES\",18,112,270,18);crate::ui_theme::set_font(rules,crate::ui_theme::FontRole::Secondary);",
        "let rules=create_static(hwnd,\"RULES\",18,112,270,18);SendMessageW(rules,0x0030,crate::ui_modern::medium_font() as usize,1);",
        "event tracker rules heading",
    );
    replace_required(
        source,
        "let edit_title=create_static(hwnd,\"RULE DETAILS\",314,112,370,18);crate::ui_theme::set_font(edit_title,crate::ui_theme::FontRole::Secondary);",
        "let edit_title=create_static(hwnd,\"RULE DETAILS\",314,112,370,18);SendMessageW(edit_title,0x0030,crate::ui_modern::medium_font() as usize,1);",
        "event tracker detail heading",
    );
}

fn patch_file(out: &Path, name: &str, kind: &str) {
    let path = out.join(name);
    assert!(path.exists(), "Pixel phase 3 expected generated file {name}");
    let mut source = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {name} for Pixel phase 3: {e}"))
        .replace("\r\n", "\n");
    match kind {
        "feature" => patch_feature_final(&mut source),
        "chat" => patch_chat_final(&mut source),
        "settings" => patch_settings_final(&mut source),
        "event" => patch_event_tracker_final(&mut source),
        _ => unreachable!(),
    }
    fs::write(path, source).unwrap_or_else(|e| panic!("write {name} for Pixel phase 3: {e}"));
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_file(&out, "feature_overlays_v170_fixed.rs", "feature");
    patch_file(&out, "overlay_v150_v1181.rs", "chat");
    patch_file(&out, "settings_ui_v1160_fixed.rs", "settings");
    patch_file(&out, "event_tracker_ui_v1160_fixed.rs", "event");
    println!("cargo:rerun-if-changed=build/legacy/build_pixel_phase3.rs");
}
