//! BPSR-native drawing helpers for ReadyAlert.
//!
//! The app stays lightweight GDI + Win32.  Two related surface families mirror
//! Blue Protocol: Star Resonance: mist glass for configuration/dialog surfaces
//! and dark glass for in-game overlays/quick actions.  Geometry intentionally
//! avoids the pill-heavy Pixel/Material look used by the previous pass.
use std::{
    ffi::c_void,
    ptr::{null, null_mut},
    sync::{
        atomic::{AtomicIsize, Ordering},
        OnceLock,
    },
};
use windows_sys::Win32::{
    Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM},
    Graphics::Gdi::{
        CreateFontW, CreatePen, CreateSolidBrush, DeleteObject, DrawTextW, FillRect, GetDC,
        GetStockObject, InvalidateRect, ReleaseDC, RoundRect, SelectObject, SetBkMode,
        SetTextColor, HBRUSH, HDC, HFONT, TRANSPARENT,
    },
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        Controls::DRAWITEMSTRUCT,
        WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, EnableWindow,
            GetClassNameW, GetClientRect, GetMessageW, GetWindowLongPtrW, GetWindowTextLengthW,
            GetWindowTextW, IsWindow, LoadCursorW, MessageBoxW, PostQuitMessage, RegisterClassW,
            SendMessageW, SetForegroundWindow, SetWindowLongPtrW, ShowWindow, TranslateMessage,
            CREATESTRUCTW, CW_USEDEFAULT, GWLP_USERDATA, IDC_ARROW, MSG, SW_SHOW, WM_CLOSE,
            WM_COMMAND, WM_CREATE, WM_CTLCOLORSTATIC, WM_DRAWITEM, WM_ERASEBKGND, WM_NCCREATE,
            WM_NCDESTROY, WNDCLASSW, WS_CAPTION, WS_CHILD, WS_POPUP, WS_SYSMENU, WS_TABSTOP,
            WS_VISIBLE,
        },
    },
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SurfaceFamily {
    Mist,
    Dark,
}

// BPSR corners are restrained: mostly clipped/small-radius glass panels, never
// large Material pills.  Values are logical px at the app's existing DPI model.
pub const RADIUS_SMALL: i32 = 3;
pub const RADIUS_MEDIUM: i32 = 4;
pub const RADIUS_LARGE: i32 = 6;

// Dark glass: overlays, meters, mechanics, chat, quick actions.
pub const DARK_BG: u32 = crate::ui_theme::rgb(14, 23, 29);
pub const DARK_SURFACE: u32 = crate::ui_theme::rgb(20, 32, 39);
pub const DARK_RAISED: u32 = crate::ui_theme::rgb(28, 42, 50);
pub const DARK_HOVER: u32 = crate::ui_theme::rgb(37, 54, 63);
pub const DARK_PRESSED: u32 = crate::ui_theme::rgb(44, 63, 72);
pub const DARK_INPUT: u32 = crate::ui_theme::rgb(23, 36, 43);
pub const DARK_BORDER: u32 = crate::ui_theme::rgb(69, 89, 99);
pub const DARK_BORDER_STRONG: u32 = crate::ui_theme::rgb(93, 115, 125);

// Mist glass: Settings and editors.  It is deliberately cooler/lighter than
// overlays while retaining enough contrast to work without real blur/acrylic.
pub const MIST_BG: u32 = crate::ui_theme::rgb(47, 62, 70);
pub const MIST_SIDEBAR: u32 = crate::ui_theme::rgb(39, 53, 61);
pub const MIST_SURFACE: u32 = crate::ui_theme::rgb(61, 78, 86);
pub const MIST_RAISED: u32 = crate::ui_theme::rgb(70, 88, 96);
pub const MIST_HOVER: u32 = crate::ui_theme::rgb(82, 102, 110);
pub const MIST_PRESSED: u32 = crate::ui_theme::rgb(91, 112, 120);
pub const MIST_INPUT: u32 = crate::ui_theme::rgb(43, 58, 66);
pub const MIST_BORDER: u32 = crate::ui_theme::rgb(102, 126, 136);
pub const MIST_BORDER_STRONG: u32 = crate::ui_theme::rgb(139, 165, 174);
pub const MIST_SELECTED: u32 = crate::ui_theme::rgb(62, 94, 104);
pub const MIST_SELECTED_HOVER: u32 = crate::ui_theme::rgb(72, 108, 118);

pub const BPSR_TEXT: u32 = crate::ui_theme::rgb(239, 246, 248);
pub const BPSR_TEXT_SECONDARY: u32 = crate::ui_theme::rgb(200, 214, 219);
pub const BPSR_MUTED: u32 = crate::ui_theme::rgb(156, 176, 183);
pub const BPSR_DISABLED: u32 = crate::ui_theme::rgb(121, 139, 145);
pub const BPSR_ACCENT: u32 = crate::ui_theme::rgb(117, 211, 236);
pub const BPSR_ACCENT_HOVER: u32 = crate::ui_theme::rgb(143, 224, 244);
pub const BPSR_ACCENT_PRESSED: u32 = crate::ui_theme::rgb(83, 176, 204);
pub const BPSR_ACCENT_TEXT: u32 = crate::ui_theme::rgb(14, 31, 38);
pub const BPSR_WARNING: u32 = crate::ui_theme::rgb(226, 157, 75);
pub const BPSR_DANGER: u32 = crate::ui_theme::rgb(205, 83, 89);
pub const BPSR_DANGER_HOVER: u32 = crate::ui_theme::rgb(225, 101, 106);

// Compatibility names used by final generated-source patches.  These are dark
// glass because the callers are overlay/detail surfaces.
pub const SELECTED_SURFACE: u32 = crate::ui_theme::rgb(30, 62, 72);
pub const SELECTED_SURFACE_HOVER: u32 = crate::ui_theme::rgb(38, 75, 86);
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
const BUTTON_SUBCLASS_ID: usize = 0x5241_4254;
const FIELD_SUBCLASS_ID: usize = 0x5241_4644;
const TME_LEAVE_: u32 = 0x0000_0002;
const BS_OWNERDRAW_: u32 = 0x000B;
const SS_LEFT_: u32 = 0x0000;
const IDOK_: i32 = 1;
const IDYES_: i32 = 6;
const IDNO_: i32 = 7;
const MB_YESNO_: u32 = 0x0000_0004;
const MB_ICONERROR_: u32 = 0x0000_0010;
const MB_ICONWARNING_: u32 = 0x0000_0030;

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
    fn DefSubclassProc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT;
}

#[link(name = "user32")]
extern "system" {
    fn TrackMouseEvent(event: *mut NativeTrackMouseEvent) -> i32;
}

#[link(name = "dwmapi")]
extern "system" {
    fn DwmSetWindowAttribute(hwnd: HWND, attribute: u32, value: *const c_void, size: u32) -> i32;
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

fn body_font() -> HFONT {
    static FONT: OnceLock<usize> = OnceLock::new();
    *FONT.get_or_init(|| make_font(-14, 400)) as HFONT
}

fn medium_font() -> HFONT {
    static FONT: OnceLock<usize> = OnceLock::new();
    *FONT.get_or_init(|| make_font(-14, 500)) as HFONT
}

fn heading_font() -> HFONT {
    static FONT: OnceLock<usize> = OnceLock::new();
    *FONT.get_or_init(|| make_font(-16, 600)) as HFONT
}

fn family_bg(family: SurfaceFamily) -> u32 {
    match family { SurfaceFamily::Mist => MIST_BG, SurfaceFamily::Dark => DARK_BG }
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
    // A one-pixel lower shadow gives the BPSR floating-panel separation without
    // pretending GDI has acrylic/blur.
    let shadow = RECT { left: rect.left + 1, top: rect.top + 2, right: rect.right + 1, bottom: rect.bottom + 2 };
    fill_round_rect(hdc, shadow, crate::ui_theme::rgb(7, 13, 17), RADIUS_LARGE);
    fill_round_rect(hdc, rect, if elevated { PANEL_RAISED } else { PANEL_SURFACE }, RADIUS_LARGE);
    stroke_round_rect(hdc, rect, DARK_BORDER, RADIUS_LARGE, 1);
}

pub unsafe fn fill_mist_background(hdc: HDC, rect: &RECT) {
    let brush = mist_bg_brush();
    FillRect(hdc, rect, brush);
}

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

pub unsafe fn mist_titlebar(hwnd: HWND) {
    if hwnd.is_null() { return; }
    let enabled: i32 = 1;
    let _ = DwmSetWindowAttribute(hwnd, 20, (&enabled as *const i32).cast(), std::mem::size_of::<i32>() as u32);
    let rounded: i32 = 1; // DWMWCP_ROUNDSMALL
    let _ = DwmSetWindowAttribute(hwnd, 33, (&rounded as *const i32).cast(), std::mem::size_of::<i32>() as u32);
    let caption = MIST_SIDEBAR;
    let border = MIST_BORDER;
    let text = BPSR_TEXT;
    let _ = DwmSetWindowAttribute(hwnd, 35, (&caption as *const u32).cast(), std::mem::size_of::<u32>() as u32);
    let _ = DwmSetWindowAttribute(hwnd, 34, (&border as *const u32).cast(), std::mem::size_of::<u32>() as u32);
    let _ = DwmSetWindowAttribute(hwnd, 36, (&text as *const u32).cast(), std::mem::size_of::<u32>() as u32);
}

pub unsafe fn dark_titlebar(hwnd: HWND) {
    if hwnd.is_null() { return; }
    let enabled: i32 = 1;
    let _ = DwmSetWindowAttribute(hwnd, 20, (&enabled as *const i32).cast(), std::mem::size_of::<i32>() as u32);
    let rounded: i32 = 1;
    let _ = DwmSetWindowAttribute(hwnd, 33, (&rounded as *const i32).cast(), std::mem::size_of::<i32>() as u32);
    let caption = DARK_SURFACE;
    let border = DARK_BORDER;
    let text = BPSR_TEXT;
    let _ = DwmSetWindowAttribute(hwnd, 35, (&caption as *const u32).cast(), std::mem::size_of::<u32>() as u32);
    let _ = DwmSetWindowAttribute(hwnd, 34, (&border as *const u32).cast(), std::mem::size_of::<u32>() as u32);
    let _ = DwmSetWindowAttribute(hwnd, 36, (&text as *const u32).cast(), std::mem::size_of::<u32>() as u32);
}

unsafe fn draw_control_text(hdc: HDC, hwnd: HWND, mut rect: RECT, color: u32, align_left: bool, medium: bool) {
    let len = GetWindowTextLengthW(hwnd).max(0) as usize;
    let mut buf = vec![0u16; len + 1];
    let got = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32).max(0) as usize;
    SetBkMode(hdc, TRANSPARENT as i32);
    SetTextColor(hdc, color);
    SelectObject(hdc, if medium { medium_font() } else { body_font() });
    if align_left { rect.left += 12; }
    let flags = (if align_left { 0 } else { 0x0001 }) | 0x0004 | 0x0020 | 0x0800 | 0x8000;
    DrawTextW(hdc, buf.as_ptr(), got as i32, &mut rect, flags);
}

unsafe fn draw_button_family(item: *const DRAWITEMSTRUCT, selected: bool, primary: bool, danger: bool, nav: bool, family: SurfaceFamily) -> isize {
    if item.is_null() { return 0; }
    let item = &*item;
    let disabled = item.itemState & 0x0004 != 0;
    let pressed = item.itemState & 0x0001 != 0;
    let focused = item.itemState & 0x0010 != 0;
    let hot = item.itemState & 0x0040 != 0 || HOVERED_CONTROL.load(Ordering::Acquire) == item.hwndItem as isize;
    let mut rect = item.rcItem;
    rect.left += 1; rect.top += 1; rect.right -= 1; rect.bottom -= 1;

    let background = if disabled {
        family_surface(family)
    } else if selected {
        if pressed { family_pressed(family) } else if hot { family_selected_hover(family) } else { family_selected(family) }
    } else if primary {
        if pressed { BPSR_ACCENT_PRESSED } else if hot { BPSR_ACCENT_HOVER } else { BPSR_ACCENT }
    } else if danger {
        if pressed { BPSR_DANGER } else if hot { BPSR_DANGER_HOVER } else { crate::ui_theme::rgb(89, 49, 53) }
    } else if pressed {
        family_pressed(family)
    } else if hot {
        family_hover(family)
    } else if nav {
        match family { SurfaceFamily::Mist => MIST_SIDEBAR, SurfaceFamily::Dark => DARK_SURFACE }
    } else {
        family_raised(family)
    };

    // Navigation rows blend into their rail until active/hovered, matching BPSR's
    // icon/category rails instead of drawing a card around every item.
    if !nav || selected || hot {
        fill_round_rect(item.hDC, rect, background, if nav { RADIUS_MEDIUM } else { RADIUS_SMALL });
    }
    if nav && selected {
        fill_round_rect(item.hDC, RECT { left: rect.left + 4, top: rect.top + 6, right: rect.left + 7, bottom: rect.bottom - 6 }, BPSR_ACCENT, 1);
    } else if !nav {
        let border = if disabled { family_border(family) }
        else if primary || selected || focused { BPSR_ACCENT }
        else if danger { BPSR_DANGER }
        else { family_border(family) };
        stroke_round_rect(item.hDC, rect, border, RADIUS_SMALL, if focused { 2 } else { 1 });
    }
    if focused && nav { stroke_round_rect(item.hDC, rect, BPSR_ACCENT, RADIUS_MEDIUM, 1); }

    let text_color = if disabled { BPSR_DISABLED } else if primary { BPSR_ACCENT_TEXT } else { BPSR_TEXT };
    let mut text_rect = item.rcItem;
    if nav { text_rect.left += 15; }
    draw_control_text(item.hDC, item.hwndItem, text_rect, text_color, nav, nav && selected);
    1
}

// Configuration controls default to Mist Glass.  Dark overlays are custom-painted
// and use the explicit dark helpers/constants below.
pub unsafe fn draw_button(item: *const DRAWITEMSTRUCT, selected: bool, primary: bool, danger: bool, nav: bool) -> isize {
    draw_button_family(item, selected, primary, danger, nav, SurfaceFamily::Mist)
}

pub unsafe fn draw_dark_button(item: *const DRAWITEMSTRUCT, selected: bool, primary: bool, danger: bool, nav: bool) -> isize {
    draw_button_family(item, selected, primary, danger, nav, SurfaceFamily::Dark)
}

unsafe fn draw_combo_family(item: *const DRAWITEMSTRUCT, family: SurfaceFamily) -> isize {
    if item.is_null() { return 0; }
    let item = &*item;
    let disabled = item.itemState & 0x0004 != 0;
    let selected = item.itemState & 0x0001 != 0;
    let focused = item.itemState & 0x0010 != 0;
    let mut rect = item.rcItem;
    rect.left += 1; rect.top += 1; rect.right -= 1; rect.bottom -= 1;
    fill_round_rect(item.hDC, rect, if disabled { family_surface(family) } else if selected { family_selected(family) } else { family_input(family) }, RADIUS_SMALL);
    stroke_round_rect(item.hDC, rect, if !disabled && focused { BPSR_ACCENT } else { family_border(family) }, RADIUS_SMALL, if focused { 2 } else { 1 });

    let index = if item.itemID == u32::MAX { SendMessageW(item.hwndItem, 0x0147, 0, 0) } else { item.itemID as isize };
    if index >= 0 {
        let len = SendMessageW(item.hwndItem, 0x0149, index as usize, 0).max(0) as usize;
        let mut buf = vec![0u16; len + 1];
        let got = SendMessageW(item.hwndItem, 0x0148, index as usize, buf.as_mut_ptr() as isize).max(0) as usize;
        SetBkMode(item.hDC, TRANSPARENT as i32);
        SetTextColor(item.hDC, if disabled { BPSR_DISABLED } else { BPSR_TEXT });
        SelectObject(item.hDC, body_font());
        let mut text = item.rcItem; text.left += 10; text.right -= 8;
        DrawTextW(item.hDC, buf.as_ptr(), got as i32, &mut text, 0x0004 | 0x0020 | 0x0800 | 0x8000);
    }
    1
}

pub unsafe fn draw_combo(item: *const DRAWITEMSTRUCT) -> isize {
    draw_combo_family(item, SurfaceFamily::Mist)
}

fn class_name(hwnd: HWND) -> String {
    unsafe {
        let mut class = [0u16; 32];
        let len = GetClassNameW(hwnd, class.as_mut_ptr(), class.len() as i32).max(0) as usize;
        String::from_utf16_lossy(&class[..len])
    }
}

unsafe fn track_leave(hwnd: HWND) {
    let mut tracking = NativeTrackMouseEvent { cb_size: std::mem::size_of::<NativeTrackMouseEvent>() as u32, flags: TME_LEAVE_, hwnd_track: hwnd, hover_time: 0 };
    let _ = TrackMouseEvent(&mut tracking);
}

unsafe fn install_button_tracking(hwnd: HWND) {
    let _ = SetWindowSubclass(hwnd, Some(button_subclass_proc), BUTTON_SUBCLASS_ID, 0);
}
unsafe fn install_field_tracking(hwnd: HWND) {
    let _ = SetWindowSubclass(hwnd, Some(field_subclass_proc), FIELD_SUBCLASS_ID, 0);
}

pub unsafe fn theme_control(hwnd: HWND) {
    if hwnd.is_null() { return; }
    crate::ui_theme::theme_control(hwnd);
    let class = class_name(hwnd);
    if class.eq_ignore_ascii_case("BUTTON") { install_button_tracking(hwnd); }
    else if class.eq_ignore_ascii_case("EDIT") {
        SendMessageW(hwnd, EM_SETMARGINS_, EC_LEFTMARGIN_ | EC_RIGHTMARGIN_, (8 | (8 << 16)) as isize);
        install_field_tracking(hwnd);
    } else if class.eq_ignore_ascii_case("LISTBOX") { install_field_tracking(hwnd); }
}

pub unsafe fn theme_combo(hwnd: HWND) {
    if hwnd.is_null() { return; }
    crate::ui_theme::theme_combo(hwnd);
}

unsafe fn paint_field_border(hwnd: HWND) {
    let dc = GetDC(hwnd);
    if dc.is_null() { return; }
    let mut rect: RECT = std::mem::zeroed();
    GetClientRect(hwnd, &mut rect);
    let focused = FOCUSED_FIELD.load(Ordering::Acquire) == hwnd as isize;
    let hovered = HOVERED_CONTROL.load(Ordering::Acquire) == hwnd as isize;
    let border = if focused { BPSR_ACCENT } else if hovered { MIST_BORDER_STRONG } else { MIST_BORDER };
    stroke_round_rect(dc, RECT { left: 0, top: 0, right: rect.right, bottom: rect.bottom }, border, RADIUS_SMALL, if focused { 2 } else { 1 });
    ReleaseDC(hwnd, dc);
}

unsafe extern "system" fn button_subclass_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM, _id: usize, _data: usize) -> LRESULT {
    match msg {
        WM_MOUSEMOVE_ => {
            let previous = HOVERED_CONTROL.swap(hwnd as isize, Ordering::AcqRel);
            if previous != hwnd as isize {
                if previous != 0 { InvalidateRect(previous as HWND, null(), 0); }
                InvalidateRect(hwnd, null(), 0);
            }
            track_leave(hwnd);
        }
        WM_MOUSELEAVE_ => {
            if HOVERED_CONTROL.compare_exchange(hwnd as isize, 0, Ordering::AcqRel, Ordering::Acquire).is_ok() { InvalidateRect(hwnd, null(), 0); }
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

unsafe extern "system" fn field_subclass_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM, _id: usize, _data: usize) -> LRESULT {
    match msg {
        WM_SETFOCUS_ => {
            FOCUSED_FIELD.store(hwnd as isize, Ordering::Release);
            let result = DefSubclassProc(hwnd, msg, wparam, lparam);
            InvalidateRect(hwnd, null(), 0);
            return result;
        }
        WM_KILLFOCUS_ => {
            FOCUSED_FIELD.compare_exchange(hwnd as isize, 0, Ordering::AcqRel, Ordering::Acquire).ok();
            let result = DefSubclassProc(hwnd, msg, wparam, lparam);
            InvalidateRect(hwnd, null(), 0);
            return result;
        }
        WM_MOUSEMOVE_ => {
            let previous = HOVERED_CONTROL.swap(hwnd as isize, Ordering::AcqRel);
            if previous != hwnd as isize {
                if previous != 0 { InvalidateRect(previous as HWND, null(), 0); }
                InvalidateRect(hwnd, null(), 0);
            }
            track_leave(hwnd);
        }
        WM_MOUSELEAVE_ => {
            if HOVERED_CONTROL.compare_exchange(hwnd as isize, 0, Ordering::AcqRel, Ordering::Acquire).is_ok() { InvalidateRect(hwnd, null(), 0); }
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

// ---- BPSR utility modal ----------------------------------------------------

const DIALOG_CLASS: &str = "BPSRReadyAlertGlassDialogV1";

struct DialogState {
    body: String,
    yes_no: bool,
    result: *mut i32,
    kind_color: u32,
}

unsafe fn decode_wide(ptr: *const u16) -> String {
    if ptr.is_null() { return String::new(); }
    let mut len = 0usize;
    while *ptr.add(len) != 0 && len < 32_768 { len += 1; }
    String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len))
}

unsafe fn register_dialog_class(instance: HINSTANCE) -> bool {
    static READY: OnceLock<bool> = OnceLock::new();
    *READY.get_or_init(|| {
        let class = wide(DIALOG_CLASS);
        let wc = WNDCLASSW {
            lpfnWndProc: Some(dialog_proc),
            hInstance: instance,
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            hbrBackground: mist_bg_brush(),
            lpszClassName: class.as_ptr(),
            ..std::mem::zeroed()
        };
        RegisterClassW(&wc) != 0 || windows_sys::Win32::Foundation::GetLastError() == 1410
    })
}

unsafe fn dialog_child(parent: HWND, class: &str, text: &str, id: i32, x: i32, y: i32, w: i32, h: i32, extra: u32) -> HWND {
    let instance = GetModuleHandleW(null());
    let hwnd = CreateWindowExW(0, wide(class).as_ptr(), wide(text).as_ptr(), WS_CHILD | WS_VISIBLE | if id != 0 { WS_TABSTOP } else { 0 } | extra, x, y, w, h, parent, id as usize as _, instance, null_mut());
    if class.eq_ignore_ascii_case("BUTTON") { theme_control(hwnd); }
    else if class.eq_ignore_ascii_case("STATIC") && !hwnd.is_null() { SendMessageW(hwnd, 0x0030, body_font() as usize, 1); }
    hwnd
}

unsafe extern "system" fn dialog_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        let cs = lparam as *const CREATESTRUCTW;
        if !cs.is_null() { SetWindowLongPtrW(hwnd, GWLP_USERDATA, (*cs).lpCreateParams as isize); }
    }
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut DialogState;
    match msg {
        WM_CREATE => {
            mist_titlebar(hwnd);
            if !ptr.is_null() {
                let state = &*ptr;
                dialog_child(hwnd, "STATIC", "", 0, 18, 20, 4, 106, SS_LEFT_);
                let accent = GetDC(hwnd);
                if !accent.is_null() {
                    fill_round_rect(accent, RECT { left: 18, top: 20, right: 22, bottom: 126 }, state.kind_color, 1);
                    ReleaseDC(hwnd, accent);
                }
                dialog_child(hwnd, "STATIC", &state.body, 0, 34, 20, 402, 104, SS_LEFT_);
                if state.yes_no {
                    dialog_child(hwnd, "BUTTON", "Yes", IDYES_, 254, 142, 84, 30, BS_OWNERDRAW_);
                    dialog_child(hwnd, "BUTTON", "Later", IDNO_, 348, 142, 84, 30, BS_OWNERDRAW_);
                } else {
                    dialog_child(hwnd, "BUTTON", "OK", IDOK_, 348, 142, 84, 30, BS_OWNERDRAW_);
                }
            }
            0
        }
        WM_ERASEBKGND => {
            let mut r: RECT = std::mem::zeroed(); GetClientRect(hwnd, &mut r); fill_mist_background(wparam as HDC, &r); 1
        }
        WM_CTLCOLORSTATIC => {
            let hdc = wparam as HDC; SetBkMode(hdc, TRANSPARENT as i32); SetTextColor(hdc, BPSR_TEXT); mist_bg_brush() as LRESULT
        }
        WM_DRAWITEM => {
            let id = (wparam & 0xffff) as i32;
            draw_button(lparam as _, false, id == IDYES_ || id == IDOK_, false, false)
        }
        WM_COMMAND => {
            let id = (wparam & 0xffff) as i32;
            if matches!(id, IDOK_ | IDYES_ | IDNO_) {
                if !ptr.is_null() && !(*ptr).result.is_null() { *(*ptr).result = id; }
                DestroyWindow(hwnd); return 0;
            }
            0
        }
        WM_CLOSE => {
            if !ptr.is_null() && !(*ptr).result.is_null() { *(*ptr).result = if (*ptr).yes_no { IDNO_ } else { IDOK_ }; }
            DestroyWindow(hwnd); 0
        }
        WM_NCDESTROY => {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            if !ptr.is_null() { drop(Box::from_raw(ptr)); }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Signature-compatible replacement for MessageBoxW used by generated native
/// code.  This lets validation, updater and confirmation paths share the BPSR
/// Mist Glass language instead of falling back to a bright/classic Win32 box.
pub unsafe fn message_box_w(owner: HWND, text: *const u16, title: *const u16, flags: u32) -> i32 {
    let body = decode_wide(text);
    let title_text = decode_wide(title);
    let yes_no = flags & 0x0f == MB_YESNO_;
    let kind_color = if flags & MB_ICONERROR_ != 0 { BPSR_DANGER }
        else if flags & MB_ICONWARNING_ == MB_ICONWARNING_ { BPSR_WARNING }
        else { BPSR_ACCENT };
    let instance = GetModuleHandleW(null());
    if !register_dialog_class(instance) {
        return MessageBoxW(owner, text, title, flags);
    }
    let mut result = if yes_no { IDNO_ } else { IDOK_ };
    let state = Box::new(DialogState { body, yes_no, result: &mut result, kind_color });
    let state_ptr = Box::into_raw(state);
    let hwnd = CreateWindowExW(
        0, wide(DIALOG_CLASS).as_ptr(), wide(&title_text).as_ptr(), WS_POPUP | WS_CAPTION | WS_SYSMENU,
        CW_USEDEFAULT, CW_USEDEFAULT, 466, 215, owner, null_mut(), instance, state_ptr.cast::<c_void>(),
    );
    if hwnd.is_null() {
        drop(Box::from_raw(state_ptr));
        return MessageBoxW(owner, text, title, flags);
    }
    crate::ui::fit_window(hwnd, owner, true);
    if !owner.is_null() { EnableWindow(owner, 0); }
    ShowWindow(hwnd, SW_SHOW); SetForegroundWindow(hwnd);
    let mut msg: MSG = std::mem::zeroed();
    while IsWindow(hwnd) != 0 {
        let got = GetMessageW(&mut msg, null_mut(), 0, 0);
        if got <= 0 {
            if got == 0 { PostQuitMessage(0); }
            break;
        }
        TranslateMessage(&msg); DispatchMessageW(&msg);
    }
    if !owner.is_null() { EnableWindow(owner, 1); SetForegroundWindow(owner); }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geometry_matches_bpsr_restrained_corners() {
        assert!((2..=4).contains(&RADIUS_SMALL));
        assert!((3..=5).contains(&RADIUS_MEDIUM));
        assert!((5..=7).contains(&RADIUS_LARGE));
    }

    #[test]
    fn mist_and_dark_families_remain_distinct() {
        assert_ne!(MIST_BG, DARK_BG);
        assert_ne!(MIST_SURFACE, DARK_SURFACE);
        assert_ne!(SELECTED_SURFACE, BPSR_ACCENT);
    }
}
