//! Shared fallback visual language for ReadyAlert's native Win32 surfaces.
//! The final BPSR pass uses `ui_modern` for Mist/Dark family-specific drawing;
//! this module deliberately defaults to BPSR Dark Glass so any native control
//! not explicitly specialized still belongs to the same application language.
use std::{ffi::c_void, ptr::null, sync::{atomic::{AtomicIsize, Ordering}, OnceLock}};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
    Graphics::Gdi::{
        CreateFontW, CreateSolidBrush, DeleteObject, DrawTextW, FillRect, InvalidateRect,
        SelectObject, SetBkMode, SetTextColor, HBRUSH, HDC, HFONT, TRANSPARENT,
    },
    UI::{
        Controls::DRAWITEMSTRUCT,
        WindowsAndMessaging::{
            GetClassNameW, GetWindowTextLengthW, GetWindowTextW, SendMessageW,
            WM_MOUSEMOVE, WM_SETFONT,
        },
    },
};

// BPSR Dark Glass fallback. Interaction color, class identity and combat
// semantics intentionally use separate constants so their meanings do not compete.
pub const BG: u32 = rgb(14, 23, 29);
pub const SIDEBAR: u32 = rgb(18, 30, 37);
pub const SURFACE: u32 = rgb(20, 32, 39);
pub const RAISED: u32 = rgb(28, 42, 50);
pub const SURFACE_HOVER: u32 = rgb(37, 54, 63);
pub const SURFACE_PRESSED: u32 = rgb(44, 63, 72);
pub const INPUT: u32 = rgb(23, 36, 43);
pub const BORDER: u32 = rgb(69, 89, 99);
pub const BORDER_STRONG: u32 = rgb(93, 115, 125);
pub const TEXT: u32 = rgb(239, 246, 248);
pub const TEXT_SECONDARY: u32 = rgb(200, 214, 219);
pub const MUTED: u32 = rgb(156, 176, 183);
pub const TEXT_DISABLED: u32 = rgb(121, 139, 145);
pub const ACCENT: u32 = rgb(117, 211, 236);
pub const ACCENT_HOVER: u32 = rgb(143, 224, 244);
pub const ACCENT_PRESSED: u32 = rgb(83, 176, 204);
pub const ACCENT_TEXT: u32 = rgb(14, 31, 38);
pub const DAMAGE: u32 = rgb(242, 96, 103);
pub const HEALING: u32 = rgb(85, 207, 139);
pub const TANK: u32 = rgb(91, 156, 218);
pub const WARNING: u32 = rgb(226, 157, 75);
pub const CRITICAL: u32 = rgb(230, 76, 83);
pub const RANK_TEXT: u32 = rgb(216, 225, 229);
pub const DANGER: u32 = rgb(205, 83, 89);
pub const DANGER_HOVER: u32 = rgb(225, 101, 106);

pub const SPACE_1: i32 = 4;
pub const SPACE_2: i32 = 8;
pub const SPACE_3: i32 = 12;
pub const SPACE_4: i32 = 16;
pub const SPACE_5: i32 = 24;
pub const CONTROL_H: i32 = 28;
pub const BUTTON_H: i32 = 30;
pub const NAV_H: i32 = 30;
pub const SETTINGS_W: i32 = 800;
pub const SETTINGS_H: i32 = 440;
pub const EVENT_W: i32 = 720;
pub const EVENT_H: i32 = 440;
pub const DPS_SETTINGS_W: i32 = 620;
pub const DPS_SETTINGS_H: i32 = 420;
pub const MECH_SETTINGS_W: i32 = 720;
pub const MECH_SETTINGS_H: i32 = 430;

#[derive(Clone, Copy)]
pub enum FontRole { Body, Secondary, Heading }

fn wide(text: &str) -> Vec<u16> { text.encode_utf16().chain(std::iter::once(0)).collect() }

fn make_font(height: i32, weight: i32) -> usize {
    unsafe {
        // Windows supplies the normal installed CJK/UI fallback chain when the
        // primary Segoe UI Variable Text face does not contain a glyph.
        let face = wide("Segoe UI Variable Text");
        CreateFontW(height, 0, 0, 0, weight, 0, 0, 0, 1, 0, 0, 5, 0, face.as_ptr()) as usize
    }
}

fn body_font() -> HFONT {
    static FONT: OnceLock<usize> = OnceLock::new();
    *FONT.get_or_init(|| make_font(-14, 400)) as HFONT
}
fn secondary_font() -> HFONT {
    static FONT: OnceLock<usize> = OnceLock::new();
    *FONT.get_or_init(|| make_font(-13, 400)) as HFONT
}
fn heading_font() -> HFONT {
    static FONT: OnceLock<usize> = OnceLock::new();
    *FONT.get_or_init(|| make_font(-17, 600)) as HFONT
}

pub unsafe fn set_font(hwnd: HWND, role: FontRole) {
    let font = match role { FontRole::Body => body_font(), FontRole::Secondary => secondary_font(), FontRole::Heading => heading_font() };
    if !hwnd.is_null() { SendMessageW(hwnd, WM_SETFONT, font as usize, 1); }
}

pub unsafe fn bg_brush() -> HBRUSH {
    static BRUSH: OnceLock<usize> = OnceLock::new();
    *BRUSH.get_or_init(|| CreateSolidBrush(BG) as usize) as HBRUSH
}

pub unsafe fn input_brush() -> HBRUSH {
    static BRUSH: OnceLock<usize> = OnceLock::new();
    *BRUSH.get_or_init(|| CreateSolidBrush(INPUT) as usize) as HBRUSH
}

pub unsafe fn surface_brush() -> HBRUSH {
    static BRUSH: OnceLock<usize> = OnceLock::new();
    *BRUSH.get_or_init(|| CreateSolidBrush(SURFACE) as usize) as HBRUSH
}

#[link(name = "dwmapi")]
extern "system" { fn DwmSetWindowAttribute(hwnd: HWND, attribute: u32, value: *const c_void, size: u32) -> i32; }
#[link(name = "uxtheme")]
extern "system" { fn SetWindowTheme(hwnd: HWND, app_name: *const u16, id_list: *const u16) -> i32; }

// windows-sys 0.59 does not expose these common-control/user32 hover helpers
// through the currently enabled feature set. Keep the tiny ABI surface local.
type SubclassProc = Option<unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM, usize, usize) -> LRESULT>;

#[repr(C)]
struct NativeTrackMouseEvent {
    cb_size: u32,
    flags: u32,
    hwnd_track: HWND,
    hover_time: u32,
}

const TME_LEAVE_NATIVE: u32 = 0x0000_0002;
const WM_MOUSELEAVE_NATIVE: u32 = 0x02A3;

#[link(name = "comctl32")]
extern "system" {
    fn SetWindowSubclass(hwnd: HWND, proc: SubclassProc, id: usize, data: usize) -> i32;
    fn DefSubclassProc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT;
}
#[link(name = "user32")]
extern "system" { fn TrackMouseEvent(event: *mut NativeTrackMouseEvent) -> i32; }

pub unsafe fn dark_titlebar(hwnd: HWND) {
    let enabled: i32 = 1;
    let ptr = (&enabled as *const i32).cast::<c_void>();
    if DwmSetWindowAttribute(hwnd, 20, ptr, std::mem::size_of::<i32>() as u32) != 0 {
        let _ = DwmSetWindowAttribute(hwnd, 19, ptr, std::mem::size_of::<i32>() as u32);
    }
    // BPSR uses restrained corners, not large rounded desktop cards.
    let rounded: i32 = 1;
    let _ = DwmSetWindowAttribute(hwnd, 33, (&rounded as *const i32).cast(), std::mem::size_of::<i32>() as u32);
    let border = BORDER;
    let caption = SIDEBAR;
    let text = TEXT;
    let _ = DwmSetWindowAttribute(hwnd, 34, (&border as *const u32).cast(), std::mem::size_of::<u32>() as u32);
    let _ = DwmSetWindowAttribute(hwnd, 35, (&caption as *const u32).cast(), std::mem::size_of::<u32>() as u32);
    let _ = DwmSetWindowAttribute(hwnd, 36, (&text as *const u32).cast(), std::mem::size_of::<u32>() as u32);
}

const BUTTON_SUBCLASS_ID: usize = 0x4250_5352;
static HOVERED_BUTTON: AtomicIsize = AtomicIsize::new(0);

unsafe fn is_button_control(hwnd: HWND) -> bool {
    let mut class = [0u16; 16];
    let len = GetClassNameW(hwnd, class.as_mut_ptr(), class.len() as i32).max(0) as usize;
    if len == 0 { return false; }
    String::from_utf16_lossy(&class[..len]).eq_ignore_ascii_case("BUTTON")
}

unsafe fn install_button_hover_tracking(hwnd: HWND) {
    if is_button_control(hwnd) {
        let _ = SetWindowSubclass(hwnd, Some(button_subclass_proc), BUTTON_SUBCLASS_ID, 0);
    }
}

pub unsafe fn theme_control(hwnd: HWND) {
    if hwnd.is_null() { return; }
    set_font(hwnd, FontRole::Body);
    let theme = wide("DarkMode_Explorer");
    let _ = SetWindowTheme(hwnd, theme.as_ptr(), null());
    install_button_hover_tracking(hwnd);
}

pub unsafe fn theme_combo(hwnd: HWND) {
    if hwnd.is_null() { return; }
    set_font(hwnd, FontRole::Body);
    // The OS owns the combo popup mechanics; DarkMode_CFD keeps it coherent
    // with the dark/mist BPSR palettes without replacing accessibility behavior.
    let theme = wide("DarkMode_CFD");
    if SetWindowTheme(hwnd, theme.as_ptr(), null()) != 0 {
        let fallback = wide("DarkMode_Explorer");
        let _ = SetWindowTheme(hwnd, fallback.as_ptr(), null());
    }
}

pub unsafe fn theme_button(hwnd: HWND) {
    if hwnd.is_null() { return; }
    theme_control(hwnd);
}

unsafe extern "system" fn button_subclass_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM, _id: usize, _data: usize) -> LRESULT {
    match msg {
        WM_MOUSEMOVE => {
            let previous = HOVERED_BUTTON.swap(hwnd as isize, Ordering::AcqRel);
            if previous != hwnd as isize {
                if previous != 0 { InvalidateRect(previous as HWND, null(), 0); }
                InvalidateRect(hwnd, null(), 0);
            }
            let mut tracking = NativeTrackMouseEvent {
                cb_size: std::mem::size_of::<NativeTrackMouseEvent>() as u32,
                flags: TME_LEAVE_NATIVE,
                hwnd_track: hwnd,
                hover_time: 0,
            };
            let _ = TrackMouseEvent(&mut tracking);
        }
        WM_MOUSELEAVE_NATIVE => {
            if HOVERED_BUTTON.compare_exchange(hwnd as isize, 0, Ordering::AcqRel, Ordering::Acquire).is_ok() {
                InvalidateRect(hwnd, null(), 0);
            }
        }
        _ => {}
    }
    DefSubclassProc(hwnd, msg, wparam, lparam)
}

unsafe fn fill(hdc: HDC, rect: &RECT, color: u32) {
    let brush = CreateSolidBrush(color);
    if !brush.is_null() { FillRect(hdc, rect, brush); DeleteObject(brush); }
}

pub unsafe fn draw_button(item: *const DRAWITEMSTRUCT, selected: bool, primary: bool, danger: bool, left_align: bool) -> isize {
    if item.is_null() { return 0; }
    let item = &*item;
    let disabled = item.itemState & 0x0004 != 0;
    let pressed = item.itemState & 0x0001 != 0;
    let hot = item.itemState & 0x0040 != 0 || HOVERED_BUTTON.load(Ordering::Acquire) == item.hwndItem as isize;
    let focused = item.itemState & 0x0010 != 0;
    let nav = left_align;

    let mut bg = if nav {
        if selected { RAISED } else { SIDEBAR }
    } else if selected {
        SURFACE_HOVER
    } else if primary {
        ACCENT
    } else if danger {
        rgb(89, 49, 53)
    } else {
        SURFACE
    };
    if disabled { bg = if nav { SIDEBAR } else { SURFACE }; }
    else if pressed { bg = if nav { SURFACE_PRESSED } else if primary { ACCENT_PRESSED } else if danger { DANGER } else { SURFACE_PRESSED }; }
    else if hot { bg = if nav { SURFACE_HOVER } else if primary { ACCENT_HOVER } else if danger { DANGER_HOVER } else { SURFACE_HOVER }; }
    fill(item.hDC, &item.rcItem, bg);

    if nav {
        if selected {
            fill(item.hDC, &RECT { left:item.rcItem.left, top:item.rcItem.top, right:item.rcItem.left+3, bottom:item.rcItem.bottom }, ACCENT);
        }
        // Sidebar rows stay visually quiet until selected/hovered; keyboard focus
        // still gets a thin visible outline and is never color-only.
        if focused {
            let top = RECT { left:item.rcItem.left, top:item.rcItem.top, right:item.rcItem.right, bottom:item.rcItem.top+1 };
            let bottom = RECT { left:item.rcItem.left, top:item.rcItem.bottom-1, right:item.rcItem.right, bottom:item.rcItem.bottom };
            let right = RECT { left:item.rcItem.right-1, top:item.rcItem.top, right:item.rcItem.right, bottom:item.rcItem.bottom };
            for r in [&top, &bottom, &right] { fill(item.hDC, r, BORDER_STRONG); }
        }
    } else {
        let border = if disabled { BORDER } else if selected || primary || focused { ACCENT } else if danger { DANGER } else { BORDER };
        let top = RECT { left:item.rcItem.left, top:item.rcItem.top, right:item.rcItem.right, bottom:item.rcItem.top+1 };
        let bottom = RECT { left:item.rcItem.left, top:item.rcItem.bottom-1, right:item.rcItem.right, bottom:item.rcItem.bottom };
        let left = RECT { left:item.rcItem.left, top:item.rcItem.top, right:item.rcItem.left + if selected { 3 } else { 1 }, bottom:item.rcItem.bottom };
        let right = RECT { left:item.rcItem.right-1, top:item.rcItem.top, right:item.rcItem.right, bottom:item.rcItem.bottom };
        for r in [&top,&bottom,&left,&right] { fill(item.hDC, r, border); }
    }

    let len = GetWindowTextLengthW(item.hwndItem).max(0) as usize;
    let mut buf = vec![0u16; len + 1];
    let got = GetWindowTextW(item.hwndItem, buf.as_mut_ptr(), buf.len() as i32).max(0) as usize;
    SetBkMode(item.hDC, TRANSPARENT as i32);
    let color = if disabled { TEXT_DISABLED } else if primary { ACCENT_TEXT } else { TEXT };
    SetTextColor(item.hDC, color);
    SelectObject(item.hDC, body_font());
    let mut rect = item.rcItem;
    if left_align { rect.left += 12; }
    let flags = (if left_align { 0 } else { 0x0001 }) | 0x0004 | 0x0020 | 0x0800;
    DrawTextW(item.hDC, buf.as_ptr(), got as i32, &mut rect, flags);
    1
}

pub unsafe fn draw_combo(item: *const DRAWITEMSTRUCT) -> isize {
    if item.is_null() { return 0; }
    let item=&*item;
    let disabled=item.itemState & 0x0004 != 0;
    let selected=item.itemState & 0x0001 != 0;
    let focused=item.itemState & 0x0010 != 0;
    fill(item.hDC,&item.rcItem,if disabled{SURFACE}else if selected{SURFACE_HOVER}else{INPUT});
    let border=if !disabled&&focused{ACCENT}else{BORDER};
    let top=RECT{left:item.rcItem.left,top:item.rcItem.top,right:item.rcItem.right,bottom:item.rcItem.top+1};
    let bottom=RECT{left:item.rcItem.left,top:item.rcItem.bottom-1,right:item.rcItem.right,bottom:item.rcItem.bottom};
    fill(item.hDC,&top,border);fill(item.hDC,&bottom,border);
    let index=if item.itemID==u32::MAX{SendMessageW(item.hwndItem,0x0147,0,0)}else{item.itemID as isize};
    if index>=0{
        let len=SendMessageW(item.hwndItem,0x0149,index as usize,0).max(0)as usize;
        let mut buf=vec![0u16;len+1];
        let got=SendMessageW(item.hwndItem,0x0148,index as usize,buf.as_mut_ptr()as isize).max(0)as usize;
        SetBkMode(item.hDC,TRANSPARENT as i32);SetTextColor(item.hDC,if disabled{TEXT_DISABLED}else{TEXT});SelectObject(item.hDC,body_font());
        let mut r=item.rcItem;r.left+=8;r.right-=6;DrawTextW(item.hDC,buf.as_ptr(),got as i32,&mut r,0x0004|0x0020|0x0800|0x8000);
    }
    1
}

pub const fn rgb(r: u8, g: u8, b: u8) -> u32 { (r as u32) | ((g as u32) << 8) | ((b as u32) << 16) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn fixed_windows_fit_small_scaled_work_area() {
        assert!(SETTINGS_W <= 820 && SETTINGS_H <= 450);
        assert!(EVENT_W <= 760 && EVENT_H <= 450);
        assert!(MECH_SETTINGS_W <= 760 && MECH_SETTINGS_H <= 450);
    }
    #[test] fn theme_has_readable_contrast_direction() {
        assert_ne!(BG, SURFACE); assert_ne!(TEXT, TEXT_SECONDARY); assert_ne!(ACCENT, BG); assert_ne!(ACCENT_TEXT, TEXT);
    }
    #[test] fn semantic_overlay_colors_are_distinct() {
        assert_ne!(ACCENT, DAMAGE); assert_ne!(DAMAGE, HEALING); assert_ne!(HEALING, TANK);
        assert_ne!(WARNING, CRITICAL); assert_ne!(RANK_TEXT, DAMAGE);
    }
}
