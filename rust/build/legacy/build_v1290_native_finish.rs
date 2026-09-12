use std::{fs, path::Path};

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.29 native finish patch {label} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn replace_all_exact(source: &mut String, from: &str, to: &str, expected: usize, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, expected, "v1.29 native finish patch {label} expected {expected} matches, found {count}");
    *source = source.replace(from, to);
}

fn patch_settings(out: &Path) {
    let path = out.join("settings_ui_v1160_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read generated Settings UI for final native polish").replace("\r\n", "\n");
    replace_once(&mut source,"crate::ui_theme::theme_control(c);","crate::ui_modern::theme_control(c);","Settings control theming");
    replace_once(&mut source,"crate::ui_theme::theme_combo(c);","crate::ui_modern::theme_combo(c);","Settings combo theming");
    replace_once(&mut source,"crate::ui_theme::draw_combo(item)","crate::ui_modern::draw_combo(item)","Settings combo owner draw");
    replace_once(&mut source,"crate::ui_theme::draw_button(item,selected,primary,danger,nav)","crate::ui_modern::draw_button(item,selected,primary,danger,nav)","Settings button owner draw");
    replace_once(&mut source,r#"    let c = create_control(hwnd, "LISTBOX", "", id, x, y, w, h, WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_VSCROLL | LBS_NOTIFY | WS_BORDER, 0); state.page_controls.push((c, page));"#,r#"    let c = create_control(hwnd, "LISTBOX", "", id, x, y, w, h, WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_VSCROLL | LBS_NOTIFY, 0); state.page_controls.push((c, page));"#,"Settings listbox legacy border");
    fs::write(path, source).expect("write final polished generated Settings UI");
}

fn patch_event_tracker(out: &Path) {
    let path = out.join("event_tracker_ui_v1160_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read generated Event Tracker UI for final native polish").replace("\r\n", "\n");
    replace_once(&mut source,"crate::ui_theme::theme_control(c);","crate::ui_modern::theme_control(c);","Event Tracker control theming");
    replace_once(&mut source,"crate::ui_theme::theme_combo(c);","crate::ui_modern::theme_combo(c);","Event Tracker combo theming");
    replace_once(&mut source,"crate::ui_theme::draw_combo(item)","crate::ui_modern::draw_combo(item)","Event Tracker combo owner draw");
    replace_once(&mut source,"crate::ui_theme::draw_button(item,false,id==ID_APPLY,id==ID_REMOVE,false)","crate::ui_modern::draw_button(item,false,id==ID_APPLY,id==ID_REMOVE,false)","Event Tracker button owner draw");
    replace_once(&mut source,r#"    create_control(hwnd, "LISTBOX", "", id, x, y, w, h, WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_VSCROLL | 0x00100000 | LBS_NOTIFY | WS_BORDER, 0)"#,r#"    create_control(hwnd, "LISTBOX", "", id, x, y, w, h, WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_VSCROLL | 0x00100000 | LBS_NOTIFY, 0)"#,"Event Tracker listbox legacy border");
    fs::write(path, source).expect("write final polished generated Event Tracker UI");
}

fn patch_feature_overlays(out: &Path) {
    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read generated feature overlays for final native polish").replace("\r\n", "\n");
    replace_once(&mut source,"crate::ui_theme::theme_control(c);c","crate::ui_modern::theme_control(c);c","feature settings control theming");
    replace_all_exact(&mut source,"crate::ui_theme::theme_combo(c);","crate::ui_modern::theme_combo(c);",2,"feature settings combo theming");
    replace_once(&mut source,"crate::ui_theme::draw_combo(item)","crate::ui_modern::draw_combo(item)","feature settings combo owner draw");
    replace_once(&mut source,"crate::ui_theme::draw_button(item,false,false,false,false)","crate::ui_modern::draw_button(item,false,false,false,false)","feature settings button owner draw");
    replace_once(&mut source,"fill(hdc,&r,crate::ui_theme::BG);outline(hdc,r,crate::ui_theme::BORDER_STRONG,1);","crate::ui_modern::fill_round_rect(hdc,r,crate::ui_theme::BG,crate::ui_modern::RADIUS_SMALL);crate::ui_modern::stroke_round_rect(hdc,r,crate::ui_theme::BORDER_STRONG,crate::ui_modern::RADIUS_SMALL,1);","wrapped tooltip surface");
    replace_once(&mut source,"fill(hdc,&target_r,crate::ui_theme::SURFACE);","crate::ui_modern::fill_round_rect(hdc,target_r,crate::ui_theme::SURFACE,crate::ui_modern::RADIUS_SMALL);","target summary surface");
    replace_once(&mut source,r#"    let back=if active{crate::ui_theme::ACCENT}else if hovered{crate::ui_theme::SURFACE_HOVER}else{crate::ui_theme::SURFACE};
    fill(hdc,&r,back);SetTextColor(hdc,if active{rgb(248,253,252)}else{crate::ui_theme::TEXT});"#,r#"    let back=if active{crate::ui_theme::ACCENT}else if hovered{crate::ui_theme::SURFACE_HOVER}else{crate::ui_theme::SURFACE};
    crate::ui_modern::fill_round_rect(hdc,r,back,crate::ui_modern::RADIUS_SMALL);SetTextColor(hdc,if active{crate::ui_theme::ACCENT_TEXT}else{crate::ui_theme::TEXT});"#,"DPS toolbar tonal actions");
    replace_once(&mut source,r#"unsafe fn paint_tab(hdc:HDC,x:i32,y:i32,width:i32,text:&str,active:bool){let r=RECT{left:x,top:y,right:x+width,bottom:y+23};fill(hdc,&r,if active{crate::ui_theme::ACCENT}else{crate::ui_theme::SURFACE});SetTextColor(hdc,if active{rgb(248,253,252)}else{crate::ui_theme::TEXT_SECONDARY});draw(hdc,text,r,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}"#,r#"unsafe fn paint_tab(hdc:HDC,x:i32,y:i32,width:i32,text:&str,active:bool){let r=RECT{left:x,top:y,right:x+width,bottom:y+23};crate::ui_modern::fill_round_rect(hdc,r,if active{crate::ui_modern::SELECTED_SURFACE}else{crate::ui_theme::SURFACE},crate::ui_modern::RADIUS_SMALL);SetTextColor(hdc,if active{crate::ui_theme::TEXT}else{crate::ui_theme::TEXT_SECONDARY});draw(hdc,text,r,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}"#,"detail tab tonal state");
    fs::write(path, source).expect("write final polished generated feature overlays");
}

fn patch_chat(out: &Path) {
    let path = out.join("overlay_v150_v1181.rs");
    let mut source = fs::read_to_string(&path).expect("read generated Chat Overlay for final native polish").replace("\r\n", "\n");
    replace_once(&mut source,"        fill(hdc, &rect, crate::ui_theme::ACCENT);","        crate::ui_modern::fill_round_rect(hdc, rect, crate::ui_theme::ACCENT, crate::ui_modern::RADIUS_SMALL);","Chat jump-to-latest pill");
    replace_once(&mut source,r#"unsafe fn draw_toolbar_button(hdc: HDC, rect: RECT, text: &str, back: u32, fore: u32) {
    fill(hdc, &rect, back);
    draw_text(hdc, text, rect, fore, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX, false);
}"#,r#"unsafe fn draw_toolbar_button(hdc: HDC, rect: RECT, text: &str, back: u32, fore: u32) {
    crate::ui_modern::fill_round_rect(hdc, rect, back, crate::ui_modern::RADIUS_SMALL);
    draw_text(hdc, text, rect, fore, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX, false);
}"#,"Chat toolbar buttons");
    replace_once(&mut source,r#"unsafe fn draw_toolbar_button_sized(hdc:HDC,rect:RECT,text:&str,back:u32,fore:u32,pixel_height:i32){
    fill(hdc,&rect,back);"#,r#"unsafe fn draw_toolbar_button_sized(hdc:HDC,rect:RECT,text:&str,back:u32,fore:u32,pixel_height:i32){
    crate::ui_modern::fill_round_rect(hdc,rect,back,crate::ui_modern::RADIUS_SMALL);"#,"Chat toolbar symbol buttons");
    replace_once(&mut source,r#"for(index,rect)in tab_rects(settings,max_right).into_iter().enumerate(){let Some(tab)=settings.chat.tabs.get(index)else{break;};let selected=tab.id==settings.chat.last_selected_tab_id;if selected{fill(hdc,&rect,crate::ui_theme::SURFACE_HOVER);fill(hdc,&RECT{left:rect.left+10,top:rect.bottom-3,right:rect.right-10,bottom:rect.bottom},crate::ui_theme::ACCENT);}let color=if selected{crate::ui_theme::TEXT}else{crate::ui_theme::TEXT_SECONDARY};draw_text(hdc,&tab.name,rect,color,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_END_ELLIPSIS|DT_NOPREFIX,false);}"#,r#"for(index,rect)in tab_rects(settings,max_right).into_iter().enumerate(){let Some(tab)=settings.chat.tabs.get(index)else{break;};let selected=tab.id==settings.chat.last_selected_tab_id;if selected{crate::ui_modern::fill_round_rect(hdc,rect,crate::ui_modern::SELECTED_SURFACE,crate::ui_modern::RADIUS_SMALL);fill(hdc,&RECT{left:rect.left+10,top:rect.bottom-3,right:rect.right-10,bottom:rect.bottom},crate::ui_theme::ACCENT);}let color=if selected{crate::ui_theme::TEXT}else{crate::ui_theme::TEXT_SECONDARY};draw_text(hdc,&tab.name,rect,color,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_END_ELLIPSIS|DT_NOPREFIX,false);}"#,"Chat selected tab");
    replace_once(&mut source,r#"if let Some(rect)=tab_overflow_rect(settings,max_right){fill(hdc,&rect,crate::ui_theme::SURFACE);draw_text(hdc,"…",rect,crate::ui_theme::TEXT_SECONDARY,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX,false);}"#,r#"if let Some(rect)=tab_overflow_rect(settings,max_right){crate::ui_modern::fill_round_rect(hdc,rect,crate::ui_theme::SURFACE,crate::ui_modern::RADIUS_SMALL);draw_text(hdc,"…",rect,crate::ui_theme::TEXT_SECONDARY,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX,false);}"#,"Chat tab overflow");
    fs::write(path, source).expect("write final polished generated Chat Overlay");
}

pub fn run(out: &Path) {
    patch_settings(out);
    patch_event_tracker(out);
    patch_feature_overlays(out);
    patch_chat(out);
}
