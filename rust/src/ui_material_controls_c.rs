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
