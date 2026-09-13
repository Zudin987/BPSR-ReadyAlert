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
    if class.eq_ignore_ascii_case("BUTTON") {
        install_button_tracking(hwnd);
    } else if class.eq_ignore_ascii_case("EDIT") {
        SendMessageW(hwnd, EM_SETMARGINS_, EC_LEFTMARGIN_ | EC_RIGHTMARGIN_, (9 | (9 << 16)) as isize);
        install_field_tracking(hwnd);
    } else if class.eq_ignore_ascii_case("LISTBOX") {
        install_field_tracking(hwnd);
    }
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
