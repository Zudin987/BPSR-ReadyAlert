use std::{fs, path::Path};

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.29 BPSR native finish patch {label} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn replace_all_exact(source: &mut String, from: &str, to: &str, expected: usize, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, expected, "v1.29 BPSR native finish patch {label} expected {expected} matches, found {count}");
    *source = source.replace(from, to);
}

fn replace_if_present(source: &mut String, from: &str, to: &str) {
    if source.contains(from) { *source = source.replace(from, to); }
}

fn patch_message_boxes(source: &mut String) {
    if !source.contains("MessageBoxW(") { return; }
    *source = source.replace("MessageBoxW(", "crate::ui_modern::message_box_w(");
    // Generated modules imported MessageBoxW only for these calls. Remove the
    // now-unused import so Clippy remains clean when warnings are denied.
    *source = source.replace("MessageBoxW,", "");
    *source = source.replace(", MessageBoxW", "");
}

fn mist_tokens(source: &mut String) {
    let replacements = [
        ("crate::ui_theme::BORDER_STRONG", "crate::ui_modern::MIST_BORDER_STRONG"),
        ("crate::ui_theme::TEXT_SECONDARY", "crate::ui_modern::BPSR_TEXT_SECONDARY"),
        ("crate::ui_theme::TEXT_DISABLED", "crate::ui_modern::BPSR_DISABLED"),
        ("crate::ui_theme::ACCENT_PRESSED", "crate::ui_modern::BPSR_ACCENT_PRESSED"),
        ("crate::ui_theme::ACCENT_HOVER", "crate::ui_modern::BPSR_ACCENT_HOVER"),
        ("crate::ui_theme::ACCENT_TEXT", "crate::ui_modern::BPSR_ACCENT_TEXT"),
        ("crate::ui_theme::SURFACE_PRESSED", "crate::ui_modern::MIST_PRESSED"),
        ("crate::ui_theme::SURFACE_HOVER", "crate::ui_modern::MIST_HOVER"),
        ("crate::ui_theme::SIDEBAR", "crate::ui_modern::MIST_SIDEBAR"),
        ("crate::ui_theme::SURFACE", "crate::ui_modern::MIST_SURFACE"),
        ("crate::ui_theme::RAISED", "crate::ui_modern::MIST_RAISED"),
        ("crate::ui_theme::INPUT", "crate::ui_modern::MIST_INPUT"),
        ("crate::ui_theme::BORDER", "crate::ui_modern::MIST_BORDER"),
        ("crate::ui_theme::MUTED", "crate::ui_modern::BPSR_MUTED"),
        ("crate::ui_theme::ACCENT", "crate::ui_modern::BPSR_ACCENT"),
        ("crate::ui_theme::TEXT", "crate::ui_modern::BPSR_TEXT"),
        ("crate::ui_theme::BG", "crate::ui_modern::MIST_BG"),
    ];
    for (from, to) in replacements { replace_if_present(source, from, to); }
    replace_if_present(source, "rgb(22, 25, 30)", "crate::ui_modern::MIST_BG");
    replace_if_present(source, "rgb(31, 36, 43)", "crate::ui_modern::MIST_INPUT");
    replace_if_present(source, "rgb(228, 232, 238)", "crate::ui_modern::BPSR_TEXT");
    replace_if_present(source, "rgb(171, 181, 194)", "crate::ui_modern::BPSR_TEXT_SECONDARY");
}

fn patch_settings(out: &Path) {
    let path = out.join("settings_ui_v1160_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read generated Settings UI for BPSR native finish").replace("\r\n", "\n");
    replace_once(&mut source,"crate::ui_theme::theme_control(c);","crate::ui_modern::theme_control(c);","Settings control theming");
    replace_once(&mut source,"crate::ui_theme::theme_combo(c);","crate::ui_modern::theme_combo(c);","Settings combo theming");
    replace_once(&mut source,"crate::ui_theme::draw_combo(item)","crate::ui_modern::draw_combo(item)","Settings combo owner draw");
    replace_once(&mut source,"crate::ui_theme::draw_button(item,selected,primary,danger,nav)","crate::ui_modern::draw_button(item,selected,primary,danger,nav)","Settings button owner draw");
    replace_once(&mut source,r#"    let c = create_control(hwnd, "LISTBOX", "", id, x, y, w, h, WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_VSCROLL | LBS_NOTIFY | WS_BORDER, 0); state.page_controls.push((c, page));"#,r#"    let c = create_control(hwnd, "LISTBOX", "", id, x, y, w, h, WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_VSCROLL | LBS_NOTIFY, 0); state.page_controls.push((c, page));"#,"Settings listbox legacy border");
    replace_if_present(&mut source, "try_dark_titlebar(hwnd);", "crate::ui_modern::mist_titlebar(hwnd);");
    replace_if_present(&mut source, "crate::ui_theme::dark_titlebar(hwnd);", "crate::ui_modern::mist_titlebar(hwnd);");
    replace_if_present(&mut source, "crate::ui_theme::bg_brush()", "crate::ui_modern::mist_bg_brush()");
    replace_if_present(&mut source, "crate::ui_theme::input_brush()", "crate::ui_modern::mist_input_brush()");
    mist_tokens(&mut source);
    patch_message_boxes(&mut source);
    fs::write(path, source).expect("write BPSR Settings UI");
}

fn patch_event_tracker(out: &Path) {
    let path = out.join("event_tracker_ui_v1160_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read generated Event Tracker UI for BPSR native finish").replace("\r\n", "\n");
    replace_once(&mut source,"crate::ui_theme::theme_control(c);","crate::ui_modern::theme_control(c);","Event Tracker control theming");
    replace_once(&mut source,"crate::ui_theme::theme_combo(c);","crate::ui_modern::theme_combo(c);","Event Tracker combo theming");
    replace_once(&mut source,"crate::ui_theme::draw_combo(item)","crate::ui_modern::draw_combo(item)","Event Tracker combo owner draw");
    replace_once(&mut source,"crate::ui_theme::draw_button(item,false,id==ID_APPLY,id==ID_REMOVE,false)","crate::ui_modern::draw_button(item,false,id==ID_APPLY,id==ID_REMOVE,false)","Event Tracker button owner draw");
    // Earlier modernization stages can already remove this border. This is a
    // cosmetic cleanup, so make it idempotent rather than fail the build chain.
    replace_if_present(&mut source,r#"    create_control(hwnd, "LISTBOX", "", id, x, y, w, h, WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_VSCROLL | 0x00100000 | LBS_NOTIFY | WS_BORDER, 0)"#,r#"    create_control(hwnd, "LISTBOX", "", id, x, y, w, h, WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_VSCROLL | 0x00100000 | LBS_NOTIFY, 0)"#);
    replace_if_present(&mut source, "try_dark_titlebar(hwnd);", "crate::ui_modern::mist_titlebar(hwnd);");
    replace_if_present(&mut source, "crate::ui_theme::dark_titlebar(hwnd);", "crate::ui_modern::mist_titlebar(hwnd);");
    mist_tokens(&mut source);
    patch_message_boxes(&mut source);
    fs::write(path, source).expect("write BPSR Event Tracker UI");
}

fn patch_feature_overlays(out: &Path) {
    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read generated feature overlays for BPSR native finish").replace("\r\n", "\n");
    replace_once(&mut source,"crate::ui_theme::theme_control(c);c","crate::ui_modern::theme_control(c);c","feature settings control theming");
    replace_all_exact(&mut source,"crate::ui_theme::theme_combo(c);","crate::ui_modern::theme_combo(c);",2,"feature settings combo theming");
    replace_once(&mut source,"crate::ui_theme::draw_combo(item)","crate::ui_modern::draw_combo(item)","feature settings combo owner draw");
    replace_once(&mut source,"crate::ui_theme::draw_button(item,false,false,false,false)","crate::ui_modern::draw_button(item,false,false,false,false)","feature settings button owner draw");
    replace_once(&mut source,"fill(hdc,&r,crate::ui_theme::BG);outline(hdc,r,crate::ui_theme::BORDER_STRONG,1);","crate::ui_modern::fill_round_rect(hdc,r,crate::ui_modern::DARK_BG,crate::ui_modern::RADIUS_SMALL);crate::ui_modern::stroke_round_rect(hdc,r,crate::ui_modern::DARK_BORDER_STRONG,crate::ui_modern::RADIUS_SMALL,1);","wrapped tooltip surface");
    replace_once(&mut source,"fill(hdc,&target_r,crate::ui_theme::SURFACE);","crate::ui_modern::fill_round_rect(hdc,target_r,crate::ui_modern::DARK_SURFACE,crate::ui_modern::RADIUS_SMALL);","target summary surface");
    replace_once(&mut source,r#"    let back=if active{crate::ui_theme::ACCENT}else if hovered{crate::ui_theme::SURFACE_HOVER}else{crate::ui_theme::SURFACE};
    fill(hdc,&r,back);SetTextColor(hdc,if active{rgb(248,253,252)}else{crate::ui_theme::TEXT});"#,r#"    let back=if active{crate::ui_modern::BPSR_ACCENT}else if hovered{crate::ui_modern::DARK_HOVER}else{crate::ui_modern::DARK_SURFACE};
    crate::ui_modern::fill_round_rect(hdc,r,back,crate::ui_modern::RADIUS_SMALL);SetTextColor(hdc,if active{crate::ui_modern::BPSR_ACCENT_TEXT}else{crate::ui_modern::BPSR_TEXT});"#,"DPS toolbar BPSR actions");
    replace_once(&mut source,r#"unsafe fn paint_tab(hdc:HDC,x:i32,y:i32,width:i32,text:&str,active:bool){let r=RECT{left:x,top:y,right:x+width,bottom:y+23};fill(hdc,&r,if active{crate::ui_theme::ACCENT}else{crate::ui_theme::SURFACE});SetTextColor(hdc,if active{rgb(248,253,252)}else{crate::ui_theme::TEXT_SECONDARY});draw(hdc,text,r,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}"#,r#"unsafe fn paint_tab(hdc:HDC,x:i32,y:i32,width:i32,text:&str,active:bool){let r=RECT{left:x,top:y,right:x+width,bottom:y+23};crate::ui_modern::fill_round_rect(hdc,r,if active{crate::ui_modern::SELECTED_SURFACE}else{crate::ui_modern::DARK_SURFACE},crate::ui_modern::RADIUS_SMALL);crate::ui_modern::stroke_round_rect(hdc,r,if active{crate::ui_modern::BPSR_ACCENT}else{crate::ui_modern::DARK_BORDER},crate::ui_modern::RADIUS_SMALL,1);SetTextColor(hdc,if active{crate::ui_modern::BPSR_TEXT}else{crate::ui_modern::BPSR_TEXT_SECONDARY});draw(hdc,text,r,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}"#,"detail tab BPSR state");

    // Map inherited overlay chrome/text literals onto the dark BPSR glass family.
    for (from,to) in [
        ("rgb(18,22,27)","crate::ui_modern::DARK_BG"),
        ("rgb(23,28,35)","crate::ui_modern::DARK_SURFACE"),
        ("rgb(27,32,39)","crate::ui_modern::DARK_RAISED"),
        ("rgb(28,33,40)","crate::ui_modern::DARK_RAISED"),
        ("rgb(24,29,35)","crate::ui_modern::DARK_SURFACE"),
        ("rgb(31,37,45)","crate::ui_modern::DARK_RAISED"),
        ("rgb(42,50,60)","crate::ui_modern::DARK_HOVER"),
        ("rgb(45,52,61)","crate::ui_modern::DARK_BORDER"),
        ("rgb(63,133,255)","crate::ui_modern::BPSR_ACCENT"),
        ("rgb(99,199,255)","crate::ui_modern::BPSR_ACCENT"),
        ("rgb(231,237,244)","crate::ui_modern::BPSR_TEXT"),
        ("rgb(238,242,247)","crate::ui_modern::BPSR_TEXT"),
        ("rgb(235,240,246)","crate::ui_modern::BPSR_TEXT"),
        ("rgb(205,215,227)","crate::ui_modern::BPSR_TEXT_SECONDARY"),
        ("rgb(210,220,231)","crate::ui_modern::BPSR_TEXT_SECONDARY"),
        ("rgb(220,229,240)","crate::ui_modern::BPSR_TEXT_SECONDARY"),
        ("rgb(170,183,199)","crate::ui_modern::BPSR_TEXT_SECONDARY"),
        ("rgb(145,160,180)","crate::ui_modern::BPSR_MUTED"),
        ("rgb(132,145,162)","crate::ui_modern::BPSR_MUTED"),
    ] { replace_if_present(&mut source,from,to); }

    // The feature settings popup is configuration, so override the otherwise
    // dark overlay canvas with Mist Glass while keeping its compact footprint.
    replace_if_present(&mut source,
        "unsafe fn paint_feature_settings(hwnd:HWND,state:&SettingsState){let mut ps:PAINTSTRUCT=std::mem::zeroed();let hdc=BeginPaint(hwnd,&mut ps);if hdc.is_null(){return;}let mut rc:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rc);fill(hdc,&rc,crate::ui_modern::DARK_BG);",
        "unsafe fn paint_feature_settings(hwnd:HWND,state:&SettingsState){let mut ps:PAINTSTRUCT=std::mem::zeroed();let hdc=BeginPaint(hwnd,&mut ps);if hdc.is_null(){return;}let mut rc:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rc);fill(hdc,&rc,crate::ui_modern::MIST_BG);");
    patch_message_boxes(&mut source);
    fs::write(path, source).expect("write BPSR feature overlays");
}

fn patch_chat(out: &Path) {
    let path = out.join("overlay_v150_v1181.rs");
    let mut source = fs::read_to_string(&path).expect("read generated Chat Overlay for BPSR native finish").replace("\r\n", "\n");
    replace_once(&mut source,"        fill(hdc, &rect, crate::ui_theme::ACCENT);","        crate::ui_modern::fill_round_rect(hdc, rect, crate::ui_modern::BPSR_ACCENT_PRESSED, crate::ui_modern::RADIUS_SMALL);","Chat jump-to-latest button");
    replace_once(&mut source,r#"unsafe fn draw_toolbar_button(hdc: HDC, rect: RECT, text: &str, back: u32, fore: u32) {
    fill(hdc, &rect, back);
    draw_text(hdc, text, rect, fore, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX, false);
}"#,r#"unsafe fn draw_toolbar_button(hdc: HDC, rect: RECT, text: &str, back: u32, fore: u32) {
    crate::ui_modern::fill_round_rect(hdc, rect, back, crate::ui_modern::RADIUS_SMALL);
    crate::ui_modern::stroke_round_rect(hdc, rect, crate::ui_modern::DARK_BORDER, crate::ui_modern::RADIUS_SMALL, 1);
    draw_text(hdc, text, rect, fore, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX, false);
}"#,"Chat toolbar buttons");
    replace_once(&mut source,r#"unsafe fn draw_toolbar_button_sized(hdc:HDC,rect:RECT,text:&str,back:u32,fore:u32,pixel_height:i32){
    fill(hdc,&rect,back);"#,r#"unsafe fn draw_toolbar_button_sized(hdc:HDC,rect:RECT,text:&str,back:u32,fore:u32,pixel_height:i32){
    crate::ui_modern::fill_round_rect(hdc,rect,back,crate::ui_modern::RADIUS_SMALL);crate::ui_modern::stroke_round_rect(hdc,rect,crate::ui_modern::DARK_BORDER,crate::ui_modern::RADIUS_SMALL,1);"#,"Chat toolbar symbol buttons");
    replace_once(&mut source,r#"for(index,rect)in tab_rects(settings,max_right).into_iter().enumerate(){let Some(tab)=settings.chat.tabs.get(index)else{break;};let selected=tab.id==settings.chat.last_selected_tab_id;if selected{fill(hdc,&rect,crate::ui_theme::SURFACE_HOVER);fill(hdc,&RECT{left:rect.left+10,top:rect.bottom-3,right:rect.right-10,bottom:rect.bottom},crate::ui_theme::ACCENT);}let color=if selected{crate::ui_theme::TEXT}else{crate::ui_theme::TEXT_SECONDARY};draw_text(hdc,&tab.name,rect,color,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_END_ELLIPSIS|DT_NOPREFIX,false);}"#,r#"for(index,rect)in tab_rects(settings,max_right).into_iter().enumerate(){let Some(tab)=settings.chat.tabs.get(index)else{break;};let selected=tab.id==settings.chat.last_selected_tab_id;if selected{crate::ui_modern::fill_round_rect(hdc,rect,crate::ui_modern::SELECTED_SURFACE,crate::ui_modern::RADIUS_SMALL);fill(hdc,&RECT{left:rect.left+10,top:rect.bottom-2,right:rect.right-10,bottom:rect.bottom},crate::ui_modern::BPSR_ACCENT);}let color=if selected{crate::ui_modern::BPSR_TEXT}else{crate::ui_modern::BPSR_TEXT_SECONDARY};draw_text(hdc,&tab.name,rect,color,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_END_ELLIPSIS|DT_NOPREFIX,false);}"#,"Chat selected tab");
    replace_once(&mut source,r#"if let Some(rect)=tab_overflow_rect(settings,max_right){fill(hdc,&rect,crate::ui_theme::SURFACE);draw_text(hdc,"…",rect,crate::ui_theme::TEXT_SECONDARY,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX,false);}"#,r#"if let Some(rect)=tab_overflow_rect(settings,max_right){crate::ui_modern::fill_round_rect(hdc,rect,crate::ui_modern::DARK_SURFACE,crate::ui_modern::RADIUS_SMALL);draw_text(hdc,"…",rect,crate::ui_modern::BPSR_TEXT_SECONDARY,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX,false);}"#,"Chat tab overflow");

    for (from,to) in [
        ("rgb(20,23,28)","crate::ui_modern::DARK_BG"),
        ("rgb(20, 23, 28)","crate::ui_modern::DARK_BG"),
        ("rgb(26,30,36)","crate::ui_modern::DARK_RAISED"),
        ("rgb(26, 30, 36)","crate::ui_modern::DARK_RAISED"),
        ("rgb(31,36,44)","crate::ui_modern::DARK_HOVER"),
        ("rgb(41,94,180)","crate::ui_modern::BPSR_ACCENT_PRESSED"),
        ("rgb(73,132,255)","crate::ui_modern::BPSR_ACCENT"),
        ("rgb(75, 86, 99)","crate::ui_modern::DARK_BORDER_STRONG"),
        ("rgb(75,86,99)","crate::ui_modern::DARK_BORDER_STRONG"),
        ("rgb(239,243,247)","crate::ui_modern::BPSR_TEXT"),
        ("rgb(170,181,194)","crate::ui_modern::BPSR_TEXT_SECONDARY"),
        ("rgb(157,170,188)","crate::ui_modern::BPSR_MUTED"),
        ("rgb(143,203,255)","crate::ui_modern::BPSR_ACCENT"),
    ] { replace_if_present(&mut source,from,to); }
    replace_if_present(&mut source, "try_dark_titlebar(hwnd);", "crate::ui_modern::dark_titlebar(hwnd);");
    fs::write(path, source).expect("write BPSR Chat Overlay");
}

fn patch_win_and_updater(out: &Path) {
    for name in ["win_v182_fixed.rs", "updater_v1241.rs"] {
        let path = out.join(name);
        if !path.exists() { continue; }
        let mut source = fs::read_to_string(&path).expect("read generated modal owner").replace("\r\n", "\n");
        patch_message_boxes(&mut source);
        fs::write(path, source).expect("write BPSR native modal owner");
    }
}

pub fn run(out: &Path) {
    patch_settings(out);
    patch_event_tracker(out);
    patch_feature_overlays(out);
    patch_chat(out);
    patch_win_and_updater(out);
}
