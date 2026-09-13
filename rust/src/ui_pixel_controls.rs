const BS_TYPEMASK_: isize = 0x000f;
const BS_CHECKBOX_: isize = 0x0002;
const BS_AUTOCHECKBOX_: isize = 0x0003;
const BS_RADIOBUTTON_: isize = 0x0004;
const BS_3STATE_: isize = 0x0005;
const BS_AUTO3STATE_: isize = 0x0006;
const BS_AUTORADIOBUTTON_: isize = 0x0009;
const BM_GETCHECK_: u32 = 0x00f0;
const BST_CHECKED_: isize = 0x0001;
const BST_INDETERMINATE_: isize = 0x0002;
const LB_GETCURSEL_: u32 = 0x0188;
const LB_GETTEXT_: u32 = 0x0189;
const LB_GETTEXTLEN_: u32 = 0x018a;
const LB_GETCOUNT_: u32 = 0x018b;
const LB_GETTOPINDEX_: u32 = 0x018e;
const LB_GETITEMHEIGHT_: u32 = 0x01a1;

fn class_name(hwnd: HWND) -> String {
    unsafe {
        let mut class = [0u16; 32];
        let len = GetClassNameW(hwnd, class.as_mut_ptr(), class.len() as i32).max(0) as usize;
        String::from_utf16_lossy(&class[..len])
    }
}

fn button_type(hwnd: HWND) -> isize {
    unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE_) & BS_TYPEMASK_ }
}

fn is_check_or_radio(hwnd: HWND) -> bool {
    matches!(button_type(hwnd), BS_CHECKBOX_ | BS_AUTOCHECKBOX_ | BS_RADIOBUTTON_ | BS_3STATE_ | BS_AUTO3STATE_ | BS_AUTORADIOBUTTON_)
}

fn is_radio(hwnd: HWND) -> bool {
    matches!(button_type(hwnd), BS_RADIOBUTTON_ | BS_AUTORADIOBUTTON_)
}

unsafe fn set_control_font(hwnd: HWND, font: HFONT) {
    if !hwnd.is_null() { SendMessageW(hwnd, 0x0030, font as usize, 1); }
}

unsafe fn install_button_tracking(hwnd: HWND) {
    let _ = SetWindowSubclass(hwnd, Some(button_subclass_proc), BUTTON_SUBCLASS_ID, 0);
}

unsafe fn install_field_tracking(hwnd: HWND) {
    let _ = SetWindowSubclass(hwnd, Some(field_subclass_proc), FIELD_SUBCLASS_ID, 0);
}

unsafe fn install_list_tracking(hwnd: HWND) {
    let _ = SetWindowSubclass(hwnd, Some(list_subclass_proc), LIST_SUBCLASS_ID, 0);
}

/// Apply the Pixel presentation layer to an existing native control without changing
/// its command ID, tab order, click behavior, geometry or accessibility semantics.
pub unsafe fn theme_control(hwnd: HWND) {
    if hwnd.is_null() { return; }
    set_control_font(hwnd, body_font());
    dark_common_control(hwnd);
    let class = class_name(hwnd);
    if class.eq_ignore_ascii_case("BUTTON") {
        install_button_tracking(hwnd);
    } else if class.eq_ignore_ascii_case("EDIT") {
        strip_native_edge(hwnd);
        SendMessageW(hwnd, EM_SETMARGINS_, EC_LEFTMARGIN_ | EC_RIGHTMARGIN_, (10 | (10 << 16)) as isize);
        install_field_tracking(hwnd);
    } else if class.eq_ignore_ascii_case("LISTBOX") {
        strip_native_edge(hwnd);
        install_list_tracking(hwnd);
    }
}

pub unsafe fn theme_combo(hwnd: HWND) {
    if hwnd.is_null() { return; }
    set_control_font(hwnd, body_font());
    let cfd = wide("DarkMode_CFD");
    if SetWindowTheme(hwnd, cfd.as_ptr(), null()) != 0 { dark_common_control(hwnd); }
}

unsafe fn get_control_text(hwnd: HWND) -> Vec<u16> {
    let len = GetWindowTextLengthW(hwnd).max(0) as usize;
    let mut buf = vec![0u16; len + 1];
    let got = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32).max(0) as usize;
    buf.truncate(got);
    buf
}

unsafe fn draw_control_text(hdc: HDC, hwnd: HWND, mut rect: RECT, color: u32, align_left: bool, medium: bool) {
    let buf = get_control_text(hwnd);
    SetBkMode(hdc, TRANSPARENT as i32);
    SetTextColor(hdc, color);
    SelectObject(hdc, if medium { medium_font() } else { body_font() });
    if align_left { rect.left += 14; rect.right -= 8; }
    let flags = (if align_left { 0 } else { 0x0001 }) | 0x0004 | 0x0020 | 0x0800 | 0x8000;
    DrawTextW(hdc, buf.as_ptr(), buf.len() as i32, &mut rect, flags);
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
    let background = if disabled {
        family_surface(family)
    } else if selected {
        if pressed { family_pressed(family) } else if hot { family_selected_hover(family) } else { family_selected(family) }
    } else if primary {
        if pressed { BPSR_ACCENT_PRESSED } else if hot { BPSR_ACCENT_HOVER } else { BPSR_ACCENT }
    } else if danger {
        if pressed { crate::ui_theme::rgb(105, 55, 57) } else if hot { crate::ui_theme::rgb(88, 49, 52) } else { error_container }
    } else if pressed {
        family_pressed(family)
    } else if hot {
        family_hover(family)
    } else if nav {
        match family { SurfaceFamily::Mist => MIST_SIDEBAR, SurfaceFamily::Dark => DARK_SURFACE }
    } else {
        family_raised(family)
    };

    // Pixel navigation is a quiet rail with one tonal selected container, not a
    // vertical stack of outlined rectangles.
    if !nav || selected || hot || focused {
        fill_round_rect(item.hDC, rect, background, if nav { RADIUS_MEDIUM } else { RADIUS_SMALL });
    }
    if nav && selected {
        let marker = RECT { left: rect.left + 4, top: rect.top + 7, right: rect.left + 8, bottom: rect.bottom - 7 };
        fill_round_rect(item.hDC, marker, BPSR_ACCENT, 2);
    }

    // Normal controls are surface-driven. A stroke appears only when it conveys
    // focus/selection/destructive semantics, avoiding the old box-within-box look.
    if focused {
        stroke_round_rect(item.hDC, rect, BPSR_ACCENT, if nav { RADIUS_MEDIUM } else { RADIUS_SMALL }, 2);
    } else if !nav && danger {
        stroke_round_rect(item.hDC, rect, crate::ui_theme::rgb(112, 64, 65), RADIUS_SMALL, 1);
    }

    let text_color = if disabled { BPSR_DISABLED }
        else if primary { BPSR_ACCENT_TEXT }
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
    if focused { stroke_round_rect(item.hDC, rect, BPSR_ACCENT, RADIUS_SMALL, 2); }

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

unsafe fn paint_checkbox(hwnd: HWND) -> LRESULT {
    let mut ps: PAINTSTRUCT = std::mem::zeroed();
    let hdc = BeginPaint(hwnd, &mut ps);
    if hdc.is_null() { return 0; }
    let mut client: RECT = std::mem::zeroed();
    GetClientRect(hwnd, &mut client);
    FillRect(hdc, &client, mist_bg_brush());

    let enabled = IsWindowEnabled(hwnd) != 0;
    let hovered = HOVERED_CONTROL.load(Ordering::Acquire) == hwnd as isize;
    let focused = GetFocus() == hwnd;
    let checked = SendMessageW(hwnd, BM_GETCHECK_, 0, 0);
    let radio = is_radio(hwnd);
    let side = 18.min((client.bottom - client.top - 4).max(12));
    let top = ((client.bottom - client.top - side) / 2).max(0);
    let glyph = RECT { left: 1, top, right: 1 + side, bottom: top + side };

    let active = checked == BST_CHECKED_ || checked == BST_INDETERMINATE_;
    let fill = if !enabled { MIST_SURFACE }
        else if active { if hovered { BPSR_ACCENT_HOVER } else { BPSR_ACCENT } }
        else if hovered { MIST_HOVER }
        else { MIST_RAISED };
    fill_round_rect(hdc, glyph, fill, if radio { side / 2 } else { 5 });
    if !active {
        stroke_round_rect(hdc, glyph, if focused { BPSR_ACCENT } else { MIST_BORDER_STRONG }, if radio { side / 2 } else { 5 }, if focused { 2 } else { 1 });
    }

    if checked == BST_CHECKED_ {
        if radio {
            let dot = RECT { left: glyph.left + 5, top: glyph.top + 5, right: glyph.right - 5, bottom: glyph.bottom - 5 };
            fill_round_rect(hdc, dot, BPSR_ACCENT_TEXT, 5);
        } else {
            let mark = wide("✓");
            let mut r = glyph;
            SetBkMode(hdc, TRANSPARENT as i32); SetTextColor(hdc, BPSR_ACCENT_TEXT); SelectObject(hdc, medium_font());
            DrawTextW(hdc, mark.as_ptr(), 1, &mut r, 0x0001 | 0x0004 | 0x0020 | 0x0800);
        }
    } else if checked == BST_INDETERMINATE_ {
        let dash = RECT { left: glyph.left + 4, top: glyph.top + side / 2 - 1, right: glyph.right - 4, bottom: glyph.top + side / 2 + 1 };
        fill_round_rect(hdc, dash, BPSR_ACCENT_TEXT, 1);
    }

    let text = get_control_text(hwnd);
    if !text.is_empty() {
        let mut text_rect = RECT { left: glyph.right + 9, top: client.top, right: client.right, bottom: client.bottom };
        SetBkMode(hdc, TRANSPARENT as i32);
        SetTextColor(hdc, if enabled { BPSR_TEXT } else { BPSR_DISABLED });
        SelectObject(hdc, body_font());
        DrawTextW(hdc, text.as_ptr(), text.len() as i32, &mut text_rect, 0x0004 | 0x0020 | 0x0800 | 0x8000);
    }
    EndPaint(hwnd, &ps);
    0
}

unsafe fn paint_field_border(hwnd: HWND) {
    let dc = GetDC(hwnd);
    if dc.is_null() { return; }
    let mut rect: RECT = std::mem::zeroed();
    GetClientRect(hwnd, &mut rect);
    let focused = FOCUSED_FIELD.load(Ordering::Acquire) == hwnd as isize;
    let hovered = HOVERED_CONTROL.load(Ordering::Acquire) == hwnd as isize;
    let color = if focused { BPSR_ACCENT } else if hovered { MIST_BORDER_STRONG } else { MIST_BORDER };
    stroke_round_rect(dc, RECT { left: 0, top: 0, right: rect.right, bottom: rect.bottom }, color, RADIUS_SMALL, if focused { 2 } else { 1 });
    ReleaseDC(hwnd, dc);
}

unsafe fn paint_listbox(hwnd: HWND) -> LRESULT {
    let mut ps: PAINTSTRUCT = std::mem::zeroed();
    let hdc = BeginPaint(hwnd, &mut ps);
    if hdc.is_null() { return 0; }
    let mut client: RECT = std::mem::zeroed(); GetClientRect(hwnd, &mut client);
    fill_round_rect(hdc, client, MIST_INPUT, RADIUS_SMALL);

    let count = SendMessageW(hwnd, LB_GETCOUNT_, 0, 0);
    let top = SendMessageW(hwnd, LB_GETTOPINDEX_, 0, 0).max(0) as i32;
    let selected = SendMessageW(hwnd, LB_GETCURSEL_, 0, 0) as i32;
    let item_h = SendMessageW(hwnd, LB_GETITEMHEIGHT_, 0, 0).max(18) as i32;
    let focused = GetFocus() == hwnd;
    if count > 0 {
        let mut index = top;
        let mut y = 3;
        while index < count as i32 && y < client.bottom {
            let len = SendMessageW(hwnd, LB_GETTEXTLEN_, index as usize, 0);
            if len >= 0 {
                let mut text = vec![0u16; len as usize + 1];
                let got = SendMessageW(hwnd, LB_GETTEXT_, index as usize, text.as_mut_ptr() as isize).max(0) as usize;
                let row = RECT { left: 4, top: y, right: client.right - 4, bottom: (y + item_h).min(client.bottom) };
                if index == selected {
                    fill_round_rect(hdc, row, if focused { MIST_SELECTED_HOVER } else { MIST_SELECTED }, 6);
                }
                let mut tr = RECT { left: row.left + 9, top: row.top, right: row.right - 7, bottom: row.bottom };
                SetBkMode(hdc, TRANSPARENT as i32);
                SetTextColor(hdc, if index == selected { BPSR_TEXT } else { BPSR_TEXT_SECONDARY });
                SelectObject(hdc, if index == selected { medium_font() } else { body_font() });
                DrawTextW(hdc, text.as_ptr(), got as i32, &mut tr, 0x0004 | 0x0020 | 0x0800 | 0x8000);
            }
            index += 1; y += item_h;
        }
    }
    if focused { stroke_round_rect(hdc, client, BPSR_ACCENT, RADIUS_SMALL, 2); }
    EndPaint(hwnd, &ps);
    0
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
        WM_SETFOCUS_ | WM_KILLFOCUS_ | WM_ENABLE_ => {
            let result = DefSubclassProc(hwnd, msg, wparam, lparam);
            InvalidateRect(hwnd, null(), 0);
            return result;
        }
        WM_PAINT_ if is_check_or_radio(hwnd) => return paint_checkbox(hwnd),
        _ => {}
    }
    DefSubclassProc(hwnd, msg, wparam, lparam)
}

unsafe extern "system" fn field_subclass_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM, _id: usize, _data: usize) -> LRESULT {
    match msg {
        WM_SETFOCUS_ => {
            FOCUSED_FIELD.store(hwnd as isize, Ordering::Release);
            let result = DefSubclassProc(hwnd, msg, wparam, lparam); InvalidateRect(hwnd, null(), 0); return result;
        }
        WM_KILLFOCUS_ => {
            FOCUSED_FIELD.compare_exchange(hwnd as isize, 0, Ordering::AcqRel, Ordering::Acquire).ok();
            let result = DefSubclassProc(hwnd, msg, wparam, lparam); InvalidateRect(hwnd, null(), 0); return result;
        }
        WM_MOUSEMOVE_ => {
            let previous = HOVERED_CONTROL.swap(hwnd as isize, Ordering::AcqRel);
            if previous != hwnd as isize { if previous != 0 { InvalidateRect(previous as HWND, null(), 0); } InvalidateRect(hwnd, null(), 0); }
            track_leave(hwnd);
        }
        WM_MOUSELEAVE_ => {
            if HOVERED_CONTROL.compare_exchange(hwnd as isize, 0, Ordering::AcqRel, Ordering::Acquire).is_ok() { InvalidateRect(hwnd, null(), 0); }
        }
        WM_PAINT_ => { let result = DefSubclassProc(hwnd, msg, wparam, lparam); paint_field_border(hwnd); return result; }
        WM_ENABLE_ => { let result = DefSubclassProc(hwnd, msg, wparam, lparam); InvalidateRect(hwnd, null(), 0); return result; }
        _ => {}
    }
    DefSubclassProc(hwnd, msg, wparam, lparam)
}

unsafe extern "system" fn list_subclass_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM, _id: usize, _data: usize) -> LRESULT {
    match msg {
        WM_PAINT_ => return paint_listbox(hwnd),
        WM_SETFOCUS_ | WM_KILLFOCUS_ | WM_ENABLE_ => {
            let result = DefSubclassProc(hwnd, msg, wparam, lparam); InvalidateRect(hwnd, null(), 0); return result;
        }
        WM_MOUSEMOVE_ => { track_leave(hwnd); }
        WM_MOUSELEAVE_ => { InvalidateRect(hwnd, null(), 0); }
        _ => {}
    }
    DefSubclassProc(hwnd, msg, wparam, lparam)
}
