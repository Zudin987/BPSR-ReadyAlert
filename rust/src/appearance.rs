//! Shared, reversible visual theme for native windows and local HTML reports.
//! Mapping happens when painting: changing the theme never changes combat data.
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex, OnceLock,
    },
};
use windows_sys::Win32::Graphics::Gdi as gdi;
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, RECT},
    Graphics::Gdi::{HBRUSH, HDC, LOGBRUSH},
    UI::WindowsAndMessaging,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VisualTheme {
    #[default]
    Notmeter,
    #[serde(other)]
    Classic,
}

static NOTMETER: AtomicBool = AtomicBool::new(true);
pub fn is_notmeter() -> bool {
    NOTMETER.load(Ordering::Relaxed)
}
pub fn set_theme(theme: VisualTheme) {
    NOTMETER.store(theme == VisualTheme::Notmeter, Ordering::Relaxed);
}

pub const fn rgb(r: u32, g: u32, b: u32) -> u32 {
    r | (g << 8) | (b << 16)
}
pub const BACKGROUND: u32 = rgb(29, 32, 31);
pub const SIDEBAR: u32 = rgb(23, 27, 26);
pub const SURFACE: u32 = rgb(36, 41, 38);
pub const RAISED: u32 = rgb(44, 51, 46);
pub const BORDER: u32 = rgb(64, 75, 64);
pub const ACCENT: u32 = rgb(190, 205, 167);
pub const TEXT: u32 = rgb(237, 240, 234);
pub const SECONDARY: u32 = rgb(192, 202, 190);
pub const MUTED: u32 = rgb(147, 160, 145);

/// Translate the application's neutral surfaces and interaction accents only.
/// Damage/healing semantics, user chat colors and specialization colors stay intact.
pub fn color(value: u32) -> u32 {
    if !is_notmeter() {
        return value;
    }
    notmeter_color(value)
}

fn notmeter_color(value: u32) -> u32 {
    if matches!(
        value,
        BACKGROUND | SIDEBAR | SURFACE | RAISED | BORDER | ACCENT | TEXT | SECONDARY | MUTED
    ) {
        return value;
    }
    let r = value & 255;
    let g = (value >> 8) & 255;
    let b = (value >> 16) & 255;
    // Existing UI uses several generations of neutral blue/gray tokens. Keep
    // the tonal hierarchy while avoiding a list tied to one generated patch.
    if r <= 80 && g <= 90 && b <= 100 && b >= r && b - r <= 38 {
        let level = (r * 299 + g * 587 + b * 114) / 1000;
        return match level {
            0..=15 => SIDEBAR,
            16..=25 => BACKGROUND,
            26..=36 => SURFACE,
            37..=53 => RAISED,
            _ => BORDER,
        };
    }
    match (r, g, b) {
        (168, 199, 250) | (117, 211, 236) | (99, 199, 255) | (159, 198, 252) => ACCENT,
        (188, 212, 252) | (143, 224, 244) => rgb(208, 222, 186),
        (139, 177, 236) | (83, 176, 204) => rgb(164, 184, 139),
        (24, 42, 66) | (14, 31, 38) => SIDEBAR,
        (232, 234, 242) | (239, 246, 248) | (241, 243, 247) => TEXT,
        (194, 198, 210) | (200, 214, 219) | (190, 200, 214) => SECONDARY,
        (144, 149, 164) | (156, 176, 183) => MUTED,
        (104, 109, 122) | (121, 139, 145) => rgb(110, 121, 108),
        (77, 107, 155) | (93, 115, 125) => rgb(119, 142, 100),
        _ => value,
    }
}

/// Preserve the original owned brush, using a borrowed theme brush for drawing.
/// Cached brushes are keyed by color, so switching back remains reversible.
pub unsafe fn themed_brush(brush: HBRUSH) -> HBRUSH {
    if !is_notmeter() || brush.is_null() {
        return brush;
    }
    let mut info: LOGBRUSH = std::mem::zeroed();
    if gdi::GetObjectW(
        brush,
        std::mem::size_of::<LOGBRUSH>() as i32,
        (&mut info as *mut LOGBRUSH).cast(),
    ) == 0
        || info.lbStyle != 0
    {
        return brush;
    }
    let mapped = color(info.lbColor);
    if mapped == info.lbColor {
        return brush;
    }
    static BRUSHES: OnceLock<Mutex<HashMap<u32, usize>>> = OnceLock::new();
    let Ok(mut brushes) = BRUSHES.get_or_init(|| Mutex::new(HashMap::new())).lock() else {
        return brush;
    };
    *brushes
        .entry(mapped)
        .or_insert_with(|| gdi::CreateSolidBrush(mapped) as usize) as HBRUSH
}

pub unsafe fn fill_rect(hdc: HDC, rect: *const RECT, brush: HBRUSH) -> i32 {
    gdi::FillRect(hdc, rect, themed_brush(brush))
}
pub unsafe fn set_text_color(hdc: HDC, value: u32) -> u32 {
    gdi::SetTextColor(hdc, color(value))
}
pub unsafe fn set_bk_color(hdc: HDC, value: u32) -> u32 {
    gdi::SetBkColor(hdc, color(value))
}

pub fn gauge_extent(width: i32, amount: i64, maximum: i64) -> i32 {
    if width <= 0 || amount <= 0 || maximum <= 0 {
        return 0;
    }
    ((width as u128 * amount.min(maximum) as u128) / maximum as u128) as i32
}

/// Class gauges have their own capsule geometry, separate from control corners.
pub unsafe fn capsule(hdc: HDC, rect: RECT, value: u32) {
    if rect.right <= rect.left || rect.bottom <= rect.top {
        return;
    }
    let brush = gdi::CreateSolidBrush(value);
    if brush.is_null() {
        return;
    }
    let old_brush = gdi::SelectObject(hdc, brush);
    let old_pen = gdi::SelectObject(hdc, gdi::GetStockObject(8));
    let diameter = (rect.bottom - rect.top).min(rect.right - rect.left);
    gdi::RoundRect(
        hdc,
        rect.left,
        rect.top,
        rect.right,
        rect.bottom,
        diameter,
        diameter,
    );
    gdi::SelectObject(hdc, old_pen);
    gdi::SelectObject(hdc, old_brush);
    gdi::DeleteObject(brush);
}

pub fn class_gauge_color(accent: u32) -> u32 {
    let channel = |shift: u32| {
        let base = (SURFACE >> shift) & 255u32;
        let hue = (accent >> shift) & 255u32;
        (base * 55 + hue * 45) / 100
    };
    rgb(channel(0), channel(8), channel(16))
}

pub unsafe fn redraw_thread_windows() {
    unsafe extern "system" fn redraw(hwnd: HWND, _: LPARAM) -> i32 {
        gdi::RedrawWindow(hwnd, std::ptr::null(), std::ptr::null_mut(), 0x0485);
        1
    }
    WindowsAndMessaging::EnumThreadWindows(
        windows_sys::Win32::System::Threading::GetCurrentThreadId(),
        Some(redraw),
        0,
    );
}

pub fn upgrade_archive(path: &Path) -> Result<(), String> {
    if !is_notmeter() {
        return Ok(());
    }
    let html = fs::read_to_string(path).map_err(|err| format!("read archive appearance: {err}"))?;
    if html.matches("</body>").count() != 1 {
        return Err("archive appearance expected one closing body".into());
    }
    let enhancement = if html.contains("function renderPlayerList(") {
        format!(
            "<script>{}</script>\n",
            include_str!("appearance_archive.js")
        )
    } else {
        String::new()
    };
    let style = format!(
        "<style id=\"readyalert-notmeter\">{}</style>\n{enhancement}",
        include_str!("appearance.css")
    );
    fs::write(path, html.replacen("</body>", &(style + "</body>"), 1))
        .map_err(|err| format!("write archive appearance: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn palette_preserves_class_and_combat_colors() {
        for value in [
            rgb(242, 96, 103),
            rgb(85, 207, 139),
            rgb(157, 107, 219),
            rgb(48, 197, 188),
        ] {
            assert_eq!(notmeter_color(value), value);
        }
        assert_ne!(notmeter_color(rgb(17, 19, 24)), rgb(17, 19, 24));
        assert_eq!(notmeter_color(rgb(168, 199, 250)), ACCENT);
    }
    #[test]
    fn themes_round_trip_and_old_settings_have_a_theme() {
        for theme in [VisualTheme::Classic, VisualTheme::Notmeter] {
            let json = serde_json::to_string(&theme).unwrap();
            assert_eq!(serde_json::from_str::<VisualTheme>(&json).unwrap(), theme);
        }
        let old: crate::feature_settings::FeatureSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(old.visual_theme, VisualTheme::Notmeter);
    }
    #[test]
    fn gauges_handle_zero_negative_and_extreme_values_without_overflow() {
        for width in [0, 1, 40, 200, 1200] {
            assert_eq!(gauge_extent(width, 0, 0), 0);
            assert_eq!(gauge_extent(width, -1, 100), 0);
            assert_eq!(gauge_extent(width, i64::MAX, i64::MAX), width);
            assert_eq!(gauge_extent(width, 200, 100), width);
            assert_eq!(gauge_extent(width, 50, 100), width / 2);
        }
    }
    #[test]
    fn theme_tokens_are_idempotent_across_nested_drawing_helpers() {
        for token in [
            BACKGROUND, SIDEBAR, SURFACE, RAISED, BORDER, ACCENT, TEXT, SECONDARY, MUTED,
        ] {
            assert_eq!(notmeter_color(token), token);
        }
    }
}
