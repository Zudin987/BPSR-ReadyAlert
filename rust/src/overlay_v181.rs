use crate::{logging, paths::AppPaths, settings::{self, AppSettings}};
use std::{ffi::c_void, sync::{Arc, RwLock}};
use windows_sys::Win32::Foundation::{HINSTANCE, HWND, RECT};

mod legacy {
    include!("overlay_v150.rs");
}

pub use legacy::{apply_style, expand_if_collapsed, push_chat, refresh, set_translation};

const MONITOR_DEFAULTTONEAREST: u32 = 2;

#[repr(C)]
struct MonitorInfo {
    cb_size: u32,
    rc_monitor: RECT,
    rc_work: RECT,
    flags: u32,
}

#[link(name = "user32")]
extern "system" {
    fn MonitorFromRect(rect: *const RECT, flags: u32) -> *mut c_void;
    fn GetMonitorInfoW(monitor: *mut c_void, info: *mut MonitorInfo) -> i32;
}

pub unsafe fn create(
    instance: HINSTANCE,
    settings_arc: Arc<RwLock<AppSettings>>,
    main_hwnd: HWND,
    paths: AppPaths,
) -> Result<HWND, String> {
    let recovered = if let Ok(mut settings) = settings_arc.write() {
        let changed = recover_chat_bounds(&mut settings);
        changed.then(|| settings.clone())
    } else {
        None
    };

    if let Some(snapshot) = recovered {
        if let Err(err) = settings::save(&paths, &snapshot) {
            logging::write(format!("chat overlay: recovered bounds save failed: {err}"));
        } else {
            logging::write("chat overlay: recovered saved window onto an active monitor");
        }
    }

    let hwnd=legacy::create(instance, settings_arc, main_hwnd, paths)?;
    crate::ui::fit_window(hwnd,hwnd,false);
    Ok(hwnd)
}

fn recover_chat_bounds(settings: &mut AppSettings) -> bool {
    let chat = &mut settings.chat;
    if chat.window_x == i32::MIN || chat.window_y == i32::MIN {
        return false;
    }

    let rect = RECT {
        left: chat.window_x,
        top: chat.window_y,
        right: chat.window_x.saturating_add(chat.window_width),
        bottom: chat.window_y.saturating_add(chat.window_height),
    };
    let monitor = unsafe { MonitorFromRect(&rect, MONITOR_DEFAULTTONEAREST) };
    if monitor.is_null() {
        return false;
    }

    let mut info = MonitorInfo {
        cb_size: std::mem::size_of::<MonitorInfo>() as u32,
        rc_monitor: unsafe { std::mem::zeroed() },
        rc_work: unsafe { std::mem::zeroed() },
        flags: 0,
    };
    if unsafe { GetMonitorInfoW(monitor, &mut info) } == 0 {
        return false;
    }

    let work = info.rc_work;
    let work_width = (work.right - work.left).max(1);
    let work_height = (work.bottom - work.top).max(1);
    let old = (chat.window_x, chat.window_y, chat.window_width, chat.window_height);

    // Keep the normal minimums where the monitor can accommodate them, but never
    // leave a previously valid overlay larger than the current work area.
    chat.window_width = chat.window_width.min(work_width);
    chat.window_height = chat.window_height.min(work_height);
    let max_x = (work.right - chat.window_width).max(work.left);
    let max_y = (work.bottom - chat.window_height).max(work.top);
    chat.window_x = chat.window_x.clamp(work.left, max_x);
    chat.window_y = chat.window_y.clamp(work.top, max_y);

    old != (chat.window_x, chat.window_y, chat.window_width, chat.window_height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_placement_is_left_to_windows() {
        let mut settings = AppSettings::default();
        assert!(!recover_chat_bounds(&mut settings));
        assert_eq!(settings.chat.window_x, i32::MIN);
        assert_eq!(settings.chat.window_y, i32::MIN);
    }
}
