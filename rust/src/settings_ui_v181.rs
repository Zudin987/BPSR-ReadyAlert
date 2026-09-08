use crate::{paths::AppPaths, settings::AppSettings};
use std::{ffi::c_void, ptr::{null, null_mut}, sync::{Arc, RwLock}};
use windows_sys::Win32::{
    Foundation::{HINSTANCE, HWND, LPARAM, POINT, RECT},
    UI::WindowsAndMessaging::{
        FindWindowW, GetClientRect, GetWindowRect, SetWindowPos,
        SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOZORDER,
    },
};

mod legacy {
    include!("settings_ui.rs");
}

const SETTINGS_CLASS: &str = "BPSRReadyAlertRustSettingsV151";
const NORMAL_OUTER_HEIGHT: i32 = 710;
const WORK_MARGIN: i32 = 8;
const MONITOR_DEFAULTTONEAREST: u32 = 2;

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
    fit_settings_window();
    Ok(())
}

unsafe fn fit_settings_window() {
    let class = wide(SETTINGS_CLASS);
    let hwnd = FindWindowW(class.as_ptr(), null());
    if hwnd.is_null() {
        return;
    }

    let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
    if monitor.is_null() {
        return;
    }
    let mut info = MonitorInfo {
        cb_size: std::mem::size_of::<MonitorInfo>() as u32,
        rc_monitor: std::mem::zeroed(),
        rc_work: std::mem::zeroed(),
        flags: 0,
    };
    if GetMonitorInfoW(monitor, &mut info) == 0 {
        return;
    }

    let work_height = (info.rc_work.bottom - info.rc_work.top).max(1);
    let target_outer_height = NORMAL_OUTER_HEIGHT.min((work_height - WORK_MARGIN).max(560));

    let mut before_window: RECT = std::mem::zeroed();
    let mut before_client: RECT = std::mem::zeroed();
    if GetWindowRect(hwnd, &mut before_window) == 0 || GetClientRect(hwnd, &mut before_client) == 0 {
        return;
    }
    let old_outer_height = before_window.bottom - before_window.top;
    if (old_outer_height - target_outer_height).abs() <= 1 {
        return;
    }
    let old_client_height = (before_client.bottom - before_client.top).max(1);
    let width = (before_window.right - before_window.left).max(1);

    SetWindowPos(
        hwnd,
        null_mut(),
        0,
        0,
        width,
        target_outer_height,
        SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
    );

    let mut after_client: RECT = std::mem::zeroed();
    if GetClientRect(hwnd, &mut after_client) == 0 {
        return;
    }
    let new_client_height = (after_client.bottom - after_client.top).max(1);
    let ratio_y = new_client_height as f64 / old_client_height as f64;
    if (ratio_y - 1.0).abs() < 0.005 {
        return;
    }

    let mut scale = ScaleChildren { parent: hwnd, ratio_y };
    EnumChildWindows(
        hwnd,
        Some(scale_child),
        (&mut scale as *mut ScaleChildren) as LPARAM,
    );
}

unsafe extern "system" fn scale_child(child: HWND, lparam: LPARAM) -> i32 {
    let scale = &*(lparam as *const ScaleChildren);
    let mut rect: RECT = std::mem::zeroed();
    if GetWindowRect(child, &mut rect) == 0 {
        return 1;
    }

    let mut top_left = POINT { x: rect.left, y: rect.top };
    if ScreenToClient(scale.parent, &mut top_left) == 0 {
        return 1;
    }
    let width = (rect.right - rect.left).max(1);
    let height = (rect.bottom - rect.top).max(1);
    let new_y = (top_left.y as f64 * scale.ratio_y).round() as i32;

    SetWindowPos(
        child,
        null_mut(),
        top_left.x,
        new_y,
        width,
        height,
        SWP_NOZORDER | SWP_NOACTIVATE,
    );
    1
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
