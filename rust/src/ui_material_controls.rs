unsafe fn draw_control_text(hdc: HDC, hwnd: HWND, mut rect: RECT, color: u32, align_left: bool, medium: bool) {
    let len = GetWindowTextLengthW(hwnd).max(0) as usize;
    let mut buf = vec![0u16; len + 1];
    let got = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32).max(0) as usize;
    SetBkMode(hdc, TRANSPARENT as i32);
    SetTextColor(hdc, color);
    SelectObject(hdc, if medium { medium_font() } else { body_font() });
    if align_left { rect.left += 13; rect.right -= 8; }
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

    let error_container = crate::ui_theme::rgb(72, 42, 45);
    let error_container_hover = crate::ui_theme::rgb(88, 49, 52);
    let background = if disabled {
        family_surface(family)
    } else if selected {
        if pressed { family_pressed(family) } else if hot { family_selected_hover(family) } else { family_selected(family) }
    } else if primary {
        if pressed { BPSR_ACCENT_PRESSED } else if hot { BPSR_ACCENT_HOVER } else { BPSR_ACCENT }
    } else if danger {
        if pressed { crate::ui_theme::rgb(105, 55, 57) } else if hot { error_container_hover } else { error_container }
    } else if pressed {
        family_pressed(family)
    } else if hot {
        family_hover(family)
    } else if nav {
        match family { SurfaceFamily::Mist => MIST_SIDEBAR, SurfaceFamily::Dark => DARK_SURFACE }
    } else {
        family_raised(family)
    };

    if !nav || selected || hot || focused {
        fill_round_rect(item.hDC, rect, background, if nav { RADIUS_MEDIUM } else { RADIUS_SMALL });
    }

    if nav && selected {
        let marker = RECT { left: rect.left + 4, top: rect.top + 7, right: rect.left + 7, bottom: rect.bottom - 7 };
        fill_round_rect(item.hDC, marker, BPSR_ACCENT, 2);
    }

    if focused {
        stroke_round_rect(item.hDC, rect, BPSR_ACCENT, if nav { RADIUS_MEDIUM } else { RADIUS_SMALL }, 2);
    } else if !nav && !primary {
        let border = if danger { crate::ui_theme::rgb(112, 64, 65) } else if selected { family_border_strong(family) } else { family_border(family) };
        stroke_round_rect(item.hDC, rect, border, RADIUS_SMALL, 1);
    }

    let text_color = if disabled { BPSR_DISABLED }
        else if primary { BPSR_ACCENT_TEXT }
        else if danger { BPSR_TEXT }
        else { BPSR_TEXT };
    let mut text_rect = item.rcItem;
    if nav { text_rect.left += 14; }
    draw_control_text(item.hDC, item.hwndItem, text_rect, text_color, nav, selected || primary);
    1
}

pub unsafe fn draw_button(item: *const DRAWITEMSTRUCT, selected: bool, primary: bool, danger: bool, nav: bool) -> isize {
    draw_button_family(item, selected, primary, danger, nav, SurfaceFamily::Mist)
}

pub unsafe fn draw_dark_button(item: *const DRAWITEMSTRUCT, selected: bool, primary: bool, danger* bool, nav: bool) -> isize {
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
    let back = if disabled { family_surface(family) } else if selected { family_selected(family) } else { family_input(family) };
    fill_round_rect(item.hDC, rect, back, RADIUS_SMALL);
    stroke_round_rect(item.hDC, rect, if focused { BPSR_ACCENT } else { family_border(family) }, RADIUS_SMALL, if focused { 2 } else { 1 });

    let index = if item.itemID == u32::MAX { SendMessageW(item.hwndItem, 0x0147, 0, 0) } else { item.itemID as isize };
    if index >= 0 {
        let len = SendMessageW(item.hwndItem, 0x0149, index as usize, 0).max(0) as usize;
        let mut buf = vec![0u16; len + 1];
        let got = SendMessageW(item.hwndItem, 0x0148, index as usize, buf.as_mut_ptr() as isize).max(0) as usize;
        SetBkMode(item.hDC, TRANSPARENT as i32);
        SetTextColor(item.hDC, if disabled { BPSR_DISABLED } else { BPSR_TEXT });
        SelectObject(item.hDC, body_font());
        let mut text = item.rcItem; text.left += 11; text.right -= 10;
        DrawTextW(item.hDC, buf.as_ptr(), got as i32, &mut text, 0x0004 | 0x0020 | 0x0800 | 0x8000);
    }
    1
}

pub unsafe fn draw_combo(item: *const DRAWITEMSTRUCT) -> isize { draw_combo_family(item, SurfaceFamily::Mist) }

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

