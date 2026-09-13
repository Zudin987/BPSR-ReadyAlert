// Owner-drawn popup rows on the existing Win32 menu manager. Windows continues to
// own command dispatch, keyboard navigation, submenus, capture and dismissal.
const QA_MENU_SUBCLASS_ID: usize = 0x5241_514d;

struct QaMenuItem {
    menu: windows_sys::Win32::UI::WindowsAndMessaging::HMENU,
    index: u32,
    old_type: u32,
    old_data: usize,
    text: Vec<u16>,
    submenu: bool,
}

unsafe fn qa_prepare_menu(menu: windows_sys::Win32::UI::WindowsAndMessaging::HMENU, items: &mut Vec<Box<QaMenuItem>>) {
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    for index in 0..GetMenuItemCount(menu).max(0) as u32 {
        let mut info: MENUITEMINFOW = std::mem::zeroed();
        info.cbSize = std::mem::size_of_val(&info) as u32;
        info.fMask = MIIM_FTYPE | MIIM_DATA | MIIM_STRING | MIIM_SUBMENU;
        if GetMenuItemInfoW(menu, index, 1, &mut info) == 0 { continue; }
        if !info.hSubMenu.is_null() { qa_prepare_menu(info.hSubMenu, items); }
        if info.fType & MFT_OWNERDRAW != 0 { continue; }
        let mut text = vec![0u16; info.cch as usize + 1];
        info.dwTypeData = text.as_mut_ptr();
        info.cch = text.len() as u32;
        if GetMenuItemInfoW(menu, index, 1, &mut info) == 0 { continue; }
        text.truncate(info.cch as usize);
        let item = Box::new(QaMenuItem { menu, index, old_type: info.fType, old_data: info.dwItemData, text, submenu: !info.hSubMenu.is_null() });
        info.fMask = MIIM_FTYPE | MIIM_DATA;
        info.fType |= MFT_OWNERDRAW;
        info.dwItemData = (&*item as *const QaMenuItem) as usize;
        if SetMenuItemInfoW(menu, index, 1, &info) != 0 { items.push(item); }
    }
}

unsafe extern "system" fn qa_menu_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM, _id: usize, data: usize) -> LRESULT {
    use windows_sys::Win32::UI::{Controls::MEASUREITEMSTRUCT, WindowsAndMessaging::*};
    if data != 0 && lparam != 0 && (msg == WM_MEASUREITEM || msg == WM_DRAWITEM) {
        let items = &*(data as *const Vec<Box<QaMenuItem>>);
        let (kind, item_data) = if msg == WM_MEASUREITEM {
            let measure = &*(lparam as *const MEASUREITEMSTRUCT);
            (measure.CtlType, measure.itemData)
        } else {
            let draw = &*(lparam as *const DRAWITEMSTRUCT);
            (draw.CtlType, draw.itemData)
        };
        if kind == 1 {
            if let Some(item) = items.iter().find(|item| (&***item as *const QaMenuItem) as usize == item_data) {
                let separator = item.old_type & MFT_SEPARATOR != 0;
                if msg == WM_MEASUREITEM {
                    let measure = &mut *(lparam as *mut MEASUREITEMSTRUCT);
                    let dc = GetDC(hwnd);
                    let old = SelectObject(dc, body_font());
                    let mut r: RECT = std::mem::zeroed();
                    DrawTextW(dc, item.text.as_ptr(), item.text.len() as i32, &mut r, 0x400 | 0x20);
                    SelectObject(dc, old);
                    ReleaseDC(hwnd, dc);
                    measure.itemWidth = (r.right + 62).max(140) as u32;
                    measure.itemHeight = if separator { 9 } else { 30 };
                } else {
                    let draw = &*(lparam as *const DRAWITEMSTRUCT);
                    let dc = draw.hDC;
                    let saved = windows_sys::Win32::Graphics::Gdi::SaveDC(dc);
                    let r = draw.rcItem;
                    qa_fill_rect(dc, &r, DARK_SURFACE);
                    if separator {
                        qa_fill_rect(dc, &RECT { left: r.left + 12, top: (r.top+r.bottom)/2, right: r.right - 12, bottom: (r.top+r.bottom)/2 + 1 }, DARK_BORDER);
                    } else {
                        let disabled = draw.itemState & (0x0002 | 0x0004) != 0;
                        if draw.itemState & 1 != 0 && !disabled {
                            fill_round_rect(dc, RECT { left:r.left+3, top:r.top+2, right:r.right-3, bottom:r.bottom-2 }, SELECTED_SURFACE, RADIUS_SMALL);
                        }
                        SetBkMode(dc, TRANSPARENT as i32);
                        SetTextColor(dc, if disabled { BPSR_DISABLED } else { BPSR_TEXT });
                        SelectObject(dc, body_font());
                        let mut text = RECT { left:r.left+30, top:r.top, right:r.right-24, bottom:r.bottom };
                        DrawTextW(dc, item.text.as_ptr(), item.text.len() as i32, &mut text, 0x4 | 0x20);
                        if draw.itemState & 8 != 0 {
                            let mut mark = RECT { left:r.left+8, top:r.top, right:r.left+26, bottom:r.bottom };
                            DrawTextW(dc, wide("✓").as_ptr(), 1, &mut mark, 0x1 | 0x4 | 0x20);
                        }
                        if item.submenu {
                            let mut arrow = RECT { left:r.right-22, top:r.top, right:r.right-6, bottom:r.bottom };
                            DrawTextW(dc, wide("›").as_ptr(), 1, &mut arrow, 0x1 | 0x4 | 0x20);
                        }
                    }
                    if saved != 0 { windows_sys::Win32::Graphics::Gdi::RestoreDC(dc, saved); }
                }
                return 1;
            }
        }
    }
    DefSubclassProc(hwnd, msg, wparam, lparam)
}

pub unsafe fn qa_track_popup_menu(menu: windows_sys::Win32::UI::WindowsAndMessaging::HMENU, flags: u32, x: i32, y: i32, reserved: i32, owner: HWND, rect: *const RECT) -> i32 {
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    let mut items = Vec::new();
    // Install before converting items. If subclassing fails, the original native
    // menu remains fully usable and there is no owner-drawn data to clean up.
    if SetWindowSubclass(owner, Some(qa_menu_proc), QA_MENU_SUBCLASS_ID, (&items as *const Vec<Box<QaMenuItem>>) as usize) == 0 {
        return TrackPopupMenu(menu, flags, x, y, reserved, owner, rect);
    }
    enable_dark_system_surfaces();
    qa_prepare_menu(menu, &mut items);
    let result = TrackPopupMenu(menu, flags, x, y, reserved, owner, rect);
    for item in &items {
        let mut info: MENUITEMINFOW = std::mem::zeroed();
        info.cbSize = std::mem::size_of_val(&info) as u32;
        info.fMask = MIIM_FTYPE | MIIM_DATA;
        info.fType = item.old_type;
        info.dwItemData = item.old_data;
        SetMenuItemInfoW(item.menu, item.index, 1, &info);
    }
    windows_sys::Win32::UI::Shell::RemoveWindowSubclass(owner, Some(qa_menu_proc), QA_MENU_SUBCLASS_ID);
    result
}
