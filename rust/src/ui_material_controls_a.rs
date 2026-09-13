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
