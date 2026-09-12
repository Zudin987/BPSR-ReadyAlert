//! Final lightweight native drawing helpers for ReadyAlert's v1.29 UI pass.
//!
//! This deliberately stays on GDI + Win32.  It only custom-draws controls where
//! stock dark theming cannot express the compact tonal/rounded states cleanly.
use std::{
    ptr::null,
    sync::{atomic::{AtomicIsize, Ordering}, OnceLock},
};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
    Graphics::Gdi::{
        CreateFontW, CreatePen, CreateSolidBrush, DeleteObject, DrawTextW, GetDC,
        GetStockObject, InvalidateRect, ReleaseDC, RoundRect, SelectObject, SetBkMode,
        SetTextColor, HDC, HFONT, TRANSPARENT,
    },
    UI::{
        Controls::DRAWITEMSTRUCT,
        WindowsAndMessaging::{
            GetClassNameW, GetWindowTextLengthW, GetWindowTextW, SendMessageW,
        },
    },
};

pub const RADIUS_SMALL: i32 = 6;
pub const RADIUS_MEDIUM: i32 = 9;
pub const RADIUS_LARGE: i32 = 12;
pub const SELECTED_SURFACE: u32 = crate::ui_theme::rgb(28, 57, 54);
pub const SELECTED_SURFACE_HOVER: u32 = crate::ui_theme::rgb(34, 68, 64);
pub const PANEL_SURFACE: u32 = crate::ui_theme::rgb(20, 26, 32);
pub const PANEL_RAISED: u32 = crate::ui_theme::rgb(25, 32, 39);

const WM_PAINT_: u32 = 0x000F;
const WM_SETFOCUS_: u32 = 0x0007;
const WM_KILLFOCUS_: u32 = 0x0008;
const WM_ENABLE_: u32 = 0x000A;
const WM_MOUSEMOVE_: u32 = 0x0200;
const WM_MOUSELEAVE_: u32 = 0x02A3;
const EM_SETMARGINS_: u32 = 0x00D3;
const EC_LEFTMARGIN_: usize = 0x0001;
const EC_RIGHTMARGIN_: usize = 0x0002;
const NULL_BRUSH_STOCK: i32 = 5;
const NULL_PEN_STOCK: i32 = 8;
const PS_SOLID_: i32 = 0;
const BUTTON_SUBCLASS_ID: usize = 0x5241_4254;
const FIELD_SUBCLASS_ID: usize = 0x5241_4644;
const TME_LEAVE_: u32 = 0x0000_0002;

static HOVERED_CONTROL: AtomicIsize = AtomicIsize::new(0);
static FOCUSED_FIELD: AtomicIsize = AtomicIsize::new(0);

#[repr(C)]
struct NativeTrackMouseEvent {
    cb_size: u32,
    flags: u32,
    hwnd_track: HWND,
    hover_time: u32,
}

type SubclassProc =
    Option<unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM, usize, usize) -> LRESULT>;

#[link(name = "comctl32")]
extern "system" {
    fn SetWindowSubclass(hwnd: HWND, proc: SubclassProc, id: usize, data: usize) -> i32;
    fn DefSubclassProc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT;
}

#[link(name = "user32")]
extern "system" {
    fn TrackMouseEvent(event: *mut NativeTrackMouseEvent) -> i32;
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn make_font(height: i32, weight: i32) -> usize {
    unsafe {
        let face = wide("Segoe UI Variable Text");
        CreateFontW(height, 0, 0, 0, weight, 0, 0, 0, 1, 0, 0, 5, 0, face.as_ptr())
            as usize
    }
}

fn body_font() -> HFONT {
    static FONT: OnceLock<usize> = OnceLock::new();
    *FONT.get_or_init(|| make_font(-14, 400)) as HFONT
}

fn medium_font() -> HFONT {
    static FONT: OnceLock<usize> = OnceLock::new();
    *FONT.get_or_init(|| make_font(-14, 500)) as HFONT
}

pub unsafe fn fill_round_rect(hdc: HDC, rect: RECT, color: u32, radius: i32) {
    if rect.right <= rect.left || rect.bottom <= rect.top {
        return;
    }
    let brush = CreateSolidBrush(color);
    if brush.is_null() {
        return;
    }
    let old_brush = SelectObject(hdc, brush as _);
    let old_pen = SelectObject(hdc, GetStockObject(NULL_PEN_STOCK));
    let diameter = radius.max(1) * 2;
    RoundRect(
        hdc,
        rect.left,
        rect.top,
        rect.right,
        rect.bottom,
        diameter,
        diameter,
    );
    SelectObject(hdc, old_pen);
    SelectObject(hdc, old_brush);
    DeleteObject(brush);
}

pub unsafe fn stroke_round_rect(
    hdc: HDC,
    rect: RECT,
    color: u32,
    radius: i32,
    width: i32,
) {
    if rect.right <= rect.left || rect.bottom <= rect.top {
        return;
    }
    let pen = CreatePen(PS_SOLID_, width.max(1), color);
    if pen.is_null() {
        return;
    }
    let old_pen = SelectObject(hdc, pen as _);
    let old_brush = SelectObject(hdc, GetStockObject(NULL_BRUSH_STOCK));
    let inset = width.max(1) / 2;
    let diameter = radius.max(1) * 2;
    RoundRect(
        hdc,
        rect.left + inset,
        rect.top + inset,
        rect.right - inset,
        rect.bottom - inset,
        diameter,
        diameter,
    );
    SelectObject(hdc, old_brush);
    SelectObject(hdc, old_pen);
    DeleteObject(pen);
}

pub unsafe fn paint_panel(hdc: HDC, rect: RECT, elevated: bool) {
    fill_round_rect(
        hdc,
        rect,
        if elevated { PANEL_RAISED } else { PANEL_SURFACE },
        RADIUS_LARGE,
    );
}

unsafe fn draw_control_text(
    hdc: HDC,
    hwnd: HWND,
    mut rect: RECT,
    color: u32,
    align_left: bool,
    medium: bool,
) {
    let len = GetWindowTextLengthW(hwnd).max(0) as usize;
    let mut buf = vec![0u16; len + 1];
    let got = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32).max(0) as usize;
    SetBkMode(hdc, TRANSPARENT as i32);
    SetTextColor(hdc, color);
    SelectObject(hdc, if medium { medium_font() } else { body_font() });
    if align_left {
        rect.left += 12;
    }
    let flags = (if align_left { 0 } else { 0x0001 }) | 0x0004 | 0x0020 | 0x0800 | 0x8000;
    DrawTextW(hdc, buf.as_ptr(), got as i32, &mut rect, flags);
}

pub unsafe fn draw_button(
    item: *const DRAWITEMSTRUCT,
    selected: bool,
    primary: bool,
    danger: bool,
    nav: bool,
) -> isize {
    if item.is_null() {
        return 0;
    }
    let item = &*item;
    let disabled = item.itemState & 0x0004 != 0;
    let pressed = item.itemState & 0x0001 != 0;
    let focused = item.itemState & 0x0010 != 0;
    let hot =
        item.itemState & 0x0040 != 0 || HOVERED_CONTROL.load(Ordering::Acquire) == item.hwndItem as isize;

    let mut rect = item.rcItem;
    rect.left += 1;
    rect.top += if nav { 0 } else { 1 };
    rect.right -= 1;
    rect.bottom -= if nav { 0 } else { 1 };

    let background = if disabled {
        if nav { crate::ui_theme::SIDEBAR } else { crate::ui_theme::SURFACE }
    } else if nav && selected {
        if pressed {
            crate::ui_theme::SURFACE_PRESSED
        } else if hot {
            SELECTED_SURFACE_HOVER
        } else {
            SELECTED_SURFACE
        }
    } else if primary {
        if pressed {
            crate::ui_theme::ACCENT_PRESSED
        } else if hot {
            crate::ui_theme::ACCENT_HOVER
        } else {
            crate::ui_theme::ACCENT
        }
    } else if danger {
        if pressed {
            crate::ui_theme::DANGER
        } else if hot {
            crate::ui_theme::DANGER_HOVER
        } else {
            crate::ui_theme::rgb(71, 37, 41)
        }
    } else if pressed {
        crate::ui_theme::SURFACE_PRESSED
    } else if hot {
        crate::ui_theme::SURFACE_HOVER
    } else if nav {
        crate::ui_theme::SIDEBAR
    } else {
        crate::ui_theme::RAISED
    };

    if !nav || selected || hot {
        fill_round_rect(
            item.hDC,
            rect,
            background,
            if nav { RADIUS_MEDIUM } else { RADIUS_SMALL },
        );
    }

    if nav && selected {
        fill_round_rect(
            item.hDC,
            RECT {
                left: rect.left + 5,
                top: rect.top + 8,
                right: rect.left + 8,
                bottom: rect.bottom - 8,
            },
            crate::ui_theme::ACCENT,
            2,
        );
    } else if !nav {
        let border = if disabled {
            crate::ui_theme::BORDER
        } else if primary || selected || focused {
            crate::ui_theme::ACCENT
        } else if danger {
            crate::ui_theme::DANGER
        } else {
            crate::ui_theme::BORDER
        };
        stroke_round_rect(item.hDC, rect, border, RADIUS_SMALL, 1);
    }

    if focused && nav {
        stroke_round_rect(item.hDC, rect, crate::ui_theme::ACCENT, RADIUS_MEDIUM, 1);
    }

    let text_color = if disabled {
        crate::ui_theme::TEXT_DISABLED
    } else if primary {
        crate::ui_theme::ACCENT_TEXT
    } else {
        crate::ui_theme::TEXT
    };
    let mut text_rect = item.rcItem;
    if nav {
        text_rect.left += 16;
    }
    draw_control_text(
        item.hDC,
        item.hwndItem,
        text_rect,
        text_color,
        nav,
        nav && selected,
    );
    1
}

pub unsafe fn draw_combo(item: *const DRAWITEMSTRUCT) -> isize {
    if item.is_null() {
        return 0;
    }
    let item = &*item;
    let disabled = item.itemState & 0x0004 != 0;
    let selected = item.itemState & 0x0001 != 0;
    let focused = item.itemState & 0x0010 != 0;
    let mut rect = item.rcItem;
    rect.left += 1;
    rect.top += 1;
    rect.right -= 1;
    rect.bottom -= 1;

    fill_round_rect(
        item.hDC,
        rect,
        if disabled {
            crate::ui_theme::SURFACE
        } else if selected {
            SELECTED_SURFACE
        } else {
            crate::ui_theme::INPUT
        },
        RADIUS_SMALL,
    );
    stroke_round_rect(
        item.hDC,
        rect,
        if !disabled && focused {
            crate::ui_theme::ACCENT
        } else {
            crate::ui_theme::BORDER
        },
        RADIUS_SMALL,
        1,
    );

    let index = if item.itemID == u32::MAX {
        SendMessageW(item.hwndItem, 0x0147, 0, 0)
    } else {
        item.itemID as isize
    };
    if index >= 0 {
        let len = SendMessageW(item.hwndItem, 0x0149, index as usize, 0).max(0) as usize;
        let mut buf = vec![0u16; len + 1];
        let got =
            SendMessageW(item.hwndItem, 0x0148, index as usize, buf.as_mut_ptr() as isize).max(0)
                as usize;
        SetBkMode(item.hDC, TRANSPARENT as i32);
        SetTextColor(
            item.hDC,
            if disabled {
                crate::ui_theme::TEXT_DISABLED
            } else {
                crate::ui_theme::TEXT
            },
        );
        SelectObject(item.hDC, body_font());
        let mut text = item.rcItem;
        text.left += 10;
        text.right -= 8;
        DrawTextW(
            item.hDC,
            buf.as_ptr(),
            got as i32,
            &mut text,
            0x0004 | 0x0020 | 0x0800 | 0x8000,
        );
    }
    1
}

fn class_name(hwnd: HWND) -> String {
    unsafe {
        let mut class = [0u16; 32];
        let len = GetClassNameW(hwnd, class.as_mut_ptr(), class.len() as i32).max(0) as usize;
        String::from_utf16_lossy(&class[..len])
    }
}

unsafe fn track_leave(hwnd: HWND) {
    let mut tracking = NativeTrackMouseEvent {
        cb_size: std::mem::size_of::<NativeTrackMouseEvent>() as u32,
        flags: TME_LEAVE_,
        hwnd_track: hwnd,
        hover_time: 0,
    };
    let _ = TrackMouseEvent(&mut tracking);
}

unsafe fn install_button_tracking(hwnd: HWND) {
    let _ = SetWindowSubclass(hwnd, Some(button_subclass_proc), BUTTON_SUBCLASS_ID, 0);
}

unsafe fn install_field_tracking(hwnd: HWND) {
    let _ = SetWindowSubclass(hwnd, Some(field_subclass_proc), FIELD_SUBCLASS_ID, 0);
}

pub unsafe fn theme_control(hwnd: HWND) {
    if hwnd.is_null() {
        return;
    }
    crate::ui_theme::theme_control(hwnd);
    let class = class_name(hwnd);
    if class.eq_ignore_ascii_case("BUTTON") {
        install_button_tracking(hwnd);
    } else if class.eq_ignore_ascii_case("EDIT") {
        SendMessageW(
            hwnd,
            EM_SETMARGINS_,
            EC_LEFTMARGIN_ | EC_RIGHTMARGIN_,
            (8 | (8 << 16)) as isize,
        );
        install_field_tracking(hwnd);
    } else if class.eq_ignore_ascii_case("LISTBOX") {
        install_field_tracking(hwnd);
    }
}

pub unsafe fn theme_combo(hwnd: HWND) {
    if hwnd.is_null() {
        return;
    }
    crate::ui_theme::theme_combo(hwnd);
}

unsafe fn paint_field_border(hwnd: HWND) {
    let dc = GetDC(hwnd);
    if dc.is_null() {
        return;
    }
    let mut rect: RECT = std::mem::zeroed();
    windows_sys::Win32::UI::WindowsAndMessaging::GetClientRect(hwnd, &mut rect);
    let focused = FOCUSED_FIELD.load(Ordering::Acquire) == hwnd as isize;
    let hovered = HOVERED_CONTROL.load(Ordering::Acquire) == hwnd as isize;
    let border = if focused {
        crate::ui_theme::ACCENT
    } else if hovered {
        crate::ui_theme::BORDER_STRONG
    } else {
        crate::ui_theme::BORDER
    };
    stroke_round_rect(
        dc,
        RECT {
            left: 0,
            top: 0,
            right: rect.right,
            bottom: rect.bottom,
        },
        border,
        RADIUS_SMALL,
        if focused { 2 } else { 1 },
    );
    ReleaseDC(hwnd, dc);
}

unsafe extern "system" fn button_subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    match msg {
        WM_MOUSEMOVE_ => {
            let previous = HOVERED_CONTROL.swap(hwnd as isize, Ordering::AcqRel);
            if previous != hwnd as isize {
                if previous != 0 {
                    InvalidateRect(previous as HWND, null(), 0);
                }
                InvalidateRect(hwnd, null(), 0);
            }
            track_leave(hwnd);
        }
        WM_MOUSELEAVE_ => {
            if HOVERED_CONTROL
                .compare_exchange(hwnd as isize, 0, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                InvalidateRect(hwnd, null(), 0);
            }
        }
        WM_ENABLE_ => {
            let result = DefSubclassProc(hwnd, msg, wparam, lparam);
            InvalidateRect(hwnd, null(), 0);
            return result;
        }
        _ => {}
    }
    DefSubclassProc(hwnd, msg, wparam, lparam)
}

unsafe extern "system" fn field_subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    match msg {
        WM_SETFOCUS_ => {
            FOCUSED_FIELD.store(hwnd as isize, Ordering::Release);
            let result = DefSubclassProc(hwnd, msg, wparam, lparam);
            InvalidateRect(hwnd, null(), 0);
            return result;
        }
        WM_KILLFOCUS_ => {
            FOCUSED_FIELD
                .compare_exchange(hwnd as isize, 0, Ordering::AcqRel, Ordering::Acquire)
                .ok();
            let result = DefSubclassProc(hwnd, msg, wparam, lparam);
            InvalidateRect(hwnd, null(), 0);
            return result;
        }
        WM_MOUSEMOVE_ => {
            let previous = HOVERED_CONTROL.swap(hwnd as isize, Ordering::AcqRel);
            if previous != hwnd as isize {
                if previous != 0 {
                    InvalidateRect(previous as HWND, null(), 0);
                }
                InvalidateRect(hwnd, null(), 0);
            }
            track_leave(hwnd);
        }
        WM_MOUSELEAVE_ => {
            if HOVERED_CONTROL
                .compare_exchange(hwnd as isize, 0, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                InvalidateRect(hwnd, null(), 0);
            }
        }
        WM_PAINT_ => {
            let result = DefSubclassProc(hwnd, msg, wparam, lparam);
            paint_field_border(hwnd);
            return result;
        }
        WM_ENABLE_ => {
            let result = DefSubclassProc(hwnd, msg, wparam, lparam);
            InvalidateRect(hwnd, null(), 0);
            return result;
        }
        _ => {}
    }
    DefSubclassProc(hwnd, msg, wparam, lparam)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geometry_stays_compact_desktop_scale() {
        assert!((5..=8).contains(&RADIUS_SMALL));
        assert!((8..=10).contains(&RADIUS_MEDIUM));
        assert!((8..=12).contains(&RADIUS_LARGE));
    }

    #[test]
    fn selected_surface_is_tonal_not_neon() {
        assert_ne!(SELECTED_SURFACE, crate::ui_theme::ACCENT);
        assert_ne!(SELECTED_SURFACE, crate::ui_theme::BG);
    }
}
