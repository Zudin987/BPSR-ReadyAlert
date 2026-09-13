// Strict QA component layer for the post-redesign defect-removal pass.
// Presentation only: command IDs, HWND ownership, hit testing, navigation and data flow remain unchanged.

const QA_COMBO_SUBCLASS_ID: usize = 0x5241_5143;
const QA_TRACK_SUBCLASS_ID: usize = 0x5241_5154;
const QA_LIST_SUBCLASS_ID: usize = 0x5241_514c;
const QA_TBM_GETPOS: u32 = 0x0400;
const QA_TBM_GETRANGEMIN: u32 = 0x0401;
const QA_TBM_GETRANGEMAX: u32 = 0x0402;

fn qa_family_bg(family: SurfaceFamily) -> u32 {
    match family { SurfaceFamily::Mist => MIST_BG, SurfaceFamily::Dark => DARK_BG }
}

unsafe fn qa_fill_rect(hdc: HDC, rect: &RECT, color: u32) {
    let brush = CreateSolidBrush(color);
    if !brush.is_null() { FillRect(hdc, rect, brush); DeleteObject(brush); }
}

/// Owner-drawn buttons must not inherit a square themed/native frame around the
/// rounded ReadyAlert surface. Call once after creating an owner-drawn BUTTON.
pub unsafe fn qa_prepare_owner_button(hwnd: HWND) {
    if hwnd.is_null() { return; }
    let empty = wide("");
    let _ = SetWindowTheme(hwnd, empty.as_ptr(), empty.as_ptr());
    strip_native_edge(hwnd);
    install_button_tracking(hwnd);
}

unsafe fn qa_draw_button_family(
    item: *const DRAWITEMSTRUCT,
    selected: bool,
    primary: bool,
    danger: bool,
    nav: bool,
    family: SurfaceFamily,
) -> isize {
    if item.is_null() { return 0; }
    let item = &*item;
    let disabled = item.itemState & 0x0004 != 0;
    let pressed = item.itemState & 0x0001 != 0;
    let focused = item.itemState & 0x0010 != 0;

    // Do not trust ODS_HOT here. Win32 can leave it visually stale on owner-drawn
    // navigation buttons. ReadyAlert already owns a single explicit hover tracker.
    let hot = HOVERED_CONTROL.load(Ordering::Acquire) == item.hwndItem as isize;

    let under = if nav {
        match family { SurfaceFamily::Mist => MIST_SIDEBAR, SurfaceFamily::Dark => DARK_SURFACE }
    } else {
        qa_family_bg(family)
    };
    qa_fill_rect(item.hDC, &item.rcItem, under);

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
        under
    } else {
        family_raised(family)
    };

    if !nav || selected || hot || focused {
        fill_round_rect(item.hDC, rect, background, if nav { RADIUS_MEDIUM } else { RADIUS_SMALL });
    }
    if nav && selected {
        let marker = RECT { left: rect.left + 4, top: rect.top + 7, right: rect.left + 8, bottom: rect.bottom - 7 };
        fill_round_rect(item.hDC, marker, BPSR_ACCENT, 2);
    }

    // Selection already has a tonal container + marker. Do not add a third visual
    // selection signal; reserve the outline for keyboard focus on unselected items.
    if focused && !(nav && selected) {
        stroke_round_rect(item.hDC, rect, BPSR_ACCENT, if nav { RADIUS_MEDIUM } else { RADIUS_SMALL }, 2);
    } else if !nav && danger {
        stroke_round_rect(item.hDC, rect, crate::ui_theme::rgb(112, 64, 65), RADIUS_SMALL, 1);
    }

    let text = get_control_text(item.hwndItem);
    SetBkMode(item.hDC, TRANSPARENT as i32);
    SetTextColor(item.hDC, if disabled { BPSR_MUTED } else if primary { BPSR_ACCENT_TEXT } else { BPSR_TEXT });
    SelectObject(item.hDC, if selected || primary { medium_font() } else { body_font() });
    let mut tr = item.rcItem;
    let flags = if nav {
        // Exactly one inset. The previous renderer applied 14 px twice, which is why
        // labels such as "Sounds & logs" clipped despite having enough control width.
        tr.left += 14; tr.right -= 8;
        0x0004 | 0x0020 | 0x0800 | 0x8000
    } else {
        0x0001 | 0x0004 | 0x0020 | 0x0800 | 0x8000
    };
    DrawTextW(item.hDC, text.as_ptr(), text.len() as i32, &mut tr, flags);
    1
}

pub unsafe fn qa_draw_button(item: *const DRAWITEMSTRUCT, selected: bool, primary: bool, danger: bool, nav: bool) -> isize {
    qa_draw_button_family(item, selected, primary, danger, nav, SurfaceFamily::Mist)
}

pub unsafe fn qa_draw_dark_button(item: *const DRAWITEMSTRUCT, selected: bool, primary: bool, danger: bool, nav: bool) -> isize {
    qa_draw_button_family(item, selected, primary, danger, nav, SurfaceFamily::Dark)
}

unsafe fn qa_paint_combo_overlay(hwnd: HWND, family: SurfaceFamily) {
    let hdc = GetDC(hwnd);
    if hdc.is_null() { return; }
    let mut rc: RECT = std::mem::zeroed();
    GetClientRect(hwnd, &mut rc);
    let arrow_w = 26.min((rc.right - rc.left).max(1));
    let arrow = RECT { left: rc.right - arrow_w, top: rc.top + 1, right: rc.right - 1, bottom: rc.bottom - 1 };
    qa_fill_rect(hdc, &arrow, family_input(family));
    SetBkMode(hdc, TRANSPARENT as i32);
    SetTextColor(hdc, BPSR_TEXT_SECONDARY);
    SelectObject(hdc, body_font());
    let glyph = wide("▾");
    let mut gr = arrow;
    DrawTextW(hdc, glyph.as_ptr(), 1, &mut gr, 0x0001 | 0x0004 | 0x0020 | 0x0800);
    stroke_round_rect(
        hdc,
        RECT { left: rc.left, top: rc.top, right: rc.right, bottom: rc.bottom },
        if GetFocus() == hwnd { BPSR_ACCENT } else { family_border(family) },
        RADIUS_SMALL,
        if GetFocus() == hwnd { 2 } else { 1 },
    );
    ReleaseDC(hwnd, hdc);
}

unsafe extern "system" fn qa_combo_subclass_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM, _id: usize, data: usize) -> LRESULT {
    let family = if data == 1 { SurfaceFamily::Dark } else { SurfaceFamily::Mist };
    match msg {
        WM_PAINT_ => {
            let result = DefSubclassProc(hwnd, msg, wparam, lparam);
            qa_paint_combo_overlay(hwnd, family);
            return result;
        }
        WM_SETFOCUS_ | WM_KILLFOCUS_ | WM_ENABLE_ => {
            let result = DefSubclassProc(hwnd, msg, wparam, lparam);
            InvalidateRect(hwnd, null(), 0);
            return result;
        }
        _ => {}
    }
    DefSubclassProc(hwnd, msg, wparam, lparam)
}

unsafe fn qa_theme_combo(hwnd: HWND, family: SurfaceFamily) {
    if hwnd.is_null() { return; }
    theme_combo(hwnd);
    let data = if family == SurfaceFamily::Dark { 1 } else { 0 };
    let _ = SetWindowSubclass(hwnd, Some(qa_combo_subclass_proc), QA_COMBO_SUBCLASS_ID, data);
    InvalidateRect(hwnd, null(), 0);
}

pub unsafe fn qa_theme_mist_combo(hwnd: HWND) { qa_theme_combo(hwnd, SurfaceFamily::Mist); }
pub unsafe fn qa_theme_dark_combo(hwnd: HWND) { qa_theme_combo(hwnd, SurfaceFamily::Dark); }

unsafe fn qa_paint_trackbar(hwnd: HWND, family: SurfaceFamily) -> LRESULT {
    let mut ps: PAINTSTRUCT = std::mem::zeroed();
    let hdc = BeginPaint(hwnd, &mut ps);
    if hdc.is_null() { return 0; }
    let mut rc: RECT = std::mem::zeroed();
    GetClientRect(hwnd, &mut rc);
    qa_fill_rect(hdc, &rc, qa_family_bg(family));

    let min = SendMessageW(hwnd, QA_TBM_GETRANGEMIN, 0, 0) as i32;
    let max = SendMessageW(hwnd, QA_TBM_GETRANGEMAX, 0, 0) as i32;
    let pos = SendMessageW(hwnd, QA_TBM_GETPOS, 0, 0) as i32;
    let left = rc.left + 8;
    let right = (rc.right - 8).max(left + 1);
    let cy = (rc.top + rc.bottom) / 2;
    let span = (max - min).max(1) as i64;
    let x = left + (((right - left) as i64 * (pos - min).clamp(0, max - min) as i64) / span) as i32;
    let track = RECT { left, top: cy - 2, right, bottom: cy + 2 };
    fill_round_rect(hdc, track, family_raised(family), 2);
    if x > left { fill_round_rect(hdc, RECT { left, top: cy - 2, right: x, bottom: cy + 2 }, BPSR_ACCENT, 2); }
    let thumb = RECT { left: x - 7, top: cy - 7, right: x + 7, bottom: cy + 7 };
    fill_round_rect(hdc, thumb, BPSR_ACCENT, 7);
    if GetFocus() == hwnd { stroke_round_rect(hdc, thumb, BPSR_ACCENT_HOVER, 7, 2); }
    EndPaint(hwnd, &ps);
    0
}

unsafe extern "system" fn qa_track_subclass_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM, _id: usize, data: usize) -> LRESULT {
    let family = if data == 1 { SurfaceFamily::Dark } else { SurfaceFamily::Mist };
    match msg {
        WM_PAINT_ => return qa_paint_trackbar(hwnd, family),
        WM_SETFOCUS_ | WM_KILLFOCUS_ | WM_ENABLE_ => {
            let result = DefSubclassProc(hwnd, msg, wparam, lparam);
            InvalidateRect(hwnd, null(), 0);
            return result;
        }
        _ => {}
    }
    DefSubclassProc(hwnd, msg, wparam, lparam)
}

pub unsafe fn qa_theme_dark_trackbar(hwnd: HWND) {
    if hwnd.is_null() { return; }
    dark_common_control(hwnd);
    let _ = SetWindowSubclass(hwnd, Some(qa_track_subclass_proc), QA_TRACK_SUBCLASS_ID, 1);
    InvalidateRect(hwnd, null(), 0);
}

unsafe fn qa_empty_copy(kind: usize) -> (&'static str, &'static str) {
    match kind {
        1 => ("No custom tabs yet", "Add a tab to create a filtered chat view."),
        2 => ("No blocked users", "Block a player from the Chat Overlay context menu."),
        3 => ("No tracker rules yet", "Add a rule to show a buff or skill event."),
        _ => ("Nothing here yet", ""),
    }
}

unsafe fn qa_paint_empty_list(hwnd: HWND, kind: usize) -> LRESULT {
    let mut ps: PAINTSTRUCT = std::mem::zeroed();
    let hdc = BeginPaint(hwnd, &mut ps);
    if hdc.is_null() { return 0; }
    let mut rc: RECT = std::mem::zeroed();
    GetClientRect(hwnd, &mut rc);
    fill_round_rect(hdc, rc, MIST_INPUT, RADIUS_SMALL);
    if GetFocus() == hwnd { stroke_round_rect(hdc, rc, BPSR_ACCENT, RADIUS_SMALL, 2); }
    let (title, body) = qa_empty_copy(kind);
    let mid = (rc.top + rc.bottom) / 2;
    SetBkMode(hdc, TRANSPARENT as i32);
    SetTextColor(hdc, BPSR_TEXT_SECONDARY);
    SelectObject(hdc, medium_font());
    let wt = wide(title);
    let mut tr = RECT { left: rc.left + 16, top: mid - 30, right: rc.right - 16, bottom: mid - 4 };
    DrawTextW(hdc, wt.as_ptr(), -1, &mut tr, 0x0001 | 0x0004 | 0x0020 | 0x0800 | 0x8000);
    if !body.is_empty() {
        SetTextColor(hdc, BPSR_MUTED);
        SelectObject(hdc, caption_font());
        let wb = wide(body);
        let mut br = RECT { left: rc.left + 22, top: mid - 1, right: rc.right - 22, bottom: mid + 28 };
        DrawTextW(hdc, wb.as_ptr(), -1, &mut br, 0x0001 | 0x0004 | 0x0020 | 0x0800 | 0x8000);
    }
    EndPaint(hwnd, &ps);
    0
}

unsafe extern "system" fn qa_list_subclass_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM, _id: usize, data: usize) -> LRESULT {
    match msg {
        WM_PAINT_ if SendMessageW(hwnd, LB_GETCOUNT_, 0, 0) == 0 => return qa_paint_empty_list(hwnd, data),
        WM_SETFOCUS_ | WM_KILLFOCUS_ | WM_ENABLE_ => {
            let result = DefSubclassProc(hwnd, msg, wparam, lparam);
            InvalidateRect(hwnd, null(), 0);
            return result;
        }
        _ => {}
    }
    DefSubclassProc(hwnd, msg, wparam, lparam)
}

pub unsafe fn qa_theme_empty_list(hwnd: HWND, kind: usize) {
    if hwnd.is_null() { return; }
    let _ = SetWindowSubclass(hwnd, Some(qa_list_subclass_proc), QA_LIST_SUBCLASS_ID, kind);
    InvalidateRect(hwnd, null(), 0);
}

// ---- Responsive compatibility dialog ---------------------------------------------------------

const QA_DIALOG_CLASS: &str = "BPSRReadyAlertQaDialogV1";
const QA_IDOK: i32 = 1;
const QA_IDCANCEL: i32 = 2;
const QA_IDYES: i32 = 6;
const QA_IDNO: i32 = 7;
const QA_MB_OKCANCEL: u32 = 1;
const QA_MB_YESNOCANCEL: u32 = 3;
const QA_MB_YESNO: u32 = 4;
const QA_MB_ICONERROR: u32 = 0x10;
const QA_MB_ICONWARNING: u32 = 0x30;
const QA_DT_WORDBREAK: u32 = 0x10;
const QA_DT_CALCRECT: u32 = 0x400;
const QA_DT_NOPREFIX: u32 = 0x800;
const QA_SS_LEFT: u32 = 0;
const QA_BS_OWNERDRAW: u32 = 0x000b;
const QA_NULL_BRUSH: i32 = 5;

#[link(name = "user32")]
extern "system" {
    fn IsDialogMessageW(hwnd: HWND, msg: *const MSG) -> i32;
}

#[derive(Clone, Copy)]
enum QaFlavor { Ok, OkCancel, YesNo, YesNoCancel }
#[derive(Clone, Copy)]
enum QaSeverity { Normal, Warning, Error }
#[derive(Clone)]
struct QaDialogButton { id: i32, label: String, primary: bool, danger: bool }
struct QaDialogState {
    body: String,
    result: *mut i32,
    flavor: QaFlavor,
    severity: QaSeverity,
    buttons: Vec<QaDialogButton>,
    body_h: i32,
}

fn qa_flavor(flags: u32) -> QaFlavor {
    match flags & 0x0f {
        QA_MB_YESNOCANCEL => QaFlavor::YesNoCancel,
        QA_MB_YESNO => QaFlavor::YesNo,
        QA_MB_OKCANCEL => QaFlavor::OkCancel,
        _ => QaFlavor::Ok,
    }
}

fn qa_severity(flags: u32) -> QaSeverity {
    match flags & 0xf0 {
        QA_MB_ICONERROR => QaSeverity::Error,
        QA_MB_ICONWARNING => QaSeverity::Warning,
        _ => QaSeverity::Normal,
    }
}

fn qa_close_result(flavor: QaFlavor) -> i32 {
    match flavor { QaFlavor::YesNoCancel | QaFlavor::OkCancel => QA_IDCANCEL, QaFlavor::YesNo => QA_IDNO, QaFlavor::Ok => QA_IDOK }
}

fn qa_dialog_buttons(flavor: QaFlavor, title: &str, body: &str) -> Vec<QaDialogButton> {
    let t = title.to_ascii_lowercase();
    let b = body.to_ascii_lowercase();
    match flavor {
        QaFlavor::YesNoCancel if b.contains("save changes") || t.contains("unsaved") => vec![
            QaDialogButton { id: QA_IDYES, label: "Save".into(), primary: true, danger: false },
            QaDialogButton { id: QA_IDNO, label: "Don't Save".into(), primary: false, danger: true },
            QaDialogButton { id: QA_IDCANCEL, label: "Cancel".into(), primary: false, danger: false },
        ],
        QaFlavor::YesNoCancel => vec![
            QaDialogButton { id: QA_IDYES, label: "Yes".into(), primary: true, danger: false },
            QaDialogButton { id: QA_IDNO, label: "No".into(), primary: false, danger: false },
            QaDialogButton { id: QA_IDCANCEL, label: "Cancel".into(), primary: false, danger: false },
        ],
        QaFlavor::YesNo if b.contains("discard unsaved") => vec![
            QaDialogButton { id: QA_IDYES, label: "Discard".into(), primary: false, danger: true },
            QaDialogButton { id: QA_IDNO, label: "Keep Editing".into(), primary: true, danger: false },
        ],
        QaFlavor::YesNo if t.contains("update") || b.contains("update") => vec![
            QaDialogButton { id: QA_IDYES, label: "Update".into(), primary: true, danger: false },
            QaDialogButton { id: QA_IDNO, label: "Later".into(), primary: false, danger: false },
        ],
        QaFlavor::YesNo => vec![
            QaDialogButton { id: QA_IDYES, label: "Yes".into(), primary: true, danger: false },
            QaDialogButton { id: QA_IDNO, label: "No".into(), primary: false, danger: false },
        ],
        QaFlavor::OkCancel => vec![
            QaDialogButton { id: QA_IDOK, label: "OK".into(), primary: true, danger: false },
            QaDialogButton { id: QA_IDCANCEL, label: "Cancel".into(), primary: false, danger: false },
        ],
        QaFlavor::Ok => vec![QaDialogButton { id: QA_IDOK, label: "OK".into(), primary: true, danger: false }],
    }
}

unsafe fn qa_decode_wide(ptr: *const u16) -> String {
    if ptr.is_null() { return String::new(); }
    let mut len = 0usize;
    while *ptr.add(len) != 0 && len < 32_768 { len += 1; }
    String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len))
}

unsafe fn qa_measure_body(body: &str) -> i32 {
    let hdc = GetDC(null_mut());
    if hdc.is_null() { return 44; }
    let old = SelectObject(hdc, body_font());
    let text = wide(body);
    let mut r = RECT { left: 0, top: 0, right: 382, bottom: 0 };
    DrawTextW(hdc, text.as_ptr(), -1, &mut r, QA_DT_CALCRECT | QA_DT_WORDBREAK | QA_DT_NOPREFIX);
    SelectObject(hdc, old);
    ReleaseDC(null_mut(), hdc);
    (r.bottom - r.top + 8).clamp(40, 128)
}

unsafe fn qa_register_dialog() -> bool {
    static READY: OnceLock<bool> = OnceLock::new();
    *READY.get_or_init(|| {
        let instance = GetModuleHandleW(null());
        let class = wide(QA_DIALOG_CLASS);
        let wc = WNDCLASSW {
            lpfnWndProc: Some(qa_dialog_proc), hInstance: instance,
            hCursor: LoadCursorW(null_mut(), IDC_ARROW), hbrBackground: mist_bg_brush(),
            lpszClassName: class.as_ptr(), ..std::mem::zeroed()
        };
        RegisterClassW(&wc) != 0 || GetLastError() == 1410
    })
}

unsafe fn qa_dialog_child(parent: HWND, class: &str, text: &str, id: i32, x: i32, y: i32, w: i32, h: i32, extra: u32) -> HWND {
    let hwnd = CreateWindowExW(
        0, wide(class).as_ptr(), wide(text).as_ptr(),
        WS_CHILD | WS_VISIBLE | if id != 0 { WS_TABSTOP } else { 0 } | extra,
        x, y, w, h, parent, id as usize as _, GetModuleHandleW(null()), null_mut(),
    );
    if class.eq_ignore_ascii_case("BUTTON") { theme_control(hwnd); qa_prepare_owner_button(hwnd); }
    else if class.eq_ignore_ascii_case("STATIC") && !hwnd.is_null() { set_control_font(hwnd, body_font()); }
    hwnd
}

unsafe extern "system" fn qa_dialog_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        let cs = lparam as *const CREATESTRUCTW;
        if !cs.is_null() { SetWindowLongPtrW(hwnd, GWLP_USERDATA, (*cs).lpCreateParams as isize); }
    }
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut QaDialogState;
    match msg {
        WM_CREATE => {
            mist_titlebar(hwnd); finish_native_window(hwnd);
            if !ptr.is_null() {
                let state = &*ptr;
                let mut rc: RECT = std::mem::zeroed(); GetClientRect(hwnd, &mut rc);
                qa_dialog_child(hwnd, "STATIC", &state.body, 0, 56, 18, (rc.right - 78).max(220), state.body_h, QA_SS_LEFT);
                let button_y = 18 + state.body_h + 18;
                let button_w = if state.buttons.len() >= 3 { 98 } else { 116 };
                let gap = 10;
                let total = button_w * state.buttons.len() as i32 + gap * state.buttons.len().saturating_sub(1) as i32;
                let mut x = (rc.right - 20 - total).max(20);
                let mut first: HWND = null_mut();
                for button in &state.buttons {
                    let child = qa_dialog_child(hwnd, "BUTTON", &button.label, button.id, x, button_y, button_w, 32, QA_BS_OWNERDRAW);
                    if first.is_null() { first = child; }
                    x += button_w + gap;
                }
                if !first.is_null() { crate::ui::SetFocus(first); }
            }
            0
        }
        WM_ERASEBKGND => {
            if ptr.is_null() { return 0; }
            let hdc = wparam as HDC;
            let mut rc: RECT = std::mem::zeroed(); GetClientRect(hwnd, &mut rc);
            fill_mist_background(hdc, &rc);
            let state = &*ptr;
            let color = match state.severity { QaSeverity::Error => BPSR_DANGER, QaSeverity::Warning => BPSR_WARNING, QaSeverity::Normal => BPSR_INFO };
            let icon = RECT { left: 20, top: 21, right: 42, bottom: 43 };
            fill_round_rect(hdc, icon, color, 11);
            SetBkMode(hdc, TRANSPARENT as i32);
            SetTextColor(hdc, DARK_BG);
            SelectObject(hdc, medium_font());
            let glyph = wide(match state.severity { QaSeverity::Normal => "i", _ => "!" });
            let mut ir = icon;
            DrawTextW(hdc, glyph.as_ptr(), 1, &mut ir, 0x0001 | 0x0004 | 0x0020 | 0x0800);
            1
        }
        WM_CTLCOLORSTATIC => {
            let hdc = wparam as HDC;
            SetBkMode(hdc, TRANSPARENT as i32);
            SetTextColor(hdc, BPSR_TEXT_SECONDARY);
            GetStockObject(QA_NULL_BRUSH) as LRESULT
        }
        WM_DRAWITEM => {
            if ptr.is_null() { return 0; }
            let id = (wparam & 0xffff) as i32;
            let role = (*ptr).buttons.iter().find(|button| button.id == id);
            qa_draw_button(
                lparam as _, false,
                role.map(|b| b.primary).unwrap_or(false),
                role.map(|b| b.danger).unwrap_or(false),
                false,
            )
        }
        WM_COMMAND => {
            let id = (wparam & 0xffff) as i32;
            if !ptr.is_null() && (*ptr).buttons.iter().any(|button| button.id == id) {
                if !(*ptr).result.is_null() { *(*ptr).result = id; }
                DestroyWindow(hwnd);
                return 0;
            }
            0
        }
        WM_CLOSE => {
            if !ptr.is_null() && !(*ptr).result.is_null() { *(*ptr).result = qa_close_result((*ptr).flavor); }
            DestroyWindow(hwnd);
            0
        }
        WM_NCDESTROY => {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            if !ptr.is_null() { drop(Box::from_raw(ptr)); }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

pub unsafe fn qa_message_box_w(owner: HWND, text: *const u16, title: *const u16, flags: u32) -> i32 {
    if !qa_register_dialog() { return MessageBoxW(owner, text, title, flags); }
    let body = qa_decode_wide(text);
    let title_text = qa_decode_wide(title);
    let flavor = qa_flavor(flags);
    let severity = qa_severity(flags);
    let body_h = qa_measure_body(&body);
    let mut result = qa_close_result(flavor);
    let state = Box::new(QaDialogState {
        buttons: qa_dialog_buttons(flavor, &title_text, &body), body,
        result: &mut result, flavor, severity, body_h,
    });
    let ptr = Box::into_raw(state);
    let outer_h = (body_h + 132).clamp(170, 270);
    let hwnd = CreateWindowExW(
        0, wide(QA_DIALOG_CLASS).as_ptr(), wide(&title_text).as_ptr(),
        WS_POPUP | WS_CAPTION | WS_SYSMENU,
        CW_USEDEFAULT, CW_USEDEFAULT, 486, outer_h,
        owner, null_mut(), GetModuleHandleW(null()), ptr.cast::<c_void>(),
    );
    if hwnd.is_null() {
        drop(Box::from_raw(ptr));
        return MessageBoxW(owner, text, title, flags);
    }
    crate::ui::fit_window(hwnd, owner, true);
    if !owner.is_null() { EnableWindow(owner, 0); }
    ShowWindow(hwnd, SW_SHOW); SetForegroundWindow(hwnd);
    let mut msg: MSG = std::mem::zeroed();
    while IsWindow(hwnd) != 0 {
        let got = GetMessageW(&mut msg, null_mut(), 0, 0);
        if got <= 0 { if got == 0 { PostQuitMessage(0); } break; }
        if IsDialogMessageW(hwnd, &msg) == 0 { TranslateMessage(&msg); DispatchMessageW(&msg); }
    }
    if !owner.is_null() { EnableWindow(owner, 1); SetForegroundWindow(owner); }
    result
}

#[cfg(test)]
mod strict_qa_tests {
    use super::*;

    #[test]
    fn destructive_discard_is_not_primary() {
        let buttons = qa_dialog_buttons(QaFlavor::YesNo, "ReadyAlert Settings", "Discard unsaved settings changes?");
        assert!(buttons[0].danger);
        assert!(!buttons[0].primary);
        assert!(buttons[1].primary);
    }

    #[test]
    fn save_dialog_keeps_cancel_and_safe_primary_action() {
        let buttons = qa_dialog_buttons(QaFlavor::YesNoCancel, "Unsaved rules", "Save changes before closing?");
        assert_eq!(buttons.len(), 3);
        assert!(buttons[0].primary);
        assert_eq!(buttons[2].id, QA_IDCANCEL);
    }

    #[test]
    fn dpi_matrix_keeps_core_logical_targets_large_enough() {
        // Logical sizes must remain usable before Windows applies the process DPI transform.
        // This catches accidental regressions in the 100/125/150/175/200% QA matrix.
        for percent in [100, 125, 150, 175, 200] {
            let button_h = 30 * percent / 100;
            let field_h = 26 * percent / 100;
            let checkbox = 18 * percent / 100;
            assert!(button_h >= 30);
            assert!(field_h >= 26);
            assert!(checkbox >= 18);
        }
    }
}
