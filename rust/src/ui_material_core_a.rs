//! Pixel/Material-inspired native drawing system for ReadyAlert.
//!
//! This module is presentation-only. It preserves the existing Win32 controls,
//! commands, geometry, shortcuts and event flow while giving every native surface
//! one coherent 2026 dark desktop visual language. No web/runtime UI is involved.
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
            CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetClassNameW,
            GetClientRect, GetMessageW, GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW,
            IsWindow, LoadCursorW, MessageBoxW, PostQuitMessage, RegisterClassW, SendMessageW,
            SetForegroundWindow, SetWindowLongPtrW, ShowWindow, TranslateMessage, CREATESTRUCTW,
            CW_USEDEFAULT, GWLP_USERDATA, IDC_ARROW, MSG, SW_SHOW, WM_CLOSE, WM_COMMAND,
            WM_CREATE, WM_CTLCOLORSTATIC, WM_DRAWITEM, WM_ERASEBKGND, WM_NCCREATE,
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

// Desktop Material geometry: rounded enough to feel contemporary, restrained
// enough for dense Win32 data surfaces. These are logical px in the existing DPI model.
pub const RADIUS_SMALL: i32 = 6;
pub const RADIUS_MEDIUM: i32 = 10;
pub const RADIUS_LARGE: i32 = 14;
pub const RADIUS_DIALOG: i32 = 16;

// Canonical dark tonal hierarchy. Pure black/white are deliberately avoided.
pub const DARK_BG: u32 = crate::ui_theme::rgb(18, 19, 24);
pub const DARK_SURFACE: u32 = crate::ui_theme::rgb(25, 27, 32);
pub const DARK_RAISED: u32 = crate::ui_theme::rgb(31, 34, 40);
pub const DARK_HOVER: u32 = crate::ui_theme::rgb(40, 43, 51);
pub const DARK_PRESSED: u32 = crate::ui_theme::rgb(48, 52, 61);
pub const DARK_INPUT: u32 = crate::ui_theme::rgb(29, 31, 37);
pub const DARK_BORDER: u32 = crate::ui_theme::rgb(53, 56, 65);
pub const DARK_BORDER_STRONG: u32 = crate::ui_theme::rgb(78, 82, 94);

// Configuration surfaces use the same system with one extra tonal step. The old
// public names remain for compatibility with generated modules; visually this is
// one product, not a separate theme.
pub const MIST_BG: u32 = crate::ui_theme::rgb(23, 25, 30);
pub const MIST_SIDEBAR: u32 = crate::ui_theme::rgb(19, 21, 26);
pub const MIST_SURFACE: u32 = crate::ui_theme::rgb(31, 33, 39);
pub const MIST_RAISED: u32 = crate::ui_theme::rgb(37, 40, 47);
pub const MIST_HOVER: u32 = crate::ui_theme::rgb(45, 48, 57);
pub const MIST_PRESSED: u32 = crate::ui_theme::rgb(53, 57, 67);
pub const MIST_INPUT: u32 = crate::ui_theme::rgb(29, 32, 38);
pub const MIST_BORDER: u32 = crate::ui_theme::rgb(55, 59, 68);
pub const MIST_BORDER_STRONG: u32 = crate::ui_theme::rgb(82, 87, 99);
pub const MIST_SELECTED: u32 = crate::ui_theme::rgb(43, 54, 72);
pub const MIST_SELECTED_HOVER: u32 = crate::ui_theme::rgb(50, 63, 84);

// Material-like semantic roles.
pub const BPSR_TEXT: u32 = crate::ui_theme::rgb(235, 238, 245);
pub const BPSR_TEXT_SECONDARY: u32 = crate::ui_theme::rgb(195, 200, 211);
pub const BPSR_MUTED: u32 = crate::ui_theme::rgb(148, 154, 167);
pub const BPSR_DISABLED: u32 = crate::ui_theme::rgb(106, 111, 123);
pub const BPSR_ACCENT: u32 = crate::ui_theme::rgb(168, 199, 250);
pub const BPSR_ACCENT_HOVER: u32 = crate::ui_theme::rgb(185, 210, 252);
pub const BPSR_ACCENT_PRESSED: u32 = crate::ui_theme::rgb(137, 178, 238);
pub const BPSR_ACCENT_TEXT: u32 = crate::ui_theme::rgb(20, 42, 72);
pub const BPSR_SUCCESS: u32 = crate::ui_theme::rgb(129, 201, 149);
pub const BPSR_WARNING: u32 = crate::ui_theme::rgb(253, 214, 99);
pub const BPSR_DANGER: u32 = crate::ui_theme::rgb(242, 139, 130);
pub const BPSR_DANGER_HOVER: u32 = crate::ui_theme::rgb(255, 165, 156);
pub const BPSR_INFO: u32 = crate::ui_theme::rgb(138, 180, 248);
pub const BPSR_LIVE: u32 = crate::ui_theme::rgb(255, 138, 128);

pub const SELECTED_SURFACE: u32 = crate::ui_theme::rgb(44, 57, 77);
pub const SELECTED_SURFACE_HOVER: u32 = crate::ui_theme::rgb(51, 66, 88);
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
