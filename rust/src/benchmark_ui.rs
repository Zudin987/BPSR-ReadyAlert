use std::{ptr::null_mut, sync::{atomic::{AtomicIsize, Ordering}, OnceLock}};
use windows_sys::Win32::{
    Foundation::{GetLastError, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM},
    Graphics::Gdi::{FillRect, SetBkColor, SetBkMode, SetTextColor, HDC, TRANSPARENT},
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, GetDlgItem, GetWindowTextLengthW,
        GetWindowTextW, IsWindow, LoadCursorW, MessageBoxW, RegisterClassW,
        SetForegroundWindow, SetWindowTextW, ShowWindow, CREATESTRUCTW, CW_USEDEFAULT,
        GWLP_USERDATA, IDC_ARROW, MB_ICONWARNING, MB_OK, SW_SHOW, WM_CLOSE, WM_COMMAND,
        WM_CREATE, WM_CTLCOLORSTATIC, WM_CTLCOLOREDIT, WM_DRAWITEM, WM_ERASEBKGND,
        WM_NCCREATE, WM_NCDESTROY, WS_CAPTION, WS_CHILD, WS_EX_CLIENTEDGE,
        WS_EX_TOOLWINDOW, WS_POPUP, WS_SYSMENU, WS_TABSTOP, WS_VISIBLE, WNDCLASSW,
    },
};

const CLASS: &str = "BPSRReadyAlertBenchmarkV1271";
const ID_NAME: i32 = 5101;
const ID_SECONDS: i32 = 5102;
const ID_START: i32 = 5103;
const ID_CANCEL: i32 = 5104;
const ES_AUTOHSCROLL: u32 = 0x0080;
const ES_NUMBER: u32 = 0x2000;
const BS_OWNERDRAW: u32 = 0x000B;
const SS_LEFT: u32 = 0x0000;
const WS_EX_TOPMOST: u32 = 0x0000_0008;

// Preserve the existing dialog footprint from the screenshot baseline.
const WINDOW_W: i32 = 440;
const WINDOW_H: i32 = 285;
const PAD: i32 = 18;
const CONTENT_W: i32 = 388;

static CLASS_READY: OnceLock<()> = OnceLock::new();
static OPEN_HWND: AtomicIsize = AtomicIsize::new(0);

pub unsafe fn show(owner: HWND) {
    let existing = OPEN_HWND.load(Ordering::Acquire) as HWND;
    if !existing.is_null() && IsWindow(existing) != 0 {
        ShowWindow(existing, SW_SHOW);
        SetForegroundWindow(existing);
        return;
    }

    let instance = GetModuleHandleW(null_mut()) as HINSTANCE;
    if CLASS_READY.get().is_none() {
        let class = wide(CLASS);
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wnd_proc),
            hInstance: instance,
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            hbrBackground: crate::ui_theme::bg_brush(),
            lpszClassName: class.as_ptr(),
            ..std::mem::zeroed()
        };
        if RegisterClassW(&wc) == 0 && GetLastError() != 1410 {
            return;
        }
        let _ = CLASS_READY.set(());
    }

    let class = wide(CLASS);
    let title = wide("ReadyAlert Benchmark");
    let hwnd = CreateWindowExW(
        WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
        class.as_ptr(),
        title.as_ptr(),
        WS_POPUP | WS_CAPTION | WS_SYSMENU,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        WINDOW_W,
        WINDOW_H,
        owner,
        null_mut(),
        instance,
        null_mut(),
    );
    if hwnd.is_null() {
        return;
    }
    OPEN_HWND.store(hwnd as isize, Ordering::Release);
    crate::ui::fit_window(hwnd, owner, true);
    ShowWindow(hwnd, SW_SHOW);
    SetForegroundWindow(hwnd);
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        let create = lparam as *const CREATESTRUCTW;
        if !create.is_null() {
            windows_sys::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(
                hwnd,
                GWLP_USERDATA,
                (*create).lpCreateParams as isize,
            );
        }
    }
    match msg {
        WM_CREATE => {
            crate::ui_theme::dark_titlebar(hwnd);
            build_form(hwnd);
            0
        }
        WM_ERASEBKGND => {
            let mut rect: RECT = std::mem::zeroed();
            GetClientRect(hwnd, &mut rect);
            FillRect(wparam as HDC, &rect, crate::ui_theme::bg_brush());
            1
        }
        WM_CTLCOLORSTATIC => {
            let hdc = wparam as HDC;
            SetBkMode(hdc, TRANSPARENT as i32);
            SetTextColor(hdc, crate::ui_theme::TEXT_SECONDARY);
            crate::ui_theme::bg_brush() as LRESULT
        }
        WM_CTLCOLOREDIT => {
            let hdc = wparam as HDC;
            SetBkColor(hdc, crate::ui_theme::INPUT);
            SetTextColor(hdc, crate::ui_theme::TEXT);
            crate::ui_theme::input_brush() as LRESULT
        }
        WM_DRAWITEM => match (wparam & 0xffff) as i32 {
            ID_START => crate::ui_theme::draw_button(lparam as _, false, true, false, false),
            ID_CANCEL => crate::ui_theme::draw_button(lparam as _, false, false, false, false),
            _ => 0,
        },
        WM_COMMAND => {
            match (wparam & 0xffff) as i32 {
                ID_START => start(hwnd),
                ID_CANCEL => { DestroyWindow(hwnd); }
                _ => {}
            }
            0
        }
        WM_CLOSE => {
            DestroyWindow(hwnd);
            0
        }
        WM_NCDESTROY => {
            OPEN_HWND.compare_exchange(hwnd as isize, 0, Ordering::AcqRel, Ordering::Acquire).ok();
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn build_form(hwnd: HWND) {
    let instance = GetModuleHandleW(null_mut()) as HINSTANCE;

    child(hwnd, instance, "STATIC", "Benchmark name (optional)", PAD, 18, CONTENT_W, 18, SS_LEFT, 0);
    child(hwnd, instance, "EDIT", "", PAD, 40, CONTENT_W, 27, ES_AUTOHSCROLL, ID_NAME);

    child(hwnd, instance, "STATIC", "Duration (seconds)", PAD, 82, 170, 18, SS_LEFT, 0);
    let seconds = child(hwnd, instance, "EDIT", "", PAD, 104, 126, 27, ES_AUTOHSCROLL | ES_NUMBER, ID_SECONDS);
    let default_seconds = wide(&crate::telemetry::default_benchmark_seconds().to_string());
    SetWindowTextW(seconds, default_seconds.as_ptr());

    let info = child(
        hwnd,
        instance,
        "STATIC",
        "Starts when your first local damage/heal is detected, not when you click Start.\r\nThe meter resets automatically when the benchmark timer ends.",
        PAD,
        147,
        CONTENT_W,
        40,
        SS_LEFT,
        0,
    );
    crate::ui_theme::set_font(info, crate::ui_theme::FontRole::Secondary);

    child(hwnd, instance, "BUTTON", "Start", 238, 202, 82, 30, BS_OWNERDRAW, ID_START);
    child(hwnd, instance, "BUTTON", "Cancel", 330, 202, 82, 30, BS_OWNERDRAW, ID_CANCEL);
}

unsafe fn child(
    parent: HWND,
    instance: HINSTANCE,
    class: &str,
    text: &str,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    extra_style: u32,
    id: i32,
) -> HWND {
    let is_edit = class.eq_ignore_ascii_case("EDIT");
    let is_button = class.eq_ignore_ascii_case("BUTTON");
    let class_w = wide(class);
    let text = wide(text);
    let hwnd = CreateWindowExW(
        if is_edit { WS_EX_CLIENTEDGE } else { 0 },
        class_w.as_ptr(),
        text.as_ptr(),
        WS_CHILD | WS_VISIBLE | (if id != 0 { WS_TABSTOP } else { 0 }) | extra_style,
        x,
        y,
        w,
        h,
        parent,
        id as usize as _,
        instance,
        null_mut(),
    );
    if is_button { crate::ui_theme::theme_button(hwnd); }
    else { crate::ui_theme::theme_control(hwnd); }
    hwnd
}

unsafe fn start(hwnd: HWND) {
    let name = read_text(GetDlgItem(hwnd, ID_NAME));
    let seconds_text = read_text(GetDlgItem(hwnd, ID_SECONDS));
    let Some(seconds) = seconds_text.trim().parse::<u32>().ok().filter(|value| (1..=3_600).contains(value)) else {
        let title = wide("ReadyAlert Benchmark");
        let body = wide("Duration must be a whole number from 1 to 3600 seconds.");
        MessageBoxW(hwnd, body.as_ptr(), title.as_ptr(), MB_OK | MB_ICONWARNING);
        return;
    };
    crate::telemetry::arm_benchmark(name, seconds);
    DestroyWindow(hwnd);
}

unsafe fn read_text(hwnd: HWND) -> String {
    if hwnd.is_null() {
        return String::new();
    }
    let len = GetWindowTextLengthW(hwnd).max(0) as usize;
    let mut buf = vec![0u16; len.saturating_add(1)];
    let got = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32).max(0) as usize;
    String::from_utf16_lossy(&buf[..got])
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
