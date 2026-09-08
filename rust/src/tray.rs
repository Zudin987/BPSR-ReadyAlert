use crate::{model::AlertKind, npcap::PcapApi, settings::AppSettings};
use std::ptr::null;
use windows_sys::Win32::{
    Foundation::{HWND, POINT},
    UI::WindowsAndMessaging::{
        AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, SetForegroundWindow,
        TrackPopupMenu, HMENU, MF_CHECKED, MF_POPUP, MF_SEPARATOR, MF_STRING,
        TPM_BOTTOMALIGN, TPM_LEFTALIGN, TPM_RETURNCMD, TPM_RIGHTBUTTON,
    },
};

const CMD_QUEUE: u32 = 1001;
const CMD_READY: u32 = 1002;
const CMD_INVITE: u32 = 1003;
const CMD_REQUEST: u32 = 1004;
const CMD_DESKTOP: u32 = 1005;
const CMD_CHAT: u32 = 1006;
const CMD_TTS: u32 = 1007;
const CMD_AUTO_LOGS: u32 = 1008;
const CMD_SHOW_CHAT: u32 = 1010;
const CMD_SETTINGS: u32 = 1011;
const CMD_CHAT_LOGS: u32 = 1012;
const CMD_LAUNCH_LOGS: u32 = 1013;
const CMD_APP_FOLDER: u32 = 1014;
const CMD_LOG_FILE: u32 = 1015;
const CMD_EXIT: u32 = 1099;

const CMD_TEST_BASE: u32 = 1200;
const CMD_VOLUME_BASE: u32 = 1300;
const CMD_ADAPTER_AUTO: u32 = 1400;
const CMD_ADAPTER_BASE: u32 = 1410;
const CMD_TAB_BASE: u32 = 2000;
const MAX_MENU_TABS: usize = 50;
const MAX_ADAPTERS: usize = 128;

#[derive(Clone, Debug)]
pub enum TrayAction {
    None,
    ToggleQueue,
    ToggleReady,
    ToggleInvite,
    ToggleRequest,
    ToggleDesktop,
    ToggleChat,
    ToggleTts,
    ToggleAutoLaunchLogs,
    ShowHideChat,
    SelectTab(usize),
    SetAlertVolume(i32),
    TestAlert(AlertKind),
    SelectAdapter(Option<String>),
    OpenSettings,
    OpenChatLogs,
    LaunchResonanceLogs,
    OpenAppFolder,
    OpenLogFile,
    Exit,
}

pub unsafe fn show(hwnd: HWND, settings: &AppSettings, api: &PcapApi) -> TrayAction {
    let menu = CreatePopupMenu();
    if menu.is_null() {
        return TrayAction::None;
    }

    append_check(menu, CMD_QUEUE, "Queue Pop Alert", settings.queue_pop_alert);
    append_check(menu, CMD_READY, "Ready Check Alert", settings.ready_check_alert);
    append_check(menu, CMD_INVITE, "Party Invite Alert", settings.party_invite_alert);
    append_check(menu, CMD_REQUEST, "Party Request Alert", settings.party_request_alert);
    append_check(menu, CMD_DESKTOP, "Desktop Notification", settings.desktop_notification);
    append_check(menu, CMD_CHAT, "Chat Overlay", settings.chat_overlay_enabled);
    append_check(menu, CMD_AUTO_LOGS, "Auto-launch Resonance Logs CN", settings.auto_launch_resonance_logs);

    let adapter_menu = CreatePopupMenu();
    let devices = api.devices().unwrap_or_default();
    if !adapter_menu.is_null() {
        append_check(
            adapter_menu,
            CMD_ADAPTER_AUTO,
            "Follow Resonance Logs CN / Auto",
            settings.npcap_device_name.trim().is_empty(),
        );
        AppendMenuW(adapter_menu, MF_SEPARATOR, 0, null());
        for (index, device) in devices.iter().take(MAX_ADAPTERS).enumerate() {
            let selected = !settings.npcap_device_name.trim().is_empty()
                && settings.npcap_device_name.eq_ignore_ascii_case(&device.name);
            append_check(
                adapter_menu,
                CMD_ADAPTER_BASE + index as u32,
                &shorten(&device.description, 64),
                selected,
            );
        }
        AppendMenuW(
            menu,
            MF_POPUP,
            adapter_menu as usize,
            wide(&adapter_label(settings, &devices)).as_ptr(),
        );
    }

    let volume_menu = CreatePopupMenu();
    if !volume_menu.is_null() {
        for step in 0..=10 {
            let volume = step * 10;
            append_check(
                volume_menu,
                CMD_VOLUME_BASE + step as u32,
                if volume == 0 { "Mute".to_string() } else { format!("{volume}%") }.as_str(),
                settings.alert_volume == volume,
            );
        }
        AppendMenuW(
            menu,
            MF_POPUP,
            volume_menu as usize,
            wide(&format!("Ready / Queue / Party Volume: {}%", settings.alert_volume.clamp(0, 100))).as_ptr(),
        );
    }

    let test_menu = CreatePopupMenu();
    if !test_menu.is_null() {
        append_string(test_menu, CMD_TEST_BASE, "Queue Pop");
        append_string(test_menu, CMD_TEST_BASE + 1, "Ready Check");
        append_string(test_menu, CMD_TEST_BASE + 2, "Party Invite");
        append_string(test_menu, CMD_TEST_BASE + 3, "Party Request");
        AppendMenuW(menu, MF_POPUP, test_menu as usize, wide("Test Alert Sounds").as_ptr());
    }

    AppendMenuW(menu, MF_SEPARATOR, 0, null());
    append_check(menu, CMD_TTS, "Chat TTS", settings.speech_translation.tts_enabled);
    append_string(menu, CMD_SHOW_CHAT, "Show / Hide Chat");

    let tabs = CreatePopupMenu();
    if !tabs.is_null() {
        for (index, tab) in settings.chat.tabs.iter().take(MAX_MENU_TABS).enumerate() {
            append_check(
                tabs,
                CMD_TAB_BASE + index as u32,
                &tab.name,
                tab.id == settings.chat.last_selected_tab_id,
            );
        }
        AppendMenuW(menu, MF_POPUP, tabs as usize, wide("Chat Tab").as_ptr());
    }

    AppendMenuW(menu, MF_SEPARATOR, 0, null());
    append_string(menu, CMD_SETTINGS, "Settings…");
    append_string(menu, CMD_LAUNCH_LOGS, "Launch Resonance Logs CN");
    append_string(menu, CMD_CHAT_LOGS, "Open Chat Logs");
    append_string(menu, CMD_APP_FOLDER, "Open App Data Folder");
    append_string(menu, CMD_LOG_FILE, "Open Log");
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

    match command as u32 {
        CMD_QUEUE => TrayAction::ToggleQueue,
        CMD_READY => TrayAction::ToggleReady,
        CMD_INVITE => TrayAction::ToggleInvite,
        CMD_REQUEST => TrayAction::ToggleRequest,
        CMD_DESKTOP => TrayAction::ToggleDesktop,
        CMD_CHAT => TrayAction::ToggleChat,
        CMD_TTS => TrayAction::ToggleTts,
        CMD_AUTO_LOGS => TrayAction::ToggleAutoLaunchLogs,
        CMD_SHOW_CHAT => TrayAction::ShowHideChat,
        CMD_SETTINGS => TrayAction::OpenSettings,
        CMD_CHAT_LOGS => TrayAction::OpenChatLogs,
        CMD_LAUNCH_LOGS => TrayAction::LaunchResonanceLogs,
        CMD_APP_FOLDER => TrayAction::OpenAppFolder,
        CMD_LOG_FILE => TrayAction::OpenLogFile,
        CMD_EXIT => TrayAction::Exit,
        CMD_ADAPTER_AUTO => TrayAction::SelectAdapter(None),
        id if id >= CMD_ADAPTER_BASE && id < CMD_ADAPTER_BASE + MAX_ADAPTERS as u32 => {
            devices
                .get((id - CMD_ADAPTER_BASE) as usize)
                .map(|d| TrayAction::SelectAdapter(Some(d.name.clone())))
                .unwrap_or(TrayAction::None)
        }
        id if id >= CMD_VOLUME_BASE && id <= CMD_VOLUME_BASE + 10 => {
            TrayAction::SetAlertVolume(((id - CMD_VOLUME_BASE) * 10) as i32)
        }
        CMD_TEST_BASE => TrayAction::TestAlert(AlertKind::Queue),
        id if id == CMD_TEST_BASE + 1 => TrayAction::TestAlert(AlertKind::Ready),
        id if id == CMD_TEST_BASE + 2 => TrayAction::TestAlert(AlertKind::PartyInvite),
        id if id == CMD_TEST_BASE + 3 => TrayAction::TestAlert(AlertKind::PartyRequest),
        id if id >= CMD_TAB_BASE && id < CMD_TAB_BASE + MAX_MENU_TABS as u32 => {
            TrayAction::SelectTab((id - CMD_TAB_BASE) as usize)
        }
        _ => TrayAction::None,
    }
}

unsafe fn append_check(menu: HMENU, id: u32, text: &str, checked: bool) {
    AppendMenuW(
        menu,
        MF_STRING | if checked { MF_CHECKED } else { 0 },
        id as usize,
        wide(text).as_ptr(),
    );
}

unsafe fn append_string(menu: HMENU, id: u32, text: &str) {
    AppendMenuW(menu, MF_STRING, id as usize, wide(text).as_ptr());
}

fn adapter_label(settings: &AppSettings, devices: &[crate::npcap::NpcapDevice]) -> String {
    if settings.npcap_device_name.trim().is_empty() {
        return "Network Adapter: Auto / Resonance Logs CN".into();
    }
    let description = devices
        .iter()
        .find(|d| d.name.eq_ignore_ascii_case(settings.npcap_device_name.trim()))
        .map(|d| d.description.as_str())
        .unwrap_or(settings.npcap_device_name.trim());
    format!("Network Adapter: {}", shorten(description, 38))
}

fn shorten(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    let head: String = value.chars().take(max.saturating_sub(3)).collect();
    format!("{head}...")
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}
