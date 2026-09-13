// Native utility modal. Behavior intentionally matches the existing MessageBoxW
// compatibility shim; only the paint system changes.
const DIALOG_CLASS: &str = "BPSRReadyAlertMaterialDialogV2";

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
                let dc = GetDC(hwnd);
                if !dc.is_null() {
                    fill_round_rect(dc, RECT { left: 18, top: 23, right: 24, bottom: 119 }, state.kind_color, 3);
                    ReleaseDC(hwnd, dc);
                }
                dialog_child(hwnd, "STATIC", &state.body, 0, 38, 20, 398, 106, SS_LEFT_);
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

pub unsafe fn message_box_w(owner: HWND, text: *const u16, title: *const u16, flags: u32) -> i32 {
    let body = decode_wide(text);
    let title_text = decode_wide(title);
    let yes_no = flags & 0x0f == MB_YESNO_;
    let kind_color = if flags & MB_ICONERROR_ != 0 { BPSR_DANGER }
        else if flags & MB_ICONWARNING_ == MB_ICONWARNING_ { BPSR_WARNING }
        else { BPSR_INFO };
    let instance = GetModuleHandleW(null());
    if !register_dialog_class(instance) { return MessageBoxW(owner, text, title, flags); }
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
    fn material_geometry_is_restrained_for_desktop_density() {
        assert_eq!((RADIUS_SMALL, RADIUS_MEDIUM, RADIUS_LARGE), (6, 10, 14));
        assert!(RADIUS_SMALL < RADIUS_MEDIUM && RADIUS_MEDIUM < RADIUS_LARGE);
    }

    #[test]
    fn tonal_hierarchy_and_selection_are_distinct() {
        assert_ne!(DARK_BG, DARK_SURFACE);
        assert_ne!(DARK_SURFACE, DARK_RAISED);
        assert_ne!(MIST_BG, MIST_SURFACE);
        assert_ne!(SELECTED_SURFACE, BPSR_ACCENT);
        assert_ne!(BPSR_TEXT, BPSR_TEXT_SECONDARY);
    }

    #[test]
    fn semantic_roles_do_not_collapse_into_primary() {
        assert_ne!(BPSR_SUCCESS, BPSR_ACCENT);
        assert_ne!(BPSR_WARNING, BPSR_ACCENT);
        assert_ne!(BPSR_DANGER, BPSR_ACCENT);
        assert_ne!(BPSR_LIVE, BPSR_ACCENT);
    }
}
