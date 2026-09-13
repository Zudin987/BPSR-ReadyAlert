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
    let shadow = RECT { left: rect.left + 1, top: rect.top + 2, right: rect.right + 1, bottom: rect.bottom + 2 };
    fill_round_rect(hdc, shadow, crate::ui_theme::rgb(10, 11, 14), RADIUS_LARGE);
    fill_round_rect(hdc, rect, if elevated { PANEL_RAISED } else { PANEL_SURFACE }, RADIUS_LARGE);
    stroke_round_rect(hdc, rect, DARK_BORDER, RADIUS_LARGE, 1);
}

pub unsafe fn fill_mist_background(hdc: HDC, rect: &RECT) {
    FillRect(hdc, rect, mist_bg_brush());
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

unsafe fn style_titlebar(hwnd: HWND, family: SurfaceFamily) {
    if hwnd.is_null() { return; }
    let enabled: i32 = 1;
    let ptr = (&enabled as *const i32).cast::<c_void>();
    if DwmSetWindowAttribute(hwnd, 20, ptr, std::mem::size_of::<i32>() as u32) != 0 {
        let _ = DwmSetWindowAttribute(hwnd, 19, ptr, std::mem::size_of::<i32>() as u32);
    }
    // DWMWCP_ROUND: modern Windows frame treatment; this does not change sizing behavior.
    let rounded: i32 = 2;
    let _ = DwmSetWindowAttribute(hwnd, 33, (&rounded as *const i32).cast(), std::mem::size_of::<i32>() as u32);
    let caption = match family { SurfaceFamily::Mist => MIST_SIDEBAR, SurfaceFamily::Dark => DARK_SURFACE };
    let border = family_border(family);
    let text = BPSR_TEXT;
    let _ = DwmSetWindowAttribute(hwnd, 35, (&caption as *const u32).cast(), std::mem::size_of::<u32>() as u32);
    let _ = DwmSetWindowAttribute(hwnd, 34, (&border as *const u32).cast(), std::mem::size_of::<u32>() as u32);
    let _ = DwmSetWindowAttribute(hwnd, 36, (&text as *const u32).cast(), std::mem::size_of::<u32>() as u32);
}

pub unsafe fn mist_titlebar(hwnd: HWND) { style_titlebar(hwnd, SurfaceFamily::Mist); }
pub unsafe fn dark_titlebar(hwnd: HWND) { style_titlebar(hwnd, SurfaceFamily::Dark); }
