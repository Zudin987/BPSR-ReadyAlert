use crate::{
    feature_settings::{
        self, FeatureSettings, ATTR_ACCURACY, ATTR_HASTE, ATTR_HEALING_MASTERY, ATTR_LUCK,
        ATTR_MASTERY, ATTR_VERSATILITY,
    },
    model::{DpsRow, DpsSnapshot, ImagineBadge, MechanicSnapshot},
    paths::AppPaths,
};
use std::{
    ffi::c_void,
    ptr::{null, null_mut},
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};
use windows_sys::Win32::{
    Foundation::{GetLastError, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM},
    Graphics::Gdi::{
        BeginPaint, CreateSolidBrush, DeleteObject, DrawTextW, EndPaint, FillRect, GetStockObject,
        InvalidateRect, SelectObject, SetBkMode, SetTextColor, DEFAULT_GUI_FONT, HDC, PAINTSTRUCT,
        TRANSPARENT,
    },
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, GetWindowLongPtrW,
        GetWindowRect, IsWindow, LoadCursorW, PostMessageW, RegisterClassW, SendMessageW,
        SetForegroundWindow, SetLayeredWindowAttributes, SetWindowLongPtrW, SetWindowPos, ShowWindow,
        TrackMouseEvent, CREATESTRUCTW, CW_USEDEFAULT, GWLP_USERDATA, HWND_TOPMOST, IDC_ARROW,
        LWA_ALPHA, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_SHOW, TME_LEAVE, TRACKMOUSEEVENT,
        WM_ERASEBKGND, WM_EXITSIZEMOVE, WM_LBUTTONDOWN, WM_MOUSELEAVE, WM_MOUSEMOVE,
        WM_MOUSEWHEEL, WM_NCCREATE, WM_NCDESTROY, WM_PAINT, WM_SIZE, WS_EX_LAYERED,
        WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_OVERLAPPEDWINDOW, WS_POPUP, WS_THICKFRAME, WNDCLASSW,
    },
};

const DPS_CLASS: &str = "BPSRReadyAlertDpsOverlayV170";
const MECH_CLASS: &str = "BPSRReadyAlertMechanicsOverlayV170";
const DETAIL_CLASS: &str = "BPSRReadyAlertDpsDetailV170";
const SETTINGS_CLASS: &str = "BPSRReadyAlertFeatureSettingsV170";
const TOOLBAR_H: i32 = 36;
const BUTTON_W: i32 = 34;
const COLLAPSED: i32 = 25;
const DPS_SUMMARY_H: i32 = 27;
const DPS_ROW_H: i32 = 47;
const MECH_ATTR_H: i32 = 30;
const MECH_ROW_H: i32 = 40;
const BADGE_W: i32 = 26;
const BADGE_GAP: i32 = 3;
const WM_NCLBUTTONDOWN_: u32 = 0x00A1;
const HTCAPTION_: usize = 2;
const DT_CENTER: u32 = 0x0001;
const DT_RIGHT: u32 = 0x0002;
const DT_VCENTER: u32 = 0x0004;
const DT_SINGLELINE: u32 = 0x0020;
const DT_NOPREFIX: u32 = 0x0800;
const DT_END_ELLIPSIS: u32 = 0x8000;

pub const CMD_HIDE_DPS: u32 = 1061;
pub const CMD_HIDE_MECHANICS: u32 = 1062;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Kind {
    Dps,
    Mechanics,
}

struct State {
    kind: Kind,
    main_hwnd: HWND,
    paths: AppPaths,
    features: Arc<RwLock<FeatureSettings>>,
    dps: DpsSnapshot,
    mechanics: MechanicSnapshot,
    collapsed: bool,
    expanded: RECT,
    scroll: usize,
    hover: Option<String>,
    hover_x: i32,
    hover_y: i32,
    mouse_tracking: bool,
    detail_hwnd: HWND,
    detail_uid: i64,
    settings_hwnd: HWND,
}

struct DetailState {
    row: DpsRow,
    scroll: usize,
}

struct SettingsState {
    kind: Kind,
    parent: HWND,
    paths: AppPaths,
    features: Arc<RwLock<FeatureSettings>>,
}

pub unsafe fn create_dps(
    instance: HINSTANCE,
    main_hwnd: HWND,
    features: Arc<RwLock<FeatureSettings>>,
    paths: AppPaths,
) -> Result<HWND, String> {
    create(instance, main_hwnd, features, paths, Kind::Dps)
}

pub unsafe fn create_mechanics(
    instance: HINSTANCE,
    main_hwnd: HWND,
    features: Arc<RwLock<FeatureSettings>>,
    paths: AppPaths,
) -> Result<HWND, String> {
    create(instance, main_hwnd, features, paths, Kind::Mechanics)
}

unsafe fn create(
    instance: HINSTANCE,
    main_hwnd: HWND,
    features: Arc<RwLock<FeatureSettings>>,
    paths: AppPaths,
    kind: Kind,
) -> Result<HWND, String> {
    let class_name = if kind == Kind::Dps { DPS_CLASS } else { MECH_CLASS };
    let class = wide(class_name);
    let wc = WNDCLASSW {
        lpfnWndProc: Some(wnd_proc),
        hInstance: instance,
        hCursor: LoadCursorW(null_mut(), IDC_ARROW),
        lpszClassName: class.as_ptr(),
        ..std::mem::zeroed()
    };
    if RegisterClassW(&wc) == 0 && GetLastError() != 1410 {
        return Err(format!("RegisterClassW({class_name}) failed: {}", GetLastError()));
    }

    let snap = features.read().map(|x| x.clone()).unwrap_or_default();
    let layout = if kind == Kind::Dps { snap.dps.clone() } else { snap.mechanics.clone() };
    let x = if layout.x == i32::MIN { CW_USEDEFAULT } else { layout.x };
    let y = if layout.y == i32::MIN { CW_USEDEFAULT } else { layout.y };
    let state = Box::new(State {
        kind,
        main_hwnd,
        paths,
        features,
        dps: DpsSnapshot::default(),
        mechanics: MechanicSnapshot::default(),
        collapsed: false,
        expanded: RECT {
            left: x,
            top: y,
            right: x.saturating_add(layout.width),
            bottom: y.saturating_add(layout.height),
        },
        scroll: 0,
        hover: None,
        hover_x: 0,
        hover_y: 0,
        mouse_tracking: false,
        detail_hwnd: null_mut(),
        detail_uid: 0,
        settings_hwnd: null_mut(),
    });
    let ptr = Box::into_raw(state);
    let title = if kind == Kind::Dps {
        "ReadyAlert DPS Meter"
    } else {
        "ReadyAlert Dungeon Mechanics"
    };
    let hwnd = CreateWindowExW(
        WS_EX_TOOLWINDOW | WS_EX_LAYERED | WS_EX_TOPMOST,
        class.as_ptr(),
        wide(title).as_ptr(),
        WS_POPUP | WS_THICKFRAME,
        x,
        y,
        layout.width,
        layout.height,
        null_mut(),
        null_mut(),
        instance,
        ptr.cast::<c_void>(),
    );
    if hwnd.is_null() {
        drop(Box::from_raw(ptr));
        return Err(format!("CreateWindowExW({class_name}) failed: {}", GetLastError()));
    }
    apply_opacity(hwnd, layout.opacity);
    SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE);
    Ok(hwnd)
}

pub unsafe fn update_dps(hwnd: HWND, snapshot: DpsSnapshot) {
    with_state(hwnd, |s| {
        let reset_scroll = snapshot.encounter_ms < s.dps.encounter_ms
            && !s.features.read().map(|f| f.meter.remember_scroll).unwrap_or(false);
        if reset_scroll {
            s.scroll = 0;
        }
        if !s.detail_hwnd.is_null() && IsWindow(s.detail_hwnd) != 0 && s.detail_uid != 0 {
            if let Some(row) = snapshot.rows.iter().find(|r| r.uid == s.detail_uid).cloned() {
                update_detail(s.detail_hwnd, row);
            }
        }
        s.scroll = s.scroll.min(snapshot.rows.len().saturating_sub(1));
        s.dps = snapshot;
    });
    InvalidateRect(hwnd, null(), 0);
}

pub unsafe fn update_mechanics(hwnd: HWND, snapshot: MechanicSnapshot) {
    with_state(hwnd, |s| s.mechanics = snapshot);
    InvalidateRect(hwnd, null(), 0);
}

pub unsafe fn expand(hwnd: HWND) {
    with_state(hwnd, |s| {
        if s.collapsed {
            expand_state(hwnd, s);
        }
    });
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        let cs = lparam as *const CREATESTRUCTW;
        if !cs.is_null() {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, (*cs).lpCreateParams as isize);
        }
    }
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut State;
    match msg {
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            if !ptr.is_null() {
                paint(hwnd, &mut *ptr);
            }
            0
        }
        WM_SIZE => {
            if !ptr.is_null() {
                clamp_scroll(hwnd, &mut *ptr);
            }
            InvalidateRect(hwnd, null(), 0);
            0
        }
        WM_LBUTTONDOWN => {
            if !ptr.is_null() {
                on_click(hwnd, &mut *ptr, lparam);
            }
            0
        }
        WM_MOUSEWHEEL => {
            if !ptr.is_null() {
                on_wheel(hwnd, &mut *ptr, wparam);
            }
            0
        }
        WM_MOUSEMOVE => {
            if !ptr.is_null() {
                on_mouse_move(hwnd, &mut *ptr, lparam);
            }
            0
        }
        WM_MOUSELEAVE => {
            if !ptr.is_null() {
                (*ptr).mouse_tracking = false;
                (*ptr).hover = None;
                InvalidateRect(hwnd, null(), 0);
            }
            0
        }
        WM_EXITSIZEMOVE => {
            if !ptr.is_null() {
                save_bounds(hwnd, &mut *ptr);
                clamp_scroll(hwnd, &mut *ptr);
            }
            0
        }
        WM_NCDESTROY => {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            if !ptr.is_null() {
                let s = &mut *ptr;
                for child in [s.detail_hwnd, s.settings_hwnd] {
                    if !child.is_null() && IsWindow(child) != 0 {
                        DestroyWindow(child);
                    }
                }
                drop(Box::from_raw(ptr));
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn on_click(hwnd: HWND, state: &mut State, lparam: LPARAM) {
    if state.collapsed {
        expand_state(hwnd, state);
        return;
    }
    let x = lo_signed(lparam);
    let y = hi_signed(lparam);
    let mut rc: RECT = std::mem::zeroed();
    GetClientRect(hwnd, &mut rc);
    if y < TOOLBAR_H {
        if x >= rc.right - BUTTON_W {
            PostMessageW(
                state.main_hwnd,
                0x0111,
                if state.kind == Kind::Dps { CMD_HIDE_DPS as usize } else { CMD_HIDE_MECHANICS as usize },
                0,
            );
        } else if x >= rc.right - BUTTON_W * 2 {
            collapse(hwnd, state);
        } else if x >= rc.right - BUTTON_W * 3 {
            open_feature_settings(hwnd, state);
        } else {
            extern "system" {
                fn ReleaseCapture() -> i32;
            }
            ReleaseCapture();
            SendMessageW(hwnd, WM_NCLBUTTONDOWN_, HTCAPTION_, 0);
        }
        return;
    }
    if state.kind == Kind::Dps {
        if let Some(row) = dps_row_at(hwnd, state, y).cloned() {
            open_detail(hwnd, state, row);
        }
    }
}

unsafe fn on_wheel(hwnd: HWND, state: &mut State, wparam: WPARAM) {
    if state.collapsed {
        return;
    }
    let delta = (((wparam >> 16) & 0xffff) as u16 as i16) as i32;
    if delta == 0 {
        return;
    }
    let steps = ((delta.abs() / 120).max(1) as usize) * 3;
    if delta > 0 {
        state.scroll = state.scroll.saturating_sub(steps);
    } else {
        state.scroll = state.scroll.saturating_add(steps);
    }
    clamp_scroll(hwnd, state);
    state.hover = None;
    InvalidateRect(hwnd, null(), 0);
}

unsafe fn on_mouse_move(hwnd: HWND, state: &mut State, lparam: LPARAM) {
    if state.collapsed || state.kind != Kind::Dps {
        return;
    }
    if !state.mouse_tracking {
        let mut tme = TRACKMOUSEEVENT {
            cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
            dwFlags: TME_LEAVE,
            hwndTrack: hwnd,
            dwHoverTime: 0,
        };
        TrackMouseEvent(&mut tme);
        state.mouse_tracking = true;
    }
    let x = lo_signed(lparam);
    let y = hi_signed(lparam);
    let next = badge_tooltip(hwnd, state, x, y);
    if next != state.hover || state.hover_x != x || state.hover_y != y {
        state.hover = next;
        state.hover_x = x;
        state.hover_y = y;
        InvalidateRect(hwnd, null(), 0);
    }
}

unsafe fn paint(hwnd: HWND, state: &mut State) {
    let mut ps: PAINTSTRUCT = std::mem::zeroed();
    let hdc = BeginPaint(hwnd, &mut ps);
    if hdc.is_null() {
        return;
    }
    let mut rc: RECT = std::mem::zeroed();
    GetClientRect(hwnd, &mut rc);
    fill(hdc, &rc, rgb(18, 22, 27));
    SelectObject(hdc, GetStockObject(DEFAULT_GUI_FONT));
    SetBkMode(hdc, TRANSPARENT as i32);

    if state.collapsed {
        SetTextColor(hdc, rgb(99, 199, 255));
        draw(
            hdc,
            if state.kind == Kind::Dps { "D" } else { "M" },
            rc,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
        EndPaint(hwnd, &ps);
        return;
    }

    let toolbar = RECT { left: 0, top: 0, right: rc.right, bottom: TOOLBAR_H };
    fill(hdc, &toolbar, rgb(23, 28, 35));
    fill(
        hdc,
        &RECT { left: 0, top: TOOLBAR_H - 2, right: rc.right, bottom: TOOLBAR_H },
        rgb(63, 133, 255),
    );
    SetTextColor(hdc, rgb(231, 237, 244));
    let title = toolbar_title(state);
    draw(
        hdc,
        &title,
        RECT { left: 10, top: 0, right: rc.right - BUTTON_W * 3 - 4, bottom: TOOLBAR_H },
        DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS,
    );
    SetTextColor(hdc, rgb(170, 183, 199));
    draw(
        hdc,
        "S",
        RECT { left: rc.right - BUTTON_W * 3, top: 0, right: rc.right - BUTTON_W * 2, bottom: TOOLBAR_H },
        DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
    );
    draw(
        hdc,
        "◀",
        RECT { left: rc.right - BUTTON_W * 2, top: 0, right: rc.right - BUTTON_W, bottom: TOOLBAR_H },
        DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
    );
    draw(
        hdc,
        "×",
        RECT { left: rc.right - BUTTON_W, top: 0, right: rc.right, bottom: TOOLBAR_H },
        DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
    );

    match state.kind {
        Kind::Dps => paint_dps(hdc, rc, state),
        Kind::Mechanics => paint_mechanics(hdc, rc, state),
    }
    if let Some(tip) = state.hover.as_deref() {
        paint_tooltip(hdc, rc, state.hover_x, state.hover_y, tip);
    }
    EndPaint(hwnd, &ps);
}

fn toolbar_title(state: &State) -> String {
    if state.kind == Kind::Mechanics {
        return "Dungeon Mechanics".into();
    }
    let show = state.features.read().map(|f| f.meter.show_target).unwrap_or(true);
    if !show {
        return "DPS Meter".into();
    }
    let Some(t) = state.dps.target.as_ref() else {
        return "DPS Meter".into();
    };
    let hp = if t.max_hp > 0 {
        format!("{:.1}%", (t.hp.max(0) as f64 * 100.0 / t.max_hp as f64).clamp(0.0, 100.0))
    } else {
        "?%".into()
    };
    let enrage = t
        .enrage_remaining_ms
        .filter(|x| *x >= 0)
        .map(|x| format!("  Enrage {}", format_time(x as u64)))
        .unwrap_or_default();
    format!("DPS Meter  |  Target: {} {}{}", t.name, hp, enrage)
}

unsafe fn paint_dps(hdc: HDC, rc: RECT, state: &State) {
    let s = &state.dps;
    let settings = state.features.read().map(|x| x.clone()).unwrap_or_default();
    let summary = format!(
        "{}   Total {}   H {}   Taken {}   Players {}",
        format_time(s.encounter_ms),
        compact(s.total_damage as f64),
        compact(s.total_healing as f64),
        compact(s.total_damage_taken as f64),
        s.rows.len()
    );
    SetTextColor(hdc, rgb(145, 160, 180));
    draw(
        hdc,
        &summary,
        RECT { left: 10, top: TOOLBAR_H + 2, right: rc.right - 10, bottom: TOOLBAR_H + DPS_SUMMARY_H },
        DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS,
    );

    let top = dps_rows_top();
    let visible = visible_dps_rows(rc.bottom);
    if s.rows.is_empty() {
        SetTextColor(hdc, rgb(132, 145, 162));
        draw(
            hdc,
            "Waiting for party / combat data...",
            RECT { left: 10, top: top + 18, right: rc.right - 10, bottom: top + 58 },
            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
        return;
    }

    for (screen_i, row) in s.rows.iter().skip(state.scroll).take(visible).enumerate() {
        let rank = state.scroll + screen_i + 1;
        let y = top + screen_i as i32 * DPS_ROW_H;
        let r = RECT { left: 6, top: y, right: rc.right - 8, bottom: y + DPS_ROW_H - 2 };
        fill(hdc, &r, if rank % 2 == 1 { rgb(28, 33, 40) } else { rgb(24, 29, 35) });
        let bar_w = ((r.right - r.left) as f64 * (row.damage_share / 100.0).clamp(0.0, 1.0)) as i32;
        if bar_w > 0 {
            fill(
                hdc,
                &RECT { left: r.left, top: r.bottom - 3, right: r.left + bar_w, bottom: r.bottom },
                rgb(92, 72, 72),
            );
        }

        SetTextColor(hdc, rgb(99, 199, 255));
        draw(
            hdc,
            &rank.to_string(),
            RECT { left: r.left + 4, top: r.top, right: r.left + 28, bottom: r.top + 24 },
            DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );

        let mut x = r.left + 30;
        if settings.meter.show_imagines {
            for badge in row.imagines.iter().take(3) {
                paint_badge(hdc, x, r.top + 3, badge);
                x += BADGE_W + BADGE_GAP;
            }
        }

        let info = identity_tail(row);
        let info_w = (rc.right / 3).clamp(145, 280);
        let name_right = (r.right - info_w - 4).max(x + 70);
        SetTextColor(hdc, if row.is_dead { rgb(255, 92, 92) } else { rgb(238, 242, 247) });
        draw(
            hdc,
            &row.name,
            RECT { left: x, top: r.top + 1, right: name_right, bottom: r.top + 24 },
            DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS,
        );
        SetTextColor(hdc, profession_color(row.profession_id));
        draw(
            hdc,
            &info,
            RECT { left: name_right + 4, top: r.top + 1, right: r.right - 5, bottom: r.top + 24 },
            DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS,
        );

        let line_y = r.top + 23;
        SetTextColor(hdc, rgb(188, 201, 218));
        draw(
            hdc,
            &format!("{}/s  {}", compact(row.dps), compact(row.damage as f64)),
            RECT { left: r.left + 30, top: line_y, right: r.left + 190, bottom: r.bottom },
            DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS,
        );
        let mut mx = r.left + 198;
        if settings.meter.show_damage_share {
            SetTextColor(hdc, rgb(255, 110, 110));
            draw_metric(hdc, &format!("Dmg {:.1}%", row.damage_share), &mut mx, r.bottom, 86);
        }
        if settings.meter.show_healing_share {
            SetTextColor(hdc, rgb(105, 225, 145));
            draw_metric(hdc, &format!("Heal {:.1}%", row.healing_share), &mut mx, r.bottom, 90);
        }
        if settings.meter.show_tank_share {
            SetTextColor(hdc, rgb(100, 180, 255));
            draw_metric(hdc, &format!("Tank {:.1}%", row.tank_share), &mut mx, r.bottom, 94);
        }
        if settings.meter.show_deaths {
            SetTextColor(hdc, if row.deaths > 0 { rgb(255, 140, 140) } else { rgb(170, 183, 199) });
            draw_metric(hdc, &format!("D {}", row.deaths), &mut mx, r.bottom, 44);
        }
    }
    paint_scrollbar(hdc, rc, s.rows.len(), visible, state.scroll, top, DPS_ROW_H);
}

unsafe fn draw_metric(hdc: HDC, text: &str, x: &mut i32, bottom: i32, width: i32) {
    draw(
        hdc,
        text,
        RECT { left: *x, top: bottom - 23, right: *x + width, bottom },
        DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS,
    );
    *x += width;
}

unsafe fn paint_badge(hdc: HDC, x: i32, y: i32, badge: &ImagineBadge) {
    let r = RECT { left: x, top: y, right: x + BADGE_W, bottom: y + 18 };
    fill(hdc, &r, badge_color(&badge.icon_key));
    SetTextColor(hdc, rgb(248, 250, 252));
    let label = if badge.icon_key.trim().is_empty() { "BI" } else { badge.icon_key.as_str() };
    draw(hdc, label, r, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
}

unsafe fn paint_mechanics(hdc: HDC, rc: RECT, state: &State) {
    let s = &state.mechanics;
    let features = state.features.read().map(|x| x.clone()).unwrap_or_default();
    let tracked: Vec<_> = features
        .mechanic_attributes
        .tracked
        .iter()
        .filter_map(|id| s.tracked_attributes.iter().find(|a| a.attr_id == *id))
        .collect();
    let mut top = TOOLBAR_H + 6;
    if !tracked.is_empty() {
        let strip = RECT { left: 6, top, right: rc.right - 6, bottom: top + MECH_ATTR_H - 3 };
        fill(hdc, &strip, rgb(27, 32, 39));
        let width = ((strip.right - strip.left) / tracked.len() as i32).max(72);
        for (i, attr) in tracked.iter().enumerate() {
            let left = strip.left + i as i32 * width;
            SetTextColor(hdc, attr_color(attr.attr_id));
            draw(
                hdc,
                &format!("{} {}", attr.label, compact_attr(attr.value)),
                RECT { left: left + 4, top: strip.top, right: (left + width - 3).min(strip.right), bottom: strip.bottom },
                DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS,
            );
        }
        top += MECH_ATTR_H;
    }
    let visible = ((rc.bottom - top) / MECH_ROW_H).max(0) as usize;
    let now = now_ms();
    let active: Vec<_> = s
        .rows
        .iter()
        .filter(|r| r.persistent || r.expires_unix_ms <= 0 || r.expires_unix_ms > now)
        .collect();
    if active.is_empty() {
        SetTextColor(hdc, rgb(132, 145, 162));
        draw(
            hdc,
            "No active mechanic",
            RECT { left: 10, top: top + 18, right: rc.right - 10, bottom: top + 58 },
            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
        return;
    }
    for (screen_i, row) in active.iter().skip(state.scroll).take(visible).enumerate() {
        let y = top + screen_i as i32 * MECH_ROW_H;
        let r = RECT { left: 6, top: y, right: rc.right - 8, bottom: y + MECH_ROW_H - 3 };
        fill(hdc, &r, if screen_i % 2 == 0 { rgb(28, 33, 40) } else { rgb(24, 29, 35) });
        let accent = if row.priority >= 3 { rgb(255, 99, 99) } else { rgb(99, 199, 255) };
        fill(hdc, &RECT { left: r.left, top: r.top, right: r.left + 3, bottom: r.bottom }, accent);
        SetTextColor(hdc, rgb(238, 242, 247));
        draw(
            hdc,
            &row.label,
            RECT { left: r.left + 10, top: r.top + 2, right: r.right - 78, bottom: r.top + 21 },
            DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS,
        );
        if let Some(target) = &row.target {
            SetTextColor(hdc, rgb(159, 221, 255));
            draw(
                hdc,
                target,
                RECT { left: r.left + 10, top: r.top + 19, right: r.right - 78, bottom: r.bottom },
                DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS,
            );
        }
        let timer = if row.persistent || row.expires_unix_ms <= 0 {
            "LIVE".to_string()
        } else {
            format!("{:.1}s", ((row.expires_unix_ms - now).max(0) as f64) / 1000.0)
        };
        SetTextColor(hdc, accent);
        draw(
            hdc,
            &timer,
            RECT { left: r.right - 72, top: r.top, right: r.right - 8, bottom: r.bottom },
            DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
    }
    paint_scrollbar(hdc, rc, active.len(), visible, state.scroll, top, MECH_ROW_H);
}

unsafe fn paint_scrollbar(
    hdc: HDC,
    rc: RECT,
    total: usize,
    visible: usize,
    scroll: usize,
    top: i32,
    _row_h: i32,
) {
    if total <= visible || visible == 0 {
        return;
    }
    let track_top = top + 2;
    let track_bottom = rc.bottom - 5;
    let track_h = (track_bottom - track_top).max(20);
    fill(
        hdc,
        &RECT { left: rc.right - 5, top: track_top, right: rc.right - 2, bottom: track_bottom },
        rgb(45, 52, 61),
    );
    let thumb_h = ((track_h as f64 * visible as f64 / total as f64) as i32).clamp(16, track_h);
    let max_scroll = total.saturating_sub(visible).max(1);
    let pos = ((track_h - thumb_h) as f64 * scroll.min(max_scroll) as f64 / max_scroll as f64) as i32;
    fill(
        hdc,
        &RECT { left: rc.right - 6, top: track_top + pos, right: rc.right - 1, bottom: track_top + pos + thumb_h },
        rgb(99, 199, 255),
    );
}

unsafe fn paint_tooltip(hdc: HDC, rc: RECT, x: i32, y: i32, text: &str) {
    let w = 250;
    let h = 28;
    let left = x.clamp(5, (rc.right - w - 5).max(5));
    let top = (y + 20).clamp(5, (rc.bottom - h - 5).max(5));
    let r = RECT { left, top, right: left + w, bottom: top + h };
    fill(hdc, &r, rgb(8, 11, 15));
    SetTextColor(hdc, rgb(241, 245, 249));
    draw(
        hdc,
        text,
        RECT { left: r.left + 7, top: r.top, right: r.right - 7, bottom: r.bottom },
        DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS,
    );
}

fn badge_tooltip(hwnd: HWND, state: &State, x: i32, y: i32) -> Option<String> {
    if state.kind != Kind::Dps {
        return None;
    }
    let row = unsafe { dps_row_at(hwnd, state, y) }?;
    let settings = state.features.read().map(|f| f.clone()).unwrap_or_default();
    if !settings.meter.show_imagines {
        return None;
    }
    let row_y = dps_rows_top() + ((y - dps_rows_top()) / DPS_ROW_H) * DPS_ROW_H;
    if y < row_y + 2 || y > row_y + 23 {
        return None;
    }
    let mut bx = 36;
    for badge in row.imagines.iter().take(3) {
        if x >= bx && x < bx + BADGE_W {
            let tier = if badge.tier > 0 { format!(" • T{}", badge.tier) } else { String::new() };
            return Some(format!("{}{}", badge.name, tier));
        }
        bx += BADGE_W + BADGE_GAP;
    }
    None
}

unsafe fn dps_row_at<'a>(hwnd: HWND, state: &'a State, y: i32) -> Option<&'a DpsRow> {
    let mut rc: RECT = std::mem::zeroed();
    GetClientRect(hwnd, &mut rc);
    let top = dps_rows_top();
    if y < top || y >= rc.bottom {
        return None;
    }
    let screen_i = ((y - top) / DPS_ROW_H) as usize;
    if screen_i >= visible_dps_rows(rc.bottom) {
        return None;
    }
    state.dps.rows.get(state.scroll + screen_i)
}

fn dps_rows_top() -> i32 {
    TOOLBAR_H + DPS_SUMMARY_H
}

fn visible_dps_rows(bottom: i32) -> usize {
    ((bottom - dps_rows_top()) / DPS_ROW_H).max(0) as usize
}

unsafe fn clamp_scroll(hwnd: HWND, state: &mut State) {
    let mut rc: RECT = std::mem::zeroed();
    GetClientRect(hwnd, &mut rc);
    let (total, visible) = if state.kind == Kind::Dps {
        (state.dps.rows.len(), visible_dps_rows(rc.bottom))
    } else {
        let selected = state
            .features
            .read()
            .map(|f| f.mechanic_attributes.tracked.len())
            .unwrap_or(0);
        let top = TOOLBAR_H + 6 + if selected > 0 { MECH_ATTR_H } else { 0 };
        let visible = ((rc.bottom - top) / MECH_ROW_H).max(0) as usize;
        let now = now_ms();
        let total = state
            .mechanics
            .rows
            .iter()
            .filter(|r| r.persistent || r.expires_unix_ms <= 0 || r.expires_unix_ms > now)
            .count();
        (total, visible)
    };
    state.scroll = state.scroll.min(total.saturating_sub(visible.max(1)));
}

unsafe fn open_detail(parent: HWND, state: &mut State, row: DpsRow) {
    if !state.detail_hwnd.is_null() && IsWindow(state.detail_hwnd) != 0 {
        DestroyWindow(state.detail_hwnd);
    }
    let instance = GetModuleHandleW(null());
    let class = wide(DETAIL_CLASS);
    let wc = WNDCLASSW {
        lpfnWndProc: Some(detail_wnd_proc),
        hInstance: instance,
        hCursor: LoadCursorW(null_mut(), IDC_ARROW),
        lpszClassName: class.as_ptr(),
        ..std::mem::zeroed()
    };
    if RegisterClassW(&wc) == 0 && GetLastError() != 1410 {
        return;
    }
    let mut pr: RECT = std::mem::zeroed();
    GetWindowRect(parent, &mut pr);
    let detail = Box::new(DetailState { row: row.clone(), scroll: 0 });
    let ptr = Box::into_raw(detail);
    let hwnd = CreateWindowExW(
        WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
        class.as_ptr(),
        wide(&format!("DPS Details - {}", row.name)).as_ptr(),
        WS_OVERLAPPEDWINDOW,
        pr.right + 8,
        pr.top + 35,
        620,
        460,
        parent,
        null_mut(),
        instance,
        ptr.cast::<c_void>(),
    );
    if hwnd.is_null() {
        drop(Box::from_raw(ptr));
        return;
    }
    state.detail_hwnd = hwnd;
    state.detail_uid = row.uid;
    ShowWindow(hwnd, SW_SHOW);
    SetForegroundWindow(hwnd);
}

unsafe fn update_detail(hwnd: HWND, row: DpsRow) {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut DetailState;
    if !ptr.is_null() {
        (*ptr).row = row;
        (*ptr).scroll = (*ptr).scroll.min((*ptr).row.skills.len().saturating_sub(1));
        InvalidateRect(hwnd, null(), 0);
    }
}

unsafe extern "system" fn detail_wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        let cs = lparam as *const CREATESTRUCTW;
        if !cs.is_null() {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, (*cs).lpCreateParams as isize);
        }
    }
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut DetailState;
    match msg {
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            if !ptr.is_null() {
                paint_detail(hwnd, &*ptr);
            }
            0
        }
        WM_SIZE => {
            InvalidateRect(hwnd, null(), 0);
            0
        }
        WM_MOUSEWHEEL => {
            if !ptr.is_null() {
                let delta = (((wparam >> 16) & 0xffff) as u16 as i16) as i32;
                let s = &mut *ptr;
                if delta > 0 {
                    s.scroll = s.scroll.saturating_sub(3);
                } else if delta < 0 {
                    s.scroll = s.scroll.saturating_add(3).min(s.row.skills.len().saturating_sub(1));
                }
                InvalidateRect(hwnd, null(), 0);
            }
            0
        }
        WM_NCDESTROY => {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            if !ptr.is_null() {
                drop(Box::from_raw(ptr));
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn paint_detail(hwnd: HWND, state: &DetailState) {
    let mut ps: PAINTSTRUCT = std::mem::zeroed();
    let hdc = BeginPaint(hwnd, &mut ps);
    if hdc.is_null() {
        return;
    }
    let mut rc: RECT = std::mem::zeroed();
    GetClientRect(hwnd, &mut rc);
    fill(hdc, &rc, rgb(18, 22, 27));
    SelectObject(hdc, GetStockObject(DEFAULT_GUI_FONT));
    SetBkMode(hdc, TRANSPARENT as i32);
    let row = &state.row;
    SetTextColor(hdc, if row.is_dead { rgb(255, 92, 92) } else { rgb(241, 245, 249) });
    draw(
        hdc,
        &row.name,
        RECT { left: 14, top: 8, right: rc.right - 14, bottom: 32 },
        DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS,
    );
    SetTextColor(hdc, profession_color(row.profession_id));
    draw(
        hdc,
        &format!("{}   ({})", identity_tail(row), profession_name(row.profession_id)),
        RECT { left: 14, top: 31, right: rc.right - 14, bottom: 52 },
        DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS,
    );
    SetTextColor(hdc, rgb(180, 194, 211));
    draw(
        hdc,
        &format!(
            "Damage {}   Healing {}   Taken {}   Crits {}   Lucky {}   Deaths {}",
            compact(row.damage as f64),
            compact(row.healing as f64),
            compact(row.damage_taken as f64),
            row.crits,
            row.lucky_hits,
            row.deaths
        ),
        RECT { left: 14, top: 51, right: rc.right - 14, bottom: 76 },
        DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS,
    );
    let head = 80;
    fill(hdc, &RECT { left: 8, top: head, right: rc.right - 8, bottom: head + 25 }, rgb(31, 37, 45));
    SetTextColor(hdc, rgb(210, 220, 231));
    draw(hdc, "Skill", RECT { left: 14, top: head, right: rc.right - 400, bottom: head + 25 }, DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
    draw(hdc, "Damage", RECT { left: rc.right - 395, top: head, right: rc.right - 295, bottom: head + 25 }, DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
    draw(hdc, "Heal", RECT { left: rc.right - 290, top: head, right: rc.right - 210, bottom: head + 25 }, DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
    draw(hdc, "Hits", RECT { left: rc.right - 205, top: head, right: rc.right - 150, bottom: head + 25 }, DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
    draw(hdc, "Crit/Luck", RECT { left: rc.right - 145, top: head, right: rc.right - 72, bottom: head + 25 }, DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
    draw(hdc, "Max", RECT { left: rc.right - 70, top: head, right: rc.right - 14, bottom: head + 25 }, DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
    let top = head + 28;
    let row_h = 31;
    let visible = ((rc.bottom - top) / row_h).max(0) as usize;
    if row.skills.is_empty() {
        SetTextColor(hdc, rgb(132, 145, 162));
        draw(hdc, "No skill events recorded yet.", RECT { left: 14, top: top + 20, right: rc.right - 14, bottom: top + 55 }, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
    } else {
        for (i, skill) in row.skills.iter().skip(state.scroll).take(visible).enumerate() {
            let y = top + i as i32 * row_h;
            let rr = RECT { left: 8, top: y, right: rc.right - 8, bottom: y + row_h - 2 };
            fill(hdc, &rr, if i % 2 == 0 { rgb(27, 32, 39) } else { rgb(23, 28, 34) });
            SetTextColor(hdc, rgb(235, 240, 246));
            draw(hdc, &skill.name, RECT { left: 14, top: y, right: rc.right - 400, bottom: y + row_h }, DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS);
            SetTextColor(hdc, rgb(255, 120, 120));
            draw(hdc, &compact(skill.damage as f64), RECT { left: rc.right - 395, top: y, right: rc.right - 295, bottom: y + row_h }, DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
            SetTextColor(hdc, rgb(105, 225, 145));
            draw(hdc, &compact(skill.healing as f64), RECT { left: rc.right - 290, top: y, right: rc.right - 210, bottom: y + row_h }, DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
            SetTextColor(hdc, rgb(185, 199, 216));
            draw(hdc, &skill.hits.to_string(), RECT { left: rc.right - 205, top: y, right: rc.right - 150, bottom: y + row_h }, DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
            draw(hdc, &format!("{}/{}", skill.crits, skill.lucky_hits), RECT { left: rc.right - 145, top: y, right: rc.right - 72, bottom: y + row_h }, DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
            draw(hdc, &compact(skill.max_value as f64), RECT { left: rc.right - 70, top: y, right: rc.right - 14, bottom: y + row_h }, DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        }
    }
    EndPaint(hwnd, &ps);
}

unsafe fn open_feature_settings(parent: HWND, state: &mut State) {
    if !state.settings_hwnd.is_null() && IsWindow(state.settings_hwnd) != 0 {
        SetForegroundWindow(state.settings_hwnd);
        return;
    }
    let instance = GetModuleHandleW(null());
    let class = wide(SETTINGS_CLASS);
    let wc = WNDCLASSW {
        lpfnWndProc: Some(settings_wnd_proc),
        hInstance: instance,
        hCursor: LoadCursorW(null_mut(), IDC_ARROW),
        lpszClassName: class.as_ptr(),
        ..std::mem::zeroed()
    };
    if RegisterClassW(&wc) == 0 && GetLastError() != 1410 {
        return;
    }
    let mut pr: RECT = std::mem::zeroed();
    GetWindowRect(parent, &mut pr);
    let ss = Box::new(SettingsState {
        kind: state.kind,
        parent,
        paths: state.paths.clone(),
        features: state.features.clone(),
    });
    let ptr = Box::into_raw(ss);
    let title = if state.kind == Kind::Dps { "DPS Meter Settings" } else { "Dungeon Mechanics Settings" };
    let hwnd = CreateWindowExW(
        WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
        class.as_ptr(),
        wide(title).as_ptr(),
        WS_OVERLAPPEDWINDOW,
        pr.left + 35,
        pr.top + 45,
        410,
        if state.kind == Kind::Dps { 395 } else { 365 },
        parent,
        null_mut(),
        instance,
        ptr.cast::<c_void>(),
    );
    if hwnd.is_null() {
        drop(Box::from_raw(ptr));
        return;
    }
    state.settings_hwnd = hwnd;
    ShowWindow(hwnd, SW_SHOW);
    SetForegroundWindow(hwnd);
}

unsafe extern "system" fn settings_wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        let cs = lparam as *const CREATESTRUCTW;
        if !cs.is_null() {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, (*cs).lpCreateParams as isize);
        }
    }
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsState;
    match msg {
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            if !ptr.is_null() {
                paint_feature_settings(hwnd, &*ptr);
            }
            0
        }
        WM_LBUTTONDOWN => {
            if !ptr.is_null() {
                feature_settings_click(hwnd, &*ptr, lo_signed(lparam), hi_signed(lparam));
            }
            0
        }
        WM_NCDESTROY => {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            if !ptr.is_null() {
                drop(Box::from_raw(ptr));
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn paint_feature_settings(hwnd: HWND, state: &SettingsState) {
    let mut ps: PAINTSTRUCT = std::mem::zeroed();
    let hdc = BeginPaint(hwnd, &mut ps);
    if hdc.is_null() {
        return;
    }
    let mut rc: RECT = std::mem::zeroed();
    GetClientRect(hwnd, &mut rc);
    fill(hdc, &rc, rgb(18, 22, 27));
    SelectObject(hdc, GetStockObject(DEFAULT_GUI_FONT));
    SetBkMode(hdc, TRANSPARENT as i32);
    let f = state.features.read().map(|x| x.clone()).unwrap_or_default();
    let layout = if state.kind == Kind::Dps { &f.dps } else { &f.mechanics };
    SetTextColor(hdc, rgb(238, 242, 247));
    draw(hdc, "Overlay", RECT { left: 16, top: 10, right: rc.right - 16, bottom: 36 }, DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
    SetTextColor(hdc, rgb(186, 199, 215));
    draw(hdc, &format!("Opacity: {}%", layout.opacity), RECT { left: 16, top: 43, right: 230, bottom: 70 }, DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
    paint_button(hdc, 250, 45, 34, 23, "-");
    paint_button(hdc, 292, 45, 34, 23, "+");
    draw(hdc, &format!("Collapse side: {}  (click to cycle)", layout.collapse_side), RECT { left: 16, top: 76, right: rc.right - 16, bottom: 103 }, DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS);

    if state.kind == Kind::Dps {
        SetTextColor(hdc, rgb(238, 242, 247));
        draw(hdc, "Meter fields", RECT { left: 16, top: 112, right: rc.right - 16, bottom: 138 }, DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        let values = [
            ("Damage %", f.meter.show_damage_share),
            ("Healing %", f.meter.show_healing_share),
            ("Tank %", f.meter.show_tank_share),
            ("Death count", f.meter.show_deaths),
            ("Imagine badges + hover", f.meter.show_imagines),
            ("Target / HP / Enrage header", f.meter.show_target),
        ];
        for (i, (label, checked)) in values.iter().enumerate() {
            paint_check(hdc, 16, 143 + i as i32 * 31, label, *checked, rc.right);
        }
    } else {
        SetTextColor(hdc, rgb(238, 242, 247));
        draw(hdc, "Track attributes", RECT { left: 16, top: 112, right: rc.right - 16, bottom: 138 }, DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        let attrs = [
            (ATTR_LUCK, "Luck"),
            (ATTR_HASTE, "Haste"),
            (ATTR_MASTERY, "Mastery"),
            (ATTR_HEALING_MASTERY, "Healing Mastery"),
            (ATTR_ACCURACY, "Accuracy"),
            (ATTR_VERSATILITY, "Versatility"),
        ];
        for (i, (id, label)) in attrs.iter().enumerate() {
            paint_check(
                hdc,
                16,
                143 + i as i32 * 31,
                label,
                f.mechanic_attributes.tracked.contains(id),
                rc.right,
            );
        }
    }
    EndPaint(hwnd, &ps);
}

unsafe fn feature_settings_click(hwnd: HWND, state: &SettingsState, x: i32, y: i32) {
    if (43..=72).contains(&y) && (240..=335).contains(&x) {
        let delta = if x < 285 { -5 } else { 5 };
        mutate_features(state, |f| {
            let l = if state.kind == Kind::Dps { &mut f.dps } else { &mut f.mechanics };
            l.opacity = (l.opacity + delta).clamp(25, 100);
        });
    } else if (75..=106).contains(&y) {
        mutate_features(state, |f| {
            let l = if state.kind == Kind::Dps { &mut f.dps } else { &mut f.mechanics };
            l.collapse_side = match l.collapse_side.to_ascii_lowercase().as_str() {
                "right" => "Bottom".into(),
                "bottom" => "Left".into(),
                "left" => "Top".into(),
                _ => "Right".into(),
            };
        });
    } else if (143..=338).contains(&y) {
        let idx = ((y - 143) / 31) as usize;
        if state.kind == Kind::Dps {
            mutate_features(state, |f| match idx {
                0 => f.meter.show_damage_share = !f.meter.show_damage_share,
                1 => f.meter.show_healing_share = !f.meter.show_healing_share,
                2 => f.meter.show_tank_share = !f.meter.show_tank_share,
                3 => f.meter.show_deaths = !f.meter.show_deaths,
                4 => f.meter.show_imagines = !f.meter.show_imagines,
                5 => f.meter.show_target = !f.meter.show_target,
                _ => {}
            });
        } else {
            let ids = [ATTR_LUCK, ATTR_HASTE, ATTR_MASTERY, ATTR_HEALING_MASTERY, ATTR_ACCURACY, ATTR_VERSATILITY];
            if let Some(id) = ids.get(idx).copied() {
                mutate_features(state, |f| {
                    if let Some(pos) = f.mechanic_attributes.tracked.iter().position(|x| *x == id) {
                        f.mechanic_attributes.tracked.remove(pos);
                    } else {
                        f.mechanic_attributes.tracked.push(id);
                    }
                });
            }
        }
    }
    InvalidateRect(hwnd, null(), 0);
    InvalidateRect(state.parent, null(), 0);
}

fn mutate_features<F: FnOnce(&mut FeatureSettings)>(state: &SettingsState, change: F) {
    let snapshot = if let Ok(mut f) = state.features.write() {
        change(&mut f);
        f.normalize();
        let snap = f.clone();
        drop(f);
        snap
    } else {
        return;
    };
    let _ = feature_settings::save(&state.paths, &snapshot);
    let opacity = if state.kind == Kind::Dps { snapshot.dps.opacity } else { snapshot.mechanics.opacity };
    unsafe { apply_opacity(state.parent, opacity) };
}

unsafe fn paint_button(hdc: HDC, x: i32, y: i32, w: i32, h: i32, text: &str) {
    let r = RECT { left: x, top: y, right: x + w, bottom: y + h };
    fill(hdc, &r, rgb(42, 50, 60));
    SetTextColor(hdc, rgb(235, 240, 246));
    draw(hdc, text, r, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
}

unsafe fn paint_check(hdc: HDC, x: i32, y: i32, label: &str, checked: bool, right: i32) {
    let box_r = RECT { left: x, top: y + 4, right: x + 18, bottom: y + 22 };
    fill(hdc, &box_r, if checked { rgb(63, 133, 255) } else { rgb(45, 52, 61) });
    if checked {
        SetTextColor(hdc, rgb(255, 255, 255));
        draw(hdc, "✓", box_r, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
    }
    SetTextColor(hdc, rgb(205, 215, 227));
    draw(
        hdc,
        label,
        RECT { left: x + 27, top: y, right: right - 16, bottom: y + 27 },
        DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS,
    );
}

unsafe fn collapse(hwnd: HWND, state: &mut State) {
    let mut r: RECT = std::mem::zeroed();
    GetWindowRect(hwnd, &mut r);
    state.expanded = r;
    let work = monitor_work(hwnd);
    let side = layout_side(state);
    let w = (r.right - r.left).max(100);
    let h = (r.bottom - r.top).max(80);
    let (x, y, nw, nh) = match side.as_str() {
        "left" => (work.left, r.top.clamp(work.top, work.bottom - COLLAPSED), COLLAPSED, h.min(work.bottom - work.top)),
        "top" => (r.left.clamp(work.left, work.right - COLLAPSED), work.top, w.min(work.right - work.left), COLLAPSED),
        "bottom" => (r.left.clamp(work.left, work.right - COLLAPSED), work.bottom - COLLAPSED, w.min(work.right - work.left), COLLAPSED),
        _ => (work.right - COLLAPSED, r.top.clamp(work.top, work.bottom - COLLAPSED), COLLAPSED, h.min(work.bottom - work.top)),
    };
    state.collapsed = true;
    state.hover = None;
    SetWindowPos(hwnd, HWND_TOPMOST, x, y, nw, nh, SWP_NOACTIVATE);
    InvalidateRect(hwnd, null(), 0);
}

unsafe fn expand_state(hwnd: HWND, state: &mut State) {
    let r = state.expanded;
    state.collapsed = false;
    SetWindowPos(
        hwnd,
        HWND_TOPMOST,
        r.left,
        r.top,
        (r.right - r.left).max(100),
        (r.bottom - r.top).max(80),
        SWP_NOACTIVATE,
    );
    InvalidateRect(hwnd, null(), 0);
}

unsafe fn save_bounds(hwnd: HWND, state: &mut State) {
    if state.collapsed {
        return;
    }
    let mut r: RECT = std::mem::zeroed();
    if GetWindowRect(hwnd, &mut r) == 0 {
        return;
    }
    state.expanded = r;
    if let Ok(mut f) = state.features.write() {
        let l = if state.kind == Kind::Dps { &mut f.dps } else { &mut f.mechanics };
        l.x = r.left;
        l.y = r.top;
        l.width = (r.right - r.left).max(1);
        l.height = (r.bottom - r.top).max(1);
        let snap = f.clone();
        drop(f);
        let _ = feature_settings::save(&state.paths, &snap);
    }
}

fn layout_side(state: &State) -> String {
    state
        .features
        .read()
        .map(|f| if state.kind == Kind::Dps { f.dps.collapse_side.clone() } else { f.mechanics.collapse_side.clone() })
        .unwrap_or_else(|_| "Right".into())
        .to_ascii_lowercase()
}

#[repr(C)]
struct MonitorInfo {
    cb_size: u32,
    rc_monitor: RECT,
    rc_work: RECT,
    flags: u32,
}
#[link(name = "user32")]
extern "system" {
    fn MonitorFromWindow(hwnd: HWND, flags: u32) -> *mut c_void;
    fn GetMonitorInfoW(monitor: *mut c_void, info: *mut MonitorInfo) -> i32;
}

unsafe fn monitor_work(hwnd: HWND) -> RECT {
    let m = MonitorFromWindow(hwnd, 2);
    let mut i = MonitorInfo {
        cb_size: std::mem::size_of::<MonitorInfo>() as u32,
        rc_monitor: std::mem::zeroed(),
        rc_work: std::mem::zeroed(),
        flags: 0,
    };
    if !m.is_null() && GetMonitorInfoW(m, &mut i) != 0 {
        i.rc_work
    } else {
        RECT { left: 0, top: 0, right: 1920, bottom: 1080 }
    }
}

unsafe fn with_state<F: FnOnce(&mut State)>(hwnd: HWND, f: F) {
    let p = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut State;
    if !p.is_null() {
        f(&mut *p);
    }
}

unsafe fn apply_opacity(hwnd: HWND, opacity: i32) {
    SetLayeredWindowAttributes(hwnd, 0, ((opacity.clamp(25, 100) * 255 / 100).clamp(1, 255)) as u8, LWA_ALPHA);
}

unsafe fn fill(hdc: HDC, r: &RECT, c: u32) {
    let b = CreateSolidBrush(c);
    if !b.is_null() {
        FillRect(hdc, r, b);
        DeleteObject(b);
    }
}

unsafe fn draw(hdc: HDC, text: &str, mut r: RECT, flags: u32) {
    let w = wide(text);
    DrawTextW(hdc, w.as_ptr(), -1, &mut r, flags);
}

fn identity_tail(row: &DpsRow) -> String {
    let spec = if !row.subprofession_name.trim().is_empty() {
        row.subprofession_name.as_str()
    } else {
        profession_name(row.profession_id)
    };
    if row.ability_score > 0 || row.illusion_break > 0 {
        format!("({} + {}) {}", score(row.ability_score), score_exact_small(row.illusion_break), spec)
    } else {
        spec.to_string()
    }
}

fn profession_name(id: i32) -> &'static str {
    match id {
        1 => "Stormblade",
        2 => "Frost Mage",
        3 => "Twin Striker",
        4 => "Wind Knight",
        5 => "Verdant Oracle",
        8 => "Hand Cannon",
        9 => "Heavy Guardian",
        11 => "Marksman",
        12 => "Shield Knight",
        13 => "Beat Performer",
        14 => "Lucy",
        15 => "Natsu",
        _ => "",
    }
}

fn profession_color(id: i32) -> u32 {
    match id {
        1 => rgb(116, 190, 255),
        2 => rgb(121, 219, 255),
        3 => rgb(255, 126, 126),
        4 => rgb(147, 229, 172),
        5 => rgb(132, 231, 153),
        8 => rgb(255, 184, 118),
        9 => rgb(255, 210, 124),
        11 => rgb(180, 225, 125),
        12 => rgb(124, 189, 255),
        13 => rgb(225, 146, 255),
        _ => rgb(190, 204, 222),
    }
}

fn badge_color(key: &str) -> u32 {
    match key {
        "TN" => rgb(105, 166, 230),
        "BL" => rgb(190, 112, 92),
        "AI" => rgb(100, 184, 145),
        _ => rgb(104, 112, 130),
    }
}

fn attr_color(id: i32) -> u32 {
    match id {
        ATTR_LUCK => rgb(255, 210, 110),
        ATTR_HASTE => rgb(110, 210, 255),
        ATTR_MASTERY => rgb(204, 152, 255),
        ATTR_HEALING_MASTERY => rgb(105, 225, 145),
        ATTR_ACCURACY => rgb(255, 157, 112),
        ATTR_VERSATILITY => rgb(171, 198, 232),
        _ => rgb(205, 215, 227),
    }
}

fn compact(v: f64) -> String {
    let a = v.abs();
    if a >= 1_000_000_000.0 {
        format!("{:.2}B", v / 1_000_000_000.0)
    } else if a >= 1_000_000.0 {
        format!("{:.2}M", v / 1_000_000.0)
    } else if a >= 1_000.0 {
        format!("{:.1}K", v / 1_000.0)
    } else {
        format!("{:.0}", v)
    }
}

fn compact_attr(v: i64) -> String {
    if v.abs() >= 100_000 {
        format!("{:.1}K", v as f64 / 1000.0)
    } else {
        v.to_string()
    }
}

fn score(v: i64) -> String {
    if v.abs() >= 10_000 {
        format!("{}k", (v as f64 / 1000.0).round() as i64)
    } else {
        v.to_string()
    }
}

fn score_exact_small(v: i64) -> String {
    if v.abs() >= 10_000 {
        format!("{}k", (v as f64 / 1000.0).round() as i64)
    } else {
        v.to_string()
    }
}

fn format_time(ms: u64) -> String {
    let s = ms / 1000;
    format!("{}:{:02}", s / 60, s % 60)
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or(0)
}

fn lo_signed(v: LPARAM) -> i32 {
    (v as u16 as i16) as i32
}

fn hi_signed(v: LPARAM) -> i32 {
    (((v >> 16) as u16) as i16) as i32
}

fn rgb(r: u8, g: u8, b: u8) -> u32 {
    u32::from(r) | (u32::from(g) << 8) | (u32::from(b) << 16)
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
