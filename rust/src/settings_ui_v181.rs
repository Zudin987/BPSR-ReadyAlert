use crate::{paths::AppPaths, settings::AppSettings};
use std::{ffi::c_void, ptr::{null, null_mut}, sync::{Arc, RwLock}};
use windows_sys::Win32::{
    Foundation::{HINSTANCE, HWND, LPARAM, POINT, RECT},
    UI::WindowsAndMessaging::{
        FindWindowW, GetClientRect, GetDlgItem, GetWindowRect, GetWindowTextLengthW, GetWindowTextW,
        SetWindowPos, SetWindowTextW,
        SWP_NOACTIVATE, SWP_NOZORDER,
    },
};

mod legacy {
    include!("settings_ui.rs");
}

const SETTINGS_CLASS: &str = "BPSRReadyAlertRustSettingsV151";
const WORK_MARGIN: i32 = 8;
const MONITOR_DEFAULTTONEAREST: u32 = 2;
const NAV_OVERLAY: i32 = 3001;
const NAV_COLORS: i32 = 3002;

#[repr(C)]
struct MonitorInfo {
    cb_size: u32,
    rc_monitor: RECT,
    rc_work: RECT,
    flags: u32,
}

#[repr(C)]
struct ScaleChildren {
    parent: HWND,
    ratio_y: f64,
}

#[link(name = "user32")]
extern "system" {
    fn MonitorFromWindow(hwnd: HWND, flags: u32) -> *mut c_void;
    fn GetMonitorInfoW(monitor: *mut c_void, info: *mut MonitorInfo) -> i32;
    fn EnumChildWindows(
        parent: HWND,
        callback: Option<unsafe extern "system" fn(HWND, LPARAM) -> i32>,
        lparam: LPARAM,
    ) -> i32;
    fn ScreenToClient(hwnd: HWND, point: *mut POINT) -> i32;
}

pub unsafe fn show(
    instance: HINSTANCE,
    main_hwnd: HWND,
    settings: Arc<RwLock<AppSettings>>,
    paths: AppPaths,
    initial_page: usize,
    add_tab: bool,
) -> Result<(), String> {
    legacy::show(instance, main_hwnd, settings, paths, initial_page, add_tab)?;
    polish_settings_copy();
    fit_settings_window();
    Ok(())
}

/// The original settings window was sized for a desktop work area and could
/// still extend below a 720p taskbar because it enforced a 560px minimum after
/// subtracting work-area margins. Clamp to the *actual* monitor work area and
/// recenter it. At common 1280x720 this is only a small vertical compression.
unsafe fn fit_settings_window() {
    let class = wide(SETTINGS_CLASS);
    let hwnd = FindWindowW(class.as_ptr(), null());
    if hwnd.is_null() { return; }

    let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
    if monitor.is_null() { return; }
    let mut info = MonitorInfo {
        cb_size: std::mem::size_of::<MonitorInfo>() as u32,
        rc_monitor: std::mem::zeroed(),
        rc_work: std::mem::zeroed(),
        flags: 0,
    };
    if GetMonitorInfoW(monitor, &mut info) == 0 { return; }

    let mut before_window: RECT = std::mem::zeroed();
    let mut before_client: RECT = std::mem::zeroed();
    if GetWindowRect(hwnd, &mut before_window) == 0 || GetClientRect(hwnd, &mut before_client) == 0 { return; }

    let old_w = (before_window.right - before_window.left).max(1);
    let old_h = (before_window.bottom - before_window.top).max(1);
    let old_client_h = (before_client.bottom - before_client.top).max(1);
    let work_w = (info.rc_work.right - info.rc_work.left).max(1);
    let work_h = (info.rc_work.bottom - info.rc_work.top).max(1);
    let max_w = (work_w - WORK_MARGIN * 2).max(320);
    let max_h = (work_h - WORK_MARGIN * 2).max(260);
    let target_w = old_w.min(max_w);
    let target_h = old_h.min(max_h);
    let x = info.rc_work.left + ((work_w - target_w) / 2).max(0);
    let y = info.rc_work.top + ((work_h - target_h) / 2).max(0);

    SetWindowPos(hwnd, null_mut(), x, y, target_w, target_h, SWP_NOZORDER | SWP_NOACTIVATE);

    // Preserve the existing Win32 control sizes; only compress vertical
    // positions when the monitor forced the window shorter. This avoids the
    // clipped bottom action row without making fonts/buttons microscopically small.
    let mut after_client: RECT = std::mem::zeroed();
    if GetClientRect(hwnd, &mut after_client) == 0 { return; }
    let new_client_h = (after_client.bottom - after_client.top).max(1);
    let ratio_y = (new_client_h as f64 / old_client_h as f64).min(1.0);
    if ratio_y < 0.995 {
        let mut scale = ScaleChildren { parent: hwnd, ratio_y };
        EnumChildWindows(hwnd, Some(scale_child), (&mut scale as *mut ScaleChildren) as LPARAM);
    }
}

/// Clarify that these pages configure Chat, not DPS/Mechanics, and update the
/// click-through recovery copy after v1.14 reserves Ctrl+Shift+F10 for combat.
unsafe fn polish_settings_copy() {
    let class = wide(SETTINGS_CLASS);
    let hwnd = FindWindowW(class.as_ptr(), null());
    if hwnd.is_null() { return; }
    let overlay = GetDlgItem(hwnd, NAV_OVERLAY);
    if !overlay.is_null() { SetWindowTextW(overlay, wide("Chat overlay").as_ptr()); }
    let colors = GetDlgItem(hwnd, NAV_COLORS);
    if !colors.is_null() { SetWindowTextW(colors, wide("Chat colors").as_ptr()); }
    EnumChildWindows(hwnd, Some(rewrite_help_copy), 0);
}

unsafe extern "system" fn rewrite_help_copy(child: HWND, _lparam: LPARAM) -> i32 {
    let len = GetWindowTextLengthW(child);
    if len <= 0 || len > 512 { return 1; }
    let mut buf = vec![0u16; len as usize + 1];
    let got = GetWindowTextW(child, buf.as_mut_ptr(), buf.len() as i32);
    if got <= 0 { return 1; }
    let text = String::from_utf16_lossy(&buf[..got as usize]);
    if text.contains("Ctrl+Shift+F10 recovers click-through") {
        let replacement = text.replace("Ctrl+Shift+F10 recovers click-through", "Ctrl+Shift+F9 recovers click-through");
        SetWindowTextW(child, wide(&replacement).as_ptr());
    }
    1
}

unsafe extern "system" fn scale_child(child: HWND, lparam: LPARAM) -> i32 {
    let scale = &*(lparam as *const ScaleChildren);
    let mut rect: RECT = std::mem::zeroed();
    if GetWindowRect(child, &mut rect) == 0 { return 1; }

    let mut top_left = POINT { x: rect.left, y: rect.top };
    if ScreenToClient(scale.parent, &mut top_left) == 0 { return 1; }
    let width = (rect.right - rect.left).max(1);
    let height = (rect.bottom - rect.top).max(1);
    let new_y = (top_left.y as f64 * scale.ratio_y).round() as i32;

    SetWindowPos(child, null_mut(), top_left.x, new_y, width, height, SWP_NOZORDER | SWP_NOACTIVATE);
    1
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
