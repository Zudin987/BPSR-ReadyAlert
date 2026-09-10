//! Shared visual language for ReadyAlert's native Win32 surfaces.
//! Keep this intentionally small: the app stays native/lightweight while all
//! configuration windows and overlays share the same palette and interaction states.
use std::{ffi::c_void, ptr::null, sync::OnceLock};
use windows_sys::Win32::{
    Foundation::{HWND, RECT},
    Graphics::Gdi::{
        CreateFontW, CreateSolidBrush, DeleteObject, DrawTextW, FillRect, SelectObject,
        SetBkMode, SetTextColor, HDC, HFONT, TRANSPARENT,
    },
    UI::{
        Controls::DRAWITEMSTRUCT,
        WindowsAndMessaging::{
            GetWindowTextLengthW, GetWindowTextW, SendMessageW, WM_SETFONT,
        },
    },
};

// Compact native dark palette. Interaction color, class identity and combat
// semantics intentionally use separate constants so their meanings do not compete.
pub const BG: u32 = rgb(15, 19, 23);
pub const SIDEBAR: u32 = rgb(21, 27, 33);
pub const SURFACE: u32 = rgb(21, 27, 33);
pub const RAISED: u32 = rgb(26, 32, 39);
pub const SURFACE_HOVER: u32 = rgb(32, 40, 48);
pub const SURFACE_PRESSED: u32 = rgb(37, 46, 55);
pub const INPUT: u32 = rgb(26, 32, 39);
pub const BORDER: u32 = rgb(46, 57, 68);
pub const BORDER_STRONG: u32 = rgb(63, 76, 88);
pub const TEXT: u32 = rgb(241, 244, 247);
pub const TEXT_SECONDARY: u32 = rgb(181, 190, 200);
pub const MUTED: u32 = rgb(131, 144, 157);
pub const TEXT_DISABLED: u32 = rgb(103, 114, 125);
pub const ACCENT: u32 = rgb(56, 184, 166);
pub const ACCENT_HOVER: u32 = rgb(66, 198, 179);
pub const ACCENT_PRESSED: u32 = rgb(47, 156, 141);
pub const DAMAGE: u32 = rgb(255, 93, 98);
pub const HEALING: u32 = rgb(69, 222, 139);
pub const TANK: u32 = rgb(102, 151, 255);
pub const WARNING: u32 = rgb(240, 184, 73);
pub const CRITICAL: u32 = rgb(255, 81, 88);
pub const RANK_TEXT: u32 = rgb(216, 222, 229);
// Destructive settings actions stay quieter than live critical combat state.
pub const DANGER: u32 = rgb(190, 70, 72);
pub const DANGER_HOVER: u32 = rgb(210, 81, 83);

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
        // The Windows font mapper supplies Segoe UI / installed CJK UI fallback
        // glyphs when Segoe UI Variable Text does not contain a character.
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

#[link(name = "dwmapi")]
extern "system" { fn DwmSetWindowAttribute(hwnd: HWND, attribute: u32, value: *const c_void, size: u32) -> i32; }
#[link(name = "uxtheme")]
extern "system" { fn SetWindowTheme(hwnd: HWND, app_name: *const u16, id_list: *const u16) -> i32; }

pub unsafe fn dark_titlebar(hwnd: HWND) {
    let enabled: i32 = 1;
    let ptr = (&enabled as *const i32).cast::<c_void>();
    if DwmSetWindowAttribute(hwnd, 20, ptr, std::mem::size_of::<i32>() as u32) != 0 {
        let _ = DwmSetWindowAttribute(hwnd, 19, ptr, std::mem::size_of::<i32>() as u32);
    }
    let rounded: i32 = 2;
    let _ = DwmSetWindowAttribute(hwnd, 33, (&rounded as *const i32).cast(), std::mem::size_of::<i32>() as u32);
}

pub unsafe fn theme_control(hwnd: HWND) {
    if hwnd.is_null() { return; }
    set_font(hwnd, FontRole::Body);
    let theme = wide("DarkMode_Explorer");
    let _ = SetWindowTheme(hwnd, theme.as_ptr(), null());
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
    let hot = item.itemState & 0x0040 != 0;
    let focused = item.itemState & 0x0010 != 0;

    let mut bg = if selected { SURFACE_HOVER } else if primary { ACCENT } else if danger { rgb(77, 35, 38) } else { SURFACE };
    if disabled { bg = SURFACE; }
    else if pressed { bg = if primary { ACCENT_PRESSED } else if danger { DANGER } else { SURFACE_PRESSED }; }
    else if hot { bg = if primary { ACCENT_HOVER } else if danger { DANGER_HOVER } else { SURFACE_HOVER }; }
    fill(item.hDC, &item.rcItem, bg);

    let border = if selected || primary || focused { ACCENT } else if danger { DANGER } else { BORDER };
    let top = RECT { left:item.rcItem.left, top:item.rcItem.top, right:item.rcItem.right, bottom:item.rcItem.top+1 };
    let bottom = RECT { left:item.rcItem.left, top:item.rcItem.bottom-1, right:item.rcItem.right, bottom:item.rcItem.bottom };
    let left = RECT { left:item.rcItem.left, top:item.rcItem.top, right:item.rcItem.left + if selected { 3 } else { 1 }, bottom:item.rcItem.bottom };
    let right = RECT { left:item.rcItem.right-1, top:item.rcItem.top, right:item.rcItem.right, bottom:item.rcItem.bottom };
    for r in [&top,&bottom,&left,&right] { fill(item.hDC, r, border); }

    let len = GetWindowTextLengthW(item.hwndItem).max(0) as usize;
    let mut buf = vec![0u16; len + 1];
    let got = GetWindowTextW(item.hwndItem, buf.as_mut_ptr(), buf.len() as i32).max(0) as usize;
    SetBkMode(item.hDC, TRANSPARENT as i32);
    let color = if disabled { TEXT_DISABLED } else if primary { rgb(248, 253, 252) } else { TEXT };
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
    let selected=item.itemState & 0x0001 != 0;
    let focused=item.itemState & 0x0010 != 0;
    fill(item.hDC,&item.rcItem,if selected{SURFACE_HOVER}else{INPUT});
    let border=if focused{ACCENT}else{BORDER};
    let top=RECT{left:item.rcItem.left,top:item.rcItem.top,right:item.rcItem.right,bottom:item.rcItem.top+1};
    let bottom=RECT{left:item.rcItem.left,top:item.rcItem.bottom-1,right:item.rcItem.right,bottom:item.rcItem.bottom};
    fill(item.hDC,&top,border);fill(item.hDC,&bottom,border);
    let index=if item.itemID==u32::MAX{SendMessageW(item.hwndItem,0x0147,0,0)}else{item.itemID as isize};
    if index>=0{
        let len=SendMessageW(item.hwndItem,0x0149,index as usize,0).max(0)as usize;
        let mut buf=vec![0u16;len+1];
        let got=SendMessageW(item.hwndItem,0x0148,index as usize,buf.as_mut_ptr()as isize).max(0)as usize;
        SetBkMode(item.hDC,TRANSPARENT as i32);SetTextColor(item.hDC,TEXT);SelectObject(item.hDC,body_font());
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
        assert_ne!(BG, SURFACE); assert_ne!(TEXT, TEXT_SECONDARY); assert_ne!(ACCENT, BG);
    }
    #[test] fn semantic_overlay_colors_are_distinct() {
        assert_ne!(ACCENT, DAMAGE); assert_ne!(DAMAGE, HEALING); assert_ne!(HEALING, TANK);
        assert_ne!(WARNING, CRITICAL); assert_ne!(RANK_TEXT, DAMAGE);
    }
}
