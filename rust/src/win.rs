use crate::{
    audio, chat, logging,
    model::{channel_name, AlertKind, AppEvent, ChatMessage},
    paths::AppPaths,
    settings::{self, AppSettings},
};
use std::{
    collections::VecDeque,
    ffi::c_void,
    path::{Path, PathBuf},
    ptr::{null, null_mut},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, TryRecvError},
        Arc, RwLock,
    },
    thread,
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE, HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM},
    Graphics::Gdi::{GetStockObject, DEFAULT_GUI_FONT},
    System::{
        LibraryLoader::GetModuleHandleW,
        Threading::{CreateMutexW, ReleaseMutex},
    },
    UI::{
        Shell::{
            ShellExecuteW, Shell_NotifyIconW, NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP,
            NIIF_ERROR, NIIF_INFO, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW,
        },
        WindowsAndMessaging::{
            AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu,
            DestroyWindow, DispatchMessageW, GetClientRect, GetCursorPos, GetDlgItem,
            GetMessageW, GetWindowLongPtrW, GetWindowTextLengthW, IsWindow, LoadCursorW,
            LoadIconW, MessageBoxW, MoveWindow, PostMessageW, PostQuitMessage, RegisterClassW,
            SendMessageW, SetForegroundWindow, SetLayeredWindowAttributes, SetTimer,
            SetWindowLongPtrW, SetWindowTextW, ShowWindow, TrackPopupMenu, TranslateMessage,
            CREATESTRUCTW, CW_USEDEFAULT, GWL_EXSTYLE, GWLP_USERDATA, IDC_ARROW, IDI_APPLICATION,
            LWA_ALPHA, MB_ICONERROR, MB_ICONINFORMATION, MB_OK, MF_CHECKED, MF_POPUP, MF_SEPARATOR,
            MF_STRING, MSG, SW_HIDE, SW_SHOWNORMAL, SW_SHOW, TPM_BOTTOMALIGN, TPM_LEFTALIGN,
            TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_APP, WM_CLOSE, WM_COMMAND, WM_DESTROY,
            HICON, HMENU, WM_LBUTTONDBLCLK, WM_NCCREATE, WM_RBUTTONUP, WM_SIZE, WM_TIMER, WNDCLASSW,
            WS_CAPTION, WS_CHILD, WS_EX_CLIENTEDGE, WS_EX_LAYERED, WS_EX_TOOLWINDOW,
            WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_OVERLAPPED, WS_POPUP, WS_SYSMENU,
            WS_THICKFRAME, WS_VISIBLE, WS_VSCROLL,
        },
    },
};

const WM_TRAY: u32 = WM_APP + 17;
const TRAY_ID: u32 = 1;
const EDIT_ID: i32 = 101;
const TIMER_ID: usize = 1;
const ES_MULTILINE: u32 = 0x0004;
const ES_AUTOVSCROLL: u32 = 0x0040;
const ES_READONLY: u32 = 0x0800;
const EM_SETSEL: u32 = 0x00B1;
const EM_REPLACESEL: u32 = 0x00C2;

const CMD_QUEUE: u32 = 1001;
const CMD_READY: u32 = 1002;
const CMD_INVITE: u32 = 1003;
const CMD_REQUEST: u32 = 1004;
const CMD_DESKTOP: u32 = 1005;
const CMD_CHAT: u32 = 1006;
const CMD_OPEN_SETTINGS: u32 = 1010;
const CMD_OPEN_LOGS: u32 = 1011;
const CMD_SHOW_CHAT: u32 = 1012;
const CMD_EXIT: u32 = 1099;
const CMD_TAB_BASE: u32 = 2000;
const MAX_MENU_TABS: usize = 50;

pub struct SingleInstance {
    handle: HANDLE,
    owned: bool,
}

impl SingleInstance {
    pub fn acquire() -> Result<Self, String> {
        let name = wide("Local\\BPSR-ReadyAlert-Rust-v1");
        let handle = unsafe { CreateMutexW(null(), 1, name.as_ptr()) };
        if handle.is_null() { return Err(format!("CreateMutexW failed: {}", unsafe { GetLastError() })); }
        let already = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
        Ok(Self { handle, owned: !already })
    }
    pub fn is_owner(&self) -> bool { self.owned }
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        unsafe {
            if self.owned { let _ = ReleaseMutex(self.handle); }
            let _ = CloseHandle(self.handle);
        }
    }
}

struct UiState {
    settings: Arc<RwLock<AppSettings>>,
    paths: AppPaths,
    rx: Receiver<AppEvent>,
    stop: Arc<AtomicBool>,
    overlay: HWND,
    overlay_lines: VecDeque<String>,
    capture_status: String,
}

pub fn run_ui(
    settings: Arc<RwLock<AppSettings>>,
    paths: AppPaths,
    rx: Receiver<AppEvent>,
    stop: Arc<AtomicBool>,
) -> Result<(), String> {
    unsafe {
        let instance = GetModuleHandleW(null());
        let main_class = wide("BPSRReadyAlertRustMain");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(main_wnd_proc),
            hInstance: instance,
            hIcon: app_icon(instance),
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            lpszClassName: main_class.as_ptr(),
            ..std::mem::zeroed()
        };
        if RegisterClassW(&wc) == 0 && GetLastError() != 1410 {
            return Err(format!("RegisterClassW(main) failed: {}", GetLastError()));
        }

        let mut state = Box::new(UiState {
            settings,
            paths,
            rx,
            stop,
            overlay: null_mut(),
            overlay_lines: VecDeque::new(),
            capture_status: "Starting capture…".into(),
        });
        let state_ptr: *mut UiState = &mut *state;
        let title = wide("BPSR Ready Alert");
        let hwnd = CreateWindowExW(
            0,
            main_class.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPED,
            0, 0, 0, 0,
            null_mut(), null_mut(), instance,
            state_ptr.cast::<c_void>(),
        );
        if hwnd.is_null() { return Err(format!("CreateWindowExW(main) failed: {}", GetLastError())); }

        state.overlay = create_overlay(instance, &state.settings.read().map(|s| s.clone()).unwrap_or_default())?;
        add_tray(hwnd, &state.capture_status)?;
        if state.settings.read().map(|s| s.chat_overlay_enabled).unwrap_or(true) {
            ShowWindow(state.overlay, SW_SHOW);
        }
        SetTimer(hwnd, TIMER_ID, 50, None);

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        state.stop.store(true, Ordering::Relaxed);
        drop(state);
        Ok(())
    }
}

unsafe extern "system" fn main_wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        let cs = lparam as *const CREATESTRUCTW;
        if !cs.is_null() {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, (*cs).lpCreateParams as isize);
        }
    }
    let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut UiState;
    match msg {
        WM_TIMER if wparam == TIMER_ID => {
            if !state_ptr.is_null() { drain_events(hwnd, &mut *state_ptr); }
            0
        }
        WM_TRAY => {
            if !state_ptr.is_null() {
                let mouse = lparam as u32;
                if mouse == WM_RBUTTONUP {
                    show_tray_menu(hwnd, &mut *state_ptr);
                } else if mouse == WM_LBUTTONDBLCLK {
                    toggle_overlay(&mut *state_ptr);
                }
            }
            0
        }
        WM_COMMAND => {
            if !state_ptr.is_null() { handle_command(hwnd, &mut *state_ptr, (wparam as u32) & 0xffff); }
            0
        }
        WM_DESTROY => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                state.stop.store(true, Ordering::Relaxed);
                if !state.overlay.is_null() && IsWindow(state.overlay) != 0 { DestroyWindow(state.overlay); }
            }
            remove_tray(hwnd);
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe extern "system" fn overlay_wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_SIZE => {
            let edit = GetDlgItem(hwnd, EDIT_ID);
            if !edit.is_null() {
                let mut rect = std::mem::zeroed();
                GetClientRect(hwnd, &mut rect);
                MoveWindow(edit, 0, 0, rect.right - rect.left, rect.bottom - rect.top, 1);
            }
            0
        }
        WM_CLOSE => { ShowWindow(hwnd, SW_HIDE); 0 }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn create_overlay(instance: HINSTANCE, settings: &AppSettings) -> Result<HWND, String> {
    let class = wide("BPSRReadyAlertRustOverlay");
    let wc = WNDCLASSW {
        lpfnWndProc: Some(overlay_wnd_proc),
        hInstance: instance,
        hIcon: app_icon(instance),
        hCursor: LoadCursorW(null_mut(), IDC_ARROW),
        lpszClassName: class.as_ptr(),
        ..std::mem::zeroed()
    };
    if RegisterClassW(&wc) == 0 && GetLastError() != 1410 {
        return Err(format!("RegisterClassW(overlay) failed: {}", GetLastError()));
    }
    let mut ex = WS_EX_TOOLWINDOW | WS_EX_LAYERED;
    if settings.chat.top_most { ex |= WS_EX_TOPMOST; }
    if settings.chat.click_through { ex |= WS_EX_TRANSPARENT; }
    let x = if settings.chat.window_x == i32::MIN { CW_USEDEFAULT } else { settings.chat.window_x };
    let y = if settings.chat.window_y == i32::MIN { CW_USEDEFAULT } else { settings.chat.window_y };
    let title = wide("BPSR Chat Overlay");
    let hwnd = CreateWindowExW(
        ex, class.as_ptr(), title.as_ptr(),
        WS_POPUP | WS_CAPTION | WS_THICKFRAME | WS_SYSMENU,
        x, y, settings.chat.window_width, settings.chat.window_height,
        null_mut(), null_mut(), instance, null(),
    );
    if hwnd.is_null() { return Err(format!("CreateWindowExW(overlay) failed: {}", GetLastError())); }
    let edit_class = wide("EDIT");
    let empty = wide("");
    let edit = CreateWindowExW(
        WS_EX_CLIENTEDGE,
        edit_class.as_ptr(), empty.as_ptr(),
        WS_CHILD | WS_VISIBLE | WS_VSCROLL | ES_MULTILINE | ES_AUTOVSCROLL | ES_READONLY,
        0, 0, settings.chat.window_width, settings.chat.window_height,
        hwnd, EDIT_ID as usize as HMENU, instance, null(),
    );
    if edit.is_null() { DestroyWindow(hwnd); return Err(format!("CreateWindowExW(edit) failed: {}", GetLastError())); }
    let font = GetStockObject(DEFAULT_GUI_FONT);
    SendMessageW(edit, 0x0030, font as usize, 1);
    let alpha = ((settings.chat.window_opacity.clamp(25, 100) * 255) / 100) as u8;
    SetLayeredWindowAttributes(hwnd, 0, alpha, LWA_ALPHA);
    Ok(hwnd)
}

unsafe fn drain_events(hwnd: HWND, state: &mut UiState) {
    loop {
        match state.rx.try_recv() {
            Ok(event) => handle_event(hwnd, state, event),
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => break,
        }
    }
}

unsafe fn handle_event(hwnd: HWND, state: &mut UiState, event: AppEvent) {
    match event {
        AppEvent::Alert(alert) => {
            let snapshot = state.settings.read().map(|s| s.clone()).unwrap_or_default();
            if alert_enabled(&snapshot, alert.kind) {
                let volume = snapshot.alert_volume;
                thread::spawn(move || audio::play_alert(alert.kind, volume));
                if snapshot.desktop_notification { balloon(hwnd, &alert.title, &alert.message, alert.kind == AlertKind::Error); }
            }
        }
        AppEvent::Chat(message) => {
            let snapshot = state.settings.read().map(|s| s.clone()).unwrap_or_default();
            if snapshot.chat_overlay_enabled && !chat::should_hide_in_overlay(&snapshot, &message) {
                append_chat(state, &snapshot, format_chat(&snapshot, &message));
                maybe_chat_sound(&snapshot, &message);
            }
        }
        AppEvent::Translation { sequence_id: _, text, source_language } => {
            let snapshot = state.settings.read().map(|s| s.clone()).unwrap_or_default();
            if snapshot.chat_overlay_enabled && snapshot.speech_translation.show_translation_in_overlay {
                let source = if source_language.trim().is_empty() { "AUTO".into() } else { source_language.to_ascii_uppercase() };
                append_chat(state, &snapshot, format!("    {source} → EN: {text}"));
            }
        }
        AppEvent::Identity(identity) => {
            logging::write(format!("identity: {} uid={}", identity.name, identity.uid));
        }
        AppEvent::CaptureStatus(status) => {
            state.capture_status = status;
            update_tip(hwnd, &state.capture_status);
        }
    }
}

fn alert_enabled(settings: &AppSettings, kind: AlertKind) -> bool {
    match kind {
        AlertKind::Queue => settings.queue_pop_alert,
        AlertKind::Ready => settings.ready_check_alert,
        AlertKind::PartyInvite => settings.party_invite_alert,
        AlertKind::PartyRequest => settings.party_request_alert,
        AlertKind::Error => settings.desktop_notification,
    }
}

fn format_chat(settings: &AppSettings, message: &ChatMessage) -> String {
    let sender = if message.sender_name.trim().is_empty() { "?" } else { message.sender_name.trim() };
    let level = if message.sender_level > 0 { format!(" Lv{}", message.sender_level) } else { String::new() };
    if settings.chat.show_time && message.unix_seconds > 0 {
        format!("[{}] {}{}: {}", channel_name(message.channel), sender, level, message.text.trim())
    } else {
        format!("[{}] {}{}: {}", channel_name(message.channel), sender, level, message.text.trim())
    }
}

unsafe fn append_chat(state: &mut UiState, settings: &AppSettings, line: String) {
    if state.overlay.is_null() || IsWindow(state.overlay) == 0 { return; }
    state.overlay_lines.push_back(line);
    while state.overlay_lines.len() > settings.chat.max_history { state.overlay_lines.pop_front(); }
    let text = state.overlay_lines.iter().map(String::as_str).collect::<Vec<_>>().join("\r\n");
    let edit = GetDlgItem(state.overlay, EDIT_ID);
    if edit.is_null() { return; }
    let wide_text = wide(&text);
    SetWindowTextW(edit, wide_text.as_ptr());
    let len = GetWindowTextLengthW(edit);
    SendMessageW(edit, EM_SETSEL, len as usize, len as isize);
    SendMessageW(edit, EM_REPLACESEL, 0, wide("\r\n").as_ptr() as isize);
}

fn maybe_chat_sound(settings: &AppSettings, message: &ChatMessage) {
    let path = if message.channel == 5 && settings.chat.private_sound_enabled {
        Some(settings.chat.private_sound_path.clone())
    } else {
        let hay = format!("{} {}", message.sender_name, message.text).to_ascii_lowercase();
        settings.chat.highlight_sound_rules.iter().find(|r| r.enabled && !r.match_text.trim().is_empty() &&
            r.match_text.split(|c: char| matches!(c, ',' | ';' | '|' | '\n')).any(|needle| {
                let needle = needle.trim().to_ascii_lowercase(); !needle.is_empty() && hay.contains(&needle)
            })).map(|r| r.sound_path.clone())
    };
    if let Some(path) = path.filter(|p| !p.trim().is_empty()) {
        let volume = settings.chat.chat_sound_volume;
        thread::spawn(move || { let _ = audio::play_file(Path::new(&path), volume); });
    }
}

unsafe fn show_tray_menu(hwnd: HWND, state: &mut UiState) {
    let snapshot = state.settings.read().map(|s| s.clone()).unwrap_or_default();
    let menu = CreatePopupMenu();
    if menu.is_null() { return; }
    append_check(menu, CMD_QUEUE, "Queue Pop alert", snapshot.queue_pop_alert);
    append_check(menu, CMD_READY, "Ready Check alert", snapshot.ready_check_alert);
    append_check(menu, CMD_INVITE, "Party Invite alert", snapshot.party_invite_alert);
    append_check(menu, CMD_REQUEST, "Party Request alert", snapshot.party_request_alert);
    AppendMenuW(menu, MF_SEPARATOR, 0, null());
    append_check(menu, CMD_DESKTOP, "Desktop notifications", snapshot.desktop_notification);
    append_check(menu, CMD_CHAT, "Chat Overlay enabled", snapshot.chat_overlay_enabled);
    append_string(menu, CMD_SHOW_CHAT, "Show / Hide Chat");

    let tabs = CreatePopupMenu();
    if !tabs.is_null() {
        for (index, tab) in snapshot.chat.tabs.iter().take(MAX_MENU_TABS).enumerate() {
            let checked = tab.id == snapshot.chat.last_selected_tab_id;
            append_check(tabs, CMD_TAB_BASE + index as u32, &tab.name, checked);
        }
        AppendMenuW(menu, MF_POPUP, tabs as usize, wide("Chat tab").as_ptr());
    }
    AppendMenuW(menu, MF_SEPARATOR, 0, null());
    append_string(menu, CMD_OPEN_SETTINGS, "Open settings.json");
    append_string(menu, CMD_OPEN_LOGS, "Open chat logs");
    AppendMenuW(menu, MF_SEPARATOR, 0, null());
    append_string(menu, CMD_EXIT, "Exit");

    let mut p: POINT = std::mem::zeroed();
    GetCursorPos(&mut p);
    SetForegroundWindow(hwnd);
    let command = TrackPopupMenu(menu, TPM_LEFTALIGN | TPM_BOTTOMALIGN | TPM_RIGHTBUTTON | TPM_RETURNCMD, p.x, p.y, 0, hwnd, null());
    DestroyMenu(menu);
    if command != 0 { PostMessageW(hwnd, WM_COMMAND, command as usize, 0); }
}

unsafe fn append_check(menu: HMENU, id: u32, text: &str, checked: bool) {
    let flags = MF_STRING | if checked { MF_CHECKED } else { 0 };
    AppendMenuW(menu, flags, id as usize, wide(text).as_ptr());
}
unsafe fn append_string(menu: HMENU, id: u32, text: &str) { AppendMenuW(menu, MF_STRING, id as usize, wide(text).as_ptr()); }

unsafe fn handle_command(hwnd: HWND, state: &mut UiState, command: u32) {
    if command == CMD_EXIT { DestroyWindow(hwnd); return; }
    if command == CMD_SHOW_CHAT { toggle_overlay(state); return; }
    if command == CMD_OPEN_SETTINGS { open_path(&state.paths.settings); return; }
    if command == CMD_OPEN_LOGS { let _ = std::fs::create_dir_all(&state.paths.chat_logs); open_path(&state.paths.chat_logs); return; }

    let mut changed = false;
    if let Ok(mut s) = state.settings.write() {
        match command {
            CMD_QUEUE => { s.queue_pop_alert = !s.queue_pop_alert; changed = true; }
            CMD_READY => { s.ready_check_alert = !s.ready_check_alert; changed = true; }
            CMD_INVITE => { s.party_invite_alert = !s.party_invite_alert; changed = true; }
            CMD_REQUEST => { s.party_request_alert = !s.party_request_alert; changed = true; }
            CMD_DESKTOP => { s.desktop_notification = !s.desktop_notification; changed = true; }
            CMD_CHAT => {
                s.chat_overlay_enabled = !s.chat_overlay_enabled;
                changed = true;
                if s.chat_overlay_enabled { ShowWindow(state.overlay, SW_SHOW); } else { ShowWindow(state.overlay, SW_HIDE); }
            }
            id if id >= CMD_TAB_BASE && id < CMD_TAB_BASE + MAX_MENU_TABS as u32 => {
                if let Some(tab) = s.chat.tabs.get((id - CMD_TAB_BASE) as usize) {
                    s.chat.last_selected_tab_id = tab.id;
                    state.overlay_lines.clear();
                    let edit = GetDlgItem(state.overlay, EDIT_ID);
                    if !edit.is_null() { SetWindowTextW(edit, wide("").as_ptr()); }
                    changed = true;
                }
            }
            _ => {}
        }
        if changed {
            s.normalize();
            let snapshot = s.clone();
            drop(s);
            if let Err(err) = settings::save(&state.paths, &snapshot) { logging::write(format!("settings: save failed: {err}")); }
            apply_overlay_style(state.overlay, &snapshot);
        }
    }
}

unsafe fn apply_overlay_style(hwnd: HWND, settings: &AppSettings) {
    if hwnd.is_null() || IsWindow(hwnd) == 0 { return; }
    let mut ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
    if settings.chat.click_through { ex |= WS_EX_TRANSPARENT; } else { ex &= !WS_EX_TRANSPARENT; }
    if settings.chat.top_most { ex |= WS_EX_TOPMOST; } else { ex &= !WS_EX_TOPMOST; }
    SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex as isize);
    let alpha = ((settings.chat.window_opacity.clamp(25, 100) * 255) / 100) as u8;
    SetLayeredWindowAttributes(hwnd, 0, alpha, LWA_ALPHA);
}

unsafe fn toggle_overlay(state: &mut UiState) {
    if state.overlay.is_null() || IsWindow(state.overlay) == 0 { return; }
    ShowWindow(state.overlay, SW_SHOW);
    SetForegroundWindow(state.overlay);
}

unsafe fn add_tray(hwnd: HWND, tip: &str) -> Result<(), String> {
    let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = TRAY_ID;
    nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
    nid.uCallbackMessage = WM_TRAY;
    nid.hIcon = app_icon(GetModuleHandleW(null()));
    write_wide(&mut nid.szTip, &format!("BPSR Ready Alert - {tip}"));
    if Shell_NotifyIconW(NIM_ADD, &nid) == 0 { return Err("Shell_NotifyIconW(NIM_ADD) failed".into()); }
    Ok(())
}

unsafe fn remove_tray(hwnd: HWND) {
    let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = TRAY_ID;
    Shell_NotifyIconW(NIM_DELETE, &nid);
}

unsafe fn update_tip(hwnd: HWND, tip: &str) {
    let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = TRAY_ID;
    nid.uFlags = NIF_TIP;
    write_wide(&mut nid.szTip, &format!("BPSR Ready Alert - {tip}"));
    Shell_NotifyIconW(NIM_MODIFY, &nid);
}

unsafe fn balloon(hwnd: HWND, title: &str, message: &str, error: bool) {
    let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = TRAY_ID;
    nid.uFlags = NIF_INFO;
    nid.dwInfoFlags = if error { NIIF_ERROR } else { NIIF_INFO };
    write_wide(&mut nid.szInfoTitle, title);
    write_wide(&mut nid.szInfo, message);
    Shell_NotifyIconW(NIM_MODIFY, &nid);
}

unsafe fn app_icon(instance: HINSTANCE) -> HICON {
    let icon = LoadIconW(instance, 1usize as *const u16);
    if icon.is_null() { LoadIconW(null_mut(), IDI_APPLICATION) } else { icon }
}

pub fn message_box(title: &str, text: &str, error: bool) {
    unsafe {
        let flags = MB_OK | if error { MB_ICONERROR } else { MB_ICONINFORMATION };
        MessageBoxW(null_mut(), wide(text).as_ptr(), wide(title).as_ptr(), flags);
    }
}

pub fn auto_launch_resonance_logs(settings: &AppSettings) {
    if !settings.auto_launch_resonance_logs { return; }
    let mut candidates: Vec<PathBuf> = Vec::new();
    if !settings.resonance_logs_path.trim().is_empty() { candidates.push(PathBuf::from(settings.resonance_logs_path.trim())); }
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        let local = PathBuf::from(local);
        candidates.push(local.join("Programs").join("resonance-logs-cn").join("Resonance Logs CN.exe"));
        candidates.push(local.join("resonance-logs-cn").join("Resonance Logs CN.exe"));
    }
    if let Some(path) = candidates.into_iter().find(|p| p.is_file()) {
        unsafe { open_path(&path); }
    }
}

unsafe fn open_path(path: &Path) {
    let op = wide("open");
    let target = wide(&path.to_string_lossy());
    ShellExecuteW(null_mut(), op.as_ptr(), target.as_ptr(), null(), null(), SW_SHOWNORMAL);
}

fn wide(text: &str) -> Vec<u16> { text.encode_utf16().chain(std::iter::once(0)).collect() }
fn write_wide<const N: usize>(target: &mut [u16; N], text: &str) {
    target.fill(0);
    for (dst, src) in target.iter_mut().take(N.saturating_sub(1)).zip(text.encode_utf16()) { *dst = src; }
}
