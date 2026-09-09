use crate::event_tracker::{self, TrackerKind, TrackerRule, TrackerScope, TrackerSettings};
use std::{ffi::c_void, ptr::{null, null_mut}};
use windows_sys::Win32::{
    Foundation::{GetLastError, HWND, LPARAM, LRESULT, RECT, WPARAM},
    Graphics::Gdi::{CreateSolidBrush, DeleteObject, GetStockObject, SetBkColor, SetTextColor, DEFAULT_GUI_FONT, HBRUSH, HDC},
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, EnableWindow, FindWindowW, GetClientRect,
        GetDlgItem, GetMonitorInfoW, GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW,
        LoadCursorW, MessageBoxW, MonitorFromWindow, MoveWindow, RegisterClassW, SendMessageW,
        SetForegroundWindow, SetWindowLongPtrW, SetWindowTextW, ShowWindow, CREATESTRUCTW,
        GWLP_USERDATA, IDC_ARROW, MB_ICONERROR, MB_OK, MONITORINFO, MONITOR_DEFAULTTONEAREST,
        SW_SHOW, WM_CLOSE, WM_COMMAND, WM_CREATE, WM_CTLCOLORBTN, WM_CTLCOLOREDIT,
        WM_CTLCOLORLISTBOX, WM_CTLCOLORSTATIC, WM_NCCREATE, WM_NCDESTROY, WM_SETFONT,
        WNDCLASSW, WS_BORDER, WS_CAPTION, WS_CHILD, WS_EX_CLIENTEDGE, WS_MINIMIZEBOX,
        WS_OVERLAPPED, WS_SYSMENU, WS_TABSTOP, WS_VISIBLE, WS_VSCROLL, HMENU,
    },
};

const CLASS: &str = "BPSRReadyAlertEventTrackerV114";
const WIDTH: i32 = 760;
const HEIGHT: i32 = 560;

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
    });
    let ptr = Box::into_raw(state);
    let title = wide("BPSR ReadyAlert - Custom Event Tracker");
    let hwnd = CreateWindowExW(
        0,
        class.as_ptr(),
        title.as_ptr(),
        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
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
    fit_to_work_area(hwnd, parent, WIDTH, HEIGHT);
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
            if !ptr.is_null() { build_controls(hwnd, &mut *ptr); }
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
        WM_CTLCOLORSTATIC | WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX | WM_CTLCOLORBTN => {
            if ptr.is_null() { return DefWindowProcW(hwnd, msg, wparam, lparam); }
            let hdc = wparam as HDC;
            SetTextColor(hdc, rgb(228, 232, 238));
            SetBkColor(hdc, rgb(22, 25, 30));
            (*ptr).brush as LRESULT
        }
        WM_CLOSE => { DestroyWindow(hwnd); 0 }
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
    try_dark_titlebar(hwnd);
    create_static(hwnd, "Custom Event Tracker", 18, 16, 250, 24);
    create_static(hwnd, "Observe packet events only. No automation or key input.", 18, 40, 530, 20);
    create_checkbox(hwnd, ID_GLOBAL_ENABLED, "Enable custom tracker", 18, 70, 210);
    create_static(hwnd, "Visible rows", 250, 73, 90, 20);
    create_edit(hwnd, ID_MAX_VISIBLE, 340, 68, 55, 25);

    create_static(hwnd, "Rules (max 24)", 18, 109, 240, 20);
    create_listbox(hwnd, ID_RULE_LIST, 18, 134, 315, 328);
    create_button(hwnd, ID_ADD, "Add", 18, 470, 95, 30);
    create_button(hwnd, ID_REMOVE, "Remove", 120, 470, 95, 30);

    let x = 365;
    create_static(hwnd, "Rule", x, 109, 240, 20);
    create_checkbox(hwnd, ID_RULE_ENABLED, "Enabled", x, 136, 120);
    create_static(hwnd, "Type", x, 173, 130, 20);
    create_combo(hwnd, ID_KIND, x, 195, 180, 160);
    create_static(hwnd, "Numeric ID", x, 238, 130, 20);
    create_edit(hwnd, ID_EVENT_ID, x, 260, 180, 25);
    create_static(hwnd, "Label (optional)", x, 300, 180, 20);
    create_edit(hwnd, ID_LABEL, x, 322, 330, 25);
    create_static(hwnd, "Scope", x, 362, 130, 20);
    create_combo(hwnd, ID_SCOPE, x, 384, 180, 180);
    create_static(hwnd, "Hold seconds (Skill only)", x, 427, 200, 20);
    create_edit(hwnd, ID_HOLD, x + 205, 423, 55, 25);
    create_button(hwnd, ID_SAVE_RULE, "Save rule", x, 468, 130, 30);

    create_static(hwnd, "Buff countdowns use the game's own duration. If no finite timer is sent, ReadyAlert shows ACTIVE without inventing one.", 18, 510, 620, 20);
    create_static(hwnd, "Hit Event ID is not exposed yet: its wire field is still unverified, so v1.14 does not guess it.", 18, 532, 620, 20);

    create_button(hwnd, ID_APPLY, "Apply", 642, 470, 88, 30);
    create_button(hwnd, ID_CLOSE, "Close", 642, 505, 88, 30);

    set_combo_items(hwnd, ID_KIND, &["Buff ID", "Skill ID"], 0);
    set_combo_items(hwnd, ID_SCOPE, &["Any", "Self", "Party"], 0);
    set_check(hwnd, ID_GLOBAL_ENABLED, state.working.enabled);
    set_text(hwnd, ID_MAX_VISIBLE, &state.working.max_visible.to_string());
    refresh_list(hwnd, state);
    set_editor_enabled(hwnd, false);
}

unsafe fn handle_command(hwnd: HWND, state: &mut UiState, id: i32, code: u16) {
    if id == ID_CLOSE { DestroyWindow(hwnd); return; }
    if id == ID_RULE_LIST && code == LBN_SELCHANGE {
        let selected = list_sel(hwnd, ID_RULE_LIST);
        state.selected = (selected >= 0).then_some(selected as usize).filter(|index| *index < state.working.rules.len());
        load_selected(hwnd, state);
        return;
    }
    match id {
        ID_ADD => {
            if state.working.rules.len() >= 24 { return; }
            let id = state.working.next_rule_id();
            state.working.rules.push(TrackerRule { rule_id: id, ..TrackerRule::default() });
            state.selected = Some(state.working.rules.len() - 1);
            refresh_list(hwnd, state);
            SendMessageW(GetDlgItem(hwnd, ID_RULE_LIST), LB_SETCURSEL, state.selected.unwrap_or(0), 0);
            load_selected(hwnd, state);
        }
        ID_REMOVE => {
            if let Some(index) = state.selected.filter(|index| *index < state.working.rules.len()) {
                state.working.rules.remove(index);
                state.selected = if state.working.rules.is_empty() { None } else { Some(index.min(state.working.rules.len() - 1)) };
                refresh_list(hwnd, state);
                if let Some(selected) = state.selected { SendMessageW(GetDlgItem(hwnd, ID_RULE_LIST), LB_SETCURSEL, selected, 0); }
                load_selected(hwnd, state);
            }
        }
        ID_SAVE_RULE => {
            save_editor(hwnd, state);
            refresh_list(hwnd, state);
            if let Some(selected) = state.selected { SendMessageW(GetDlgItem(hwnd, ID_RULE_LIST), LB_SETCURSEL, selected, 0); }
        }
        ID_APPLY => {
            save_editor(hwnd, state);
            state.working.enabled = get_check(hwnd, ID_GLOBAL_ENABLED);
            state.working.max_visible = read_i32(hwnd, ID_MAX_VISIBLE, state.working.max_visible as i32).clamp(1, 12) as usize;
            state.working.normalize();
            match event_tracker::replace_settings(state.working.clone()) {
                Ok(()) => {
                    set_text(hwnd, ID_MAX_VISIBLE, &state.working.max_visible.to_string());
                    refresh_list(hwnd, state);
                }
                Err(err) => message_error(hwnd, &format!("Could not save Event Tracker settings.\n\n{err}")),
            }
        }
        _ => {}
    }
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
}

unsafe fn save_editor(hwnd: HWND, state: &mut UiState) {
    let Some(index) = state.selected.filter(|index| *index < state.working.rules.len()) else { return; };
    let rule = &mut state.working.rules[index];
    rule.enabled = get_check(hwnd, ID_RULE_ENABLED);
    rule.kind = if combo_sel(hwnd, ID_KIND) == 1 { TrackerKind::Skill } else { TrackerKind::Buff };
    rule.scope = match combo_sel(hwnd, ID_SCOPE) { 1 => TrackerScope::SelfOnly, 2 => TrackerScope::Party, _ => TrackerScope::Any };
    rule.event_id = read_i32(hwnd, ID_EVENT_ID, rule.event_id).max(0);
    rule.label = get_text(hwnd, ID_LABEL);
    rule.hold_seconds = read_i32(hwnd, ID_HOLD, i32::from(rule.hold_seconds)).clamp(1, 30) as u8;
    state.working.normalize();
    set_text(hwnd, ID_EVENT_ID, &rule.event_id.to_string());
    set_text(hwnd, ID_LABEL, &rule.label);
    set_text(hwnd, ID_HOLD, &rule.hold_seconds.to_string());
}

unsafe fn set_editor_enabled(hwnd: HWND, enabled: bool) {
    for id in [ID_RULE_ENABLED, ID_KIND, ID_EVENT_ID, ID_LABEL, ID_SCOPE, ID_HOLD, ID_SAVE_RULE] {
        EnableWindow(GetDlgItem(hwnd, id), enabled as i32);
    }
}

unsafe fn fit_to_work_area(hwnd: HWND, parent: HWND, requested_w: i32, requested_h: i32) {
    let monitor = MonitorFromWindow(if parent.is_null() { hwnd } else { parent }, MONITOR_DEFAULTTONEAREST);
    if monitor.is_null() { return; }
    let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..std::mem::zeroed() };
    if GetMonitorInfoW(monitor, &mut info) == 0 { return; }
    let work = info.rcWork;
    let available_w = (work.right - work.left).max(320);
    let available_h = (work.bottom - work.top).max(300);
    let w = requested_w.min(available_w - 12).max(640.min(available_w));
    let h = requested_h.min(available_h - 12).max(480.min(available_h));
    let x = work.left + ((available_w - w) / 2).max(0);
    let y = work.top + ((available_h - h) / 2).max(0);
    MoveWindow(hwnd, x, y, w, h, 1);
}

unsafe fn create_static(hwnd: HWND, text: &str, x: i32, y: i32, w: i32, h: i32) -> HWND {
    create_control(hwnd, "STATIC", text, 0, x, y, w, h, WS_CHILD | WS_VISIBLE, 0)
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
    create_control(hwnd, "LISTBOX", "", id, x, y, w, h, WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_VSCROLL | LBS_NOTIFY | WS_BORDER, WS_EX_CLIENTEDGE)
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
