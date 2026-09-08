use std::{
    ptr::{null, null_mut},
    sync::{
        atomic::{AtomicBool, AtomicIsize, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
    Graphics::Gdi::{
        CreateSolidBrush, DeleteObject, FillRect, RedrawWindow, HDC, RDW_ALLCHILDREN, RDW_ERASE,
        RDW_INVALIDATE, RDW_UPDATENOW,
    },
    UI::WindowsAndMessaging::{
        CallWindowProcW, DefWindowProcW, FindWindowW, GetClientRect, IsWindow, SetWindowLongPtrW,
        GWLP_WNDPROC, WM_APP, WM_COMMAND, WM_ERASEBKGND, WM_NCDESTROY, WNDPROC,
    },
};

const SETTINGS_CLASS: &str = "BPSRReadyAlertRustSettingsV151";
const WM_SHOW_PAGE: u32 = WM_APP + 91;
const SETTINGS_BACKGROUND: u32 = rgb(22, 25, 30);

static OLD_PROC: AtomicIsize = AtomicIsize::new(0);
static SUBCLASSED_HWND: AtomicIsize = AtomicIsize::new(0);

pub fn start(stop: Arc<AtomicBool>) -> thread::JoinHandle<()> {
    thread::Builder::new()
        .name("readyalert-settings-paint".into())
        .spawn(move || {
            let class = wide(SETTINGS_CLASS);
            while !stop.load(Ordering::Relaxed) {
                unsafe {
                    let hwnd = FindWindowW(class.as_ptr(), null());
                    if !hwnd.is_null() && IsWindow(hwnd) != 0 {
                        let tracked = SUBCLASSED_HWND.load(Ordering::Acquire) as HWND;
                        if tracked != hwnd {
                            install(hwnd);
                        }
                    }
                }
                thread::sleep(Duration::from_millis(150));
            }
        })
        .expect("spawn settings repaint helper")
}

unsafe fn install(hwnd: HWND) {
    let previous = SetWindowLongPtrW(hwnd, GWLP_WNDPROC, settings_wnd_proc as *const () as isize);
    if previous == 0 {
        return;
    }
    OLD_PROC.store(previous, Ordering::Release);
    SUBCLASSED_HWND.store(hwnd as isize, Ordering::Release);
    redraw(hwnd);
}

unsafe extern "system" fn settings_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_ERASEBKGND {
        let hdc = wparam as HDC;
        let mut rect: RECT = std::mem::zeroed();
        if !hdc.is_null() && GetClientRect(hwnd, &mut rect) != 0 {
            let brush = CreateSolidBrush(SETTINGS_BACKGROUND);
            if !brush.is_null() {
                FillRect(hdc, &rect, brush);
                DeleteObject(brush);
            }
        }
        return 1;
    }

    let result = call_original(hwnd, msg, wparam, lparam);

    if msg == WM_COMMAND || msg == WM_SHOW_PAGE {
        redraw(hwnd);
    } else if msg == WM_NCDESTROY {
        if SUBCLASSED_HWND.load(Ordering::Acquire) == hwnd as isize {
            SUBCLASSED_HWND.store(0, Ordering::Release);
            OLD_PROC.store(0, Ordering::Release);
        }
    }

    result
}

unsafe fn call_original(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let previous = OLD_PROC.load(Ordering::Acquire);
    if previous == 0 {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }
    let proc: WNDPROC = std::mem::transmute(previous);
    CallWindowProcW(proc, hwnd, msg, wparam, lparam)
}

unsafe fn redraw(hwnd: HWND) {
    RedrawWindow(
        hwnd,
        null(),
        null_mut(),
        RDW_INVALIDATE | RDW_ERASE | RDW_ALLCHILDREN | RDW_UPDATENOW,
    );
}

const fn rgb(r: u8, g: u8, b: u8) -> u32 {
    r as u32 | ((g as u32) << 8) | ((b as u32) << 16)
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}
