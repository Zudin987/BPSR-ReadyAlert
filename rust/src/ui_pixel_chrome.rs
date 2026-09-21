// Dark non-client painting for app-owned captioned Win32 settings/auxiliary dialogs.
// DWM dark caption attributes are only preferences and can fail on some builds.
// Repaint the actual native frame after DefSubclassProc instead of painting the
// CLIENT or turning Windows visual styles off. Hit testing, caption dragging,
// resize, system menu, native close/minimize and modal ownership stay native.
// Not installed on borderless overlays, popup listboxes or OS-owned dialogs.

const SHAPE_CAPTION_SUBCLASS_ID: usize = 0x5241_4348;
#[link(name="user32")]
extern "system" {
    fn shape_get_window_dc(hwnd: HWND) -> HDC;
    fn shape_client_to_screen(hwnd: HWND, point: *mut windows_sys::Win32::Foundation::POINT) -> i32;
    fn shape_get_system_metrics(index: i32) -> i32;
    fn shape_get_class_long_ptr(hwnd: HWND, index: i32) -> usize;
}
#[link(name="user32")]
extern "system" {
    #[link_name="GetWindowDC"] fn shape_get_nonclient_dc(hwnd: HWND) -> HDC;
    #[link_name="ClientToScreen"] fn shape_client_origin(hwnd: HWND, point: *mut windows_sys::Win32::Foundation::POINT) -> i32;
    #[link_name="GetSystemMetrics"] fn shape_metric(index: i32) -> i32;
    #[link_name="GetClassLongPtrW"] fn shape_class_icon(hwnd: HWND, index: i32) -> usize;
    #[link_name="DrawIconEx"] fn shape_draw_icon(hdc:HDC,x:i32,y:i32,icon:*mut c_void,cx:i32,cy:i32,step:u32,brush:HBRUSH,flags:u32)->i32;
}
#[link(name="comctl32")]
extern "system" {
    #[link_name="RemoveWindowSubclass"] fn shape_remove_caption_subclass(hwnd:HWND,proc:SubclassProc,id:usize)->i32;
}

fn shape_clip_coord(value:i32,limit:i32)->i32 { value.clamp(0,limit.max(0)) }
unsafe fn shape_paint_caption(hwnd: HWND, close_hover: bool) {
    if hwnd.is_null(){return;}
    let mut window: RECT=std::mem::zeroed();
    if windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect(hwnd,&mut window)==0{return;}
    let width=(window.right-window.left).max(0);
    let height=(window.bottom-window.top).max(0);
    if width==0||height==0{return;}
    let mut client:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut client);
    let mut origin=windows_sys::Win32::Foundation::POINT{x:0,y:0};
    if shape_client_origin(hwnd,&mut origin)==0{return;}
    let x=shape_clip_coord(origin.x-window.left,width);
    let y=shape_clip_coord(origin.y-window.top,height);
    let cr=shape_clip_coord(x+(client.right-client.left),width);
    let cb=shape_clip_coord(y+(client.bottom-client.top),height);
    let dc=shape_get_nonclient_dc(hwnd);if dc.is_null(){return;}
    let edge=CreateSolidBrush(DARK_BORDER);
    let face=CreateSolidBrush(DARK_SURFACE);
    if edge.is_null()||face.is_null(){if !edge.is_null(){DeleteObject(edge);}if !face.is_null(){DeleteObject(face);}ReleaseDC(hwnd,dc);return;}
    // Cover every OS-drawn non-client strip. Do not touch any client pixels.
    for rect in [RECT{left:0,top:0,right:width,bottom:y},RECT{left:0,top:y,right:x,bottom:cb},RECT{left:cr,top:y,right:width,bottom:cb},RECT{left:0,top:cb,right:width,bottom:height}] {
        if rect.right>rect.left&&rect.bottom>rect.top {FillRect(dc,&rect,edge);}
    }
    let title=RECT{left:1,top:1,right:width-1,bottom:y.max(1)};
    if title.right>title.left&&title.bottom>title.top {FillRect(dc,&title,face);}
    if y>8 {
        let icon_size=shape_metric(49).clamp(12,(y-4).max(12));
        let mut icon=SendMessageW(hwnd,0x007F,2,0) as *mut c_void; // WM_GETICON / ICON_SMALL2
        if icon.is_null(){icon=shape_class_icon(hwnd,-34) as *mut c_void;}
        if !icon.is_null(){shape_draw_icon(dc,5,(y-icon_size)/2,icon,icon_size,icon_size,0,null_mut(),3);}
        let caption_size=shape_metric(30).clamp(28,48); // SM_CXSIZE
        let close=RECT{left:(width-caption_size-2).max(0),top:2,right:width-3,bottom:(y-2).max(2)};
        if close_hover&&close.right>close.left&&close.bottom>close.top {
            let hover=CreateSolidBrush(DARK_HOVER);
            if !hover.is_null(){FillRect(dc,&close,hover);DeleteObject(hover);}
        }
        let mut name=vec![0u16;GetWindowTextLengthW(hwnd).max(0) as usize+1];
        let len=GetWindowTextW(hwnd,name.as_mut_ptr(),name.len() as i32).max(0);
        if len>0{
            let mut r=RECT{left:28,top:1,right:(width-caption_size-7).max(28),bottom:y};
            SelectObject(dc,body_font());SetBkMode(dc,TRANSPARENT as i32);SetTextColor(dc,BPSR_TEXT);
            DrawTextW(dc,name.as_ptr(),len,&mut r,0x0004|0x0020|0x0800|0x8000);
        }
        let x_icon=wide("×");let mut r=close;
        SelectObject(dc,medium_font());SetBkMode(dc,TRANSPARENT as i32);SetTextColor(dc,BPSR_TEXT);
        if r.right>r.left&&r.bottom>r.top{DrawTextW(dc,x_icon.as_ptr(),1,&mut r,0x0001|0x0004|0x0020|0x0800);}
    }
    DeleteObject(face);DeleteObject(edge);ReleaseDC(hwnd,dc);
}
unsafe extern "system" fn shape_caption_proc(hwnd:HWND,msg:u32,wparam:WPARAM,lparam:LPARAM,id:usize,_data:usize)->LRESULT {
    if msg==0x0082 { // WM_NCDESTROY
        shape_remove_caption_subclass(hwnd,Some(shape_caption_proc),id);
        return DefSubclassProc(hwnd,msg,wparam,lparam);
    }
    let result=DefSubclassProc(hwnd,msg,wparam,lparam);
    if matches!(msg,0x0085|0x0086|0x00A0|0x00A2|0x000C|0x0006) {
        let over_close=msg==0x00A0&&wparam==20; // HT_CLOSE
        shape_paint_caption(hwnd,over_close);
    }
    result
}

pub unsafe fn shape_install_dark_caption(hwnd:HWND){
    if hwnd.is_null(){return;}
    // WS_CAPTION only: leave overlays, system menus and popup listboxes alone.
    if GetWindowLongPtrW(hwnd,-16)&0x00C0_0000==0 {return;}
    SetWindowSubclass(hwnd,Some(shape_caption_proc),SHAPE_CAPTION_SUBCLASS_ID,0);
    shape_paint_caption(hwnd,false);
}

#[cfg(test)] mod shape_caption_geometry_tests {
    use super::*;
    #[test] fn client_origin_clamped_inside_outer_dark_frame(){
        assert_eq!(shape_clip_coord(-8,600),0);
        assert_eq!(shape_clip_coord(605,600),600);
        assert_eq!(shape_clip_coord(24,600),24);
    }
}
