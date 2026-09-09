//! Small native-window helpers shared by forms and custom-drawn overlays.
//! Coordinates use the application's existing Windows DPI virtualization.
use std::ptr::{null, null_mut};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, POINT, RECT, WPARAM},
    Graphics::Gdi::{GetMonitorInfoW, InvalidateRect, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST},
    UI::WindowsAndMessaging::{
        EnumChildWindows, GetAncestor, GetClassNameW, GetClientRect, GetParent, GetScrollInfo,
        GetWindowRect, IsDialogMessageW, IsWindowVisible, MoveWindow, SendMessageW,
        SetWindowPos, GA_ROOT, MSG, SCROLLINFO, SB_HORZ, SB_VERT,
        SIF_ALL, SWP_NOACTIVATE, SWP_NOZORDER, WM_CLOSE, WM_KEYDOWN,
        WM_MOUSEWHEEL, WM_VSCROLL,
    },
};

#[link(name = "user32")]
extern "system" {
    pub fn SetFocus(hwnd: HWND) -> HWND;
    pub fn GetFocus() -> HWND;
    pub fn GetKeyState(key: i32) -> i16;
    fn ScreenToClient(hwnd: HWND, point: *mut POINT) -> i32;
    pub fn EnableWindow(hwnd: HWND, enable: i32) -> i32;
    // windows-sys 0.59 exposes SCROLLINFO/GetScrollInfo but not this user32
    // declaration. Keep the crate pinned and bind this one stable Win32 call
    // directly instead of growing the dependency surface.
    fn SetScrollInfo(hwnd: HWND, bar: i32, info: *const SCROLLINFO, redraw: i32) -> i32;
}

pub fn fit_rect(request: RECT, work: RECT) -> RECT {
    let available_w = (work.right - work.left).max(1);
    let available_h = (work.bottom - work.top).max(1);
    let w = (request.right - request.left).clamp(1, available_w);
    let h = (request.bottom - request.top).clamp(1, available_h);
    let x = request.left.clamp(work.left, work.right - w);
    let y = request.top.clamp(work.top, work.bottom - h);
    RECT { left: x, top: y, right: x + w, bottom: y + h }
}

pub unsafe fn work_area(hwnd: HWND) -> RECT {
    let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..std::mem::zeroed() };
    if GetMonitorInfoW(MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST), &mut info) != 0 {
        info.rcWork
    } else { RECT { left: 0, top: 0, right: 1280, bottom: 680 } }
}

pub unsafe fn fit_window(hwnd: HWND, owner: HWND, center: bool) {
    let work = work_area(if owner.is_null() { hwnd } else { owner });
    let mut r: RECT = std::mem::zeroed();
    if GetWindowRect(hwnd, &mut r) == 0 { return; }
    if center {
        let w = r.right - r.left; let h = r.bottom - r.top;
        r.left = work.left + ((work.right - work.left - w) / 2).max(0);
        r.top = work.top + ((work.bottom - work.top - h) / 2).max(0);
        r.right = r.left + w; r.bottom = r.top + h;
    }
    r = fit_rect(r, work);
    SetWindowPos(hwnd, null_mut(), r.left, r.top, r.right-r.left, r.bottom-r.top, SWP_NOZORDER | SWP_NOACTIVATE);
}

pub unsafe fn min_window(hwnd: HWND, lparam: LPARAM, width: i32, height: i32) {
    let work=work_area(hwnd);
    let m=&mut *(lparam as *mut windows_sys::Win32::UI::WindowsAndMessaging::MINMAXINFO);
    m.ptMinTrackSize.x=width.min(work.right-work.left);
    m.ptMinTrackSize.y=height.min(work.bottom-work.top);
}

pub unsafe fn scrollbar(hwnd: HWND, vertical: bool, total: i32, page: i32, pos: i32) {
    let info=SCROLLINFO{cbSize:std::mem::size_of::<SCROLLINFO>()as u32,fMask:SIF_ALL,
        nMin:0,nMax:total.max(1)-1,nPage:page.max(1)as u32,nPos:pos,nTrackPos:pos};
    SetScrollInfo(hwnd,if vertical{SB_VERT}else{SB_HORZ},&info,1);
}

pub unsafe fn scroll_command(hwnd: HWND, vertical: bool, wparam: WPARAM, pos: i32, page: i32) -> i32 {
    let mut info:SCROLLINFO=std::mem::zeroed();info.cbSize=std::mem::size_of::<SCROLLINFO>()as u32;info.fMask=SIF_ALL;
    GetScrollInfo(hwnd,if vertical{SB_VERT}else{SB_HORZ},&mut info);
    match wparam as u16{0=>pos-1,1=>pos+1,2=>pos-page,3=>pos+page,4|5=>info.nTrackPos,6=>0,7=>info.nMax,_=>pos}
}

#[derive(Default)]
pub struct ScrollForm { children: Vec<(HWND, RECT)>, x: i32, y: i32, width: i32, height: i32, busy: bool }

impl ScrollForm {
    pub unsafe fn capture(&mut self, hwnd: HWND, width: i32, height: i32) {
        self.width = width; self.height = height;
        self.children.clear();
        let mut data = (hwnd, &mut self.children);
        EnumChildWindows(hwnd, Some(collect_child), (&mut data as *mut (HWND, &mut Vec<(HWND, RECT)>)) as LPARAM);
        self.layout(hwnd);
    }
    pub unsafe fn layout(&mut self, hwnd: HWND) {
        if self.width<=0 || self.height<=0 || self.busy { return; }
        self.busy=true;
        let mut client: RECT = std::mem::zeroed(); GetClientRect(hwnd, &mut client);
        self.x = self.x.clamp(0, (self.width-client.right).max(0));
        self.y = self.y.clamp(0, (self.height-client.bottom).max(0));
        for (bar, extent, page, pos) in [(SB_HORZ,self.width,client.right,self.x),(SB_VERT,self.height,client.bottom,self.y)] {
            let info = SCROLLINFO { cbSize: std::mem::size_of::<SCROLLINFO>() as u32, fMask: SIF_ALL,
                nMin: 0, nMax: extent-1, nPage: page.max(1) as u32, nPos: pos, nTrackPos: pos };
            SetScrollInfo(hwnd, bar, &info, 1);
        }
        // Reposition first, then repaint once. The old path repainted every child
        // while moving it and erased the full parent afterwards, which visibly
        // flashed native controls during resize/scroll operations.
        for &(child, r) in &self.children {
            MoveWindow(child, r.left-self.x, r.top-self.y, r.right-r.left, r.bottom-r.top, 0);
        }
        for &(child, _) in &self.children { InvalidateRect(child, null(), 0); }
        InvalidateRect(hwnd, null(), 0);
        self.busy=false;
    }
    pub unsafe fn scroll(&mut self, hwnd: HWND, msg: u32, wparam: WPARAM) {
        if msg == WM_MOUSEWHEEL {
            let delta = (wparam >> 16) as u16 as i16 as i32;
            if GetKeyState(0x10) < 0 { self.x -= delta.signum()*48; } else { self.y -= delta.signum()*48; }
        } else {
            let vertical = msg == WM_VSCROLL;
            let mut info: SCROLLINFO = std::mem::zeroed();
            info.cbSize = std::mem::size_of::<SCROLLINFO>() as u32; info.fMask = SIF_ALL;
            GetScrollInfo(hwnd, if vertical { SB_VERT } else { SB_HORZ }, &mut info);
            let pos = if vertical { &mut self.y } else { &mut self.x };
            *pos = match wparam as u16 { 0 => *pos-32, 1 => *pos+32, 2 => *pos-info.nPage as i32,
                3 => *pos+info.nPage as i32, 4|5 => info.nTrackPos, 6 => 0, 7 => info.nMax, _ => *pos };
        }
        self.layout(hwnd);
    }
    pub unsafe fn reveal_focus(&mut self, hwnd: HWND) {
        let focus = GetFocus();
        let Some((_, r)) = self.children.iter().find(|(child, _)| *child == focus).copied() else { return; };
        let mut client: RECT = std::mem::zeroed(); GetClientRect(hwnd, &mut client);
        let old = (self.x,self.y);
        if r.left < self.x { self.x=r.left-8; } else if r.right > self.x+client.right { self.x=r.right-client.right+8; }
        if r.top < self.y { self.y=r.top-8; } else if r.bottom > self.y+client.bottom { self.y=r.bottom-client.bottom+8; }
        if old != (self.x,self.y) { self.layout(hwnd); }
    }
    pub unsafe fn reset(&mut self, hwnd: HWND) { self.x=0; self.y=0; self.layout(hwnd); }
}

unsafe extern "system" fn collect_child(child: HWND, data: LPARAM) -> i32 {
    let (parent, children) = &mut *(data as *mut (HWND, &mut Vec<(HWND, RECT)>));
    if GetParent(child) != *parent { return 1; }
    let mut r: RECT = std::mem::zeroed(); GetWindowRect(child, &mut r);
    let mut point = POINT { x:r.left,y:r.top }; ScreenToClient(*parent,&mut point);
    r.right = point.x + r.right-r.left; r.bottom=point.y+r.bottom-r.top; r.left=point.x; r.top=point.y;
    children.push((child,r)); 1
}

pub const WM_REVEAL_FOCUS: u32 = 0x8000 + 125;

/// Route modeless native forms through the dialog keyboard manager. A focused
/// edit retains its arrows; Tab, Shift+Tab, mnemonics and Escape reach the form.
pub unsafe fn dialog_message(msg: &MSG) -> bool {
    if msg.hwnd.is_null() { return false; }
    let root = GetAncestor(msg.hwnd, GA_ROOT);
    let mut class = [0u16; 96];
    let n = GetClassNameW(root, class.as_mut_ptr(), class.len() as i32).max(0) as usize;
    let class = String::from_utf16_lossy(&class[..n]);
    if !matches!(class.as_str(), "BPSRReadyAlertRustSettingsV151" | "BPSRReadyAlertEventTrackerV114" | "BPSRReadyAlertFeatureSettingsV180") || IsWindowVisible(root)==0 { return false; }
    if msg.message == WM_KEYDOWN && msg.wParam == 0x1b { SendMessageW(root, WM_CLOSE, 0, 0); return true; }
    // DefWindowProc forwards unused wheel events from native child controls.
    let handled = IsDialogMessageW(root, msg) != 0;
    if handled { SendMessageW(root, WM_REVEAL_FOCUS, 0, 0); }
    handled
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn work_bounds_fit_small_and_negative_monitors() {
        for work in [RECT {left:0,top:0,right:1280,bottom:680}, RECT {left:-1280,top:-200,right:0,bottom:480}, RECT {left:0,top:0,right:853,bottom:440}] {
            let r=fit_rect(RECT {left:1500,top:1000,right:2500,bottom:1800},work);
            assert!(r.left>=work.left && r.right<=work.right && r.top>=work.top && r.bottom<=work.bottom);
            assert!(r.right>r.left && r.bottom>r.top);
        }
    }
}
