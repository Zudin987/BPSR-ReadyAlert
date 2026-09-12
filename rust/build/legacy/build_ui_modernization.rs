use std::{env, fs, path::{Path, PathBuf}};

mod prior {
    include!("build_v1271_history_button.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "UI modernization patch {label} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn replace_between(source: &mut String, start: &str, end: &str, replacement: &str, label: &str) {
    let start_count = source.matches(start).count();
    let end_count = source.matches(end).count();
    assert_eq!(start_count, 1, "UI modernization patch {label} start expected one match, found {start_count}");
    assert_eq!(end_count, 1, "UI modernization patch {label} end expected one match, found {end_count}");
    let begin = source.find(start).expect("UI modernization start anchor");
    let finish = source[begin..]
        .find(end)
        .map(|offset| begin + offset)
        .expect("UI modernization end anchor");
    source.replace_range(begin..finish, replacement);
}

fn replace_all_exact(source: &mut String, from: &str, to: &str, expected: usize, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, expected, "UI modernization patch {label} expected {expected} matches, found {count}");
    *source = source.replace(from, to);
}

fn patch_settings(out: &Path) {
    let path = out.join("settings_ui_v1160_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .expect("read generated Settings UI")
        .replace("\r\n", "\n");

    replace_once(
        &mut source,
        r#"        ID_TAB_LIST if notify == LBN_SELCHANGE => {
            store_tab_fields(hwnd, state);
            let sel = list_sel(hwnd, ID_TAB_LIST);
            state.selected_tab = if sel >= 0 { Some(sel as usize) } else { None };
            load_tab_fields(hwnd, state);
        }"#,
        r#"        ID_TAB_LIST if notify == LBN_SELCHANGE => {
            store_tab_fields(hwnd, state);
            let sel = list_sel(hwnd, ID_TAB_LIST);
            state.selected_tab = if sel >= 0 { Some(sel as usize) } else { None };
            load_tab_fields(hwnd, state);
            refresh_tab_actions(hwnd, state);
        }"#,
        "tab selection state",
    );
    replace_once(
        &mut source,
        "        ID_UNBLOCK => unblock_selected(hwnd, state),",
        "        ID_BLOCKED_LIST if notify == LBN_SELCHANGE => refresh_blocked_actions(hwnd, state),\n        ID_UNBLOCK => unblock_selected(hwnd, state),",
        "blocked-user selection state",
    );

    replace_between(
        &mut source,
        "unsafe fn refresh_tab_list(hwnd: HWND, state: &mut SettingsState) {",
        "unsafe fn store_tab_fields(hwnd: HWND, state: &mut SettingsState) {",
        r#"unsafe fn refresh_tab_list(hwnd: HWND, state: &mut SettingsState) {
    let list = GetDlgItem(hwnd, ID_TAB_LIST);
    SendMessageW(list, LB_RESETCONTENT, 0, 0);
    for tab in &state.working.chat.tabs {
        let text = wide(&tab.name);
        SendMessageW(list, LB_ADDSTRING, 0, text.as_ptr() as isize);
    }
    let selected = state.working.chat.tabs.iter().position(|t| t.id == state.working.chat.last_selected_tab_id).unwrap_or(0);
    state.selected_tab = state.working.chat.tabs.get(selected).map(|_| selected);
    SendMessageW(list, LB_SETCURSEL, selected, 0);
    load_tab_fields(hwnd, state);
    refresh_tab_actions(hwnd, state);
}

unsafe fn refresh_tab_actions(hwnd: HWND, state: &SettingsState) {
    let can_delete = state.working.chat.tabs.len() > 1 && state.selected_tab.is_some();
    EnableWindow(GetDlgItem(hwnd, ID_TAB_DELETE), can_delete as i32);
}

"#,
        "tab action enablement",
    );

    replace_between(
        &mut source,
        "unsafe fn refresh_blocked_list(hwnd: HWND, state: &SettingsState) {",
        "unsafe fn unblock_selected(hwnd: HWND, state: &mut SettingsState) {",
        r#"unsafe fn refresh_blocked_list(hwnd: HWND, state: &SettingsState) {
    let list = GetDlgItem(hwnd, ID_BLOCKED_LIST);
    SendMessageW(list, LB_RESETCONTENT, 0, 0);
    for user in &state.working.chat.blocked_users {
        let name = if user.name.trim().is_empty() { format!("UID {}", user.id) } else { format!("{}  (UID {})", user.name, user.id) };
        let text = wide(&name);
        SendMessageW(list, LB_ADDSTRING, 0, text.as_ptr() as isize);
    }
    refresh_blocked_actions(hwnd, state);
}

unsafe fn refresh_blocked_actions(hwnd: HWND, state: &SettingsState) {
    let has_users = !state.working.chat.blocked_users.is_empty();
    let selected = list_sel(hwnd, ID_BLOCKED_LIST) >= 0;
    EnableWindow(GetDlgItem(hwnd, ID_UNBLOCK), (has_users && selected) as i32);
    EnableWindow(GetDlgItem(hwnd, ID_CLEAR_BLOCKED), has_users as i32);
}

"#,
        "blocked-user action enablement",
    );

    fs::write(path, source).expect("write polished generated Settings UI");
}

fn patch_event_tracker(out: &Path) {
    let path = out.join("event_tracker_ui_v1160_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .expect("read generated Event Tracker UI")
        .replace("\r\n", "\n");

    replace_once(
        &mut source,
        r#"unsafe fn create_combo(hwnd: HWND, id: i32, x: i32, y: i32, w: i32, h: i32) -> HWND {
    create_control(hwnd, "COMBOBOX", "", id, x, y, w, h, WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_VSCROLL | CBS_DROPDOWNLIST | 0x0010 | 0x0200, 0)
}"#,
        r#"unsafe fn create_combo(hwnd: HWND, id: i32, x: i32, y: i32, w: i32, h: i32) -> HWND {
    let c = create_control(hwnd, "COMBOBOX", "", id, x, y, w, h, WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_VSCROLL | CBS_DROPDOWNLIST | 0x0010 | 0x0200, 0);
    crate::ui_theme::theme_combo(c);
    c
}"#,
        "Event Tracker combo theme",
    );

    fs::write(path, source).expect("write polished generated Event Tracker UI");
}

fn patch_overlay(out: &Path) {
    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .expect("read generated feature overlays")
        .replace("\r\n", "\n");

    replace_once(
        &mut source,
        r#"unsafe fn paint_hover(hdc:HDC,rc:RECT,state:&State){let Some(text)=state.hover_text.as_ref()else{return;};let width=((text.chars().count()as i32)*7+20).clamp(110,340);let height=25;let mut left=state.hover_x+14;let mut top=state.hover_y+16;if left+width>rc.right-4{left=(state.hover_x-width-10).max(4);}if top+height>rc.bottom-4{top=(state.hover_y-height-8).max(4);}let r=RECT{left,top,right:left+width,bottom:top+height};fill(hdc,&r,rgb(12,16,21));outline(hdc,r,crate::ui_theme::ACCENT,1);SetTextColor(hdc,rgb(238,242,247));draw(hdc,text,RECT{left:r.left+8,top:r.top,right:r.right-8,bottom:r.bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}"#,
        r#"unsafe fn paint_hover(hdc:HDC,rc:RECT,state:&State){
    let Some(text)=state.hover_text.as_ref()else{return;};
    const DT_WORDBREAK_:u32=0x0010;const DT_CALCRECT_:u32=0x0400;
    let max_width=(rc.right-8).clamp(160,420);let wide_text=wide(text);let mut measured=RECT{left:0,top:0,right:max_width-16,bottom:0};
    DrawTextW(hdc,wide_text.as_ptr(),-1,&mut measured,DT_WORDBREAK_|DT_NOPREFIX|DT_CALCRECT_);
    let content_w=(measured.right-measured.left).clamp(96,max_width-16);let content_h=(measured.bottom-measured.top).clamp(18,96);
    let width=content_w+16;let height=content_h+10;let mut left=state.hover_x+14;let mut top=state.hover_y+16;
    if left+width>rc.right-4{left=(state.hover_x-width-10).max(4);}if top+height>rc.bottom-4{top=(state.hover_y-height-8).max(4);}
    let r=RECT{left,top,right:left+width,bottom:top+height};fill(hdc,&r,crate::ui_theme::BG);outline(hdc,r,crate::ui_theme::BORDER_STRONG,1);
    SetTextColor(hdc,crate::ui_theme::TEXT);draw(hdc,text,RECT{left:r.left+8,top:r.top+5,right:r.right-8,bottom:r.bottom-5},DT_WORDBREAK_|DT_NOPREFIX);
}"#,
        "wrapped overlay hover text",
    );

    replace_all_exact(&mut source, "rgb(38,38,38)", "crate::ui_theme::RAISED", 4, "detail raised rows");
    replace_all_exact(&mut source, "rgb(34,34,34)", "crate::ui_theme::SURFACE", 4, "detail surface rows");
    replace_all_exact(&mut source, "rgb(150,150,150)", "crate::ui_theme::MUTED", 4, "detail empty-state text");
    replace_all_exact(&mut source, "rgb(235,235,235)", "crate::ui_theme::TEXT", 3, "detail body text");
    replace_all_exact(&mut source, "rgb(49,49,49)", "crate::ui_theme::RAISED", 1, "skill header surface");
    replace_all_exact(&mut source, "rgb(225,225,225)", "crate::ui_theme::TEXT_SECONDARY", 1, "skill header text");

    fs::write(path, source).expect("write polished generated feature overlays");
}

fn main() {
    prior::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_settings(&out);
    patch_event_tracker(&out);
    patch_overlay(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_ui_modernization.rs");
    println!("cargo:rerun-if-changed=src/ui_theme.rs");
}
