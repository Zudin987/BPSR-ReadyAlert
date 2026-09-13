use std::{env, fs, path::{Path, PathBuf}};

mod previous {
    include!("build_pixel_phase3.rs");
    pub fn run() { main(); }
}

fn replace_required(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "strict UI QA target {label:?} expected once, found {count}");
    *source = source.replacen(from, to, 1);
}

fn replace_if_present(source: &mut String, from: &str, to: &str) {
    if source.contains(from) { *source = source.replace(from, to); }
}

fn patch_common(source: &mut String) {
    // Route compatibility messages to the responsive QA modal. This keeps result IDs
    // and command semantics intact while fixing sizing, action spacing and destructive emphasis.
    replace_if_present(
        source,
        "crate::telemetry::ui_audit_v1302::message_box_w",
        "crate::ui_modern::qa_message_box_w",
    );
}

fn patch_settings(source: &mut String) {
    replace_required(
        source,
        "create_button(hwnd, NAV_BASE + i as i32, name, 10, 14 + i as i32 * 38, 136, 30);",
        "create_button(hwnd, NAV_BASE + i as i32, name, 10, 14 + i as i32 * 38, 144, 30);",
        "settings navigation width",
    );
    replace_required(
        source,
        "checkbox_at(hwnd,state,PAGE_SPEECH,ID_TRANSLATE_PARTY,\"Party / Team\",338,116,100);",
        "checkbox_at(hwnd,state,PAGE_SPEECH,ID_TRANSLATE_PARTY,\"Party / Team\",338,116,118);",
        "translation Party Team fit",
    );
    replace_required(
        source,
        "checkbox_at(hwnd,state,PAGE_SPEECH,ID_TTS_PARTY,\"Party / Team\",558,116,110);",
        "checkbox_at(hwnd,state,PAGE_SPEECH,ID_TTS_PARTY,\"Party / Team\",558,116,120);",
        "TTS Party Team fit",
    );
    replace_required(
        source,
        "let apply=create_button(hwnd,ID_APPLY,\"Apply\",610,365,80,30);EnableWindow(apply,0);",
        "let apply=create_button(hwnd,ID_APPLY,\"Apply\",600,365,80,30);EnableWindow(apply,0);",
        "settings Apply margin",
    );
    replace_required(
        source,
        "create_button(hwnd, ID_CLOSE, \"Close\", 700, 365, 80, 30);",
        "create_button(hwnd, ID_CLOSE, \"Close\", 690, 365, 80, 30);",
        "settings Close margin",
    );
    replace_required(
        source,
        "let c=create_control(hwnd,\"COMBOBOX\",\"\",id,x,y,w,h,WS_CHILD|WS_VISIBLE|WS_TABSTOP|WS_VSCROLL|CBS_DROPDOWNLIST|0x0010|0x0200,0);crate::ui_modern::theme_combo(c);state.page_controls.push((c,page));",
        "let c=create_control(hwnd,\"COMBOBOX\",\"\",id,x,y,w,h,WS_CHILD|WS_VISIBLE|WS_TABSTOP|WS_VSCROLL|CBS_DROPDOWNLIST|0x0010|0x0200,0);crate::ui_modern::qa_theme_mist_combo(c);state.page_controls.push((c,page));",
        "settings combo finish",
    );
    replace_required(
        source,
        "let c = create_control(hwnd, \"LISTBOX\", \"\", id, x, y, w, h, WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_VSCROLL | LBS_NOTIFY, 0); state.page_controls.push((c, page));",
        "let c = create_control(hwnd, \"LISTBOX\", \"\", id, x, y, w, h, WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_VSCROLL | LBS_NOTIFY, 0); let empty_kind=if id==ID_BLOCKED_LIST{2}else if id==ID_TAB_LIST{1}else{0};if empty_kind!=0{crate::ui_modern::qa_theme_empty_list(c,empty_kind);}state.page_controls.push((c, page));",
        "settings empty list states",
    );
    replace_required(
        source,
        "unsafe fn create_button(hwnd: HWND, id: i32, text: &str, x: i32, y: i32, w: i32, h: i32) -> HWND { create_control(hwnd, \"BUTTON\", text, id, x, y, w, h, WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON | 0x0000_000B, 0) }",
        "unsafe fn create_button(hwnd: HWND, id: i32, text: &str, x: i32, y: i32, w: i32, h: i32) -> HWND { let c=create_control(hwnd, \"BUTTON\", text, id, x, y, w, h, WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON | 0x0000_000B, 0);crate::ui_modern::qa_prepare_owner_button(c);c }",
        "settings owner button preparation",
    );
    replace_required(
        source,
        "crate::ui_modern::draw_button(item,selected,primary,danger,nav)",
        "crate::ui_modern::qa_draw_button(item,selected,primary,danger,nav)",
        "settings clean button renderer",
    );
}

fn patch_event_tracker(source: &mut String) {
    replace_required(
        source,
        "create_static(hwnd,\"Scope\",314,272,90,20);create_combo(hwnd,ID_SCOPE,408,266,150,120);",
        "create_static(hwnd,\"Scope\",314,272,90,20);create_combo(hwnd,ID_SCOPE,408,266,200,120);",
        "event tracker Scope fit",
    );
    replace_required(
        source,
        "crate::ui_modern::theme_combo(c);",
        "crate::ui_modern::qa_theme_mist_combo(c);",
        "event tracker combo finish",
    );
    replace_required(
        source,
        "unsafe fn create_listbox(hwnd: HWND, id: i32, x: i32, y: i32, w: i32, h: i32) -> HWND {\n    create_control(hwnd, \"LISTBOX\", \"\", id, x, y, w, h, WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_VSCROLL | 0x00100000 | LBS_NOTIFY, 0)\n}",
        "unsafe fn create_listbox(hwnd: HWND, id: i32, x: i32, y: i32, w: i32, h: i32) -> HWND {\n    let c=create_control(hwnd, \"LISTBOX\", \"\", id, x, y, w, h, WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_VSCROLL | LBS_NOTIFY, 0);crate::ui_modern::qa_theme_empty_list(c,3);c\n}",
        "event tracker list surface",
    );
    replace_required(
        source,
        "unsafe fn create_button(hwnd: HWND, id: i32, text: &str, x: i32, y: i32, w: i32, h: i32) -> HWND {\n    create_control(hwnd, \"BUTTON\", text, id, x, y, w, h, WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON | 0x0000_000B, 0)\n}",
        "unsafe fn create_button(hwnd: HWND, id: i32, text: &str, x: i32, y: i32, w: i32, h: i32) -> HWND {\n    let c=create_control(hwnd, \"BUTTON\", text, id, x, y, w, h, WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON | 0x0000_000B, 0);crate::ui_modern::qa_prepare_owner_button(c);c\n}",
        "event tracker owner button preparation",
    );
    replace_required(
        source,
        "crate::ui_modern::draw_button(item,false,id==ID_APPLY,id==ID_REMOVE,false)",
        "crate::ui_modern::qa_draw_button(item,false,id==ID_APPLY,id==ID_REMOVE,false)",
        "event tracker clean button renderer",
    );
}

fn patch_feature_overlays(source: &mut String) {
    replace_required(
        source,
        "let status_w=if row.is_dead{dps_adaptive_logical_px(76,scale)}else{0};",
        "let status_w=if row.is_dead{dps_adaptive_logical_px(94,scale)}else{0};",
        "compact CAN REVIVE fit",
    );
    replace_required(
        source,
        "let c=feature_control(hwnd,\"COMBOBOX\",id,\"\",x,y,155,180,0x00210213);crate::ui_modern::theme_combo(c);",
        "let c=feature_control(hwnd,\"COMBOBOX\",id,\"\",x,y,155,180,0x00210213);crate::ui_modern::qa_theme_dark_combo(c);",
        "feature combo finish fixed width",
    );
    replace_required(
        source,
        "let c=feature_control(hwnd,\"COMBOBOX\",id,\"\",x,y,w,180,0x00210213);crate::ui_modern::theme_combo(c);",
        "let c=feature_control(hwnd,\"COMBOBOX\",id,\"\",x,y,w,180,0x00210213);crate::ui_modern::qa_theme_dark_combo(c);",
        "feature combo finish sized",
    );
    replace_required(
        source,
        "let c=feature_control(hwnd,\"msctls_trackbar32\",id,\"\",x,y,w,20,0x00000010);",
        "let c=feature_control(hwnd,\"msctls_trackbar32\",id,\"\",x,y,w,20,0x00000010);crate::ui_modern::qa_theme_dark_trackbar(c);",
        "feature custom trackbar",
    );
    replace_required(
        source,
        "unsafe fn feature_button(hwnd:HWND,id:i32,text:&str,x:i32,y:i32,w:i32) {feature_control(hwnd,\"BUTTON\",id,text,x,y,w,30,0x00010000|0x0000000B);}",
        "unsafe fn feature_button(hwnd:HWND,id:i32,text:&str,x:i32,y:i32,w:i32) {let c=feature_control(hwnd,\"BUTTON\",id,text,x,y,w,30,0x00010000|0x0000000B);crate::ui_modern::qa_prepare_owner_button(c);}",
        "feature owner button preparation",
    );
    replace_required(
        source,
        "crate::ui_modern::draw_button(item,false,false,false,false)",
        "crate::ui_modern::qa_draw_dark_button(item,false,false,false,false)",
        "feature clean button renderer",
    );
    replace_required(
        source,
        "crate::ui_modern::fill_round_rect(hdc,r,crate::ui_modern::DARK_BG,crate::ui_modern::RADIUS_SMALL);crate::ui_modern::stroke_round_rect(hdc,r,crate::ui_modern::DARK_BORDER_STRONG,crate::ui_modern::RADIUS_SMALL,1);",
        "crate::ui_modern::fill_round_rect(hdc,RECT{left:r.left+2,top:r.top+2,right:r.right+2,bottom:r.bottom+2},crate::ui_theme::rgb(8,9,12),crate::ui_modern::RADIUS_MEDIUM);crate::ui_modern::fill_round_rect(hdc,r,crate::ui_modern::DARK_RAISED,crate::ui_modern::RADIUS_MEDIUM);",
        "soft meter tooltip",
    );
    replace_required(
        source,
        "return Some(\"Target context • name/ID, live HP, Enrage / Power Seal timer and encounter time\".into());",
        "return Some(format!(\"{} • live HP • Enrage / Power Seal timer • encounter time\",target_title(view_snapshot(state))));",
        "full target tooltip",
    );
    replace_required(
        source,
        "fill(hdc,&RECT{left:r.left,top:bar_top,right:r.left+filled,bottom:bar_bottom},mode_color(state.sort_mode));",
        "fill(hdc,&RECT{left:r.left,top:bar_top,right:r.left+filled,bottom:bar_bottom},crate::ui_modern::mix_color(crate::ui_modern::DARK_RAISED,mode_color(state.sort_mode),55));",
        "regular row progress restraint",
    );
    replace_required(
        source,
        "let color=match state.sort_mode{SortMode::Damage=>rgb(235,74,74),SortMode::Heal=>rgb(55,205,105),SortMode::Tank=>rgb(65,145,235)};",
        "let color=crate::ui_modern::mix_color(crate::ui_modern::DARK_RAISED,mode_color(state.sort_mode),55);",
        "raid row progress restraint",
    );
    replace_required(
        source,
        "let half=(w/2).max(1);SetTextColor(hdc,food);draw(hdc,\"F\",RECT{left:x,top:y,right:x+half,bottom:y+h},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);SetTextColor(hdc,serum);draw(hdc,\"S\",RECT{left:x+half,top:y,right:x+w,bottom:y+h},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);SelectObject(hdc,old);",
        "let half=(w/2).max(1);if food!=crate::ui_modern::BPSR_MUTED{SetTextColor(hdc,food);draw(hdc,\"F\",RECT{left:x,top:y,right:x+half,bottom:y+h},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}if serum!=crate::ui_modern::BPSR_MUTED{SetTextColor(hdc,serum);draw(hdc,\"S\",RECT{left:x+half,top:y,right:x+w,bottom:y+h},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}SelectObject(hdc,old);",
        "raid consumable noise",
    );
    replace_required(
        source,
        "fill(hdc,&RECT{left:mid,top:top+2,right:mid+1,bottom:(top+RAID_ROWS_PER_COLUMN as i32*row_h).min(rc.bottom)},crate::ui_modern::DARK_RAISED);",
        "let gutter_bottom=(top+RAID_ROWS_PER_COLUMN as i32*row_h).min(rc.bottom);crate::ui_modern::fill_round_rect(hdc,RECT{left:mid-gap/2,top:top+2,right:mid+gap/2,bottom:gutter_bottom},crate::ui_modern::DARK_SURFACE,crate::ui_modern::RADIUS_SMALL);fill(hdc,&RECT{left:mid,top:top+4,right:mid+1,bottom:gutter_bottom-2},crate::ui_modern::DARK_BORDER);",
        "raid center gutter",
    );
    replace_required(
        source,
        "if state.kind==Kind::Dps{if !state.compact_mode&&hover_badge_at(hwnd,state,x,y).is_some(){return None;}",
        "if state.kind==Kind::Dps{if raid_active(state)&&!state.compact_mode{let scale=dps_layout_scale(state);let gap=dps_adaptive_logical_px(RAID_CENTER_GAP,scale).max(56);let mid=rc.right/2;if y>=dps_rows_top_for(state)&&x>=mid-gap/2&&x<=mid+gap/2{return Some(\"Raid consumables • F = Food • S = Serum\".into());}}if !state.compact_mode&&hover_badge_at(hwnd,state,x,y).is_some(){return None;}",
        "raid consumable tooltip",
    );
    replace_required(
        source,
        "DpsLayoutTier::Compact=>(24,24,24,30,28,28,28,28,\"Ra\",\"L\",\"Cp\",\"B\",\"Rs\"),DpsLayoutTier::Dense=>(22,22,20,28,26,26,26,26,\"Ra\",\"L\",\"Cp\",\"B\",\"Rs\"),DpsLayoutTier::Minimum=>(22,22,18,26,24,24,24,24,\"Ra\",\"L\",\"Cp\",\"B\",\"Rs\")",
        "DpsLayoutTier::Compact=>(28,28,28,36,32,32,32,32,\"20\",\"L\",\"Cp\",\"B\",\"Rs\"),DpsLayoutTier::Dense=>(26,26,26,34,30,30,30,30,\"20\",\"L\",\"Cp\",\"B\",\"Rs\"),DpsLayoutTier::Minimum=>(24,24,24,32,28,28,28,28,\"20\",\"L\",\"Cp\",\"B\",\"Rs\")",
        "meter compact toolbar grammar",
    );
    replace_required(
        source,
        "if state.compact_mode{take(ToolbarAction::More,24,\"...\");take(ToolbarAction::Compact,26,\"C\");take(ToolbarAction::Newer,20,\">\");take(ToolbarAction::Live,38,\"Live\");take(ToolbarAction::Older,20,\"<\");}",
        "if state.compact_mode{take(ToolbarAction::More,28,\"⋯\");take(ToolbarAction::Compact,28,\"C\");take(ToolbarAction::Newer,24,\"›\");take(ToolbarAction::Live,40,\"Live\");take(ToolbarAction::Older,24,\"‹\");}",
        "compact meter toolbar spacing",
    );
    replace_required(
        source,
        "unsafe fn paint_mode_tab(hdc:HDC,x:i32,y:i32,width:i32,text:&str,mode:SortMode,active:bool){let r=RECT{left:x,top:y,right:x+width,bottom:y+23};fill(hdc,&r,crate::ui_modern::DARK_SURFACE);if active{fill(hdc,&RECT{left:r.left,top:r.bottom-3,right:r.right,bottom:r.bottom},mode_color(mode));}SetTextColor(hdc,if active{crate::ui_modern::BPSR_TEXT}else{crate::ui_modern::BPSR_TEXT_SECONDARY});draw(hdc,text,r,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}",
        "unsafe fn paint_mode_tab(hdc:HDC,x:i32,y:i32,width:i32,text:&str,mode:SortMode,active:bool){let r=RECT{left:x,top:y,right:x+width,bottom:y+23};if active{crate::ui_modern::fill_round_rect(hdc,r,crate::ui_modern::mix_color(crate::ui_modern::DARK_RAISED,mode_color(mode),18),crate::ui_modern::RADIUS_SMALL);let marker_w=20.min(width-12).max(8);let cx=(r.left+r.right)/2;crate::ui_modern::fill_round_rect(hdc,RECT{left:cx-marker_w/2,top:r.bottom-3,right:cx+(marker_w+1)/2,bottom:r.bottom-1},mode_color(mode),2);}SetTextColor(hdc,if active{crate::ui_modern::BPSR_TEXT}else{crate::ui_modern::BPSR_TEXT_SECONDARY});draw(hdc,text,r,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}",
        "meter mode tabs",
    );
    replace_required(
        source,
        "let hwnd=CreateWindowExW(WS_EX_TOOLWINDOW|WS_EX_TOPMOST,class.as_ptr(),title.as_ptr(),WS_POPUP|0x00c00000|0x00080000|0x02000000,0,0,w,h,parent,null_mut(),instance,ptr.cast::<c_void>());",
        "let hwnd=CreateWindowExW(WS_EX_TOPMOST,class.as_ptr(),title.as_ptr(),WS_POPUP|0x00c00000|0x00080000|0x02000000,0,0,w,h,parent,null_mut(),instance,ptr.cast::<c_void>());",
        "feature settings titlebar consistency",
    );
    // Slightly more breathing room inside Inspector metric cards without moving them.
    replace_required(
        source,
        "draw(hdc,title,RECT{left:r.left+10,top:r.top+3,right:r.right-7,bottom:r.top+20},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);SetTextColor(hdc,crate::ui_modern::BPSR_TEXT_SECONDARY);for(i,text)in lines.iter().take(4).enumerate(){draw(hdc,text,RECT{left:r.left+10,top:r.top+20+i as i32*14,right:r.right-7,bottom:r.top+35+i as i32*14},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}",
        "draw(hdc,title,RECT{left:r.left+13,top:r.top+3,right:r.right-10,bottom:r.top+20},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);SetTextColor(hdc,crate::ui_modern::BPSR_TEXT_SECONDARY);for(i,text)in lines.iter().take(4).enumerate(){draw(hdc,text,RECT{left:r.left+13,top:r.top+20+i as i32*14,right:r.right-10,bottom:r.top+35+i as i32*14},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}",
        "inspector card padding",
    );
}

fn patch_file(out: &Path, name: &str, kind: &str) {
    let path = out.join(name);
    assert!(path.exists(), "strict UI QA expected generated file {name}");
    let mut source = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {name} for strict UI QA: {e}"))
        .replace("\r\n", "\n");
    patch_common(&mut source);
    match kind {
        "settings" => patch_settings(&mut source),
        "event" => patch_event_tracker(&mut source),
        "feature" => patch_feature_overlays(&mut source),
        "common" => {},
        _ => unreachable!(),
    }
    fs::write(path, source).unwrap_or_else(|e| panic!("write {name} for strict UI QA: {e}"));
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_file(&out, "settings_ui_v1160_fixed.rs", "settings");
    patch_file(&out, "event_tracker_ui_v1160_fixed.rs", "event");
    patch_file(&out, "feature_overlays_v170_fixed.rs", "feature");
    patch_file(&out, "overlay_v150_v1181.rs", "common");
    patch_file(&out, "benchmark_ui_v1302.rs", "common");
    patch_file(&out, "win_v182_fixed.rs", "common");
    patch_file(&out, "updater_v1241.rs", "common");
    println!("cargo:rerun-if-changed=build/legacy/build_pixel_strict_qa.rs");
    println!("cargo:rerun-if-changed=src/ui_pixel_qa.rs");
}
