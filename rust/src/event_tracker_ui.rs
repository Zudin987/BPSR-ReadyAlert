use crate::event_tracker::{self, TrackerKind, TrackerRule, TrackerScope, TrackerSettings};
use std::{ffi::c_void, ptr::{null, null_mut}};
use windows_sys::Win32::{
    Foundation::{GetLastError, HWND, LPARAM, LRESULT, RECT, WPARAM},
    Graphics::Gdi::{CreateSolidBrush, DeleteObject, GetStockObject, SetBkColor, SetTextColor, DEFAULT_GUI_FONT, HBRUSH, HDC, GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST},
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, FindWindowW, GetClientRect,
        GetDlgItem, GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW,
        LoadCursorW, MessageBoxW, MoveWindow, RegisterClassW, SendMessageW,
        SetForegroundWindow, SetWindowLongPtrW, SetWindowTextW, ShowWindow, CREATESTRUCTW,
        GWLP_USERDATA, IDC_ARROW, MB_ICONERROR, MB_OK,
        SW_SHOW, WM_CLOSE, WM_COMMAND, WM_CREATE, WM_CTLCOLORBTN, WM_CTLCOLOREDIT,
        WM_CTLCOLORLISTBOX, WM_CTLCOLORSTATIC, WM_NCCREATE, WM_NCDESTROY, WM_SETFONT,
        WNDCLASSW, WS_BORDER, WS_CAPTION, WS_CHILD, WS_EX_CLIENTEDGE, WS_MINIMIZEBOX,
        WS_OVERLAPPED, WS_SYSMENU, WS_TABSTOP, WS_VISIBLE, WS_VSCROLL, HMENU,
    },
};

const CLASS: &str = "BPSRReadyAlertEventTrackerV114";
const WIDTH: i32 = 760;
const HEIGHT: i32 = 600;

const ID_GLOBAL_ENABLED: i32 = 6100;
const ID_MAX_VISIBLE: i32 = 6101;
const ID_RULE_LIST: i32 = 6110;
const ID_ADD: i32 = 6111;
const ID_REMOVE: i32 = 6112;
const ID_KIND: i32 = 6120;
const ID_EVENT_ID: i32 = 6121;
const ID_LABEL: i32 = 6122;
const ID_SCOPE: i32 = 6123;
const ID_HOLD: i32 = 6124;
const ID_RULE_ENABLED: i32 = 6125;
const ID_SAVE_RULE: i32 = 6126;
const ID_APPLY: i32 = 6198;
const ID_CLOSE: i32 = 6199;

const BM_GETCHECK: u32 = 0x00F0;
const BM_SETCHECK: u32 = 0x00F1;
const BST_CHECKED: usize = 1;
const BS_AUTOCHECKBOX: u32 = 0x0000_0003;
const BS_PUSHBUTTON: u32 = 0;
const ES_AUTOHSCROLL: u32 = 0x0080;
const CBS_DROPDOWNLIST: u32 = 0x0003;
const LBS_NOTIFY: u32 = 0x0001;
const LB_ADDSTRING: u32 = 0x0180;
const LB_RESETCONTENT: u32 = 0x0184;
const LB_SETCURSEL: u32 = 0x0186;
const LB_GETCURSEL: u32 = 0x0188;
const CB_ADDSTRING: u32 = 0x0143;
const CB_RESETCONTENT: u32 = 0x014B;
const CB_GETCURSEL: u32 = 0x0147;
const CB_SETCURSEL: u32 = 0x014E;
const LBN_SELCHANGE: u16 = 1;

#[link(name = "user32")]
extern "system" { fn EnableWindow(hwnd: HWND, enable: i32) -> i32; }

#[link(name = "dwmapi")]
extern "system" {
    fn DwmSetWindowAttribute(hwnd: HWND, attribute: u32, value: *const c_void, size: u32) -> i32;
}
#[link(name = "uxtheme")]
extern "system" {
    fn SetWindowTheme(hwnd: HWND, sub_app_name: *const u16, sub_id_list: *const u16) -> i32;
}

struct UiState {
    working: TrackerSettings,
    selected: Option<usize>,
    brush: HBRUSH,
    form: crate::ui::ScrollForm,
}

pub unsafe fn show(parent: HWND) {
    let class = wide(CLASS);
    let existing = FindWindowW(class.as_ptr(), null());
    if !existing.is_null() {
        ShowWindow(existing, SW_SHOW);
        SetForegroundWindow(existing);
        return;
    }

    let instance = GetModuleHandleW(null());
    let wc = WNDCLASSW {
        lpfnWndProc: Some(wnd_proc),
        hInstance: instance,
        hCursor: LoadCursorW(null_mut(), IDC_ARROW),
        hbrBackground: null_mut(),
        lpszClassName: class.as_ptr(),
        ..std::mem::zeroed()
    };
    if RegisterClassW(&wc) == 0 && GetLastError() != 1410 {
        message_error(parent, &format!("Could not open Event Tracker settings ({}).", GetLastError()));
        return;
    }

    let state = Box::new(UiState {
        working: event_tracker::current_settings(),
        selected: None,
        brush: CreateSolidBrush(rgb(22, 25, 30)),
        form: Default::default(),
    });
    let ptr = Box::into_raw(state);
    let title = wide("BPSR ReadyAlert - Custom Event Tracker");
    let hwnd = CreateWindowExW(
        0,
        class.as_ptr(),
        title.as_ptr(),
        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | 0x00040000 | WS_VSCROLL | 0x00100000,
        0, 0, WIDTH, HEIGHT,
        parent,
        null_mut(),
        instance,
        ptr.cast::<c_void>(),
    );
    if hwnd.is_null() {
        let state = Box::from_raw(ptr);
        if !state.brush.is_null() { DeleteObject(state.brush); }
        message_error(parent, &format!("Could not create Event Tracker settings ({}).", GetLastError()));
        return;
    }
    crate::ui::fit_window(hwnd, parent, true);
    try_dark_titlebar(hwnd);
    ShowWindow(hwnd, SW_SHOW);
    SetForegroundWindow(hwnd);
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        let create = lparam as *const CREATESTRUCTW;
        if !create.is_null() { SetWindowLongPtrW(hwnd, GWLP_USERDATA, (*create).lpCreateParams as isize); }
    }
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut UiState;
    match msg {
        WM_CREATE => {
            if !ptr.is_null() { build_controls(hwnd, &mut *ptr); (*ptr).form.capture(hwnd, 740, 550); }
            0
        }
        WM_COMMAND => {
            if !ptr.is_null() {
                let id = (wparam as u32 & 0xffff) as i32;
                let code = ((wparam as u32 >> 16) & 0xffff) as u16;
                handle_command(hwnd, &mut *ptr, id, code);
            }
            0
        }
        0x0005 => { if !ptr.is_null() { (*ptr).form.layout(hwnd); } 0 }
        0x020a | 0x0115 | 0x0114 => { if !ptr.is_null() { (*ptr).form.scroll(hwnd,msg,wparam); } 0 }
        crate::ui::WM_REVEAL_FOCUS => { if !ptr.is_null() { (*ptr).form.reveal_focus(hwnd); } 0 }
        0x0014 => { if !ptr.is_null() { let mut r:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut r);windows_sys::Win32::Graphics::Gdi::FillRect(wparam as HDC,&r,(*ptr).brush);return 1; } DefWindowProcW(hwnd,msg,wparam,lparam) }
        WM_CTLCOLORSTATIC | WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX | WM_CTLCOLORBTN => {
            if ptr.is_null() { return DefWindowProcW(hwnd, msg, wparam, lparam); }
            let hdc = wparam as HDC;
            SetTextColor(hdc, rgb(228, 232, 238));
            SetBkColor(hdc, rgb(22, 25, 30));
            (*ptr).brush as LRESULT
        }
        WM_CLOSE => { if !ptr.is_null() { close_form(hwnd,&mut *ptr); } 0 }
        WM_NCDESTROY => {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            if !ptr.is_null() {
                let state = Box::from_raw(ptr);
                if !state.brush.is_null() { DeleteObject(state.brush); }
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn build_controls(hwnd: HWND, state: &mut UiState) {
    create_static(hwnd,"Custom Event Tracker",18,16,650,26);
    create_static(hwnd,"Rules appear in Dungeon Mechanics when matching events are observed.",18,46,690,24);
    create_checkbox(hwnd,ID_GLOBAL_ENABLED,"Enable tracker",18,80,190);
    create_static(hwnd,"Max rows (1–12)",230,83,130,24);
    create_edit(hwnd,ID_MAX_VISIBLE,370,80,56,26);
    create_static(hwnd,"Rules (up to 24)",18,120,300,24);
    create_listbox(hwnd,ID_RULE_LIST,18,148,310,302);
    SendMessageW(GetDlgItem(hwnd,ID_RULE_LIST),0x0194,760,0);
    create_button(hwnd,ID_ADD,"&Add",18,458,100,32);
    create_button(hwnd,ID_REMOVE,"&Remove",128,458,100,32);
    let x=354;
    create_checkbox(hwnd,ID_RULE_ENABLED,"Rule enabled",x,120,200);
    create_static(hwnd,"Type",x,158,160,20);
    create_combo(hwnd,ID_KIND,x,180,178,140);
    create_static(hwnd,"Numeric ID (positive number)",x,220,346,20);
    create_edit(hwnd,ID_EVENT_ID,x,242,178,26);
    create_static(hwnd,"Label (optional, up to 40 characters)",x,280,346,20);
    create_edit(hwnd,ID_LABEL,x,302,346,26);
    create_static(hwnd,"Scope",x,340,160,20);
    create_combo(hwnd,ID_SCOPE,x,362,178,140);
    create_static(hwnd,"Skill display seconds (1–30)",x,408,250,20);
    create_edit(hwnd,ID_HOLD,620,404,60,26);
    create_button(hwnd,ID_APPLY,"&Apply",510,458,94,32);
    create_button(hwnd,ID_CLOSE,"&Close",616,458,94,32);
    create_static(hwnd,"Buff: game countdown or ACTIVE. Skill: observed damage/heal hits; display time is not a cooldown. New rules start observing after Apply.",18,504,690,42);
    set_combo_items(hwnd,ID_KIND,&["Buff ID","Skill ID"],0);
    set_combo_items(hwnd,ID_SCOPE,&["Any","Self","Party (includes self)"],0);
    set_check(hwnd,ID_GLOBAL_ENABLED,state.working.enabled);
    set_text(hwnd,ID_MAX_VISIBLE,&state.working.max_visible.to_string());
    for id in [ID_EVENT_ID,ID_MAX_VISIBLE,ID_HOLD] { SendMessageW(GetDlgItem(hwnd,id),0x00c5,10,0); }
    SendMessageW(GetDlgItem(hwnd,ID_LABEL),0x00c5,80,0);
    state.selected=if state.working.rules.is_empty(){None}else{Some(0)};
    refresh_list(hwnd,state); load_selected(hwnd,state);
}

unsafe fn handle_command(hwnd: HWND, state: &mut UiState, id: i32, code: u16) {
    if id==ID_CLOSE || id==2 { close_form(hwnd,state); return; }
    if id==ID_RULE_LIST && code==LBN_SELCHANGE {
        let selected=list_sel(hwnd,ID_RULE_LIST);
        if !save_editor(hwnd,state) { if let Some(i)=state.selected {SendMessageW(GetDlgItem(hwnd,ID_RULE_LIST),LB_SETCURSEL,i,0);} return; }
        state.selected=(selected>=0).then_some(selected as usize).filter(|i|*i<state.working.rules.len());
        refresh_list(hwnd,state); load_selected(hwnd,state); return;
    }
    match id {
        ID_ADD => {
            if state.working.rules.len()>=24 || !save_editor(hwnd,state) {return;}
            let id=state.working.next_rule_id();
            state.working.rules.push(TrackerRule{rule_id:id,..TrackerRule::default()});
            state.selected=Some(state.working.rules.len()-1);
            refresh_list(hwnd,state); load_selected(hwnd,state);
            crate::ui::SetFocus(GetDlgItem(hwnd,ID_EVENT_ID));state.form.reveal_focus(hwnd);
        }
        ID_REMOVE => {
            if let Some(index)=state.selected.filter(|i|*i<state.working.rules.len()) {
                state.working.rules.remove(index);
                state.selected=if state.working.rules.is_empty(){None}else{Some(index.min(state.working.rules.len()-1))};
                refresh_list(hwnd,state);load_selected(hwnd,state);
            }
        }
        ID_KIND => { EnableWindow(GetDlgItem(hwnd,ID_HOLD),(combo_sel(hwnd,ID_KIND)==1) as i32); }
        ID_APPLY | 1 => { apply_settings(hwnd,state); }
        _=>{}
    }
}

unsafe fn apply_settings(hwnd: HWND, state: &mut UiState) -> bool {
    if !save_editor(hwnd,state) {return false;}
    let Some(max)=positive_number(hwnd,ID_MAX_VISIBLE,12,"Max rows must be from 1 to 12.") else{return false;};
    state.working.enabled=get_check(hwnd,ID_GLOBAL_ENABLED);
    state.working.max_visible=max as usize; state.working.normalize();
    if let Err(err)=event_tracker::replace_settings(state.working.clone()) {message_error(hwnd,&format!("Could not save Event Tracker settings.\n\n{err}"));return false;}
    refresh_list(hwnd,state); load_selected(hwnd,state);
    SetWindowTextW(hwnd,wide("BPSR ReadyAlert - Custom Event Tracker (saved)").as_ptr()); true
}

unsafe fn close_form(hwnd: HWND, state: &mut UiState) {
    let mut dirty=state.working!=event_tracker::current_settings()
        || get_check(hwnd,ID_GLOBAL_ENABLED)!=state.working.enabled
        || get_text(hwnd,ID_MAX_VISIBLE)!=state.working.max_visible.to_string();
    if let Some(i)=state.selected {
        let r=&state.working.rules[i];
        dirty |= get_text(hwnd,ID_EVENT_ID)!=r.event_id.to_string() || get_text(hwnd,ID_LABEL)!=r.label
            || get_check(hwnd,ID_RULE_ENABLED)!=r.enabled || (combo_sel(hwnd,ID_KIND)==1)!=(r.kind==TrackerKind::Skill)
            || combo_sel(hwnd,ID_SCOPE)!=match r.scope {TrackerScope::Any=>0,TrackerScope::SelfOnly=>1,TrackerScope::Party=>2}
            || (r.kind==TrackerKind::Skill && get_text(hwnd,ID_HOLD)!=r.hold_seconds.to_string());
    }
    if dirty {
        let choice=MessageBoxW(hwnd,wide("Save changes to Event Tracker before closing?").as_ptr(),wide("Unsaved rules").as_ptr(),0x00000003|0x00000020);
        if choice==2 || (choice==6 && !apply_settings(hwnd,state)) {return;}
    }
    DestroyWindow(hwnd);
}

unsafe fn refresh_list(hwnd: HWND, state: &UiState) {
    let list = GetDlgItem(hwnd, ID_RULE_LIST);
    SendMessageW(list, LB_RESETCONTENT, 0, 0);
    for rule in &state.working.rules {
        let status = if rule.enabled { "on" } else { "off" };
        let text = format!("[{status}] {} {}  |  {}  |  {}", rule.kind.label(), rule.event_id, event_tracker::rule_label(rule), rule.scope.label());
        let w = wide(&text);
        SendMessageW(list, LB_ADDSTRING, 0, w.as_ptr() as isize);
    }
    if let Some(index)=state.selected { SendMessageW(list,LB_SETCURSEL,index,0); }
    EnableWindow(GetDlgItem(hwnd,ID_ADD),(state.working.rules.len()<24) as i32);
    EnableWindow(GetDlgItem(hwnd,ID_REMOVE),state.selected.is_some() as i32);
}

unsafe fn load_selected(hwnd: HWND, state: &UiState) {
    let Some(index) = state.selected.filter(|index| *index < state.working.rules.len()) else {
        set_editor_enabled(hwnd, false);
        set_text(hwnd, ID_EVENT_ID, "");
        set_text(hwnd, ID_LABEL, "");
        set_text(hwnd, ID_HOLD, "6");
        set_check(hwnd, ID_RULE_ENABLED, false);
        return;
    };
    let rule = &state.working.rules[index];
    set_editor_enabled(hwnd, true);
    set_check(hwnd, ID_RULE_ENABLED, rule.enabled);
    SendMessageW(GetDlgItem(hwnd, ID_KIND), CB_SETCURSEL, if rule.kind == TrackerKind::Buff { 0 } else { 1 }, 0);
    SendMessageW(GetDlgItem(hwnd, ID_SCOPE), CB_SETCURSEL, match rule.scope { TrackerScope::Any => 0, TrackerScope::SelfOnly => 1, TrackerScope::Party => 2 }, 0);
    set_text(hwnd, ID_EVENT_ID, &rule.event_id.to_string());
    set_text(hwnd, ID_LABEL, &rule.label);
    set_text(hwnd, ID_HOLD, &rule.hold_seconds.to_string());
    EnableWindow(GetDlgItem(hwnd,ID_HOLD),(rule.kind==TrackerKind::Skill) as i32);
}

unsafe fn positive_number(hwnd: HWND, id: i32, max: i32, message: &str) -> Option<i32> {
    if let Ok(n)=get_text(hwnd,id).trim().parse::<i32>() { if (1..=max).contains(&n) {return Some(n);} }
    message_error(hwnd,message);crate::ui::SetFocus(GetDlgItem(hwnd,id));None
}
unsafe fn save_editor(hwnd: HWND, state: &mut UiState) -> bool {
    let Some(index)=state.selected.filter(|i|*i<state.working.rules.len()) else{return true;};
    let Some(id)=positive_number(hwnd,ID_EVENT_ID,i32::MAX,"Enter a positive numeric Buff or Skill ID.") else{return false;};
    let kind=if combo_sel(hwnd,ID_KIND)==1 {TrackerKind::Skill}else{TrackerKind::Buff};
    let hold=if kind==TrackerKind::Skill {
        let Some(n)=positive_number(hwnd,ID_HOLD,30,"Skill display seconds must be from 1 to 30.") else{return false;};n as u8
    }else{state.working.rules[index].hold_seconds};
    let rule=&mut state.working.rules[index];
    rule.event_id=id;rule.kind=kind;rule.enabled=get_check(hwnd,ID_RULE_ENABLED);
    rule.scope=match combo_sel(hwnd,ID_SCOPE){1=>TrackerScope::SelfOnly,2=>TrackerScope::Party,_=>TrackerScope::Any};
    rule.label=get_text(hwnd,ID_LABEL);rule.hold_seconds=hold;state.working.normalize();true
}

unsafe fn set_editor_enabled(hwnd: HWND, enabled: bool) {
    for id in [ID_RULE_ENABLED, ID_KIND, ID_EVENT_ID, ID_LABEL, ID_SCOPE, ID_HOLD, ID_SAVE_RULE] {
        EnableWindow(GetDlgItem(hwnd, id), enabled as i32);
    }
}

unsafe fn create_static(hwnd: HWND, text: &str, x: i32, y: i32, w: i32, h: i32) -> HWND {
    create_control(hwnd, "STATIC", text, 0, x, y, w, h, WS_CHILD | WS_VISIBLE | 0x80, 0)
}
unsafe fn create_button(hwnd: HWND, id: i32, text: &str, x: i32, y: i32, w: i32, h: i32) -> HWND {
    create_control(hwnd, "BUTTON", text, id, x, y, w, h, WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON, 0)
}
unsafe fn create_checkbox(hwnd: HWND, id: i32, text: &str, x: i32, y: i32, w: i32) -> HWND {
    create_control(hwnd, "BUTTON", text, id, x, y, w, 26, WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_AUTOCHECKBOX, 0)
}
unsafe fn create_edit(hwnd: HWND, id: i32, x: i32, y: i32, w: i32, h: i32) -> HWND {
    create_control(hwnd, "EDIT", "", id, x, y, w, h, WS_CHILD | WS_VISIBLE | WS_TABSTOP | ES_AUTOHSCROLL, WS_EX_CLIENTEDGE)
}
unsafe fn create_combo(hwnd: HWND, id: i32, x: i32, y: i32, w: i32, h: i32) -> HWND {
    create_control(hwnd, "COMBOBOX", "", id, x, y, w, h, WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_VSCROLL | CBS_DROPDOWNLIST, 0)
}
unsafe fn create_listbox(hwnd: HWND, id: i32, x: i32, y: i32, w: i32, h: i32) -> HWND {
    create_control(hwnd, "LISTBOX", "", id, x, y, w, h, WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_VSCROLL | 0x00100000 | LBS_NOTIFY | WS_BORDER, WS_EX_CLIENTEDGE)
}
unsafe fn create_control(hwnd: HWND, class: &str, text: &str, id: i32, x: i32, y: i32, w: i32, h: i32, style: u32, ex: u32) -> HWND {
    let c = CreateWindowExW(ex, wide(class).as_ptr(), wide(text).as_ptr(), style, x, y, w, h, hwnd, id as usize as HMENU, GetModuleHandleW(null()), null());
    if !c.is_null() {
        SendMessageW(c, WM_SETFONT, GetStockObject(DEFAULT_GUI_FONT) as usize, 1);
        let theme = wide("DarkMode_Explorer");
        let _ = SetWindowTheme(c, theme.as_ptr(), null());
    }
    c
}

unsafe fn set_check(hwnd: HWND, id: i32, checked: bool) { SendMessageW(GetDlgItem(hwnd, id), BM_SETCHECK, if checked { BST_CHECKED } else { 0 }, 0); }
unsafe fn get_check(hwnd: HWND, id: i32) -> bool { SendMessageW(GetDlgItem(hwnd, id), BM_GETCHECK, 0, 0) as usize == BST_CHECKED }
unsafe fn set_text(hwnd: HWND, id: i32, text: &str) { let child = GetDlgItem(hwnd, id); if !child.is_null() { SetWindowTextW(child, wide(text).as_ptr()); } }
unsafe fn get_text(hwnd: HWND, id: i32) -> String {
    let child = GetDlgItem(hwnd, id); if child.is_null() { return String::new(); }
    let len = GetWindowTextLengthW(child).max(0) as usize;
    let mut buf = vec![0u16; len + 1];
    let got = GetWindowTextW(child, buf.as_mut_ptr(), buf.len() as i32).max(0) as usize;
    String::from_utf16_lossy(&buf[..got])
}
unsafe fn read_i32(hwnd: HWND, id: i32, fallback: i32) -> i32 { get_text(hwnd, id).trim().parse().unwrap_or(fallback) }
unsafe fn set_combo_items(hwnd: HWND, id: i32, values: &[&str], selected: usize) {
    let child = GetDlgItem(hwnd, id);
    SendMessageW(child, CB_RESETCONTENT, 0, 0);
    for value in values { let w = wide(value); SendMessageW(child, CB_ADDSTRING, 0, w.as_ptr() as isize); }
    SendMessageW(child, CB_SETCURSEL, selected.min(values.len().saturating_sub(1)), 0);
}
unsafe fn combo_sel(hwnd: HWND, id: i32) -> isize { SendMessageW(GetDlgItem(hwnd, id), CB_GETCURSEL, 0, 0) }
unsafe fn list_sel(hwnd: HWND, id: i32) -> isize { SendMessageW(GetDlgItem(hwnd, id), LB_GETCURSEL, 0, 0) }

unsafe fn message_error(hwnd: HWND, text: &str) {
    MessageBoxW(hwnd, wide(text).as_ptr(), wide("BPSR ReadyAlert").as_ptr(), MB_OK | MB_ICONERROR);
}
unsafe fn try_dark_titlebar(hwnd: HWND) {
    let enabled: i32 = 1;
    let ptr = (&enabled as *const i32).cast::<c_void>();
    if DwmSetWindowAttribute(hwnd, 20, ptr, std::mem::size_of::<i32>() as u32) != 0 {
        let _ = DwmSetWindowAttribute(hwnd, 19, ptr, std::mem::size_of::<i32>() as u32);
    }
}
fn rgb(r: u8, g: u8, b: u8) -> u32 { u32::from(r) | (u32::from(g) << 8) | (u32::from(b) << 16) }
fn wide(text: &str) -> Vec<u16> { text.encode_utf16().chain(std::iter::once(0)).collect() }
