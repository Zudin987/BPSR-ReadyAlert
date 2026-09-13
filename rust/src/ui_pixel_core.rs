// Pixel-inspired native rendering primitives for ReadyAlert.
//
// This layer is presentation-only: existing HWNDs, commands, geometry, shortcuts,
// data flow and interaction behavior stay intact.  It deliberately keeps the app
// on lightweight Win32/GDI/DWM rather than introducing a web or retained GUI runtime.
use std::{
    ffi::c_void,
    ptr::{null, null_mut},
    sync::{
        atomic::{AtomicIsize, Ordering},
        OnceLock,
    },
};
use windows_sys::Win32::{
    Foundation::{GetLastError, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM},
    Graphics::Gdi::{
        BeginPaint, CreateFontW, CreatePen, CreateSolidBrush, DeleteObject, DrawTextW, EndPaint,
        FillRect, GetDC, GetStockObject, InvalidateRect, ReleaseDC, RoundRect, SelectObject,
        SetBkMode, SetTextColor, HBRUSH, HDC, HFONT, PAINTSTRUCT, TRANSPARENT,
    },
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        Controls::DRAWITEMSTRUCT,
        WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetClassNameW,
            GetClientRect, GetMessageW, GetWindowLongPtrW, GetWindowTextLengthW,
            GetWindowTextW, IsWindow, LoadCursorW, MessageBoxW, PostQuitMessage,
            RegisterClassW, SendMessageW, SetForegroundWindow, SetWindowLongPtrW, SetWindowPos,
            ShowWindow, TranslateMessage, CREATESTRUCTW, CW_USEDEFAULT, GWLP_USERDATA, IDC_ARROW,
            MSG, SW_SHOW, WM_CLOSE, WM_COMMAND, WM_CREATE, WM_CTLCOLORSTATIC, WM_DRAWITEM,
            WM_ERASEBKGND, WM_NCCREATE, WM_NCDESTROY, WNDCLASSW, WS_CAPTION, WS_CHILD,
            WS_POPUP, WS_SYSMENU, WS_TABSTOP, WS_VISIBLE,
        },
    },
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SurfaceFamily {
    Mist,
    Dark,
}

// Desktop Pixel geometry: visibly softer than legacy Win32, but not giant mobile pills.
pub const RADIUS_SMALL: i32 = 8;
pub const RADIUS_MEDIUM: i32 = 12;
pub const RADIUS_LARGE: i32 = 16;
pub const RADIUS_DIALOG: i32 = 20;

// Neutral dark tonal hierarchy. Borders are deliberately low-emphasis; surface tone
// and spacing do most of the grouping work.
pub const DARK_BG: u32 = crate::ui_theme::rgb(17, 19, 24);
pub const DARK_SURFACE: u32 = crate::ui_theme::rgb(25, 27, 32);
pub const DARK_RAISED: u32 = crate::ui_theme::rgb(32, 35, 42);
pub const DARK_HOVER: u32 = crate::ui_theme::rgb(42, 45, 54);
pub const DARK_PRESSED: u32 = crate::ui_theme::rgb(50, 54, 64);
pub const DARK_INPUT: u32 = crate::ui_theme::rgb(29, 32, 38);
pub const DARK_BORDER: u32 = crate::ui_theme::rgb(54, 57, 66);
pub const DARK_BORDER_STRONG: u32 = crate::ui_theme::rgb(78, 82, 94);

// Configuration surfaces stay dark too; the extra tonal step makes Settings read as
// a first-party system surface rather than the previous blue/teal glass utility.
pub const MIST_BG: u32 = crate::ui_theme::rgb(20, 21, 27);
pub const MIST_SIDEBAR: u32 = crate::ui_theme::rgb(24, 25, 31);
pub const MIST_SURFACE: u32 = crate::ui_theme::rgb(29, 31, 38);
pub const MIST_RAISED: u32 = crate::ui_theme::rgb(36, 39, 47);
pub const MIST_HOVER: u32 = crate::ui_theme::rgb(45, 48, 58);
pub const MIST_PRESSED: u32 = crate::ui_theme::rgb(53, 57, 68);
pub const MIST_INPUT: u32 = crate::ui_theme::rgb(31, 33, 40);
pub const MIST_BORDER: u32 = crate::ui_theme::rgb(56, 59, 68);
pub const MIST_BORDER_STRONG: u32 = crate::ui_theme::rgb(83, 87, 99);
pub const MIST_SELECTED: u32 = crate::ui_theme::rgb(42, 54, 73);
pub const MIST_SELECTED_HOVER: u32 = crate::ui_theme::rgb(50, 64, 86);

pub const BPSR_TEXT: u32 = crate::ui_theme::rgb(232, 234, 242);
pub const BPSR_TEXT_SECONDARY: u32 = crate::ui_theme::rgb(194, 198, 210);
pub const BPSR_MUTED: u32 = crate::ui_theme::rgb(144, 149, 164);
pub const BPSR_DISABLED: u32 = crate::ui_theme::rgb(104, 109, 122);
pub const BPSR_ACCENT: u32 = crate::ui_theme::rgb(168, 199, 250);
pub const BPSR_ACCENT_HOVER: u32 = crate::ui_theme::rgb(188, 212, 252);
pub const BPSR_ACCENT_PRESSED: u32 = crate::ui_theme::rgb(139, 177, 236);
pub const BPSR_ACCENT_TEXT: u32 = crate::ui_theme::rgb(24, 42, 66);
pub const BPSR_SUCCESS: u32 = crate::ui_theme::rgb(129, 201, 149);
pub const BPSR_WARNING: u32 = crate::ui_theme::rgb(245, 202, 94);
pub const BPSR_DANGER: u32 = crate::ui_theme::rgb(242, 139, 130);
pub const BPSR_DANGER_HOVER: u32 = crate::ui_theme::rgb(255, 163, 155);
pub const BPSR_INFO: u32 = crate::ui_theme::rgb(138, 180, 248);
pub const BPSR_LIVE: u32 = crate::ui_theme::rgb(255, 138, 128);

pub const SELECTED_SURFACE: u32 = crate::ui_theme::rgb(42, 54, 73);
pub const SELECTED_SURFACE_HOVER: u32 = crate::ui_theme::rgb(50, 64, 86);
pub const PANEL_SURFACE: u32 = DARK_SURFACE;
pub const PANEL_RAISED: u32 = DARK_RAISED;

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
const TME_LEAVE_: u32 = 0x0000_0002;
const BUTTON_SUBCLASS_ID: usize = 0x5241_4254;
const FIELD_SUBCLASS_ID: usize = 0x5241_4644;
const LIST_SUBCLASS_ID: usize = 0x5241_4c53;
const GWL_STYLE_: i32 = -16;
const GWL_EXSTYLE_: i32 = -20;
const WS_BORDER_: isize = 0x0080_0000;
const WS_EX_CLIENTEDGE_: isize = 0x0000_0200;
const SWP_REFRESH_FRAME_: u32 = 0x0037; // NOSIZE|NOMOVE|NOZORDER|NOACTIVATE|FRAMECHANGED

static HOVERED_CONTROL: AtomicIsize = AtomicIsize::new(0);
static FOCUSED_FIELD: AtomicIsize = AtomicIsize::new(0);

#[repr(C)]
struct NativeTrackMouseEvent {
    cb_size: u32,
    flags: u32,
    hwnd_track: HWND,
    hover_time: u32,
}

type SubclassProc = Option<unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM, usize, usize) -> LRESULT>;

#[link(name = "comctl32")]
extern "system" {
    fn SetWindowSubclass(hwnd: HWND, proc: SubclassProc, id: usize, data: usize) -> i32;
    fn DefSubclassProc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT;
}

#[link(name = "user32")]
extern "system" {
    fn TrackMouseEvent(event: *mut NativeTrackMouseEvent) -> i32;
    fn EnableWindow(hwnd: HWND, enable: i32) -> i32;
}

#[link(name = "dwmapi")]
extern "system" {
    fn DwmSetWindowAttribute(hwnd: HWND, attribute: u32, value: *const c_void, size: u32) -> i32;
}

#[link(name = "uxtheme")]
extern "system" {
    fn SetWindowTheme(hwnd: HWND, app_name: *const u16, id_list: *const u16) -> i32;
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn make_font(height: i32, weight: i32) -> usize {
    unsafe {
        let face = wide("Segoe UI Variable Text");
        CreateFontW(height, 0, 0, 0, weight, 0, 0, 0, 1, 0, 0, 5, 0, face.as_ptr()) as usize
    }
}

pub fn body_font() -> HFONT {
    static FONT: OnceLock<usize> = OnceLock::new();
    *FONT.get_or_init(|| make_font(-14, 400)) as HFONT
}

pub fn medium_font() -> HFONT {
    static FONT: OnceLock<usize> = OnceLock::new();
    *FONT.get_or_init(|| make_font(-14, 600)) as HFONT
}

pub fn heading_font() -> HFONT {
    static FONT: OnceLock<usize> = OnceLock::new();
    *FONT.get_or_init(|| make_font(-18, 600)) as HFONT
}

pub fn title_font() -> HFONT {
    static FONT: OnceLock<usize> = OnceLock::new();
    *FONT.get_or_init(|| make_font(-22, 600)) as HFONT
}

pub fn caption_font() -> HFONT {
    static FONT: OnceLock<usize> = OnceLock::new();
    *FONT.get_or_init(|| make_font(-12, 400)) as HFONT
}

fn family_surface(family: SurfaceFamily) -> u32 {
    match family { SurfaceFamily::Mist => MIST_SURFACE, SurfaceFamily::Dark => DARK_SURFACE }
}
fn family_raised(family: SurfaceFamily) -> u32 {
    match family { SurfaceFamily::Mist => MIST_RAISED, SurfaceFamily::Dark => DARK_RAISED }
}
fn family_hover(family: SurfaceFamily) -> u32 {
    match family { SurfaceFamily::Mist => MIST_HOVER, SurfaceFamily::Dark => DARK_HOVER }
}
fn family_pressed(family: SurfaceFamily) -> u32 {
    match family { SurfaceFamily::Mist => MIST_PRESSED, SurfaceFamily::Dark => DARK_PRESSED }
}
fn family_input(family: SurfaceFamily) -> u32 {
    match family { SurfaceFamily::Mist => MIST_INPUT, SurfaceFamily::Dark => DARK_INPUT }
}
fn family_border(family: SurfaceFamily) -> u32 {
    match family { SurfaceFamily::Mist => MIST_BORDER, SurfaceFamily::Dark => DARK_BORDER }
}
fn family_border_strong(family: SurfaceFamily) -> u32 {
    match family { SurfaceFamily::Mist => MIST_BORDER_STRONG, SurfaceFamily::Dark => DARK_BORDER_STRONG }
}
fn family_selected(family: SurfaceFamily) -> u32 {
    match family { SurfaceFamily::Mist => MIST_SELECTED, SurfaceFamily::Dark => SELECTED_SURFACE }
}
fn family_selected_hover(family: SurfaceFamily) -> u32 {
    match family { SurfaceFamily::Mist => MIST_SELECTED_HOVER, SurfaceFamily::Dark => SELECTED_SURFACE_HOVER }
}

pub unsafe fn fill_round_rect(hdc: HDC, rect: RECT, color: u32, radius: i32) {
    if rect.right <= rect.left || rect.bottom <= rect.top { return; }
    let brush = CreateSolidBrush(color);
    if brush.is_null() { return; }
    let old_brush = SelectObject(hdc, brush as _);
    let old_pen = SelectObject(hdc, GetStockObject(NULL_PEN_STOCK));
    let diameter = radius.max(1) * 2;
    RoundRect(hdc, rect.left, rect.top, rect.right, rect.bottom, diameter, diameter);
    SelectObject(hdc, old_pen);
    SelectObject(hdc, old_brush);
    DeleteObject(brush);
}

pub unsafe fn stroke_round_rect(hdc: HDC, rect: RECT, color: u32, radius: i32, width: i32) {
    if rect.right <= rect.left || rect.bottom <= rect.top { return; }
    let pen = CreatePen(PS_SOLID_, width.max(1), color);
    if pen.is_null() { return; }
    let old_pen = SelectObject(hdc, pen as _);
    let old_brush = SelectObject(hdc, GetStockObject(NULL_BRUSH_STOCK));
    let inset = width.max(1) / 2;
    let diameter = radius.max(1) * 2;
    RoundRect(hdc, rect.left + inset, rect.top + inset, rect.right - inset, rect.bottom - inset, diameter, diameter);
    SelectObject(hdc, old_brush);
    SelectObject(hdc, old_pen);
    DeleteObject(pen);
}

pub unsafe fn paint_panel(hdc: HDC, rect: RECT, elevated: bool) {
    let shadow = RECT { left: rect.left, top: rect.top + 2, right: rect.right, bottom: rect.bottom + 2 };
    fill_round_rect(hdc, shadow, crate::ui_theme::rgb(9, 10, 14), RADIUS_LARGE);
    fill_round_rect(hdc, rect, if elevated { PANEL_RAISED } else { PANEL_SURFACE }, RADIUS_LARGE);
    // One low-contrast hairline is reserved for true elevated cards; ordinary groups
    // are separated by tone instead of nested boxes.
    if elevated { stroke_round_rect(hdc, rect, DARK_BORDER, RADIUS_LARGE, 1); }
}

pub unsafe fn fill_mist_background(hdc: HDC, rect: &RECT) { FillRect(hdc, rect, mist_bg_brush()); }

pub unsafe fn mist_bg_brush() -> HBRUSH {
    static BRUSH: OnceLock<usize> = OnceLock::new();
    *BRUSH.get_or_init(|| CreateSolidBrush(MIST_BG) as usize) as HBRUSH
}

pub unsafe fn mist_input_brush() -> HBRUSH {
    static BRUSH: OnceLock<usize> = OnceLock::new();
    *BRUSH.get_or_init(|| CreateSolidBrush(MIST_INPUT) as usize) as HBRUSH
}

pub unsafe fn dark_bg_brush() -> HBRUSH {
    static BRUSH: OnceLock<usize> = OnceLock::new();
    *BRUSH.get_or_init(|| CreateSolidBrush(DARK_BG) as usize) as HBRUSH
}

unsafe fn style_titlebar(hwnd: HWND, family: SurfaceFamily) {
    if hwnd.is_null() { return; }
    let enabled: i32 = 1;
    let ptr = (&enabled as *const i32).cast::<c_void>();
    if DwmSetWindowAttribute(hwnd, 20, ptr, std::mem::size_of::<i32>() as u32) != 0 {
        let _ = DwmSetWindowAttribute(hwnd, 19, ptr, std::mem::size_of::<i32>() as u32);
    }
    let rounded: i32 = 2; // DWMWCP_ROUND
    let _ = DwmSetWindowAttribute(hwnd, 33, (&rounded as *const i32).cast(), std::mem::size_of::<i32>() as u32);
    let caption = match family { SurfaceFamily::Mist => MIST_SIDEBAR, SurfaceFamily::Dark => DARK_SURFACE };
    let border = match family { SurfaceFamily::Mist => MIST_BORDER, SurfaceFamily::Dark => DARK_BORDER };
    let text = BPSR_TEXT;
    let _ = DwmSetWindowAttribute(hwnd, 35, (&caption as *const u32).cast(), std::mem::size_of::<u32>() as u32);
    let _ = DwmSetWindowAttribute(hwnd, 34, (&border as *const u32).cast(), std::mem::size_of::<u32>() as u32);
    let _ = DwmSetWindowAttribute(hwnd, 36, (&text as *const u32).cast(), std::mem::size_of::<u32>() as u32);
}

pub unsafe fn mist_titlebar(hwnd: HWND) { style_titlebar(hwnd, SurfaceFamily::Mist); }
pub unsafe fn dark_titlebar(hwnd: HWND) { style_titlebar(hwnd, SurfaceFamily::Dark); }

unsafe fn dark_common_control(hwnd: HWND) {
    let theme = wide("DarkMode_Explorer");
    let _ = SetWindowTheme(hwnd, theme.as_ptr(), null());
}

unsafe fn strip_native_edge(hwnd: HWND) {
    let style = GetWindowLongPtrW(hwnd, GWL_STYLE_);
    let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE_);
    if style & WS_BORDER_ != 0 { SetWindowLongPtrW(hwnd, GWL_STYLE_, style & !WS_BORDER_); }
    if ex_style & WS_EX_CLIENTEDGE_ != 0 { SetWindowLongPtrW(hwnd, GWL_EXSTYLE_, ex_style & !WS_EX_CLIENTEDGE_); }
    let _ = SetWindowPos(hwnd, null_mut(), 0, 0, 0, 0, SWP_REFRESH_FRAME_);
}

unsafe fn track_leave(hwnd: HWND) {
    let mut tracking = NativeTrackMouseEvent {
        cb_size: std::mem::size_of::<NativeTrackMouseEvent>() as u32,
        flags: TME_LEAVE_, hwnd_track: hwnd, hover_time: 0,
    };
    let _ = TrackMouseEvent(&mut tracking);
}
