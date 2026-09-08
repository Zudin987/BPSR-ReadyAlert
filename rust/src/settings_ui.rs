use crate::{
    chat,
    paths::AppPaths,
    settings::{self, AppSettings, ChatSoundRule, ChatTabSettings},
};
use std::{
    ffi::c_void,
    ptr::{null, null_mut},
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};
use windows_sys::Win32::{
    Foundation::{GetLastError, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
    Graphics::Gdi::{
        CreateSolidBrush, DeleteObject, GetStockObject, SetBkColor, SetTextColor, DEFAULT_GUI_FONT,
        HBRUSH, HDC,
    },
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, FindWindowW, GetDlgItem,
        GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW, LoadCursorW, MessageBoxW,
        PostMessageW, RegisterClassW, SendMessageW, SetForegroundWindow, SetWindowLongPtrW,
        SetWindowTextW, ShowWindow, CREATESTRUCTW, CW_USEDEFAULT, GWLP_USERDATA, IDC_ARROW,
        MB_ICONERROR, MB_ICONINFORMATION, MB_OK, SW_HIDE, SW_SHOW, WM_APP, WM_CLOSE, WM_COMMAND,
        WM_CREATE, WM_CTLCOLORBTN, WM_CTLCOLOREDIT, WM_CTLCOLORSTATIC, WM_NCCREATE,
        WM_NCDESTROY, WM_SETFONT, WNDCLASSW, WS_BORDER, WS_CAPTION, WS_CHILD,
        WS_EX_CLIENTEDGE, WS_MINIMIZEBOX, WS_OVERLAPPED, WS_SYSMENU, WS_TABSTOP, WS_VISIBLE,
        WS_VSCROLL, HMENU,
    },
};

const CLASS_NAME: &str = "BPSRReadyAlertRustSettingsV151";
const WM_SHOW_PAGE: u32 = WM_APP + 91;

// Shared message/command IDs with win.rs.
const CMD_SETTINGS_APPLIED: u32 = 1020;
const CMD_TTS_TEST: u32 = 1021;
const CMD_OPEN_SETTINGS_JSON: u32 = 1022;
const CMD_OPEN_APP_FOLDER: u32 = 1023;
const CMD_OPEN_LOGS: u32 = 1011;

const PAGE_GENERAL: usize = 0;
const PAGE_OVERLAY: usize = 1;
const PAGE_COLORS: usize = 2;
const PAGE_SPEECH: usize = 3;
const PAGE_TABS: usize = 4;
const PAGE_SOUNDS: usize = 5;
const PAGE_NETWORK: usize = 6;
const PAGE_BLOCKED: usize = 7;
const PAGE_COUNT: usize = 8;

const NAV_BASE: i32 = 3000;
const ID_APPLY: i32 = 3098;
const ID_CLOSE: i32 = 3099;

const ID_QUEUE: i32 = 3200;
const ID_READY: i32 = 3201;
const ID_INVITE: i32 = 3202;
const ID_REQUEST: i32 = 3203;
const ID_DESKTOP: i32 = 3204;
const ID_AUTO_LOGS: i32 = 3205;
const ID_ALERT_VOLUME: i32 = 3206;

const ID_CHAT_ENABLED: i32 = 3210;
const ID_TOPMOST: i32 = 3211;
const ID_COMPACT: i32 = 3212;
const ID_SHOW_TIME: i32 = 3213;
const ID_TIME_AGO: i32 = 3214;
const ID_CLICKTHROUGH: i32 = 3215;
const ID_CLICK_HOTKEY: i32 = 3216;
const ID_BOLD: i32 = 3217;
const ID_SHADOW: i32 = 3218;
const ID_SEPARATORS: i32 = 3219;
const ID_ZEBRA: i32 = 3220;
const ID_COLOR_BAND: i32 = 3221;
const ID_WINDOW_OPACITY: i32 = 3222;
const ID_FONT_FAMILY: i32 = 3223;
const ID_FONT_SIZE: i32 = 3224;
const ID_MAX_HISTORY: i32 = 3225;
const ID_COLLAPSE_SIDE: i32 = 3226;
const ID_HIDE_STICKERS: i32 = 3227;

const ID_TRANSLATE: i32 = 3240;
const ID_TRANSLATE_WORLD: i32 = 3241;
const ID_TRANSLATE_GUILD: i32 = 3242;
const ID_TRANSLATE_PARTY: i32 = 3243;
const ID_TRANSLATE_OVERLAY: i32 = 3244;
const ID_TTS: i32 = 3245;
const ID_TTS_GUILD: i32 = 3246;
const ID_TTS_PARTY: i32 = 3247;
const ID_TTS_SENDER: i32 = 3248;
const ID_TTS_USERNAME: i32 = 3249;
const ID_TTS_VOLUME: i32 = 3250;
const ID_HIDE_RICH: i32 = 3251;
const ID_TEST_TTS: i32 = 3252;

const ID_TAB_LIST: i32 = 3270;
const ID_TAB_ADD: i32 = 3271;
const ID_TAB_DELETE: i32 = 3272;
const ID_TAB_NAME: i32 = 3273;
const ID_TAB_MIN_LEVEL: i32 = 3274;
const ID_TAB_SHOW: i32 = 3275;
const ID_TAB_HIDE: i32 = 3276;
const ID_TAB_CHANNEL_BASE: i32 = 3280;

const ID_PRIVATE_HIGHLIGHT: i32 = 3300;
const ID_PRIVATE_SOUND: i32 = 3301;
const ID_PRIVATE_SOUND_PATH: i32 = 3302;
const ID_CHAT_SOUND_VOLUME: i32 = 3303;
const ID_RULE1_ENABLED: i32 = 3304;
const ID_RULE1_MATCH: i32 = 3305;
const ID_RULE1_PATH: i32 = 3306;
const ID_RULE2_ENABLED: i32 = 3307;
const ID_RULE2_MATCH: i32 = 3308;
const ID_RULE2_PATH: i32 = 3309;
const ID_LOGS_ENABLED: i32 = 3310;
const ID_LOG_RETENTION: i32 = 3311;
const ID_OPEN_LOGS: i32 = 3312;

const ID_RESONANCE_PATH: i32 = 3330;
const ID_NPCAP_DEVICE: i32 = 3331;
const ID_OPEN_JSON: i32 = 3332;
const ID_OPEN_FOLDER: i32 = 3333;

const ID_BLOCKED_LIST: i32 = 3350;
const ID_UNBLOCK: i32 = 3351;
const ID_CLEAR_BLOCKED: i32 = 3352;

const ID_COLOR_BASE: i32 = 3400;
const ID_HIGHLIGHT_COLOR: i32 = 3420;
const ID_PRIVATE_COLOR: i32 = 3421;
const ID_HIGHLIGHT_EXPR: i32 = 3422;

const BM_GETCHECK: u32 = 0x00F0;
const BM_SETCHECK: u32 = 0x00F1;
const BST_CHECKED: usize = 1;
const BS_AUTOCHECKBOX: u32 = 0x0000_0003;
const BS_PUSHBUTTON: u32 = 0;
const ES_AUTOHSCROLL: u32 = 0x0080;
const ES_MULTILINE: u32 = 0x0004;
const ES_AUTOVSCROLL: u32 = 0x0040;
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
    fn SetWindowTheme(hwnd: HWND, app_name: *const u16, id_list: *const u16) -> i32;
}

#[link(name = "user32")]
extern "system" {
    fn EnableWindow(hwnd: HWND, enable: i32) -> i32;
}

struct SettingsState {
    settings: Arc<RwLock<AppSettings>>,
    paths: AppPaths,
    main_hwnd: HWND,
    working: AppSettings,
    page_controls: Vec<(HWND, usize)>,
    current_page: usize,
    selected_tab: Option<usize>,
    background: HBRUSH,
    input_background: HBRUSH,
}

pub unsafe fn show(
    instance: HINSTANCE,
    main_hwnd: HWND,
    settings: Arc<RwLock<AppSettings>>,
    paths: AppPaths,
    initial_page: usize,
    add_tab: bool,
) -> Result<(), String> {
    let class = wide(CLASS_NAME);
    let existing = FindWindowW(class.as_ptr(), null());
    if !existing.is_null() {
        ShowWindow(existing, SW_SHOW);
        SetForegroundWindow(existing);
        SendMessageW(existing, WM_SHOW_PAGE, initial_page.min(PAGE_COUNT - 1), add_tab as isize);
        return Ok(());
    }

    let wc = WNDCLASSW {
        lpfnWndProc: Some(settings_wnd_proc),
        hInstance: instance,
        hCursor: LoadCursorW(null_mut(), IDC_ARROW),
        lpszClassName: class.as_ptr(),
        ..std::mem::zeroed()
    };
    if RegisterClassW(&wc) == 0 && GetLastError() != 1410 {
        return Err(format!("RegisterClassW(settings) failed: {}", GetLastError()));
    }

    let mut working = settings.read().map(|s| s.clone()).unwrap_or_default();
    while working.chat.highlight_sound_rules.len() < 2 {
        working.chat.highlight_sound_rules.push(ChatSoundRule::default());
    }
    let state = Box::new(SettingsState {
        settings,
        paths,
        main_hwnd,
        working,
        page_controls: Vec::new(),
        current_page: initial_page.min(PAGE_COUNT - 1),
        selected_tab: None,
        background: CreateSolidBrush(rgb(22, 25, 30)),
        input_background: CreateSolidBrush(rgb(31, 36, 43)),
    });
    let state_ptr = Box::into_raw(state);
    let hwnd = CreateWindowExW(
        0,
        class.as_ptr(),
        wide("BPSR ReadyAlert Settings").as_ptr(),
        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
        CW_USEDEFAULT, CW_USEDEFAULT, 900, 710,
        null_mut(), null_mut(), instance,
        state_ptr.cast::<c_void>(),
    );
    if hwnd.is_null() {
        DeleteObject((*state_ptr).background);
        DeleteObject((*state_ptr).input_background);
        drop(Box::from_raw(state_ptr));
        return Err(format!("CreateWindowExW(settings) failed: {}", GetLastError()));
    }
    try_dark_titlebar(hwnd);
    ShowWindow(hwnd, SW_SHOW);
    SetForegroundWindow(hwnd);
    if add_tab { SendMessageW(hwnd, WM_SHOW_PAGE, PAGE_TABS, 1); }
    Ok(())
}

unsafe extern "system" fn settings_wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        let cs = lparam as *const CREATESTRUCTW;
        if !cs.is_null() { SetWindowLongPtrW(hwnd, GWLP_USERDATA, (*cs).lpCreateParams as isize); }
    }
    let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsState;
    match msg {
        WM_CREATE => {
            if !state_ptr.is_null() {
                build_ui(hwnd, &mut *state_ptr);
                load_all(hwnd, &mut *state_ptr);
                let page = (*state_ptr).current_page;
                show_page(&mut *state_ptr, page);
            }
            0
        }
        WM_SHOW_PAGE => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                let page = (wparam as usize).min(PAGE_COUNT - 1);
                if state.current_page == PAGE_TABS { store_tab_fields(hwnd, state); }
                show_page(state, page);
                if lparam != 0 && page == PAGE_TABS { add_tab(hwnd, state); }
            }
            0
        }
        WM_COMMAND => {
            if !state_ptr.is_null() { handle_command(hwnd, &mut *state_ptr, wparam); }
            0
        }
        WM_CTLCOLORSTATIC | WM_CTLCOLORBTN => {
            if !state_ptr.is_null() {
                let hdc = wparam as HDC;
                SetTextColor(hdc, rgb(225, 231, 238));
                SetBkColor(hdc, rgb(22, 25, 30));
                return (*state_ptr).background as isize;
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_CTLCOLOREDIT => {
            if !state_ptr.is_null() {
                let hdc = wparam as HDC;
                SetTextColor(hdc, rgb(235, 239, 244));
                SetBkColor(hdc, rgb(31, 36, 43));
                return (*state_ptr).input_background as isize;
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_CLOSE => { DestroyWindow(hwnd); 0 }
        WM_NCDESTROY => {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            if !state_ptr.is_null() {
                DeleteObject((*state_ptr).background);
                DeleteObject((*state_ptr).input_background);
                drop(Box::from_raw(state_ptr));
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn build_ui(hwnd: HWND, state: &mut SettingsState) {
    let nav = ["General", "Overlay", "Colors", "Speech", "Tabs & filters", "Sounds & logs", "Network", "Blocked users"];
    for (i, name) in nav.iter().enumerate() {
        create_button(hwnd, NAV_BASE + i as i32, name, 12, 18 + i as i32 * 44, 142, 34);
    }
    create_button(hwnd, ID_APPLY, "Apply", 690, 630, 86, 34);
    create_button(hwnd, ID_CLOSE, "Close", 782, 630, 86, 34);
    build_general(hwnd, state);
    build_overlay(hwnd, state);
    build_colors(hwnd, state);
    build_speech(hwnd, state);
    build_tabs(hwnd, state);
    build_sounds(hwnd, state);
    build_network(hwnd, state);
    build_blocked(hwnd, state);
}

unsafe fn build_general(hwnd: HWND, state: &mut SettingsState) {
    heading(hwnd, state, PAGE_GENERAL, "General alerts", 180, 18);
    checkbox(hwnd, state, PAGE_GENERAL, ID_QUEUE, "Queue Pop alert", 184, 62);
    checkbox(hwnd, state, PAGE_GENERAL, ID_READY, "Ready Check alert", 184, 94);
    checkbox(hwnd, state, PAGE_GENERAL, ID_INVITE, "Party Invite alert", 184, 126);
    checkbox(hwnd, state, PAGE_GENERAL, ID_REQUEST, "Party Request alert", 184, 158);
    checkbox(hwnd, state, PAGE_GENERAL, ID_DESKTOP, "Desktop notifications", 184, 202);
    checkbox(hwnd, state, PAGE_GENERAL, ID_AUTO_LOGS, "Auto-launch Resonance Logs CN", 184, 234);
    field(hwnd, state, PAGE_GENERAL, "Alert volume (0-100)", ID_ALERT_VOLUME, 184, 286, 110);
    info(hwnd, state, PAGE_GENERAL, "Ready / Queue alert volume is independent from chat sounds and TTS volume.", 184, 356, 610, 42);
}

unsafe fn build_overlay(hwnd: HWND, state: &mut SettingsState) {
    heading(hwnd, state, PAGE_OVERLAY, "Chat overlay", 180, 18);
    checkbox(hwnd, state, PAGE_OVERLAY, ID_CHAT_ENABLED, "Enable Chat Overlay", 184, 58);
    checkbox(hwnd, state, PAGE_OVERLAY, ID_TOPMOST, "Always on top", 184, 90);
    checkbox(hwnd, state, PAGE_OVERLAY, ID_COMPACT, "Compact message layout", 184, 122);
    checkbox(hwnd, state, PAGE_OVERLAY, ID_SHOW_TIME, "Show message time", 184, 154);
    checkbox(hwnd, state, PAGE_OVERLAY, ID_TIME_AGO, "Show time as ago (43s / 2m)", 184, 186);
    checkbox(hwnd, state, PAGE_OVERLAY, ID_CLICKTHROUGH, "Click-through overlay", 184, 218);
    checkbox(hwnd, state, PAGE_OVERLAY, ID_BOLD, "Bold message text", 184, 250);
    checkbox(hwnd, state, PAGE_OVERLAY, ID_SHADOW, "Text shadow", 184, 282);

    checkbox(hwnd, state, PAGE_OVERLAY, ID_SEPARATORS, "Row separators", 470, 58);
    checkbox(hwnd, state, PAGE_OVERLAY, ID_ZEBRA, "Zebra rows", 470, 90);
    checkbox(hwnd, state, PAGE_OVERLAY, ID_COLOR_BAND, "Channel color band", 470, 122);
    checkbox(hwnd, state, PAGE_OVERLAY, ID_HIDE_STICKERS, "Hide sticker / picture messages", 470, 154);

    field(hwnd, state, PAGE_OVERLAY, "Click-through recovery hotkey", ID_CLICK_HOTKEY, 184, 332, 260);
    field(hwnd, state, PAGE_OVERLAY, "Window opacity (25-100)", ID_WINDOW_OPACITY, 184, 390, 110);
    field(hwnd, state, PAGE_OVERLAY, "Font family", ID_FONT_FAMILY, 470, 332, 230);
    field(hwnd, state, PAGE_OVERLAY, "Font size (8-24)", ID_FONT_SIZE, 470, 390, 110);
    field(hwnd, state, PAGE_OVERLAY, "Max history (10-500)", ID_MAX_HISTORY, 184, 448, 110);
    label(hwnd, state, PAGE_OVERLAY, "Collapse edge", 470, 448, 140, 22);
    combo(hwnd, state, PAGE_OVERLAY, ID_COLLAPSE_SIDE, 470, 472, 180, 180);
    info(hwnd, state, PAGE_OVERLAY, "Collapse shrinks chat to a 24px Left / Right / Top / Bottom screen-edge handle. Click the handle to expand. Ctrl+Shift+F10 recovers click-through by default.", 184, 520, 610, 58);
}

unsafe fn build_colors(hwnd: HWND, state: &mut SettingsState) {
    heading(hwnd, state, PAGE_COLORS, "Overlay colors & visual highlight", 180, 18);
    let entries = ["World", "Local", "Team", "Guild", "Private", "Group", "Notice", "Play", "Newbie", "System"];
    for (i, name) in entries.iter().enumerate() {
        let col = i / 5;
        let row = i % 5;
        field(hwnd, state, PAGE_COLORS, name, ID_COLOR_BASE + i as i32, 184 + col as i32 * 300, 58 + row as i32 * 54, 160);
    }
    field(hwnd, state, PAGE_COLORS, "Visual highlight rule", ID_HIGHLIGHT_EXPR, 184, 340, 610);
    field(hwnd, state, PAGE_COLORS, "Keyword highlight color", ID_HIGHLIGHT_COLOR, 184, 400, 170);
    field(hwnd, state, PAGE_COLORS, "Private highlight color", ID_PRIVATE_COLOR, 484, 400, 170);
    info(hwnd, state, PAGE_COLORS, "Filter syntax matches v1.3.6: OR / || / spaced |, AND / &&, or regular expressions. Example: serum | food | raid. Colors use #RRGGBB.", 184, 480, 610, 58);
}

unsafe fn build_speech(hwnd: HWND, state: &mut SettingsState) {
    heading(hwnd, state, PAGE_SPEECH, "Speech & translation", 180, 18);
    checkbox(hwnd, state, PAGE_SPEECH, ID_TRANSLATE, "Translate non-English chat to English", 184, 58);
    checkbox_at(hwnd, state, PAGE_SPEECH, ID_TRANSLATE_WORLD, "World", 204, 92, 90);
    checkbox_at(hwnd, state, PAGE_SPEECH, ID_TRANSLATE_GUILD, "Guild", 304, 92, 90);
    checkbox_at(hwnd, state, PAGE_SPEECH, ID_TRANSLATE_PARTY, "Party / Team", 404, 92, 150);
    checkbox(hwnd, state, PAGE_SPEECH, ID_TRANSLATE_OVERLAY, "Show English translation under the original message", 204, 126);
    checkbox(hwnd, state, PAGE_SPEECH, ID_TTS, "Enable chat text-to-speech", 184, 184);
    checkbox_at(hwnd, state, PAGE_SPEECH, ID_TTS_GUILD, "Guild", 204, 218, 90);
    checkbox_at(hwnd, state, PAGE_SPEECH, ID_TTS_PARTY, "Party / Team", 304, 218, 150);
    checkbox(hwnd, state, PAGE_SPEECH, ID_TTS_SENDER, "Read sender name before the message", 204, 252);
    field(hwnd, state, PAGE_SPEECH, "Manual own username override (optional)", ID_TTS_USERNAME, 184, 302, 350);
    field(hwnd, state, PAGE_SPEECH, "TTS volume (0-100)", ID_TTS_VOLUME, 184, 366, 110);
    checkbox(hwnd, state, PAGE_SPEECH, ID_HIDE_RICH, "Hide emoji-only and linked-item noise", 184, 424);
    button(hwnd, state, PAGE_SPEECH, ID_TEST_TTS, "Test Google English TTS", 184, 470, 190, 32);
    info(hwnd, state, PAGE_SPEECH, "TTS is intentionally limited to Guild and Party / Team. World chat is never spoken. Translation/TTS run off the capture thread.", 184, 522, 610, 54);
}

unsafe fn build_tabs(hwnd: HWND, state: &mut SettingsState) {
    heading(hwnd, state, PAGE_TABS, "Tabs & filters", 180, 18);
    listbox(hwnd, state, PAGE_TABS, ID_TAB_LIST, 184, 58, 220, 210);
    button(hwnd, state, PAGE_TABS, ID_TAB_ADD, "+ Add tab", 184, 278, 104, 30);
    button(hwnd, state, PAGE_TABS, ID_TAB_DELETE, "Delete", 296, 278, 104, 30);
    field(hwnd, state, PAGE_TABS, "Tab name", ID_TAB_NAME, 430, 58, 310);
    field(hwnd, state, PAGE_TABS, "Minimum player level", ID_TAB_MIN_LEVEL, 430, 116, 110);
    field(hwnd, state, PAGE_TABS, "Show only if message matches", ID_TAB_SHOW, 430, 174, 390);
    field(hwnd, state, PAGE_TABS, "Hide if message matches", ID_TAB_HIDE, 430, 232, 390);
    label(hwnd, state, PAGE_TABS, "Channels", 430, 292, 140, 22);
    let channels = ["World", "Local", "Team", "Guild", "Private", "Group", "Notice", "Play", "Newbie", "System"];
    for (i, name) in channels.iter().enumerate() {
        checkbox_at(hwnd, state, PAGE_TABS, ID_TAB_CHANNEL_BASE + i as i32, name, 430 + (i % 2) as i32 * 170, 322 + (i / 2) as i32 * 32, 160);
    }
    info(hwnd, state, PAGE_TABS, "Filter syntax: OR / || / spaced |, AND / &&, or regex. Compact regex pipes such as foo|bar remain regex alternation. +Tab opens this page and creates a new tab.", 184, 500, 610, 58);
}

unsafe fn build_sounds(hwnd: HWND, state: &mut SettingsState) {
    heading(hwnd, state, PAGE_SOUNDS, "Highlights, sounds & logs", 180, 18);
    checkbox(hwnd, state, PAGE_SOUNDS, ID_PRIVATE_HIGHLIGHT, "Highlight Private / Talk messages", 184, 58);
    checkbox(hwnd, state, PAGE_SOUNDS, ID_PRIVATE_SOUND, "Play sound for Private / Talk", 184, 90);
    field(hwnd, state, PAGE_SOUNDS, "Private sound path", ID_PRIVATE_SOUND_PATH, 184, 132, 560);
    field(hwnd, state, PAGE_SOUNDS, "Chat sound volume (0-100)", ID_CHAT_SOUND_VOLUME, 184, 190, 110);
    checkbox(hwnd, state, PAGE_SOUNDS, ID_RULE1_ENABLED, "Keyword sound rule 1", 184, 246);
    field(hwnd, state, PAGE_SOUNDS, "Match (same OR / AND / regex syntax)", ID_RULE1_MATCH, 204, 278, 280);
    field(hwnd, state, PAGE_SOUNDS, "Sound path", ID_RULE1_PATH, 500, 278, 300);
    checkbox(hwnd, state, PAGE_SOUNDS, ID_RULE2_ENABLED, "Keyword sound rule 2", 184, 336);
    field(hwnd, state, PAGE_SOUNDS, "Match (same OR / AND / regex syntax)", ID_RULE2_MATCH, 204, 368, 280);
    field(hwnd, state, PAGE_SOUNDS, "Sound path", ID_RULE2_PATH, 500, 368, 300);
    checkbox(hwnd, state, PAGE_SOUNDS, ID_LOGS_ENABLED, "Keep local chat logs", 184, 440);
    label(hwnd, state, PAGE_SOUNDS, "Retention", 470, 440, 100, 22);
    combo(hwnd, state, PAGE_SOUNDS, ID_LOG_RETENTION, 550, 436, 140, 120);
    button(hwnd, state, PAGE_SOUNDS, ID_OPEN_LOGS, "Open chat logs", 184, 492, 130, 30);
}

unsafe fn build_network(hwnd: HWND, state: &mut SettingsState) {
    heading(hwnd, state, PAGE_NETWORK, "Network & integration", 180, 18);
    field(hwnd, state, PAGE_NETWORK, "Resonance Logs CN executable path", ID_RESONANCE_PATH, 184, 66, 600);
    field(hwnd, state, PAGE_NETWORK, "Npcap device name (blank = follow Resonance Logs / auto)", ID_NPCAP_DEVICE, 184, 136, 600);
    button(hwnd, state, PAGE_NETWORK, ID_OPEN_JSON, "Advanced: settings.json", 184, 218, 180, 32);
    button(hwnd, state, PAGE_NETWORK, ID_OPEN_FOLDER, "Open ReadyAlert folder", 376, 218, 170, 32);
    info(hwnd, state, PAGE_NETWORK, "Normal settings should be changed in this window. Raw JSON is retained only as an advanced / recovery option. Blank Npcap device is recommended.", 184, 282, 610, 58);
}

unsafe fn build_blocked(hwnd: HWND, state: &mut SettingsState) {
    heading(hwnd, state, PAGE_BLOCKED, "Blocked users", 180, 18);
    listbox(hwnd, state, PAGE_BLOCKED, ID_BLOCKED_LIST, 184, 58, 600, 360);
    button(hwnd, state, PAGE_BLOCKED, ID_UNBLOCK, "Unblock selected", 184, 432, 150, 32);
    button(hwnd, state, PAGE_BLOCKED, ID_CLEAR_BLOCKED, "Clear all", 346, 432, 110, 32);
    info(hwnd, state, PAGE_BLOCKED, "Blocked players are suppressed from overlay display, chat sounds, translation and TTS. Players blocked from the overlay right-click menu appear here.", 184, 492, 610, 54);
}

unsafe fn load_all(hwnd: HWND, state: &mut SettingsState) {
    while state.working.chat.highlight_sound_rules.len() < 2 {
        state.working.chat.highlight_sound_rules.push(ChatSoundRule::default());
    }
    let s = &state.working;
    set_check(hwnd, ID_QUEUE, s.queue_pop_alert);
    set_check(hwnd, ID_READY, s.ready_check_alert);
    set_check(hwnd, ID_INVITE, s.party_invite_alert);
    set_check(hwnd, ID_REQUEST, s.party_request_alert);
    set_check(hwnd, ID_DESKTOP, s.desktop_notification);
    set_check(hwnd, ID_AUTO_LOGS, s.auto_launch_resonance_logs);
    set_text(hwnd, ID_ALERT_VOLUME, &s.alert_volume.to_string());

    set_check(hwnd, ID_CHAT_ENABLED, s.chat_overlay_enabled);
    set_check(hwnd, ID_TOPMOST, s.chat.top_most);
    set_check(hwnd, ID_COMPACT, s.chat.compact_mode);
    set_check(hwnd, ID_SHOW_TIME, s.chat.show_time);
    set_check(hwnd, ID_TIME_AGO, s.chat.show_time_as_ago);
    set_check(hwnd, ID_CLICKTHROUGH, s.chat.click_through);
    set_check(hwnd, ID_BOLD, s.chat.bold_message_text);
    set_check(hwnd, ID_SHADOW, s.chat.text_shadow);
    set_check(hwnd, ID_SEPARATORS, s.chat.show_separators);
    set_check(hwnd, ID_ZEBRA, s.chat.show_zebra_stripes);
    set_check(hwnd, ID_COLOR_BAND, s.chat.show_color_band);
    set_check(hwnd, ID_HIDE_STICKERS, s.chat.hide_stickers);
    set_text(hwnd, ID_CLICK_HOTKEY, &s.chat.click_through_hotkey);
    set_text(hwnd, ID_WINDOW_OPACITY, &s.chat.window_opacity.to_string());
    set_text(hwnd, ID_FONT_FAMILY, &s.chat.font_family);
    set_text(hwnd, ID_FONT_SIZE, &format!("{:.1}", s.chat.font_size));
    set_text(hwnd, ID_MAX_HISTORY, &s.chat.max_history.to_string());
    let collapse = match s.chat.collapse_side.to_ascii_lowercase().as_str() { "right" => 1, "top" => 2, "bottom" => 3, _ => 0 };
    set_combo_items(hwnd, ID_COLLAPSE_SIDE, &["Left", "Right", "Top", "Bottom"], collapse);

    let channels = [1,2,3,4,5,6,7,8,9,99];
    for (i, channel) in channels.iter().enumerate() {
        let fallback = "#C7C7C7";
        set_text(hwnd, ID_COLOR_BASE + i as i32, s.chat.channel_colors.get(channel).map(String::as_str).unwrap_or(fallback));
    }
    set_text(hwnd, ID_HIGHLIGHT_EXPR, &s.chat.highlight_if_matches);
    set_text(hwnd, ID_HIGHLIGHT_COLOR, &s.chat.highlight_color);
    set_text(hwnd, ID_PRIVATE_COLOR, &s.chat.private_highlight_color);

    let sp = &s.speech_translation;
    set_check(hwnd, ID_TRANSLATE, sp.translation_enabled);
    set_check(hwnd, ID_TRANSLATE_WORLD, sp.translation_world);
    set_check(hwnd, ID_TRANSLATE_GUILD, sp.translation_guild);
    set_check(hwnd, ID_TRANSLATE_PARTY, sp.translation_party_team);
    set_check(hwnd, ID_TRANSLATE_OVERLAY, sp.show_translation_in_overlay);
    set_check(hwnd, ID_TTS, sp.tts_enabled);
    set_check(hwnd, ID_TTS_GUILD, sp.tts_guild);
    set_check(hwnd, ID_TTS_PARTY, sp.tts_party_team);
    set_check(hwnd, ID_TTS_SENDER, sp.read_sender_name);
    set_text(hwnd, ID_TTS_USERNAME, &sp.ignore_own_username);
    set_text(hwnd, ID_TTS_VOLUME, &sp.tts_volume.to_string());
    set_check(hwnd, ID_HIDE_RICH, sp.hide_emoji_messages && sp.hide_linked_item_messages);

    set_check(hwnd, ID_PRIVATE_HIGHLIGHT, s.chat.private_highlight_enabled);
    set_check(hwnd, ID_PRIVATE_SOUND, s.chat.private_sound_enabled);
    set_text(hwnd, ID_PRIVATE_SOUND_PATH, &s.chat.private_sound_path);
    set_text(hwnd, ID_CHAT_SOUND_VOLUME, &s.chat.chat_sound_volume.to_string());
    let r1 = &s.chat.highlight_sound_rules[0];
    set_check(hwnd, ID_RULE1_ENABLED, r1.enabled);
    set_text(hwnd, ID_RULE1_MATCH, &r1.match_text);
    set_text(hwnd, ID_RULE1_PATH, &r1.sound_path);
    let r2 = &s.chat.highlight_sound_rules[1];
    set_check(hwnd, ID_RULE2_ENABLED, r2.enabled);
    set_text(hwnd, ID_RULE2_MATCH, &r2.match_text);
    set_text(hwnd, ID_RULE2_PATH, &r2.sound_path);
    set_check(hwnd, ID_LOGS_ENABLED, s.chat.keep_local_chat_logs24_hours);
    set_combo_items(hwnd, ID_LOG_RETENTION, &["24 hours", "72 hours", "7 days"], match s.chat.local_chat_log_retention_hours { 24 => 0, 72 => 1, _ => 2 });

    set_text(hwnd, ID_RESONANCE_PATH, &s.resonance_logs_path);
    set_text(hwnd, ID_NPCAP_DEVICE, &s.npcap_device_name);
    refresh_tab_list(hwnd, state);
    refresh_blocked_list(hwnd, state);
    refresh_speech_enabled(hwnd);
}

unsafe fn apply(hwnd: HWND, state: &mut SettingsState) {
    store_tab_fields(hwnd, state);

    let highlight_expr = get_text(hwnd, ID_HIGHLIGHT_EXPR);
    if !validate_filter(hwnd, "Visual highlight rule", &highlight_expr) { return; }
    for (index, tab) in state.working.chat.tabs.iter().enumerate() {
        if !validate_filter(hwnd, &format!("Tab {} show filter", index + 1), &tab.show_if_matches) { return; }
        if !validate_filter(hwnd, &format!("Tab {} hide filter", index + 1), &tab.hide_if_matches) { return; }
    }
    let rule1 = get_text(hwnd, ID_RULE1_MATCH);
    let rule2 = get_text(hwnd, ID_RULE2_MATCH);
    if get_check(hwnd, ID_RULE1_ENABLED) && !validate_filter(hwnd, "Keyword sound rule 1", &rule1) { return; }
    if get_check(hwnd, ID_RULE2_ENABLED) && !validate_filter(hwnd, "Keyword sound rule 2", &rule2) { return; }

    let s = &mut state.working;
    s.queue_pop_alert = get_check(hwnd, ID_QUEUE);
    s.ready_check_alert = get_check(hwnd, ID_READY);
    s.party_invite_alert = get_check(hwnd, ID_INVITE);
    s.party_request_alert = get_check(hwnd, ID_REQUEST);
    s.desktop_notification = get_check(hwnd, ID_DESKTOP);
    s.auto_launch_resonance_logs = get_check(hwnd, ID_AUTO_LOGS);
    s.alert_volume = read_i32(hwnd, ID_ALERT_VOLUME, s.alert_volume);

    s.chat_overlay_enabled = get_check(hwnd, ID_CHAT_ENABLED);
    s.chat.top_most = get_check(hwnd, ID_TOPMOST);
    s.chat.compact_mode = get_check(hwnd, ID_COMPACT);
    s.chat.show_time = get_check(hwnd, ID_SHOW_TIME);
    s.chat.show_time_as_ago = get_check(hwnd, ID_TIME_AGO);
    s.chat.click_through = get_check(hwnd, ID_CLICKTHROUGH);
    s.chat.bold_message_text = get_check(hwnd, ID_BOLD);
    s.chat.text_shadow = get_check(hwnd, ID_SHADOW);
    s.chat.show_separators = get_check(hwnd, ID_SEPARATORS);
    s.chat.show_zebra_stripes = get_check(hwnd, ID_ZEBRA);
    s.chat.show_color_band = get_check(hwnd, ID_COLOR_BAND);
    s.chat.hide_stickers = get_check(hwnd, ID_HIDE_STICKERS);
    s.chat.click_through_hotkey = get_text(hwnd, ID_CLICK_HOTKEY);
    s.chat.window_opacity = read_i32(hwnd, ID_WINDOW_OPACITY, s.chat.window_opacity);
    s.chat.font_family = get_text(hwnd, ID_FONT_FAMILY);
    s.chat.font_size = get_text(hwnd, ID_FONT_SIZE).parse::<f32>().unwrap_or(s.chat.font_size);
    s.chat.max_history = read_i32(hwnd, ID_MAX_HISTORY, s.chat.max_history as i32).max(1) as usize;
    s.chat.collapse_side = match combo_sel(hwnd, ID_COLLAPSE_SIDE) { 1 => "Right", 2 => "Top", 3 => "Bottom", _ => "Left" }.into();

    let channels = [1,2,3,4,5,6,7,8,9,99];
    for (i, channel) in channels.iter().enumerate() { s.chat.channel_colors.insert(*channel, get_text(hwnd, ID_COLOR_BASE + i as i32)); }
    s.chat.highlight_if_matches = highlight_expr;
    s.chat.highlight_color = get_text(hwnd, ID_HIGHLIGHT_COLOR);
    s.chat.private_highlight_color = get_text(hwnd, ID_PRIVATE_COLOR);

    let sp = &mut s.speech_translation;
    sp.translation_enabled = get_check(hwnd, ID_TRANSLATE);
    sp.translation_world = get_check(hwnd, ID_TRANSLATE_WORLD);
    sp.translation_guild = get_check(hwnd, ID_TRANSLATE_GUILD);
    sp.translation_party_team = get_check(hwnd, ID_TRANSLATE_PARTY);
    sp.show_translation_in_overlay = get_check(hwnd, ID_TRANSLATE_OVERLAY);
    sp.tts_enabled = get_check(hwnd, ID_TTS);
    sp.tts_guild = get_check(hwnd, ID_TTS_GUILD);
    sp.tts_party_team = get_check(hwnd, ID_TTS_PARTY);
    sp.read_sender_name = get_check(hwnd, ID_TTS_SENDER);
    sp.ignore_own_username = get_text(hwnd, ID_TTS_USERNAME);
    sp.tts_volume = read_i32(hwnd, ID_TTS_VOLUME, sp.tts_volume);
    let hide_rich = get_check(hwnd, ID_HIDE_RICH);
    sp.hide_emoji_messages = hide_rich;
    sp.hide_linked_item_messages = hide_rich;

    s.chat.private_highlight_enabled = get_check(hwnd, ID_PRIVATE_HIGHLIGHT);
    s.chat.private_sound_enabled = get_check(hwnd, ID_PRIVATE_SOUND);
    s.chat.private_sound_path = get_text(hwnd, ID_PRIVATE_SOUND_PATH);
    s.chat.chat_sound_volume = read_i32(hwnd, ID_CHAT_SOUND_VOLUME, s.chat.chat_sound_volume);
    while s.chat.highlight_sound_rules.len() < 2 { s.chat.highlight_sound_rules.push(ChatSoundRule::default()); }
    s.chat.highlight_sound_rules[0].enabled = get_check(hwnd, ID_RULE1_ENABLED);
    s.chat.highlight_sound_rules[0].match_text = rule1;
    s.chat.highlight_sound_rules[0].sound_path = get_text(hwnd, ID_RULE1_PATH);
    s.chat.highlight_sound_rules[1].enabled = get_check(hwnd, ID_RULE2_ENABLED);
    s.chat.highlight_sound_rules[1].match_text = rule2;
    s.chat.highlight_sound_rules[1].sound_path = get_text(hwnd, ID_RULE2_PATH);
    s.chat.keep_local_chat_logs24_hours = get_check(hwnd, ID_LOGS_ENABLED);
    s.chat.local_chat_log_retention_hours = match combo_sel(hwnd, ID_LOG_RETENTION) { 0 => 24, 1 => 72, _ => 168 };

    s.resonance_logs_path = get_text(hwnd, ID_RESONANCE_PATH);
    s.npcap_device_name = get_text(hwnd, ID_NPCAP_DEVICE);
    s.normalize();

    if let Ok(mut guard) = state.settings.write() { *guard = s.clone(); }
    if let Err(err) = settings::save(&state.paths, s) {
        MessageBoxW(hwnd, wide(&format!("Could not save settings.\r\n\r\n{err}")).as_ptr(), wide("ReadyAlert Settings").as_ptr(), MB_OK | MB_ICONERROR);
        return;
    }
    PostMessageW(state.main_hwnd, WM_COMMAND, CMD_SETTINGS_APPLIED as usize, 0);
    load_all(hwnd, state);
}

unsafe fn validate_filter(hwnd: HWND, label: &str, expression: &str) -> bool {
    match chat::validate_expression(expression) {
        Ok(()) => true,
        Err(err) => {
            let text = format!("{label} is invalid.\r\n\r\n{err}\r\n\r\nUse OR / || / spaced |, AND / &&, or a valid regular expression.");
            MessageBoxW(hwnd, wide(&text).as_ptr(), wide("Invalid chat filter").as_ptr(), MB_OK | MB_ICONERROR);
            false
        }
    }
}

unsafe fn handle_command(hwnd: HWND, state: &mut SettingsState, wparam: WPARAM) {
    let id = (wparam as u32 & 0xffff) as i32;
    let notify = ((wparam as u32 >> 16) & 0xffff) as u16;
    if id >= NAV_BASE && id < NAV_BASE + PAGE_COUNT as i32 {
        if state.current_page == PAGE_TABS { store_tab_fields(hwnd, state); }
        show_page(state, (id - NAV_BASE) as usize);
        return;
    }
    match id {
        ID_APPLY => apply(hwnd, state),
        ID_CLOSE => { DestroyWindow(hwnd); }
        ID_TTS | ID_TRANSLATE => refresh_speech_enabled(hwnd),
        ID_TEST_TTS => {
            let volume = read_i32(hwnd, ID_TTS_VOLUME, state.working.speech_translation.tts_volume).clamp(0,100);
            if volume <= 0 {
                MessageBoxW(hwnd, wide("TTS volume is muted at 0%. Raise it before testing.").as_ptr(), wide("TTS is muted").as_ptr(), MB_OK | MB_ICONINFORMATION);
            } else {
                PostMessageW(state.main_hwnd, WM_COMMAND, CMD_TTS_TEST as usize, volume as isize);
            }
        }
        ID_TAB_LIST if notify == LBN_SELCHANGE => {
            store_tab_fields(hwnd, state);
            let sel = list_sel(hwnd, ID_TAB_LIST);
            state.selected_tab = if sel >= 0 { Some(sel as usize) } else { None };
            load_tab_fields(hwnd, state);
        }
        ID_TAB_ADD => add_tab(hwnd, state),
        ID_TAB_DELETE => delete_tab(hwnd, state),
        ID_OPEN_LOGS => { PostMessageW(state.main_hwnd, WM_COMMAND, CMD_OPEN_LOGS as usize, 0); }
        ID_OPEN_JSON => { PostMessageW(state.main_hwnd, WM_COMMAND, CMD_OPEN_SETTINGS_JSON as usize, 0); }
        ID_OPEN_FOLDER => { PostMessageW(state.main_hwnd, WM_COMMAND, CMD_OPEN_APP_FOLDER as usize, 0); }
        ID_UNBLOCK => unblock_selected(hwnd, state),
        ID_CLEAR_BLOCKED => { state.working.chat.blocked_users.clear(); refresh_blocked_list(hwnd, state); }
        _ => {}
    }
}

unsafe fn show_page(state: &mut SettingsState, page: usize) {
    state.current_page = page.min(PAGE_COUNT - 1);
    for (control, p) in &state.page_controls { ShowWindow(*control, if *p == state.current_page { SW_SHOW } else { SW_HIDE }); }
}

unsafe fn add_tab(hwnd: HWND, state: &mut SettingsState) {
    store_tab_fields(hwnd, state);
    let id = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(1).max(1);
    state.working.chat.tabs.push(ChatTabSettings {
        id,
        name: "New Tab".into(),
        channels: vec![1],
        min_level: 1,
        show_if_matches: String::new(),
        hide_if_matches: String::new(),
    });
    state.working.chat.last_selected_tab_id = id;
    state.selected_tab = Some(state.working.chat.tabs.len() - 1);
    refresh_tab_list(hwnd, state);
    load_tab_fields(hwnd, state);
    SetForegroundWindow(hwnd);
}

unsafe fn delete_tab(hwnd: HWND, state: &mut SettingsState) {
    if state.working.chat.tabs.len() <= 1 {
        MessageBoxW(hwnd, wide("At least one chat tab is required.").as_ptr(), wide("Chat tab").as_ptr(), MB_OK | MB_ICONINFORMATION);
        return;
    }
    store_tab_fields(hwnd, state);
    let Some(index) = state.selected_tab.filter(|i| *i < state.working.chat.tabs.len()) else { return; };
    state.working.chat.tabs.remove(index);
    let next = index.min(state.working.chat.tabs.len() - 1);
    state.selected_tab = Some(next);
    state.working.chat.last_selected_tab_id = state.working.chat.tabs[next].id;
    refresh_tab_list(hwnd, state);
    load_tab_fields(hwnd, state);
}

unsafe fn refresh_tab_list(hwnd: HWND, state: &mut SettingsState) {
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
}

unsafe fn store_tab_fields(hwnd: HWND, state: &mut SettingsState) {
    let Some(index) = state.selected_tab.filter(|i| *i < state.working.chat.tabs.len()) else { return; };
    let tab = &mut state.working.chat.tabs[index];
    tab.name = get_text(hwnd, ID_TAB_NAME);
    tab.min_level = read_i32(hwnd, ID_TAB_MIN_LEVEL, tab.min_level);
    tab.show_if_matches = get_text(hwnd, ID_TAB_SHOW);
    tab.hide_if_matches = get_text(hwnd, ID_TAB_HIDE);
    let channels = [1,2,3,4,5,6,7,8,9,99];
    tab.channels.clear();
    for (i, channel) in channels.iter().enumerate() {
        if get_check(hwnd, ID_TAB_CHANNEL_BASE + i as i32) { tab.channels.push(*channel); }
    }
    if tab.channels.is_empty() { tab.channels.push(1); }
    tab.name = if tab.name.trim().is_empty() { "Chat".into() } else { tab.name.trim().to_string() };
}

unsafe fn load_tab_fields(hwnd: HWND, state: &SettingsState) {
    let Some(index) = state.selected_tab.filter(|i| *i < state.working.chat.tabs.len()) else { return; };
    let tab = &state.working.chat.tabs[index];
    set_text(hwnd, ID_TAB_NAME, &tab.name);
    set_text(hwnd, ID_TAB_MIN_LEVEL, &tab.min_level.to_string());
    set_text(hwnd, ID_TAB_SHOW, &tab.show_if_matches);
    set_text(hwnd, ID_TAB_HIDE, &tab.hide_if_matches);
    let channels = [1,2,3,4,5,6,7,8,9,99];
    for (i, channel) in channels.iter().enumerate() { set_check(hwnd, ID_TAB_CHANNEL_BASE + i as i32, tab.channels.contains(channel)); }
}

unsafe fn refresh_blocked_list(hwnd: HWND, state: &SettingsState) {
    let list = GetDlgItem(hwnd, ID_BLOCKED_LIST);
    SendMessageW(list, LB_RESETCONTENT, 0, 0);
    for user in &state.working.chat.blocked_users {
        let name = if user.name.trim().is_empty() { format!("UID {}", user.id) } else { format!("{}  (UID {})", user.name, user.id) };
        let text = wide(&name);
        SendMessageW(list, LB_ADDSTRING, 0, text.as_ptr() as isize);
    }
}

unsafe fn unblock_selected(hwnd: HWND, state: &mut SettingsState) {
    let sel = list_sel(hwnd, ID_BLOCKED_LIST);
    if sel < 0 { return; }
    let index = sel as usize;
    if index < state.working.chat.blocked_users.len() {
        state.working.chat.blocked_users.remove(index);
        refresh_blocked_list(hwnd, state);
    }
}

unsafe fn refresh_speech_enabled(hwnd: HWND) {
    let translate = get_check(hwnd, ID_TRANSLATE);
    for id in [ID_TRANSLATE_WORLD, ID_TRANSLATE_GUILD, ID_TRANSLATE_PARTY, ID_TRANSLATE_OVERLAY] { EnableWindow(GetDlgItem(hwnd, id), translate as i32); }
    let tts = get_check(hwnd, ID_TTS);
    for id in [ID_TTS_GUILD, ID_TTS_PARTY, ID_TTS_SENDER, ID_TTS_USERNAME, ID_TTS_VOLUME, ID_TEST_TTS] { EnableWindow(GetDlgItem(hwnd, id), tts as i32); }
}

unsafe fn heading(hwnd: HWND, state: &mut SettingsState, page: usize, text: &str, x: i32, y: i32) {
    let h = create_static(hwnd, text, x, y, 650, 28); state.page_controls.push((h, page));
}
unsafe fn info(hwnd: HWND, state: &mut SettingsState, page: usize, text: &str, x: i32, y: i32, w: i32, h: i32) {
    let c = create_static(hwnd, text, x, y, w, h); state.page_controls.push((c, page));
}
unsafe fn label(hwnd: HWND, state: &mut SettingsState, page: usize, text: &str, x: i32, y: i32, w: i32, h: i32) {
    let c = create_static(hwnd, text, x, y, w, h); state.page_controls.push((c, page));
}
unsafe fn checkbox(hwnd: HWND, state: &mut SettingsState, page: usize, id: i32, text: &str, x: i32, y: i32) {
    checkbox_at(hwnd, state, page, id, text, x, y, 380);
}
unsafe fn checkbox_at(hwnd: HWND, state: &mut SettingsState, page: usize, id: i32, text: &str, x: i32, y: i32, w: i32) {
    let c = create_control(hwnd, "BUTTON", text, id, x, y, w, 26, WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_AUTOCHECKBOX, 0); state.page_controls.push((c, page));
}
unsafe fn edit(hwnd: HWND, state: &mut SettingsState, page: usize, id: i32, x: i32, y: i32, w: i32, h: i32, multiline: bool) {
    let mut style = WS_CHILD | WS_VISIBLE | WS_TABSTOP | ES_AUTOHSCROLL;
    if multiline { style |= ES_MULTILINE | ES_AUTOVSCROLL | WS_VSCROLL; }
    let c = create_control(hwnd, "EDIT", "", id, x, y, w, h, style, WS_EX_CLIENTEDGE); state.page_controls.push((c, page));
}
unsafe fn field(hwnd: HWND, state: &mut SettingsState, page: usize, text: &str, id: i32, x: i32, y: i32, w: i32) {
    label(hwnd, state, page, text, x, y, w.max(180), 20); edit(hwnd, state, page, id, x, y + 22, w, 26, false);
}
unsafe fn button(hwnd: HWND, state: &mut SettingsState, page: usize, id: i32, text: &str, x: i32, y: i32, w: i32, h: i32) {
    let c = create_button(hwnd, id, text, x, y, w, h); state.page_controls.push((c, page));
}
unsafe fn combo(hwnd: HWND, state: &mut SettingsState, page: usize, id: i32, x: i32, y: i32, w: i32, h: i32) {
    let c = create_control(hwnd, "COMBOBOX", "", id, x, y, w, h, WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_VSCROLL | CBS_DROPDOWNLIST, 0); state.page_controls.push((c, page));
}
unsafe fn listbox(hwnd: HWND, state: &mut SettingsState, page: usize, id: i32, x: i32, y: i32, w: i32, h: i32) {
    let c = create_control(hwnd, "LISTBOX", "", id, x, y, w, h, WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_VSCROLL | LBS_NOTIFY | WS_BORDER, WS_EX_CLIENTEDGE); state.page_controls.push((c, page));
}
unsafe fn create_static(hwnd: HWND, text: &str, x: i32, y: i32, w: i32, h: i32) -> HWND { create_control(hwnd, "STATIC", text, 0, x, y, w, h, WS_CHILD | WS_VISIBLE, 0) }
unsafe fn create_button(hwnd: HWND, id: i32, text: &str, x: i32, y: i32, w: i32, h: i32) -> HWND { create_control(hwnd, "BUTTON", text, id, x, y, w, h, WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON, 0) }

unsafe fn create_control(hwnd: HWND, class: &str, text: &str, id: i32, x: i32, y: i32, w: i32, h: i32, style: u32, ex: u32) -> HWND {
    let instance = GetModuleHandleW(null());
    let c = CreateWindowExW(ex, wide(class).as_ptr(), wide(text).as_ptr(), style, x, y, w, h, hwnd, id as usize as HMENU, instance, null());
    if !c.is_null() {
        let font = GetStockObject(DEFAULT_GUI_FONT);
        SendMessageW(c, WM_SETFONT, font as usize, 1);
        let theme = wide("DarkMode_Explorer");
        let _ = SetWindowTheme(c, theme.as_ptr(), null());
    }
    c
}

unsafe fn set_check(hwnd: HWND, id: i32, checked: bool) { SendMessageW(GetDlgItem(hwnd, id), BM_SETCHECK, if checked { BST_CHECKED } else { 0 }, 0); }
unsafe fn get_check(hwnd: HWND, id: i32) -> bool { SendMessageW(GetDlgItem(hwnd, id), BM_GETCHECK, 0, 0) as usize == BST_CHECKED }
unsafe fn set_text(hwnd: HWND, id: i32, text: &str) { let c = GetDlgItem(hwnd, id); if !c.is_null() { SetWindowTextW(c, wide(text).as_ptr()); } }
unsafe fn get_text(hwnd: HWND, id: i32) -> String {
    let c = GetDlgItem(hwnd, id); if c.is_null() { return String::new(); }
    let len = GetWindowTextLengthW(c).max(0) as usize;
    let mut buf = vec![0u16; len + 1];
    let got = GetWindowTextW(c, buf.as_mut_ptr(), buf.len() as i32).max(0) as usize;
    String::from_utf16_lossy(&buf[..got])
}
unsafe fn read_i32(hwnd: HWND, id: i32, fallback: i32) -> i32 { get_text(hwnd, id).trim().parse::<i32>().unwrap_or(fallback) }
unsafe fn set_combo_items(hwnd: HWND, id: i32, values: &[&str], selected: usize) {
    let c = GetDlgItem(hwnd, id);
    SendMessageW(c, CB_RESETCONTENT, 0, 0);
    for value in values { let w = wide(value); SendMessageW(c, CB_ADDSTRING, 0, w.as_ptr() as isize); }
    SendMessageW(c, CB_SETCURSEL, selected.min(values.len().saturating_sub(1)), 0);
}
unsafe fn combo_sel(hwnd: HWND, id: i32) -> isize { SendMessageW(GetDlgItem(hwnd, id), CB_GETCURSEL, 0, 0) }
unsafe fn list_sel(hwnd: HWND, id: i32) -> isize { SendMessageW(GetDlgItem(hwnd, id), LB_GETCURSEL, 0, 0) }

unsafe fn try_dark_titlebar(hwnd: HWND) {
    let enabled: i32 = 1;
    let ptr = (&enabled as *const i32).cast::<c_void>();
    if DwmSetWindowAttribute(hwnd, 20, ptr, std::mem::size_of::<i32>() as u32) != 0 { let _ = DwmSetWindowAttribute(hwnd, 19, ptr, std::mem::size_of::<i32>() as u32); }
}

fn rgb(r: u8, g: u8, b: u8) -> u32 { u32::from(r) | (u32::from(g) << 8) | (u32::from(b) << 16) }
fn wide(text: &str) -> Vec<u16> { text.encode_utf16().chain(std::iter::once(0)).collect() }
