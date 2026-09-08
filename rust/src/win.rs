use crate::{
    audio, chat, logging, overlay,
    model::{AlertKind, AppEvent, ChatMessage},
    paths::AppPaths,
    settings::{self, AppSettings},
};
use std::{
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
            DestroyWindow, DispatchMessageW, GetCursorPos, GetMessageW, GetWindowLongPtrW,
            IsWindow, IsWindowVisible, LoadCursorW, LoadIconW, MessageBoxW, PostMessageW,
            PostQuitMessage, RegisterClassW, SetForegroundWindow, SetTimer, SetWindowLongPtrW,
            ShowWindow, TrackPopupMenu, TranslateMessage, CREATESTRUCTW, GWLP_USERDATA,
            IDC_ARROW, IDI_APPLICATION, MB_ICONERROR, MB_ICONINFORMATION, MB_OK, MF_CHECKED,
            MF_POPUP, MF_SEPARATOR, MF_STRING, MSG, SW_HIDE, SW_SHOWNORMAL, SW_SHOW,
            TPM_BOTTOMALIGN, TPM_LEFTALIGN, TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_APP, WM_COMMAND,
            WM_DESTROY, WM_LBUTTONDBLCLK, WM_NCCREATE, WM_RBUTTONUP, WM_TIMER, WNDCLASSW,
            WS_OVERLAPPED, HICON, HMENU,
        },
    },
};

const WM_TRAY: u32 = WM_APP + 17;
const TRAY_ID: u32 = 1;
const TIMER_ID: usize = 1;

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
        if handle.is_null() {
            return Err(format!("CreateMutexW failed: {}", unsafe { GetLastError() }));
        }
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
            capture_status: "Starting capture…".into(),
        });
        let state_ptr: *mut UiState = &mut *state;
        let title = wide("BPSR Ready Alert");
        let hwnd = CreateWindowExW(
            0,
            main_class.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            null_mut(),
            null_mut(),
            instance,
            state_ptr.cast::<c_void>(),
        );
        if hwnd.is_null() {
            return Err(format!("CreateWindowExW(main) failed: {}", GetLastError()));
        }

        state.overlay = overlay::create(instance, state.settings.clone(), hwnd, state.paths.clone())?;
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
            if !state_ptr.is_null() {
                handle_command(hwnd, &mut *state_ptr, (wparam as u32) & 0xffff);
            }
            0
        }
        WM_DESTROY => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                state.stop.store(true, Ordering::Relaxed);
                if !state.overlay.is_null() && IsWindow(state.overlay) != 0 {
                    DestroyWindow(state.overlay);
                }
            }
            remove_tray(hwnd);
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn drain_events(hwnd: HWND, state: &mut UiState) {
    loop {
        match state.rx.try_recv() {
            Ok(event) => handle_event(hwnd, state, event),
            Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => break,
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
                if snapshot.desktop_notification {
                    balloon(hwnd, &alert.title, &alert.message, alert.kind == AlertKind::Error);
                }
            }
        }
        AppEvent::Chat(message) => {
            let snapshot = state.settings.read().map(|s| s.clone()).unwrap_or_default();
            if snapshot.chat_overlay_enabled && !chat::should_hide_globally(&snapshot, &message) {
                overlay::push_chat(state.overlay, message.clone());
                maybe_chat_sound(&snapshot, &message);
            }
        }
        AppEvent::Translation { sequence_id, text, source_language } => {
            let snapshot = state.settings.read().map(|s| s.clone()).unwrap_or_default();
            if snapshot.chat_overlay_enabled && snapshot.speech_translation.show_translation_in_overlay {
                overlay::set_translation(state.overlay, sequence_id, text, source_language);
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

fn maybe_chat_sound(settings: &AppSettings, message: &ChatMessage) {
    let path = if message.channel == 5 && settings.chat.private_sound_enabled {
        Some(settings.chat.private_sound_path.clone())
    } else {
        let hay = format!("{} {}", message.sender_name, message.text).to_ascii_lowercase();
        settings.chat.highlight_sound_rules.iter().find(|r| {
            r.enabled && !r.match_text.trim().is_empty() &&
                r.match_text.split(|c: char| matches!(c, ',' | ';' | '|' | '\n')).any(|needle| {
                    let needle = needle.trim().to_ascii_lowercase();
                    !needle.is_empty() && hay.contains(&needle)
                })
        }).map(|r| r.sound_path.clone())
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
            append_check(tabs, CMD_TAB_BASE + index as u32, &tab.name, tab.id == snapshot.chat.last_selected_tab_id);
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
    let command = TrackPopupMenu(
        menu,
        TPM_LEFTALIGN | TPM_BOTTOMALIGN | TPM_RIGHTBUTTON | TPM_RETURNCMD,
        p.x,
        p.y,
        0,
        hwnd,
        null(),
    );
    DestroyMenu(menu);
    if command != 0 { PostMessageW(hwnd, WM_COMMAND, command as usize, 0); }
}

unsafe fn append_check(menu: HMENU, id: u32, text: &str, checked: bool) {
    AppendMenuW(menu, MF_STRING | if checked { MF_CHECKED } else { 0 }, id as usize, wide(text).as_ptr());
}

unsafe fn append_string(menu: HMENU, id: u32, text: &str) {
    AppendMenuW(menu, MF_STRING, id as usize, wide(text).as_ptr());
}

unsafe fn handle_command(hwnd: HWND, state: &mut UiState, command: u32) {
    if command == CMD_EXIT { DestroyWindow(hwnd); return; }
    if command == CMD_SHOW_CHAT { toggle_overlay(state); return; }
    if command == CMD_OPEN_SETTINGS { open_path(&state.paths.settings); return; }
    if command == CMD_OPEN_LOGS {
        let _ = std::fs::create_dir_all(&state.paths.chat_logs);
        open_path(&state.paths.chat_logs);
        return;
    }

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
                ShowWindow(state.overlay, if s.chat_overlay_enabled { SW_SHOW } else { SW_HIDE });
            }
            id if id >= CMD_TAB_BASE && id < CMD_TAB_BASE + MAX_MENU_TABS as u32 => {
                if let Some(tab) = s.chat.tabs.get((id - CMD_TAB_BASE) as usize) {
                    s.chat.last_selected_tab_id = tab.id;
                    changed = true;
                }
            }
            _ => {}
        }
        if changed {
            s.normalize();
            let snapshot = s.clone();
            drop(s);
            if let Err(err) = settings::save(&state.paths, &snapshot) {
                logging::write(format!("settings: save failed: {err}"));
            }
            overlay::apply_style(state.overlay, &snapshot);
            overlay::refresh(state.overlay);
        }
    }
}

unsafe fn toggle_overlay(state: &mut UiState) {
    if state.overlay.is_null() || IsWindow(state.overlay) == 0 { return; }
    if IsWindowVisible(state.overlay) != 0 {
        ShowWindow(state.overlay, SW_HIDE);
    } else {
        ShowWindow(state.overlay, SW_SHOW);
        if !state.settings.read().map(|s| s.chat.click_through).unwrap_or(false) {
            SetForegroundWindow(state.overlay);
        }
    }
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
    if Shell_NotifyIconW(NIM_ADD, &nid) == 0 {
        return Err("Shell_NotifyIconW(NIM_ADD) failed".into());
    }
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
        MessageBoxW(
            null_mut(),
            wide(text).as_ptr(),
            wide(title).as_ptr(),
            MB_OK | if error { MB_ICONERROR } else { MB_ICONINFORMATION },
        );
    }
}

pub fn auto_launch_resonance_logs(settings: &AppSettings) {
    if !settings.auto_launch_resonance_logs { return; }
    let mut candidates: Vec<PathBuf> = Vec::new();
    if !settings.resonance_logs_path.trim().is_empty() {
        candidates.push(PathBuf::from(settings.resonance_logs_path.trim()));
    }
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
    ShellExecuteW(
        null_mut(),
        wide("open").as_ptr(),
        wide(&path.to_string_lossy()).as_ptr(),
        null(),
        null(),
        SW_SHOWNORMAL,
    );
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn write_wide<const N: usize>(target: &mut [u16; N], text: &str) {
    target.fill(0);
    for (dst, src) in target.iter_mut().take(N.saturating_sub(1)).zip(text.encode_utf16()) {
        *dst = src;
    }
}
