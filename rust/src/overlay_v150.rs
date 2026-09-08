use crate::{
    chat,
    logging,
    model::{channel_name, ChatMessage},
    paths::AppPaths,
    settings::{self, AppSettings},
};
use std::{
    collections::VecDeque,
    ffi::c_void,
    ptr::{null, null_mut},
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};
use windows_sys::Win32::{
    Foundation::{FILETIME, GetLastError, HINSTANCE, HWND, LPARAM, LRESULT, RECT, SYSTEMTIME, WPARAM},
    Graphics::Gdi::{
        BeginPaint, CreateSolidBrush, DeleteObject, DrawTextW, EndPaint, FillRect, GetStockObject,
        InvalidateRect, SelectObject, SetBkMode, SetTextColor, DEFAULT_GUI_FONT, HDC, PAINTSTRUCT,
        TRANSPARENT,
    },
    UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, GetClientRect, GetWindowLongPtrW, GetWindowRect,
        IsWindow, LoadCursorW, PostMessageW, RegisterClassW, SendMessageW,
        SetLayeredWindowAttributes, SetWindowLongPtrW, SetWindowPos, ShowWindow, CREATESTRUCTW,
        CW_USEDEFAULT, GWL_EXSTYLE, GWLP_USERDATA, HWND_NOTOPMOST, HWND_TOPMOST, IDC_ARROW,
        LWA_ALPHA, SW_HIDE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
        WM_CLOSE, WM_COMMAND, WM_ERASEBKGND, WM_EXITSIZEMOVE, WM_HOTKEY, WM_LBUTTONDOWN,
        WM_MOUSEWHEEL, WM_NCCREATE, WM_NCDESTROY, WM_PAINT, WM_SIZE, WS_EX_LAYERED,
        WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP, WNDCLASSW,
    },
};

const CLASS_NAME: &str = "BPSRReadyAlertRustOverlayV150";
const CMD_OPEN_SETTINGS: u32 = 1010;
const CMD_TTS_TOGGLE: u32 = 1013;
const CMD_ADD_TAB: u32 = 1014;
const CMD_TAB_BASE: u32 = 2000;
const MAX_MENU_TABS: usize = 50;
const TOOLBAR_HEIGHT: i32 = 42;
const DRAG_WIDTH: i32 = 38;
const ADD_WIDTH: i32 = 66;
const TTS_WIDTH: i32 = 52;
const GEAR_WIDTH: i32 = 40;
const COLLAPSE_WIDTH: i32 = 38;
const HIDE_WIDTH: i32 = 38;
const ROW_GAP: i32 = 4;
const RESIZE_GRIP: i32 = 10;
const COLLAPSED_THICKNESS: i32 = 24;
const CLICK_HOTKEY_ID: i32 = 0x5141;
const WM_NCHITTEST_: u32 = 0x0084;
const WM_NCLBUTTONDOWN_: u32 = 0x00A1;
const HTCLIENT_: isize = 1;
const HTCAPTION_: usize = 2;
const HTLEFT_: isize = 10;
const HTRIGHT_: isize = 11;
const HTTOP_: isize = 12;
const HTTOPLEFT_: isize = 13;
const HTTOPRIGHT_: isize = 14;
const HTBOTTOM_: isize = 15;
const HTBOTTOMLEFT_: isize = 16;
const HTBOTTOMRIGHT_: isize = 17;

const DT_CENTER: u32 = 0x0001;
const DT_VCENTER: u32 = 0x0004;
const DT_WORDBREAK: u32 = 0x0010;
const DT_SINGLELINE: u32 = 0x0020;
const DT_CALCRECT: u32 = 0x0400;
const DT_NOPREFIX: u32 = 0x0800;
const DT_END_ELLIPSIS: u32 = 0x8000;

#[repr(C)]
struct MonitorInfo {
    cb_size: u32,
    rc_monitor: RECT,
    rc_work: RECT,
    flags: u32,
}

#[link(name = "dwmapi")]
extern "system" {
    fn DwmSetWindowAttribute(hwnd: HWND, attribute: u32, value: *const c_void, size: u32) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    fn FileTimeToLocalFileTime(file_time: *const FILETIME, local_file_time: *mut FILETIME) -> i32;
    fn FileTimeToSystemTime(file_time: *const FILETIME, system_time: *mut SYSTEMTIME) -> i32;
}

#[link(name = "user32")]
extern "system" {
    fn MonitorFromWindow(hwnd: HWND, flags: u32) -> *mut c_void;
    fn GetMonitorInfoW(monitor: *mut c_void, info: *mut MonitorInfo) -> i32;
    fn RegisterHotKey(hwnd: HWND, id: i32, modifiers: u32, vk: u32) -> i32;
    fn UnregisterHotKey(hwnd: HWND, id: i32) -> i32;
    fn ReleaseCapture() -> i32;
}

#[derive(Clone)]
struct OverlayItem {
    message: ChatMessage,
    translation: Option<(String, String)>,
}

struct OverlayState {
    settings: Arc<RwLock<AppSettings>>,
    main_hwnd: HWND,
    paths: AppPaths,
    items: VecDeque<OverlayItem>,
    scroll_from_bottom: usize,
    collapsed: bool,
    expanded_bounds: RECT,
    hotkey_registered: bool,
}

#[derive(Clone, Copy)]
struct ActionRects {
    add: RECT,
    tts: RECT,
    gear: RECT,
    collapse: RECT,
    hide: RECT,
}

pub unsafe fn create(
    instance: HINSTANCE,
    settings: Arc<RwLock<AppSettings>>,
    main_hwnd: HWND,
    paths: AppPaths,
) -> Result<HWND, String> {
    let class = wide(CLASS_NAME);
    let wc = WNDCLASSW {
        lpfnWndProc: Some(overlay_wnd_proc),
        hInstance: instance,
        hCursor: LoadCursorW(null_mut(), IDC_ARROW),
        lpszClassName: class.as_ptr(),
        ..std::mem::zeroed()
    };
    if RegisterClassW(&wc) == 0 && GetLastError() != 1410 {
        return Err(format!("RegisterClassW(overlay) failed: {}", GetLastError()));
    }

    let snapshot = settings.read().map(|s| s.clone()).unwrap_or_default();
    let mut ex = WS_EX_TOOLWINDOW | WS_EX_LAYERED;
    if snapshot.chat.top_most { ex |= WS_EX_TOPMOST; }
    if snapshot.chat.click_through { ex |= WS_EX_TRANSPARENT; }
    let x = if snapshot.chat.window_x == i32::MIN { CW_USEDEFAULT } else { snapshot.chat.window_x };
    let y = if snapshot.chat.window_y == i32::MIN { CW_USEDEFAULT } else { snapshot.chat.window_y };

    let state = Box::new(OverlayState {
        settings,
        main_hwnd,
        paths,
        items: VecDeque::new(),
        scroll_from_bottom: 0,
        collapsed: false,
        expanded_bounds: RECT { left: 0, top: 0, right: snapshot.chat.window_width, bottom: snapshot.chat.window_height },
        hotkey_registered: false,
    });
    let state_ptr = Box::into_raw(state);
    let hwnd = CreateWindowExW(
        ex,
        class.as_ptr(),
        wide("BPSR Chat Overlay").as_ptr(),
        WS_POPUP,
        x,
        y,
        snapshot.chat.window_width,
        snapshot.chat.window_height,
        null_mut(),
        null_mut(),
        instance,
        state_ptr.cast::<c_void>(),
    );
    if hwnd.is_null() {
        drop(Box::from_raw(state_ptr));
        return Err(format!("CreateWindowExW(overlay) failed: {}", GetLastError()));
    }
    let mut rect: RECT = std::mem::zeroed();
    if GetWindowRect(hwnd, &mut rect) != 0 { (*state_ptr).expanded_bounds = rect; }
    try_dark_titlebar(hwnd);
    apply_style(hwnd, &snapshot);
    sync_hotkey(hwnd, &mut *state_ptr, &snapshot);
    Ok(hwnd)
}

pub unsafe fn push_chat(hwnd: HWND, message: ChatMessage) {
    let Some(state) = state_mut(hwnd) else { return; };
    let duplicate = state.items.iter().rev().take(64).any(|item| {
        if message.message_id != 0 {
            item.message.message_id == message.message_id && item.message.channel == message.channel
        } else {
            item.message.sender_id == message.sender_id
                && item.message.channel == message.channel
                && item.message.unix_seconds == message.unix_seconds
                && item.message.kind == message.kind
                && item.message.text == message.text
        }
    });
    if duplicate { return; }
    let snapshot = state.settings.read().map(|s| s.clone()).unwrap_or_default();
    let visible_now = !chat::should_hide_in_overlay(&snapshot, &message);
    if state.scroll_from_bottom > 0 && visible_now {
        state.scroll_from_bottom = state.scroll_from_bottom.saturating_add(1);
    }
    state.items.push_back(OverlayItem { message, translation: None });
    while state.items.len() > snapshot.chat.max_history.max(10) {
        state.items.pop_front();
        state.scroll_from_bottom = state.scroll_from_bottom.saturating_sub(1);
    }
    InvalidateRect(hwnd, null(), 0);
}

pub unsafe fn set_translation(hwnd: HWND, sequence_id: u64, text: String, source_language: String) {
    let Some(state) = state_mut(hwnd) else { return; };
    if let Some(item) = state.items.iter_mut().rev().find(|x| x.message.sequence_id == sequence_id) {
        let source = if source_language.trim().is_empty() { "AUTO".to_string() } else { source_language.trim().to_ascii_uppercase() };
        item.translation = Some((source, text));
        InvalidateRect(hwnd, null(), 0);
    }
}

pub unsafe fn refresh(hwnd: HWND) {
    if hwnd.is_null() || IsWindow(hwnd) == 0 { return; }
    if let Some(state) = state_mut(hwnd) {
        state.scroll_from_bottom = 0;
        let snapshot = state.settings.read().map(|s| s.clone()).unwrap_or_default();
        sync_hotkey(hwnd, state, &snapshot);
    }
    InvalidateRect(hwnd, null(), 0);
}

pub unsafe fn apply_style(hwnd: HWND, settings: &AppSettings) {
    if hwnd.is_null() || IsWindow(hwnd) == 0 { return; }
    let mut ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
    if settings.chat.click_through { ex |= WS_EX_TRANSPARENT; } else { ex &= !WS_EX_TRANSPARENT; }
    if settings.chat.top_most { ex |= WS_EX_TOPMOST; } else { ex &= !WS_EX_TOPMOST; }
    SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex as isize);
    SetWindowPos(
        hwnd,
        if settings.chat.top_most { HWND_TOPMOST } else { HWND_NOTOPMOST },
        0, 0, 0, 0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
    );
    let collapsed = state_mut(hwnd).map(|s| s.collapsed).unwrap_or(false);
    let percent = if collapsed { settings.chat.window_opacity.clamp(25, 58) } else { settings.chat.window_opacity.clamp(25, 100) };
    SetLayeredWindowAttributes(hwnd, 0, ((percent * 255) / 100) as u8, LWA_ALPHA);
    try_dark_titlebar(hwnd);
    InvalidateRect(hwnd, null(), 0);
}

pub unsafe fn expand_if_collapsed(hwnd: HWND) {
    if let Some(state) = state_mut(hwnd) {
        if state.collapsed { expand_from_edge(hwnd, state); }
    }
}

unsafe extern "system" fn overlay_wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        let cs = lparam as *const CREATESTRUCTW;
        if !cs.is_null() { SetWindowLongPtrW(hwnd, GWLP_USERDATA, (*cs).lpCreateParams as isize); }
    }
    match msg {
        WM_PAINT => {
            if let Some(state) = state_mut(hwnd) { paint(hwnd, state); }
            else { let mut ps: PAINTSTRUCT = std::mem::zeroed(); BeginPaint(hwnd, &mut ps); EndPaint(hwnd, &ps); }
            0
        }
        WM_ERASEBKGND => 1,
        WM_SIZE => { InvalidateRect(hwnd, null(), 0); 0 }
        WM_EXITSIZEMOVE => {
            if let Some(state) = state_mut(hwnd) { if !state.collapsed { persist_bounds(hwnd, state); } }
            0
        }
        WM_MOUSEWHEEL => {
            if let Some(state) = state_mut(hwnd) {
                if state.collapsed { return 0; }
                let delta = ((wparam >> 16) & 0xffff) as u16 as i16 as i32;
                let snapshot = state.settings.read().map(|s| s.clone()).unwrap_or_default();
                let visible = state.items.iter().filter(|x| !chat::should_hide_in_overlay(&snapshot, &x.message)).count();
                let max_scroll = visible.saturating_sub(1);
                if delta > 0 { state.scroll_from_bottom = (state.scroll_from_bottom + 3).min(max_scroll); }
                else if delta < 0 { state.scroll_from_bottom = state.scroll_from_bottom.saturating_sub(3); }
                InvalidateRect(hwnd, null(), 0);
            }
            0
        }
        WM_LBUTTONDOWN => {
            if let Some(state) = state_mut(hwnd) {
                if state.collapsed { expand_from_edge(hwnd, state); return 0; }
                let x = (lparam & 0xffff) as u16 as i16 as i32;
                let y = ((lparam >> 16) & 0xffff) as u16 as i16 as i32;
                if y >= 0 && y < TOOLBAR_HEIGHT {
                    let snapshot = state.settings.read().map(|s| s.clone()).unwrap_or_default();
                    let mut client: RECT = std::mem::zeroed();
                    GetClientRect(hwnd, &mut client);
                    let actions = action_rects(client.right);
                    if hit(actions.hide, x, y) { ShowWindow(hwnd, SW_HIDE); return 0; }
                    if hit(actions.collapse, x, y) { collapse_to_edge(hwnd, state); return 0; }
                    if hit(actions.gear, x, y) { PostMessageW(state.main_hwnd, WM_COMMAND, CMD_OPEN_SETTINGS as usize, 0); return 0; }
                    if hit(actions.tts, x, y) { PostMessageW(state.main_hwnd, WM_COMMAND, CMD_TTS_TOGGLE as usize, 0); return 0; }
                    if hit(actions.add, x, y) { PostMessageW(state.main_hwnd, WM_COMMAND, CMD_ADD_TAB as usize, 0); return 0; }
                    if x < DRAG_WIDTH {
                        ReleaseCapture();
                        SendMessageW(hwnd, WM_NCLBUTTONDOWN_, HTCAPTION_, 0);
                        return 0;
                    }
                    for (index, rect) in tab_rects(&snapshot, actions.add.left - 4).into_iter().enumerate() {
                        if index >= MAX_MENU_TABS { break; }
                        if hit(rect, x, y) {
                            PostMessageW(state.main_hwnd, WM_COMMAND, (CMD_TAB_BASE + index as u32) as usize, 0);
                            return 0;
                        }
                    }
                }
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_HOTKEY => {
            if wparam as i32 == CLICK_HOTKEY_ID {
                if let Some(state) = state_mut(hwnd) { toggle_clickthrough(hwnd, state); }
                return 0;
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_NCHITTEST_ => {
            if let Some(state) = state_mut(hwnd) {
                if state.collapsed { return HTCLIENT_; }
                let sx = (lparam & 0xffff) as u16 as i16 as i32;
                let sy = ((lparam >> 16) & 0xffff) as u16 as i16 as i32;
                let mut r: RECT = std::mem::zeroed();
                if GetWindowRect(hwnd, &mut r) != 0 {
                    let left = sx <= r.left + RESIZE_GRIP;
                    let right = sx >= r.right - RESIZE_GRIP;
                    let top = sy <= r.top + RESIZE_GRIP;
                    let bottom = sy >= r.bottom - RESIZE_GRIP;
                    return if left && top { HTTOPLEFT_ }
                    else if right && top { HTTOPRIGHT_ }
                    else if left && bottom { HTBOTTOMLEFT_ }
                    else if right && bottom { HTBOTTOMRIGHT_ }
                    else if left { HTLEFT_ }
                    else if right { HTRIGHT_ }
                    else if top { HTTOP_ }
                    else if bottom { HTBOTTOM_ }
                    else { HTCLIENT_ };
                }
            }
            HTCLIENT_
        }
        WM_CLOSE => { ShowWindow(hwnd, SW_HIDE); 0 }
        WM_NCDESTROY => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut OverlayState;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            if !ptr.is_null() {
                if (*ptr).hotkey_registered { let _ = UnregisterHotKey(hwnd, CLICK_HOTKEY_ID); }
                drop(Box::from_raw(ptr));
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn toggle_clickthrough(hwnd: HWND, state: &mut OverlayState) {
    let snapshot = if let Ok(mut s) = state.settings.write() {
        s.chat.click_through = !s.chat.click_through;
        s.normalize();
        s.clone()
    } else { return; };
    if let Err(err) = settings::save(&state.paths, &snapshot) { logging::write(format!("settings: save failed: {err}")); }
    apply_style(hwnd, &snapshot);
    sync_hotkey(hwnd, state, &snapshot);
}

unsafe fn sync_hotkey(hwnd: HWND, state: &mut OverlayState, snapshot: &AppSettings) {
    if state.hotkey_registered {
        let _ = UnregisterHotKey(hwnd, CLICK_HOTKEY_ID);
        state.hotkey_registered = false;
    }
    if let Some((mods, vk)) = parse_hotkey(&snapshot.chat.click_through_hotkey) {
        state.hotkey_registered = RegisterHotKey(hwnd, CLICK_HOTKEY_ID, mods, vk) != 0;
        if !state.hotkey_registered { logging::write("chat: click-through hotkey registration failed"); }
    }
}

fn parse_hotkey(value: &str) -> Option<(u32, u32)> {
    let mut mods = 0u32;
    let mut key = None;
    for token in value.split('+').map(str::trim).filter(|x| !x.is_empty()) {
        match token.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => mods |= 0x0002,
            "shift" => mods |= 0x0004,
            "alt" => mods |= 0x0001,
            "win" | "windows" => mods |= 0x0008,
            other => {
                let upper = other.to_ascii_uppercase();
                let parsed = if let Some(rest) = upper.strip_prefix('F') {
                    rest.parse::<u32>().ok().filter(|n| (1..=24).contains(n)).map(|n| 0x70 + n - 1)
                } else if upper.len() == 1 {
                    upper.bytes().next().map(u32::from).filter(|v| (*v >= b'A' as u32 && *v <= b'Z' as u32) || (*v >= b'0' as u32 && *v <= b'9' as u32))
                } else { None };
                key = parsed.or(key);
            }
        }
    }
    key.map(|vk| (mods | 0x4000, vk))
}

unsafe fn collapse_to_edge(hwnd: HWND, state: &mut OverlayState) {
    if state.collapsed { return; }
    let mut expanded: RECT = std::mem::zeroed();
    if GetWindowRect(hwnd, &mut expanded) == 0 { return; }
    state.expanded_bounds = expanded;
    persist_bounds(hwnd, state);
    let work = monitor_work_area(hwnd).unwrap_or(RECT { left: 0, top: 0, right: 1920, bottom: 1080 });
    let snapshot = state.settings.read().map(|s| s.clone()).unwrap_or_default();
    let side = snapshot.chat.collapse_side.to_ascii_lowercase();
    let expanded_w = (expanded.right - expanded.left).max(420);
    let expanded_h = (expanded.bottom - expanded.top).max(220);
    let (x, y, w, h) = if side == "left" || side == "right" {
        let h = expanded_h.min(work.bottom - work.top);
        let y = expanded.top.clamp(work.top, (work.bottom - h).max(work.top));
        let x = if side == "left" { work.left } else { work.right - COLLAPSED_THICKNESS };
        (x, y, COLLAPSED_THICKNESS, h)
    } else {
        let w = expanded_w.min(work.right - work.left);
        let x = expanded.left.clamp(work.left, (work.right - w).max(work.left));
        let y = if side == "top" { work.top } else { work.bottom - COLLAPSED_THICKNESS };
        (x, y, w, COLLAPSED_THICKNESS)
    };
    state.collapsed = true;
    SetWindowPos(hwnd, null_mut(), x, y, w, h, SWP_NOACTIVATE | SWP_NOZORDER);
    apply_style(hwnd, &snapshot);
    InvalidateRect(hwnd, null(), 0);
}

unsafe fn expand_from_edge(hwnd: HWND, state: &mut OverlayState) {
    if !state.collapsed { return; }
    state.collapsed = false;
    let r = state.expanded_bounds;
    SetWindowPos(hwnd, null_mut(), r.left, r.top, (r.right-r.left).max(420), (r.bottom-r.top).max(220), SWP_NOACTIVATE | SWP_NOZORDER);
    let snapshot = state.settings.read().map(|s| s.clone()).unwrap_or_default();
    apply_style(hwnd, &snapshot);
    InvalidateRect(hwnd, null(), 0);
}

unsafe fn monitor_work_area(hwnd: HWND) -> Option<RECT> {
    let monitor = MonitorFromWindow(hwnd, 2);
    if monitor.is_null() { return None; }
    let mut info = MonitorInfo {
        cb_size: std::mem::size_of::<MonitorInfo>() as u32,
        rc_monitor: std::mem::zeroed(),
        rc_work: std::mem::zeroed(),
        flags: 0,
    };
    if GetMonitorInfoW(monitor, &mut info) == 0 { None } else { Some(info.rc_work) }
}

unsafe fn persist_bounds(hwnd: HWND, state: &mut OverlayState) {
    let rect = if state.collapsed {
        state.expanded_bounds
    } else {
        let mut r: RECT = std::mem::zeroed();
        if GetWindowRect(hwnd, &mut r) == 0 { return; }
        state.expanded_bounds = r;
        r
    };
    let snapshot = if let Ok(mut guard) = state.settings.write() {
        guard.chat.window_x = rect.left;
        guard.chat.window_y = rect.top;
        guard.chat.window_width = (rect.right - rect.left).max(420);
        guard.chat.window_height = (rect.bottom - rect.top).max(220);
        guard.normalize();
        guard.clone()
    } else { return; };
    let _ = settings::save(&state.paths, &snapshot);
}

unsafe fn state_mut(hwnd: HWND) -> Option<&'static mut OverlayState> {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut OverlayState;
    ptr.as_mut()
}

unsafe fn paint(hwnd: HWND, state: &mut OverlayState) {
    let snapshot = state.settings.read().map(|s| s.clone()).unwrap_or_default();
    let mut ps: PAINTSTRUCT = std::mem::zeroed();
    let hdc = BeginPaint(hwnd, &mut ps);
    if hdc.is_null() { return; }
    SetBkMode(hdc, TRANSPARENT as i32);
    let font = GetStockObject(DEFAULT_GUI_FONT);
    let old_font = SelectObject(hdc, font);
    let mut client: RECT = std::mem::zeroed();
    GetClientRect(hwnd, &mut client);

    if state.collapsed {
        fill(hdc, &client, rgb(26, 30, 36));
        let glyph = match snapshot.chat.collapse_side.to_ascii_lowercase().as_str() {
            "left" => "▶", "top" => "▼", "bottom" => "▲", _ => "◀"
        };
        draw_text(hdc, glyph, client, rgb(215, 223, 232), DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX, false);
        SelectObject(hdc, old_font);
        EndPaint(hwnd, &ps);
        return;
    }

    let window_back = scaled_color((20, 23, 28), snapshot.chat.background_opacity);
    fill(hdc, &client, window_back);
    draw_border(hdc, client);
    let toolbar = RECT { left: 2, top: 2, right: client.right - 2, bottom: TOOLBAR_HEIGHT };
    fill(hdc, &toolbar, scaled_color((26, 30, 36), snapshot.chat.toolbar_opacity));
    draw_toolbar(hdc, &snapshot, client.right);

    let filtered: Vec<&OverlayItem> = state.items.iter().filter(|x| !chat::should_hide_in_overlay(&snapshot, &x.message)).collect();
    let max_scroll = filtered.len().saturating_sub(1);
    state.scroll_from_bottom = state.scroll_from_bottom.min(max_scroll);
    let end = filtered.len().saturating_sub(state.scroll_from_bottom);
    let content_left = 9;
    let content_right = (client.right - 9).max(content_left + 40);
    let available_width = (content_right - content_left).max(40);
    let mut y = client.bottom - 6;
    let mut ordinal = end;
    for item in filtered[..end].iter().rev() {
        let row_height = measure_row(hdc, &snapshot, item, available_width);
        let top = y - row_height;
        if top < TOOLBAR_HEIGHT + 2 { break; }
        ordinal = ordinal.saturating_sub(1);
        draw_row(hdc, &snapshot, item, ordinal, RECT { left: 4, top, right: client.right - 4, bottom: y });
        y = top - ROW_GAP;
    }
    if state.scroll_from_bottom > 0 {
        let label = format!("↓ {} new message{}", state.scroll_from_bottom, if state.scroll_from_bottom == 1 { "" } else { "s" });
        let rect = RECT { left: client.right.saturating_sub(170), top: TOOLBAR_HEIGHT + 5, right: client.right - 10, bottom: TOOLBAR_HEIGHT + 29 };
        fill(hdc, &rect, rgb(41, 94, 180));
        draw_text(hdc, &label, rect, rgb(255,255,255), DT_CENTER | DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX, false);
    }
    SelectObject(hdc, old_font);
    EndPaint(hwnd, &ps);
}

unsafe fn draw_border(hdc: HDC, client: RECT) {
    let c = rgb(75, 86, 99);
    fill(hdc, &RECT { left: 0, top: 0, right: client.right, bottom: 2 }, c);
    fill(hdc, &RECT { left: 0, top: client.bottom - 2, right: client.right, bottom: client.bottom }, c);
    fill(hdc, &RECT { left: 0, top: 0, right: 2, bottom: client.bottom }, c);
    fill(hdc, &RECT { left: client.right - 2, top: 0, right: client.right, bottom: client.bottom }, c);
}

unsafe fn draw_toolbar(hdc: HDC, snapshot: &AppSettings, width: i32) {
    let actions = action_rects(width);
    let drag = RECT { left: 2, top: 2, right: DRAG_WIDTH, bottom: TOOLBAR_HEIGHT };
    draw_text(hdc, "⋮⋮", drag, rgb(220,226,233), DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX, false);
    draw_tabs(hdc, snapshot, actions.add.left - 4);
    draw_toolbar_button(hdc, actions.add, "+ Tab", rgb(26,30,36), rgb(239,243,247));
    let speech = &snapshot.speech_translation;
    let (tts_back, tts_text) = if !speech.tts_enabled {
        (rgb(112,47,52), rgb(255,255,255))
    } else if speech.tts_volume <= 0 || (!speech.tts_guild && !speech.tts_party_team) {
        (rgb(126,83,31), rgb(255,255,255))
    } else {
        (rgb(38,105,70), rgb(255,255,255))
    };
    draw_toolbar_button(hdc, actions.tts, "TTS", tts_back, tts_text);
    draw_toolbar_button(hdc, actions.gear, "⚙", rgb(26,30,36), rgb(239,243,247));
    let collapse = match snapshot.chat.collapse_side.to_ascii_lowercase().as_str() { "left" => "◀", "top" => "▲", "bottom" => "▼", _ => "▶" };
    draw_toolbar_button(hdc, actions.collapse, collapse, rgb(26,30,36), rgb(239,243,247));
    draw_toolbar_button(hdc, actions.hide, "×", rgb(26,30,36), rgb(239,243,247));
}

unsafe fn draw_toolbar_button(hdc: HDC, rect: RECT, text: &str, back: u32, fore: u32) {
    fill(hdc, &rect, back);
    draw_text(hdc, text, rect, fore, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX, false);
}

fn action_rects(width: i32) -> ActionRects {
    let mut r = (width - 2).max(240);
    let hide = RECT { left: r - HIDE_WIDTH, top: 2, right: r, bottom: TOOLBAR_HEIGHT }; r -= HIDE_WIDTH;
    let collapse = RECT { left: r - COLLAPSE_WIDTH, top: 2, right: r, bottom: TOOLBAR_HEIGHT }; r -= COLLAPSE_WIDTH;
    let gear = RECT { left: r - GEAR_WIDTH, top: 2, right: r, bottom: TOOLBAR_HEIGHT }; r -= GEAR_WIDTH;
    let tts = RECT { left: r - TTS_WIDTH, top: 2, right: r, bottom: TOOLBAR_HEIGHT }; r -= TTS_WIDTH;
    let add = RECT { left: r - ADD_WIDTH, top: 2, right: r, bottom: TOOLBAR_HEIGHT };
    ActionRects { add, tts, gear, collapse, hide }
}

unsafe fn draw_tabs(hdc: HDC, settings: &AppSettings, max_right: i32) {
    for (index, rect) in tab_rects(settings, max_right).into_iter().enumerate() {
        let Some(tab) = settings.chat.tabs.get(index) else { break; };
        let selected = tab.id == settings.chat.last_selected_tab_id;
        if selected {
            fill(hdc, &rect, rgb(31, 36, 44));
            fill(hdc, &RECT { left: rect.left + 10, top: rect.bottom - 3, right: rect.right - 10, bottom: rect.bottom }, rgb(73, 132, 255));
        }
        let color = if selected { rgb(239,243,247) } else { rgb(170,181,194) };
        draw_text(hdc, &tab.name, rect, color, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX, false);
    }
}

fn tab_rects(settings: &AppSettings, max_right: i32) -> Vec<RECT> {
    let mut x = DRAG_WIDTH + 4;
    let mut out = Vec::new();
    for tab in settings.chat.tabs.iter().take(MAX_MENU_TABS) {
        let guessed = 28 + tab.name.chars().count() as i32 * 8;
        let w = guessed.clamp(68, 150);
        if x + w > max_right && !out.is_empty() { break; }
        if x >= max_right { break; }
        out.push(RECT { left: x, top: 2, right: (x + w).min(max_right), bottom: TOOLBAR_HEIGHT });
        x += w + 3;
    }
    out
}

fn hit(rect: RECT, x: i32, y: i32) -> bool { x >= rect.left && x < rect.right && y >= rect.top && y < rect.bottom }

unsafe fn measure_row(hdc: HDC, settings: &AppSettings, item: &OverlayItem, width: i32) -> i32 {
    let message_width = (width - 16).max(50);
    let message_h = measure_text(hdc, item.message.text.trim(), message_width).max(18);
    let translation_h = item.translation.as_ref().map(|(source, text)| measure_text(hdc, &format!("{source} → EN: {text}"), message_width).max(16) + 4).unwrap_or(0);
    if settings.chat.compact_mode { message_h + translation_h + 14 } else { 22 + message_h + translation_h + 14 }
}

unsafe fn draw_row(hdc: HDC, settings: &AppSettings, item: &OverlayItem, ordinal: usize, rect: RECT) {
    let base = if settings.chat.show_zebra_stripes && (ordinal & 1) == 1 { (26,30,36) } else { (20,23,28) };
    let mut back = base;
    let hay = format!("{} {}", item.message.sender_name, item.message.text).to_ascii_lowercase();
    if item.message.channel == 5 && settings.chat.private_highlight_enabled {
        back = blend_tuple(parse_hex(&settings.chat.private_highlight_color).unwrap_or((86,53,93)), back, 40);
    } else if !settings.chat.highlight_if_matches.trim().is_empty() && chat::matches_expression(&hay, &settings.chat.highlight_if_matches) {
        back = blend_tuple(parse_hex(&settings.chat.highlight_color).unwrap_or((107,90,58)), back, 36);
    }
    let back_ref = rgb(back.0, back.1, back.2);
    fill(hdc, &rect, back_ref);
    let channel_rgb = settings.chat.channel_colors.get(&item.message.channel).and_then(|x| parse_hex(x)).unwrap_or((199,199,199));
    let channel = rgb(channel_rgb.0, channel_rgb.1, channel_rgb.2);
    if settings.chat.show_color_band { fill(hdc, &RECT { left: rect.left, top: rect.top + 2, right: rect.left + 3, bottom: rect.bottom - 2 }, channel); }
    let left = rect.left + if settings.chat.show_color_band { 10 } else { 7 };
    let right = rect.right - 9;
    let mut y = rect.top + 5;
    let sender = if item.message.sender_name.trim().is_empty() { "?" } else { item.message.sender_name.trim() };
    let sender_label = if item.message.sender_level > 0 { format!("{} Lv{}", sender, item.message.sender_level) } else { sender.to_string() };
    let sender_color = sender_color(&item.message);
    let text_color = blend_color(rgb(239,243,247), back_ref, settings.chat.text_opacity);
    let meta_color = blend_color(rgb(157,170,188), back_ref, settings.chat.text_opacity);
    let shadow = settings.chat.text_shadow;
    if settings.chat.compact_mode {
        let mut x = left;
        x = draw_inline(hdc, &format!("{} · ", channel_name(item.message.channel)), x, y, right, channel, shadow);
        if settings.chat.show_time { x = draw_inline(hdc, &format!("{}  ", time_text(&item.message, settings.chat.show_time_as_ago)), x, y, right, meta_color, shadow); }
        x = draw_inline(hdc, &format!("{}  ", sender_label), x, y, right, sender_color, shadow);
        draw_text(hdc, item.message.text.trim(), RECT { left: x.min(right-20), top: y, right, bottom: rect.bottom - 5 - if item.translation.is_some() {20} else {0} }, text_color, DT_WORDBREAK | DT_NOPREFIX, shadow);
    } else {
        let mut x = left;
        x = draw_inline(hdc, &format!("{}  ", channel_name(item.message.channel)), x, y, right, channel, shadow);
        x = draw_inline(hdc, &sender_label, x, y, right, sender_color, shadow);
        if settings.chat.show_time { let _ = draw_inline(hdc, &format!("   {}", time_text(&item.message, settings.chat.show_time_as_ago)), x, y, right, meta_color, shadow); }
        y += 22;
        let translation_space = if item.translation.is_some() { 22 } else { 0 };
        draw_text(hdc, item.message.text.trim(), RECT { left, top: y, right, bottom: rect.bottom - 5 - translation_space }, text_color, DT_WORDBREAK | DT_NOPREFIX, shadow);
    }
    if let Some((source, text)) = &item.translation {
        draw_text(hdc, &format!("{source} → EN: {text}"), RECT { left, top: rect.bottom-24, right, bottom: rect.bottom-4 }, blend_color(rgb(143,203,255), back_ref, settings.chat.text_opacity), DT_WORDBREAK | DT_NOPREFIX, shadow);
    }
    if settings.chat.show_separators { fill(hdc, &RECT { left: rect.left+9, top: rect.bottom-1, right: rect.right-2, bottom: rect.bottom }, blend_color(rgb(255,255,255), back_ref, 10)); }
}

unsafe fn measure_text(hdc: HDC, text: &str, width: i32) -> i32 {
    if text.trim().is_empty() { return 0; }
    let w = wide(text);
    let mut rect = RECT { left: 0, top: 0, right: width.max(20), bottom: 0 };
    DrawTextW(hdc, w.as_ptr(), -1, &mut rect, DT_CALCRECT | DT_WORDBREAK | DT_NOPREFIX);
    (rect.bottom - rect.top).max(0)
}

unsafe fn draw_inline(hdc: HDC, text: &str, x: i32, y: i32, right: i32, color: u32, shadow: bool) -> i32 {
    if x >= right { return x; }
    let w = wide(text);
    let mut measure = RECT { left: 0, top: 0, right: (right-x).max(1), bottom: 0 };
    DrawTextW(hdc, w.as_ptr(), -1, &mut measure, DT_CALCRECT | DT_SINGLELINE | DT_NOPREFIX);
    let width = (measure.right-measure.left).max(1).min((right-x).max(1));
    draw_text(hdc, text, RECT { left: x, top: y, right: x+width, bottom: y+20 }, color, DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX, shadow);
    x + width
}

unsafe fn draw_text(hdc: HDC, text: &str, rect: RECT, color: u32, flags: u32, shadow: bool) {
    if text.is_empty() || rect.right <= rect.left || rect.bottom <= rect.top { return; }
    let w = wide(text);
    if shadow {
        let mut sr = RECT { left: rect.left+1, top: rect.top+1, right: rect.right+1, bottom: rect.bottom+1 };
        SetTextColor(hdc, rgb(0,0,0));
        DrawTextW(hdc, w.as_ptr(), -1, &mut sr, flags);
    }
    let mut dr = rect;
    SetTextColor(hdc, color);
    DrawTextW(hdc, w.as_ptr(), -1, &mut dr, flags);
}

unsafe fn fill(hdc: HDC, rect: &RECT, color: u32) {
    let brush = CreateSolidBrush(color);
    if !brush.is_null() { FillRect(hdc, rect, brush); DeleteObject(brush); }
}

unsafe fn try_dark_titlebar(hwnd: HWND) {
    let enabled: i32 = 1;
    let ptr = (&enabled as *const i32).cast::<c_void>();
    if DwmSetWindowAttribute(hwnd, 20, ptr, std::mem::size_of::<i32>() as u32) != 0 { let _ = DwmSetWindowAttribute(hwnd, 19, ptr, std::mem::size_of::<i32>() as u32); }
}

fn sender_color(message: &ChatMessage) -> u32 {
    const PALETTE: [(u8,u8,u8);10] = [(121,192,255),(255,174,121),(143,237,143),(255,181,194),(205,170,255),(255,214,102),(115,220,210),(255,151,196),(159,184,255),(190,224,126)];
    let mut h = 14695981039346656037u64;
    if message.sender_id != 0 { for b in message.sender_id.to_le_bytes() { h=(h^u64::from(b)).wrapping_mul(1099511628211); } }
    else { for b in message.sender_name.as_bytes() { h=(h^u64::from(*b)).wrapping_mul(1099511628211); } }
    let c=PALETTE[(h as usize)%PALETTE.len()]; rgb(c.0,c.1,c.2)
}

fn time_text(message: &ChatMessage, ago: bool) -> String {
    if message.unix_seconds <= 0 { return String::new(); }
    if ago {
        let now=SystemTime::now().duration_since(UNIX_EPOCH).map(|d|d.as_secs() as i64).unwrap_or(message.unix_seconds);
        let secs=now.saturating_sub(message.unix_seconds).max(0);
        return if secs < 60 { format!("{}s", secs) } else if secs < 3600 { format!("{}m", secs/60) } else if secs < 86_400 { format!("{}h", secs/3600) } else { format!("{}d", secs/86_400) };
    }
    unsafe {
        const WINDOWS_TO_UNIX_SECONDS:i128=11_644_473_600;
        let ticks=(i128::from(message.unix_seconds)+WINDOWS_TO_UNIX_SECONDS)*10_000_000;
        if ticks<0 || ticks>i128::from(u64::MAX) { return String::new(); }
        let ticks=ticks as u64;
        let utc=FILETIME{dwLowDateTime:ticks as u32,dwHighDateTime:(ticks>>32) as u32};
        let mut local_ft:FILETIME=std::mem::zeroed(); let mut local:SYSTEMTIME=std::mem::zeroed();
        if FileTimeToLocalFileTime(&utc,&mut local_ft)!=0 && FileTimeToSystemTime(&local_ft,&mut local)!=0 { format!("{:02}:{:02}",local.wHour,local.wMinute) } else { String::new() }
    }
}

fn scaled_color(color:(u8,u8,u8),opacity:i32)->u32 { let p=opacity.clamp(0,100) as u32; rgb(((color.0 as u32*p)/100)as u8,((color.1 as u32*p)/100)as u8,((color.2 as u32*p)/100)as u8) }
fn blend_tuple(fg:(u8,u8,u8),bg:(u8,u8,u8),percent:i32)->(u8,u8,u8){ let p=percent.clamp(0,100)as u32; let q=100-p; (((fg.0 as u32*p+bg.0 as u32*q)/100)as u8,((fg.1 as u32*p+bg.1 as u32*q)/100)as u8,((fg.2 as u32*p+bg.2 as u32*q)/100)as u8) }
fn blend_color(fg:u32,bg:u32,percent:i32)->u32{ let f=((fg&0xff)as u8,((fg>>8)&0xff)as u8,((fg>>16)&0xff)as u8); let b=((bg&0xff)as u8,((bg>>8)&0xff)as u8,((bg>>16)&0xff)as u8); let c=blend_tuple(f,b,percent); rgb(c.0,c.1,c.2) }
fn parse_hex(value:&str)->Option<(u8,u8,u8)>{ let s=value.trim().strip_prefix('#')?; if s.len()!=6{return None;} let n=u32::from_str_radix(s,16).ok()?; Some(((n>>16)as u8,(n>>8)as u8,n as u8)) }
const fn rgb(r:u8,g:u8,b:u8)->u32{r as u32|((g as u32)<<8)|((b as u32)<<16)}
fn wide(text:&str)->Vec<u16>{text.encode_utf16().chain(std::iter::once(0)).collect()}
