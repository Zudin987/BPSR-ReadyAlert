use crate::{
    audio, chat, logging, overlay, settings_ui, tray::{self, TrayAction},
    model::{AlertKind, AppEvent, ChatMessage},
    npcap::{CaptureHandle, PcapApi},
    paths::AppPaths,
    settings::{self, AppSettings},
};
use std::{
    ffi::c_void,
    io::Read,
    path::{Path, PathBuf},
    ptr::{null, null_mut},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, TryRecvError},
        Arc, RwLock,
    },
    thread,
    time::Duration,
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
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
            CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
            GetWindowLongPtrW, IsWindow, IsWindowVisible, LoadCursorW, LoadIconW, MessageBoxW,
            PostQuitMessage, RegisterClassW, SetForegroundWindow, SetTimer, SetWindowLongPtrW,
            ShowWindow, TranslateMessage, CREATESTRUCTW, GWLP_USERDATA, IDC_ARROW,
            IDI_APPLICATION, MB_ICONERROR, MB_ICONINFORMATION, MB_OK, MSG, SW_HIDE,
            SW_SHOWNORMAL, SW_SHOW, WM_APP, WM_COMMAND, WM_DESTROY, WM_LBUTTONDBLCLK,
            WM_NCCREATE, WM_RBUTTONUP, WM_TIMER, WNDCLASSW, WS_OVERLAPPED, HICON,
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
const CMD_TTS_TOGGLE: u32 = 1013;
const CMD_ADD_TAB: u32 = 1014;
const CMD_SETTINGS_APPLIED: u32 = 1020;
const CMD_TTS_TEST: u32 = 1021;
const CMD_OPEN_SETTINGS_JSON: u32 = 1022;
const CMD_OPEN_APP_FOLDER: u32 = 1023;
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
    api: Arc<PcapApi>,
    overlay: HWND,
    capture_status: String,
}

pub fn run_ui(
    settings: Arc<RwLock<AppSettings>>,
    paths: AppPaths,
    rx: Receiver<AppEvent>,
    stop: Arc<AtomicBool>,
    api: Arc<PcapApi>,
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
            api,
            overlay: null_mut(),
            capture_status: "Starting capture…".into(),
        });
        let state_ptr: *mut UiState = &mut *state;
        let hwnd = CreateWindowExW(
            0,
            main_class.as_ptr(),
            wide("BPSR Ready Alert").as_ptr(),
            WS_OVERLAPPED,
            0, 0, 0, 0,
            null_mut(), null_mut(), instance,
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
        if !cs.is_null() { SetWindowLongPtrW(hwnd, GWLP_USERDATA, (*cs).lpCreateParams as isize); }
    }
    let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut UiState;
    match msg {
        WM_TIMER if wparam == TIMER_ID => {
            if !state_ptr.is_null() { drain_events(hwnd, &mut *state_ptr); }
            0
        }
        WM_TRAY => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                let mouse = lparam as u32;
                if mouse == WM_RBUTTONUP {
                    let snapshot = state.settings.read().map(|s| s.clone()).unwrap_or_default();
                    let action = tray::show(hwnd, &snapshot, &state.api);
                    handle_tray_action(hwnd, state, action);
                } else if mouse == WM_LBUTTONDBLCLK {
                    toggle_overlay(state);
                }
            }
            0
        }
        WM_COMMAND => {
            if !state_ptr.is_null() { handle_command(hwnd, &mut *state_ptr, (wparam as u32) & 0xffff, lparam); }
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
            let enabled = alert.kind == AlertKind::Error || alert_enabled(&snapshot, alert.kind);
            if enabled {
                if alert.kind != AlertKind::Error {
                    let volume = snapshot.alert_volume;
                    thread::spawn(move || audio::play_alert(alert.kind, volume));
                }
                if snapshot.desktop_notification || alert.kind == AlertKind::Error {
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
        AppEvent::Identity(identity) => logging::write(format!("identity: {} uid={}", identity.name, identity.uid)),
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
        AlertKind::Error => true,
    }
}

fn maybe_chat_sound(settings: &AppSettings, message: &ChatMessage) {
    let path = if message.channel == 5 && settings.chat.private_sound_enabled {
        Some(settings.chat.private_sound_path.clone())
    } else {
        settings.chat.highlight_sound_rules.iter().find(|r| {
            r.enabled && !r.match_text.trim().is_empty() && chat::matches_expression(&message.text, &r.match_text)
        }).map(|r| r.sound_path.clone())
    };
    if let Some(path) = path.filter(|p| !p.trim().is_empty()) {
        let volume = settings.chat.chat_sound_volume;
        thread::spawn(move || { let _ = audio::play_file(Path::new(&path), volume); });
    }
}

unsafe fn handle_tray_action(hwnd: HWND, state: &mut UiState, action: TrayAction) {
    match action {
        TrayAction::None => {}
        TrayAction::Exit => DestroyWindow(hwnd),
        TrayAction::ShowHideChat => toggle_overlay(state),
        TrayAction::OpenSettings => handle_command(hwnd, state, CMD_OPEN_SETTINGS, 0),
        TrayAction::OpenChatLogs => handle_command(hwnd, state, CMD_OPEN_LOGS, 0),
        TrayAction::OpenAppFolder => handle_command(hwnd, state, CMD_OPEN_APP_FOLDER, 0),
        TrayAction::OpenLogFile => {
            if !state.paths.log.exists() {
                let _ = std::fs::write(&state.paths.log, "No log entries yet.\r\n");
            }
            open_path(&state.paths.log);
        }
        TrayAction::LaunchResonanceLogs => {
            let snapshot = state.settings.read().map(|s| s.clone()).unwrap_or_default();
            if !launch_resonance_logs(&snapshot) {
                message_box(
                    "BPSR Ready Alert",
                    "Could not find Resonance Logs CN. Set its executable path in Settings > Network.",
                    true,
                );
            }
        }
        TrayAction::TestAlert(kind) => {
            let volume = state.settings.read().map(|s| s.alert_volume).unwrap_or(100);
            thread::spawn(move || audio::play_alert(kind, volume));
        }
        TrayAction::SetAlertVolume(volume) => update_settings(state, |s| s.alert_volume = volume.clamp(0, 100)),
        TrayAction::ToggleAutoLaunchLogs => update_settings(state, |s| s.auto_launch_resonance_logs = !s.auto_launch_resonance_logs),
        TrayAction::SelectAdapter(device) => select_adapter(state, device),
        TrayAction::SelectTab(index) => {
            update_settings(state, |s| {
                if let Some(tab) = s.chat.tabs.get(index) {
                    s.chat.last_selected_tab_id = tab.id;
                }
            });
            overlay::refresh(state.overlay);
        }
        TrayAction::ToggleQueue => handle_command(hwnd, state, CMD_QUEUE, 0),
        TrayAction::ToggleReady => handle_command(hwnd, state, CMD_READY, 0),
        TrayAction::ToggleInvite => handle_command(hwnd, state, CMD_INVITE, 0),
        TrayAction::ToggleRequest => handle_command(hwnd, state, CMD_REQUEST, 0),
        TrayAction::ToggleDesktop => handle_command(hwnd, state, CMD_DESKTOP, 0),
        TrayAction::ToggleChat => handle_command(hwnd, state, CMD_CHAT, 0),
        TrayAction::ToggleTts => handle_command(hwnd, state, CMD_TTS_TOGGLE, 0),
    }
}

fn update_settings<F>(state: &UiState, update: F)
where
    F: FnOnce(&mut AppSettings),
{
    let snapshot = if let Ok(mut s) = state.settings.write() {
        update(&mut s);
        s.normalize();
        s.clone()
    } else {
        return;
    };
    if let Err(err) = settings::save(&state.paths, &snapshot) {
        logging::write(format!("settings: save failed: {err}"));
    }
}

fn select_adapter(state: &UiState, device: Option<String>) {
    let next = device.unwrap_or_default();
    let current = state.settings.read().map(|s| s.npcap_device_name.clone()).unwrap_or_default();
    if current.eq_ignore_ascii_case(&next) {
        return;
    }

    if !next.is_empty() {
        match CaptureHandle::open(state.api.clone(), &next) {
            Ok(handle) => {
                logging::write(format!("capture: adapter preflight ok datalink={} device={}", handle.datalink, next));
                drop(handle);
            }
            Err(err) => {
                logging::write(format!("capture: adapter preflight rejected device={next}: {err}"));
                message_box(
                    "BPSR Ready Alert - Network Adapter",
                    &format!("Could not activate that Npcap adapter. ReadyAlert kept the current adapter.\n\n{err}"),
                    true,
                );
                return;
            }
        }
    }

    update_settings(state, |s| s.npcap_device_name = next.clone());
    logging::write(format!(
        "capture: adapter preference updated to {}",
        if next.is_empty() { "Auto / Resonance Logs CN" } else { next.as_str() }
    ));
}

unsafe fn handle_command(hwnd: HWND, state: &mut UiState, command: u32, lparam: LPARAM) {
    if command == CMD_EXIT { DestroyWindow(hwnd); return; }
    if command == CMD_SHOW_CHAT { toggle_overlay(state); return; }
    if command == CMD_OPEN_LOGS {
        let _ = std::fs::create_dir_all(&state.paths.chat_logs);
        open_path(&state.paths.chat_logs);
        return;
    }
    if command == CMD_OPEN_SETTINGS_JSON { open_path(&state.paths.settings); return; }
    if command == CMD_OPEN_APP_FOLDER { open_path(&state.paths.root); return; }
    if command == CMD_OPEN_SETTINGS {
        let instance = GetModuleHandleW(null());
        if let Err(err) = settings_ui::show(instance, hwnd, state.settings.clone(), state.paths.clone(), 1, false) {
            logging::write(format!("settings ui: {err}"));
            message_box("ReadyAlert Settings", &err, true);
        }
        return;
    }
    if command == CMD_ADD_TAB {
        let instance = GetModuleHandleW(null());
        if let Err(err) = settings_ui::show(instance, hwnd, state.settings.clone(), state.paths.clone(), 4, true) {
            logging::write(format!("settings ui: {err}"));
            message_box("ReadyAlert Settings", &err, true);
        }
        return;
    }
    if command == CMD_TTS_TEST {
        let volume = (lparam as i32).clamp(0, 100);
        thread::spawn(move || {
            if let Err(err) = test_tts(volume) { logging::write(format!("tts: test failed: {err}")); }
        });
        return;
    }
    if command == CMD_SETTINGS_APPLIED {
        let snapshot = state.settings.read().map(|s| s.clone()).unwrap_or_default();
        ShowWindow(state.overlay, if snapshot.chat_overlay_enabled { SW_SHOW } else { SW_HIDE });
        overlay::apply_style(state.overlay, &snapshot);
        overlay::refresh(state.overlay);
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
            CMD_TTS_TOGGLE => { s.speech_translation.tts_enabled = !s.speech_translation.tts_enabled; changed = true; }
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
            if let Err(err) = settings::save(&state.paths, &snapshot) { logging::write(format!("settings: save failed: {err}")); }
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
        overlay::expand_if_collapsed(state.overlay);
        ShowWindow(state.overlay, SW_SHOW);
        if !state.settings.read().map(|s| s.chat.click_through).unwrap_or(false) { SetForegroundWindow(state.overlay); }
    }
}

fn test_tts(volume: i32) -> Result<(), String> {
    if volume <= 0 { return Err("TTS volume is muted at 0%.".into()); }
    let sample = "ReadyAlert text to speech test.";
    let url = format!(
        "https://translate.google.com/translate_tts?ie=UTF-8&client=tw-ob&tl=en&total=1&idx=0&textlen={}&q={}",
        sample.chars().count(),
        urlencoding::encode(sample),
    );
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(4))
        .timeout_read(Duration::from_secs(8))
        .timeout_write(Duration::from_secs(8))
        .build();
    let response = agent.get(&url).set("Accept", "audio/mpeg,*/*;q=0.8").call().map_err(|e| e.to_string())?;
    let content_type = response.header("content-type").unwrap_or("").to_ascii_lowercase();
    if !content_type.starts_with("audio/") { return Err(format!("unexpected TTS content-type {content_type}")); }
    let mut bytes = Vec::new();
    response.into_reader().take(4 * 1024 * 1024 + 1).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    if bytes.len() < 200 || bytes.len() > 4 * 1024 * 1024 { return Err(format!("invalid TTS payload {} bytes", bytes.len())); }
    audio::play_mp3(&bytes, volume)
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
        MessageBoxW(null_mut(), wide(text).as_ptr(), wide(title).as_ptr(), MB_OK | if error { MB_ICONERROR } else { MB_ICONINFORMATION });
    }
}

pub fn auto_launch_resonance_logs(settings: &AppSettings) {
    if settings.auto_launch_resonance_logs {
        let _ = launch_resonance_logs(settings);
    }
}

fn launch_resonance_logs(settings: &AppSettings) -> bool {
    if let Some(path) = resonance_logs_candidates(settings).into_iter().find(|p| p.is_file()) {
        unsafe { open_path(&path); }
        true
    } else {
        false
    }
}

fn resonance_logs_candidates(settings: &AppSettings) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if !settings.resonance_logs_path.trim().is_empty() {
        candidates.push(PathBuf::from(settings.resonance_logs_path.trim()));
    }
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        let local = PathBuf::from(local);
        candidates.push(local.join("Programs").join("resonance-logs-cn").join("Resonance Logs CN.exe"));
        candidates.push(local.join("resonance-logs-cn").join("Resonance Logs CN.exe"));
    }
    candidates
}

unsafe fn open_path(path: &Path) {
    ShellExecuteW(null_mut(), wide("open").as_ptr(), wide(&path.to_string_lossy()).as_ptr(), null(), null(), SW_SHOWNORMAL);
}

fn wide(text: &str) -> Vec<u16> { text.encode_utf16().chain(std::iter::once(0)).collect() }
fn write_wide<const N: usize>(target: &mut [u16; N], text: &str) {
    target.fill(0);
    for (dst, src) in target.iter_mut().take(N.saturating_sub(1)).zip(text.encode_utf16()) { *dst = src; }
}
