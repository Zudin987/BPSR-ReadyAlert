// Included inside the existing native overlay module by build/design.rs.
// Reuse its measured columns and hit regions instead of creating a second meter.
unsafe fn paint_notmeter_gauge(hdc: HDC, state: &State, row: &DpsRow, r: RECT, leader: i64) {
    if !crate::appearance::is_notmeter() {
        return;
    }
    let bounds = RECT {
        left: r.left + 1,
        top: r.top + 1,
        right: r.right - 1,
        bottom: r.bottom - 1,
    };
    crate::appearance::capsule(hdc, bounds, crate::appearance::SURFACE);
    let width = crate::appearance::gauge_extent(
        bounds.right - bounds.left,
        mode_metric(row, state.sort_mode),
        leader,
    );
    if width > 0 {
        crate::appearance::capsule(
            hdc,
            RECT {
                right: bounds.left + width,
                ..bounds
            },
            crate::appearance::class_gauge_color(spec_accent_color(row)),
        );
    }
}

const DESIGN_THEME: i32 = 7350;
const DESIGN_NAV: i32 = 7360;
const DESIGN_LAYOUT: i32 = 7380;
const DESIGN_PREVIEW: i32 = 7390;
const DESIGN_LEFT: i32 = 180;
const DESIGN_TOP: i32 = 50;
const DESIGN_W: i32 = 980;
const DESIGN_H: i32 = 830;

unsafe fn build_feature_form(hwnd: HWND, state: &mut SettingsState) {
    build_classic_feature_form(hwnd, state);
    unsafe extern "system" fn shift(child: HWND, data: LPARAM) -> i32 {
        use windows_sys::Win32::UI::WindowsAndMessaging as wm;
        let owner = data as HWND;
        if wm::GetParent(child) != owner || matches!(wm::GetDlgCtrlID(child), 2 | 7300) {
            return 1;
        }
        let mut r: RECT = std::mem::zeroed();
        GetWindowRect(child, &mut r);
        windows_sys::Win32::Graphics::Gdi::MapWindowPoints(
            null_mut(),
            owner,
            (&mut r as *mut RECT).cast(),
            2,
        );
        wm::MoveWindow(
            child,
            r.left + DESIGN_LEFT,
            r.top + DESIGN_TOP,
            r.right - r.left,
            r.bottom - r.top,
            0,
        );
        1
    }
    windows_sys::Win32::UI::WindowsAndMessaging::EnumChildWindows(
        hwnd,
        Some(shift),
        hwnd as LPARAM,
    );
    let brand = feature_label(hwnd, 0, "ReadyAlert", 16, 20, 148, 28);
    crate::ui_theme::set_font(brand, crate::ui_theme::FontRole::Heading);
    feature_label(hwnd, 0, "SETTINGS", 16, 54, 148, 20);
    for (index, label) in [
        "Meter",
        "Combat Assist",
        "Alerts",
        "Chat & TTS",
        "History",
        "Appearance",
        "General",
        "Network",
    ]
    .iter()
    .enumerate()
    {
        feature_button(
            hwnd,
            DESIGN_NAV + index as i32,
            label,
            12,
            92 + index as i32 * 38,
            152,
        );
    }
    if state.kind == Kind::Dps {
        for (index, (label, width)) in [
            ("Normal", 76),
            ("Compact", 80),
            ("Raid", 64),
            ("Compact Raid", 116),
        ]
        .iter()
        .enumerate()
        {
            let x = [200, 284, 372, 444][index];
            feature_button(hwnd, DESIGN_LAYOUT + index as i32, label, x, 12, *width);
        }
    }
    feature_label(hwnd, 0, "Visual theme", 574, 66, 104, 24);
    feature_combo_sized(
        hwnd,
        DESIGN_THEME,
        684,
        60,
        190,
        &["NotMeter", "ReadyAlert Classic"],
    );
    refresh_design_theme(hwnd, state);
    create_design_preview(hwnd, state.parent);
    state.form.capture(hwnd, DESIGN_W, DESIGN_H);
    place_design_footer(hwnd);
}

unsafe fn refresh_design_theme(hwnd: HWND, state: &SettingsState) {
    let theme = state
        .features
        .read()
        .map(|f| f.visual_theme)
        .unwrap_or_default();
    SendMessageW(
        fc(hwnd, DESIGN_THEME),
        0x014e,
        (theme == crate::appearance::VisualTheme::Classic) as usize,
        0,
    );
}

unsafe fn design_settings_command(hwnd: HWND, state: &SettingsState, id: i32, code: u16) -> bool {
    if id == DESIGN_THEME {
        if code == 1 {
            let selected = SendMessageW(fc(hwnd, id), 0x0147, 0, 0);
            let theme = if selected == 1 {
                crate::appearance::VisualTheme::Classic
            } else {
                crate::appearance::VisualTheme::Notmeter
            };
            mutate_features(state, |f| f.visual_theme = theme);
            crate::appearance::set_theme(theme);
            crate::appearance::redraw_thread_windows();
            refresh_design_theme(hwnd, state);
        }
        return true;
    }
    if (DESIGN_LAYOUT..DESIGN_LAYOUT + 4).contains(&id) {
        let ptr = GetWindowLongPtrW(state.parent, GWLP_USERDATA) as *mut State;
        if !ptr.is_null() {
            let (compact, raid) = match id - DESIGN_LAYOUT {
                1 => (true, false),
                2 => (false, true),
                3 => (true, true),
                _ => (false, false),
            };
            reference_set_layout(state.parent, &mut *ptr, compact, raid);
            InvalidateRect(state.parent, null(), 0);
            for item in DESIGN_LAYOUT..DESIGN_LAYOUT + 4 {
                InvalidateRect(fc(hwnd, item), null(), 0);
            }
        }
        return true;
    }
    if (DESIGN_NAV..DESIGN_NAV + 8).contains(&id) {
        let ptr = GetWindowLongPtrW(state.parent, GWLP_USERDATA) as *mut State;
        if ptr.is_null() {
            return true;
        }
        match id - DESIGN_NAV {
            0 => show_settings(false),
            1 => show_settings(true),
            2 => {
                PostMessageW((*ptr).main_hwnd, 0x0111, 1010, 0);
            }
            3 => {
                PostMessageW((*ptr).main_hwnd, 0x0111, 1010, 3);
            }
            4 => crate::encounter_archive::open_local(hwnd),
            5 => {
                crate::ui::SetFocus(fc(hwnd, DESIGN_THEME));
                SendMessageW(fc(hwnd, DESIGN_THEME), 0x014f, 1, 0);
            }
            6 => {
                PostMessageW((*ptr).main_hwnd, 0x0111, 1010, 1);
            }
            7 => {
                PostMessageW((*ptr).main_hwnd, 0x0111, 1010, 6);
            }
            _ => {}
        }
        return true;
    }
    false
}

unsafe fn place_design_footer(hwnd: HWND) {
    let mut r: RECT = std::mem::zeroed();
    GetClientRect(hwnd, &mut r);
    let y = (r.bottom - 48).max(0);
    windows_sys::Win32::UI::WindowsAndMessaging::MoveWindow(
        fc(hwnd, 2),
        (r.right - 108).max(0),
        y,
        92,
        30,
        1,
    );
    let note = fc(hwnd, 7300);
    if !note.is_null() {
        windows_sys::Win32::UI::WindowsAndMessaging::MoveWindow(
            note,
            200.min((r.right - 300).max(0)),
            y + 4,
            280,
            24,
            1,
        );
    }
}

unsafe fn paint_design_settings(hdc: HDC, rc: RECT) {
    if crate::appearance::is_notmeter() {
        fill(
            hdc,
            &RECT {
                right: DESIGN_LEFT - 4,
                ..rc
            },
            crate::appearance::SIDEBAR,
        );
        fill(
            hdc,
            &RECT {
                left: DESIGN_LEFT - 4,
                right: DESIGN_LEFT - 3,
                ..rc
            },
            crate::appearance::BORDER,
        );
    }
}

unsafe fn create_design_preview(owner: HWND, overlay: HWND) {
    const CLASS: &str = "ReadyAlertDesignPreview";
    let instance = GetModuleHandleW(null());
    let class = wide(CLASS);
    let wc = WNDCLASSW {
        lpfnWndProc: Some(design_preview_proc),
        hInstance: instance,
        hCursor: LoadCursorW(null_mut(), IDC_ARROW),
        lpszClassName: class.as_ptr(),
        ..std::mem::zeroed()
    };
    if RegisterClassW(&wc) == 0 && GetLastError() != 1410 {
        return;
    }
    let child = CreateWindowExW(
        0,
        class.as_ptr(),
        wide("Live preview").as_ptr(),
        0x50000000,
        200,
        564,
        744,
        208,
        owner,
        DESIGN_PREVIEW as usize as _,
        instance,
        overlay.cast(),
    );
    if !child.is_null() {
        windows_sys::Win32::UI::WindowsAndMessaging::SetTimer(child, 1, 250, None);
    }
}

unsafe extern "system" fn design_preview_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_NCCREATE {
        let cs = lparam as *const CREATESTRUCTW;
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, (*cs).lpCreateParams as isize);
    }
    match msg {
        WM_ERASEBKGND => 1,
        0x0113 => {
            if windows_sys::Win32::UI::WindowsAndMessaging::IsWindowVisible(hwnd) != 0 {
                InvalidateRect(hwnd, null(), 0);
            }
            0
        }
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            let mut rc: RECT = std::mem::zeroed();
            GetClientRect(hwnd, &mut rc);
            if !hdc.is_null() {
                fill(hdc, &rc, crate::ui_modern::DARK_SURFACE);
                SetBkMode(hdc, TRANSPARENT as i32);
                SelectObject(hdc, dps_cached_font(100, false));
                SetTextColor(hdc, crate::ui_modern::BPSR_TEXT_SECONDARY);
                draw(
                    hdc,
                    "Live preview",
                    RECT {
                        left: 12,
                        top: 4,
                        right: rc.right - 12,
                        bottom: 28,
                    },
                    DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
                );
                let overlay = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as HWND;
                let ptr = if IsWindow(overlay) != 0 {
                    GetWindowLongPtrW(overlay, GWLP_USERDATA) as *mut State
                } else {
                    null_mut()
                };
                if !ptr.is_null() {
                    let state = &*ptr;
                    let save = windows_sys::Win32::Graphics::Gdi::SaveDC(hdc);
                    windows_sys::Win32::Graphics::Gdi::IntersectClipRect(
                        hdc,
                        8,
                        32,
                        rc.right - 8,
                        rc.bottom - 8,
                    );
                    windows_sys::Win32::Graphics::Gdi::SetViewportOrgEx(hdc, 0, 32, null_mut());
                    let scale = clamp_overlay_scale(state.scale_percent);
                    SetMapMode(hdc, 8);
                    SetWindowExtEx(hdc, 100, 100, null_mut());
                    SetViewportExtEx(hdc, scale, scale, null_mut());
                    let area = RECT {
                        left: 0,
                        top: 0,
                        right: physical_extent_to_logical(rc.right - 16, scale),
                        bottom: physical_extent_to_logical(rc.bottom - 40, scale),
                    };
                    if state.kind == Kind::Dps {
                        if meter_rows(state).is_empty() {
                            paint_design_sample(hdc, area);
                        } else {
                            paint_toolbar(hdc, area, state);
                            paint_dps(hdc, area, state);
                        }
                    } else {
                        paint_toolbar(hdc, area, state);
                        paint_mechanics(hdc, area, state);
                    }
                    windows_sys::Win32::Graphics::Gdi::RestoreDC(hdc, save);
                }
            }
            EndPaint(hwnd, &ps);
            0
        }
        WM_NCDESTROY => {
            windows_sys::Win32::UI::WindowsAndMessaging::KillTimer(hwnd, 1);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn paint_design_sample(hdc: HDC, rc: RECT) {
    SelectObject(hdc, dps_cached_font(100, true));
    SetTextColor(hdc, crate::ui_modern::BPSR_TEXT);
    draw(
        hdc,
        "Sample preview · live data appears during combat",
        RECT {
            left: 12,
            top: 2,
            right: rc.right - 12,
            bottom: 30,
        },
        DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS,
    );
    for (index, (name, spec, value, hue)) in [
        ("Player one", "Moonstrike", "591.9K /s", rgb(129, 182, 92)),
        ("Player two", "Frostbeam", "337.5K /s", rgb(104, 158, 217)),
        ("Player three", "Smite", "250.0K /s", rgb(191, 143, 78)),
    ]
    .iter()
    .enumerate()
    {
        let r = RECT {
            left: 12,
            top: 38 + index as i32 * 34,
            right: rc.right - 12,
            bottom: 68 + index as i32 * 34,
        };
        if crate::appearance::is_notmeter() {
            crate::appearance::capsule(hdc, r, crate::appearance::SURFACE);
            crate::appearance::capsule(
                hdc,
                RECT {
                    right: r.left + (r.right - r.left) * [100, 57, 42][index] / 100,
                    ..r
                },
                crate::appearance::class_gauge_color(*hue),
            );
        } else {
            fill(hdc, &r, crate::ui_modern::DARK_RAISED);
        }
        SetTextColor(hdc, crate::ui_modern::BPSR_TEXT);
        draw(
            hdc,
            name,
            RECT {
                left: r.left + 12,
                right: r.left + 160,
                ..r
            },
            DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
        SetTextColor(hdc, *hue);
        draw(
            hdc,
            spec,
            RECT {
                left: r.left + 170,
                right: r.right - 160,
                ..r
            },
            DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
        SetTextColor(hdc, crate::ui_modern::BPSR_TEXT);
        draw(
            hdc,
            value,
            RECT {
                left: r.right - 150,
                right: r.right - 12,
                ..r
            },
            DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
    }
}

unsafe fn audit_place_feature_footer(hwnd: HWND) {
    place_design_footer(hwnd);
}

unsafe fn paint_design_settings_button(
    item: *const windows_sys::Win32::UI::Controls::DRAWITEMSTRUCT,
    state: &SettingsState,
) -> isize {
    let id = (*item).CtlID as i32;
    let nav = (DESIGN_NAV..DESIGN_NAV + 8).contains(&id);
    let mut selected = nav && id == DESIGN_NAV + (state.kind == Kind::Mechanics) as i32;
    if (DESIGN_LAYOUT..DESIGN_LAYOUT + 4).contains(&id) {
        let ptr = GetWindowLongPtrW(state.parent, GWLP_USERDATA) as *mut State;
        if !ptr.is_null() {
            let current = match ((*ptr).compact_mode, raid_active(&*ptr)) {
                (true, true) => 3,
                (false, true) => 2,
                (true, false) => 1,
                _ => 0,
            };
            selected = id == DESIGN_LAYOUT + current;
        }
    }
    crate::ui_modern::qa_draw_dark_button(item, selected, false, false, nav)
}

#[cfg(test)]
#[path = "notmeter_ui_tests.rs"]
mod notmeter_ui_tests;
