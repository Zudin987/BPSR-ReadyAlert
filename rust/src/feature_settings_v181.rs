use crate::{logging, paths::AppPaths};
use std::{ffi::c_void, fs, io, path::PathBuf};

#[cfg(windows)]
use windows_sys::Win32::Foundation::RECT;

#[cfg(windows)]
const MONITOR_DEFAULTTONEAREST: u32 = 2;

#[cfg(windows)]
#[repr(C)]
struct MonitorInfo {
    cb_size: u32,
    rc_monitor: RECT,
    rc_work: RECT,
    flags: u32,
}

#[cfg(windows)]
#[link(name = "user32")]
extern "system" {
    fn MonitorFromRect(rect: *const RECT, flags: u32) -> *mut c_void;
    fn GetMonitorInfoW(monitor: *mut c_void, info: *mut MonitorInfo) -> i32;
}

mod legacy {
    include!("feature_settings_v170.rs");
}

// Re-export the protocol ids, settings types and formatting helpers used by the
// existing telemetry and overlay modules. The explicit load/save functions below
// shadow the legacy persistence helpers with crash-recovery aware versions.
pub use legacy::*;

fn feature_path(paths: &AppPaths) -> PathBuf {
    paths.root.join("features.json")
}

pub fn load(paths: &AppPaths) -> FeatureSettings {
    let primary = feature_path(paths);
    let backup = primary.with_extension("json.bak");
    let temp = primary.with_extension("json.new");

    for (candidate, label) in [(&primary, "primary"), (&backup, "backup"), (&temp, "pending") ] {
        let Ok(text) = fs::read_to_string(candidate) else { continue; };
        match serde_json::from_str::<FeatureSettings>(&text) {
            Ok(mut value) => {
                value.normalize();
                let moved = recover_overlay_bounds(&mut value);
                if label != "primary" {
                    logging::write(format!("feature settings: recovered from {label}"));
                }
                if label != "primary" || moved {
                    if let Err(err) = save(paths, &value) {
                        logging::write(format!("feature settings: recovery save failed: {err}"));
                    }
                }
                return value;
            }
            Err(err) => logging::write(format!(
                "feature settings: load failed {}: {err}",
                candidate.display()
            )),
        }
    }

    let mut value = FeatureSettings::default();
    value.normalize();
    recover_overlay_bounds(&mut value);
    if let Err(err) = save(paths, &value) {
        logging::write(format!("feature settings: default save failed: {err}"));
    }
    value
}

pub fn save(paths: &AppPaths, settings: &FeatureSettings) -> io::Result<()> {
    let mut value = settings.clone();
    value.normalize();
    recover_overlay_bounds(&mut value);

    let primary = feature_path(paths);
    let backup = primary.with_extension("json.bak");
    let temp = primary.with_extension("json.new");
    let json = serde_json::to_string_pretty(&value).map_err(io::Error::other)?;

    fs::write(&temp, json.as_bytes())?;
    // Verify the exact bytes written before replacing the last known-good file.
    let _: FeatureSettings = serde_json::from_slice(&fs::read(&temp)?).map_err(io::Error::other)?;

    if primary.exists() {
        // Never overwrite a good backup with a corrupt primary recovered on the
        // previous launch. Only rotate a primary that is itself parseable.
        let primary_is_valid = fs::read_to_string(&primary)
            .ok()
            .and_then(|text| serde_json::from_str::<FeatureSettings>(&text).ok())
            .is_some();
        if primary_is_valid {
            fs::copy(&primary, &backup)?;
        }
        fs::remove_file(&primary)?;
    }
    fs::rename(&temp, &primary)?;
    Ok(())
}

fn recover_overlay_bounds(settings: &mut FeatureSettings) -> bool {
    let mut changed = false;
    changed |= recover_layout(&mut settings.dps);
    changed |= recover_layout(&mut settings.mechanics);
    changed
}

#[cfg(windows)]
fn recover_layout(layout: &mut OverlayLayout) -> bool {
    if layout.x == i32::MIN || layout.y == i32::MIN {
        return false;
    }

    unsafe {
        let rect = RECT {
            left: layout.x,
            top: layout.y,
            right: layout.x.saturating_add(layout.width),
            bottom: layout.y.saturating_add(layout.height),
        };
        let monitor = MonitorFromRect(&rect, MONITOR_DEFAULTTONEAREST);
        if monitor.is_null() {
            return false;
        }

        let mut info = MonitorInfo {
            cb_size: std::mem::size_of::<MonitorInfo>() as u32,
            rc_monitor: std::mem::zeroed(),
            rc_work: std::mem::zeroed(),
            flags: 0,
        };
        if GetMonitorInfoW(monitor, &mut info) == 0 {
            return false;
        }

        let work = info.rc_work;
        let work_width = (work.right - work.left).max(1);
        let work_height = (work.bottom - work.top).max(1);
        let old = (layout.x, layout.y, layout.width, layout.height);

        layout.width = layout.width.min(work_width);
        layout.height = layout.height.min(work_height);
        let max_x = (work.right - layout.width).max(work.left);
        let max_y = (work.bottom - layout.height).max(work.top);
        layout.x = layout.x.clamp(work.left, max_x);
        layout.y = layout.y.clamp(work.top, max_y);

        old != (layout.x, layout.y, layout.width, layout.height)
    }
}

#[cfg(not(windows))]
fn recover_layout(_layout: &mut OverlayLayout) -> bool {
    false
}
