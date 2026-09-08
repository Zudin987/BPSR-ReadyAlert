use crate::{
    feature_settings::{self, FeatureSettings},
    model::{DpsSnapshot, MechanicSnapshot},
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
        InvalidateRect, SelectObject, SetBkMode, SetTextColor, DEFAULT_GUI_FONT, PAINTSTRUCT,
        TRANSPARENT,
    },
    UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, GetClientRect, GetWindowLongPtrW, GetWindowRect, LoadCursorW,
        PostMessageW, RegisterClassW, SendMessageW, SetLayeredWindowAttributes, SetWindowLongPtrW,
        SetWindowPos, ShowWindow, CREATESTRUCTW, CW_USEDEFAULT, GWLP_USERDATA, HWND_TOPMOST,
        IDC_ARROW, LWA_ALPHA, SW_HIDE, SWP_NOACTIVATE, WM_ERASEBKGND, WM_EXITSIZEMOVE,
        WM_LBUTTONDOWN, WM_NCCREATE, WM_NCDESTROY, WM_PAINT, WM_SIZE, WS_EX_LAYERED,
        WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WNDCLASSW,
    },
};

const DPS_CLASS: &str = "BPSRReadyAlertDpsOverlayV160";
const MECH_CLASS: &str = "BPSRReadyAlertMechanicsOverlayV160";
const TOOLBAR_H: i32 = 36;
const BUTTON_W: i32 = 34;
const COLLAPSED: i32 = 25;
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
enum Kind { Dps, Mechanics }

struct State {
    kind: Kind,
    main_hwnd: HWND,
    paths: AppPaths,
    features: Arc<RwLock<FeatureSettings>>,
    dps: DpsSnapshot,
    mechanics: MechanicSnapshot,
    collapsed: bool,
    expanded: RECT,
}

pub unsafe fn create_dps(instance: HINSTANCE, main_hwnd: HWND, features: Arc<RwLock<FeatureSettings>>, paths: AppPaths) -> Result<HWND,String> {
    create(instance, main_hwnd, features, paths, Kind::Dps)
}
pub unsafe fn create_mechanics(instance: HINSTANCE, main_hwnd: HWND, features: Arc<RwLock<FeatureSettings>>, paths: AppPaths) -> Result<HWND,String> {
    create(instance, main_hwnd, features, paths, Kind::Mechanics)
}

unsafe fn create(instance:HINSTANCE, main_hwnd:HWND, features:Arc<RwLock<FeatureSettings>>, paths:AppPaths, kind:Kind)->Result<HWND,String>{
    let class_name=if kind==Kind::Dps{DPS_CLASS}else{MECH_CLASS};
    let class=wide(class_name);
    let wc=WNDCLASSW{lpfnWndProc:Some(wnd_proc),hInstance:instance,hCursor:LoadCursorW(null_mut(),IDC_ARROW),lpszClassName:class.as_ptr(),..std::mem::zeroed()};
    if RegisterClassW(&wc)==0 && GetLastError()!=1410{return Err(format!("RegisterClassW({class_name}) failed: {}",GetLastError()));}
    let snap=features.read().map(|x|x.clone()).unwrap_or_default();
    let layout=if kind==Kind::Dps{snap.dps.clone()}else{snap.mechanics.clone()};
    let x=if layout.x==i32::MIN{CW_USEDEFAULT}else{layout.x};
    let y=if layout.y==i32::MIN{CW_USEDEFAULT}else{layout.y};
    let state=Box::new(State{kind,main_hwnd,paths,features,dps:DpsSnapshot::default(),mechanics:MechanicSnapshot::default(),collapsed:false,expanded:RECT{left:x,top:y,right:x.saturating_add(layout.width),bottom:y.saturating_add(layout.height)}});
    let ptr=Box::into_raw(state);
    let hwnd=CreateWindowExW(WS_EX_TOOLWINDOW|WS_EX_LAYERED|WS_EX_TOPMOST,class.as_ptr(),wide(if kind==Kind::Dps{"ReadyAlert DPS Meter"}else{"ReadyAlert Dungeon Mechanics"}).as_ptr(),WS_POPUP,x,y,layout.width,layout.height,null_mut(),null_mut(),instance,ptr.cast::<c_void>());
    if hwnd.is_null(){drop(Box::from_raw(ptr));return Err(format!("CreateWindowExW({class_name}) failed: {}",GetLastError()));}
    SetLayeredWindowAttributes(hwnd,0,((layout.opacity*255/100).clamp(1,255)) as u8,LWA_ALPHA);
    SetWindowPos(hwnd,HWND_TOPMOST,0,0,0,0,SWP_NOACTIVATE|0x0001|0x0002);
    Ok(hwnd)
}

pub unsafe fn update_dps(hwnd:HWND, snapshot:DpsSnapshot){
    with_state(hwnd,|s|s.dps=snapshot); InvalidateRect(hwnd,null(),0);
}
pub unsafe fn update_mechanics(hwnd:HWND, snapshot:MechanicSnapshot){
    with_state(hwnd,|s|s.mechanics=snapshot); InvalidateRect(hwnd,null(),0);
}
pub unsafe fn expand(hwnd:HWND){with_state(hwnd,|s|{if s.collapsed{expand_state(hwnd,s);}});}

unsafe extern "system" fn wnd_proc(hwnd:HWND,msg:u32,wparam:WPARAM,lparam:LPARAM)->LRESULT{
    if msg==WM_NCCREATE{let cs=lparam as *const CREATESTRUCTW;if !cs.is_null(){SetWindowLongPtrW(hwnd,GWLP_USERDATA,(*cs).lpCreateParams as isize);}}
    let ptr=GetWindowLongPtrW(hwnd,GWLP_USERDATA) as *mut State;
    match msg{
        WM_ERASEBKGND=>1,
        WM_PAINT=>{if !ptr.is_null(){paint(hwnd,&mut *ptr);}0}
        WM_SIZE=>{InvalidateRect(hwnd,null(),0);0}
        WM_LBUTTONDOWN=>{if !ptr.is_null(){on_click(hwnd,&mut *ptr,lparam);}0}
        WM_EXITSIZEMOVE=>{if !ptr.is_null(){save_bounds(hwnd,&mut *ptr);}0}
        WM_NCDESTROY=>{SetWindowLongPtrW(hwnd,GWLP_USERDATA,0);if !ptr.is_null(){drop(Box::from_raw(ptr));}DefWindowProcW(hwnd,msg,wparam,lparam)}
        _=>DefWindowProcW(hwnd,msg,wparam,lparam),
    }
}

unsafe fn on_click(hwnd:HWND,state:&mut State,lparam:LPARAM){
    if state.collapsed{expand_state(hwnd,state);return;}
    let x=(lparam as i16) as i32;let y=((lparam>>16) as i16) as i32;
    let mut rc:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rc);
    if y<TOOLBAR_H{
        if x>=rc.right-BUTTON_W{
            PostMessageW(state.main_hwnd,0x0111,if state.kind==Kind::Dps{CMD_HIDE_DPS as usize}else{CMD_HIDE_MECHANICS as usize},0);
        }else if x>=rc.right-BUTTON_W*2{
            collapse(hwnd,state);
        }else{
            extern "system"{fn ReleaseCapture()->i32;}
            ReleaseCapture();SendMessageW(hwnd,WM_NCLBUTTONDOWN_,HTCAPTION_,0);
        }
    }
}

unsafe fn paint(hwnd:HWND,state:&mut State){
    let mut ps:PAINTSTRUCT=std::mem::zeroed();let hdc=BeginPaint(hwnd,&mut ps);if hdc.is_null(){return;}
    let mut rc:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rc);
    fill(hdc,&rc,rgb(18,22,27));
    SelectObject(hdc,GetStockObject(DEFAULT_GUI_FONT));SetBkMode(hdc,TRANSPARENT);
    if state.collapsed{
        SetTextColor(hdc,rgb(99,199,255));draw(hdc,if state.kind==Kind::Dps{"D"}else{"M"},rc,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);EndPaint(hwnd,&ps);return;
    }
    let toolbar=RECT{left:0,top:0,right:rc.right,bottom:TOOLBAR_H};fill(hdc,&toolbar,rgb(23,28,35));
    let accent=RECT{left:0,top:TOOLBAR_H-2,right:rc.right,bottom:TOOLBAR_H};fill(hdc,&accent,rgb(63,133,255));
    SetTextColor(hdc,rgb(231,237,244));
    let title=if state.kind==Kind::Dps{"DPS Meter"}else{"Dungeon Mechanics"};
    draw(hdc,title,RECT{left:10,top:0,right:rc.right-110,bottom:TOOLBAR_H},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);
    SetTextColor(hdc,rgb(170,183,199));draw(hdc,"◀",RECT{left:rc.right-BUTTON_W*2,top:0,right:rc.right-BUTTON_W,bottom:TOOLBAR_H},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);
    draw(hdc,"×",RECT{left:rc.right-BUTTON_W,top:0,right:rc.right,bottom:TOOLBAR_H},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);
    match state.kind{Kind::Dps=>paint_dps(hdc,rc,&state.dps),Kind::Mechanics=>paint_mechanics(hdc,rc,&state.mechanics)}
    EndPaint(hwnd,&ps);
}

unsafe fn paint_dps(hdc:isize,rc:RECT,s:&DpsSnapshot){
    let summary=format!("{}  Total {}",format_time(s.encounter_ms),compact(s.total_damage as f64));
    SetTextColor(hdc,rgb(145,160,180));draw(hdc,&summary,RECT{left:10,top:TOOLBAR_H+4,right:rc.right-10,bottom:TOOLBAR_H+27},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);
    let top=TOOLBAR_H+30;let row_h=29;let max=((rc.bottom-top)/row_h).max(0) as usize;
    if s.rows.is_empty(){SetTextColor(hdc,rgb(132,145,162));draw(hdc,"Waiting for combat...",RECT{left:10,top:top+18,right:rc.right-10,bottom:top+55},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);return;}
    for(i,row)in s.rows.iter().take(max).enumerate(){let y=top+i as i32*row_h;let r=RECT{left:6,top:y,right:rc.right-6,bottom:y+row_h-2};fill(hdc,&r,if i%2==0{rgb(28,33,40)}else{rgb(24,29,35)});let bar_w=((r.right-r.left) as f64*(row.share/100.0).clamp(0.0,1.0)) as i32;if bar_w>0{fill(hdc,&RECT{left:r.left,top:r.bottom-3,right:r.left+bar_w,bottom:r.bottom},rgb(63,133,255));}
        SetTextColor(hdc,rgb(99,199,255));draw(hdc,&format!("{}",i+1),RECT{left:r.left+5,top:r.top,right:r.left+28,bottom:r.bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);
        SetTextColor(hdc,rgb(235,240,246));draw(hdc,&row.name,RECT{left:r.left+30,top:r.top,right:r.right-205,bottom:r.bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);
        SetTextColor(hdc,rgb(182,198,216));draw(hdc,&format!("{}/s",compact(row.dps)),RECT{left:r.right-200,top:r.top,right:r.right-108,bottom:r.bottom},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);
        draw(hdc,&compact(row.damage as f64),RECT{left:r.right-105,top:r.top,right:r.right-45,bottom:r.bottom},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);
        SetTextColor(hdc,rgb(143,234,143));draw(hdc,&format!("{:.0}%",row.share),RECT{left:r.right-43,top:r.top,right:r.right-5,bottom:r.bottom},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);
    }
}

unsafe fn paint_mechanics(hdc:isize,rc:RECT,s:&MechanicSnapshot){
    let top=TOOLBAR_H+7;let row_h=40;let max=((rc.bottom-top)/row_h).max(0)as usize;let now=now_ms();
    if s.rows.is_empty(){SetTextColor(hdc,rgb(132,145,162));draw(hdc,"No active mechanic",RECT{left:10,top:top+18,right:rc.right-10,bottom:top+58},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);return;}
    for(i,row)in s.rows.iter().filter(|r|r.persistent||r.expires_unix_ms<=0||r.expires_unix_ms>now).take(max).enumerate(){let y=top+i as i32*row_h;let r=RECT{left:6,top:y,right:rc.right-6,bottom:y+row_h-3};fill(hdc,&r,if i%2==0{rgb(28,33,40)}else{rgb(24,29,35)});let accent=if row.priority>=3{rgb(255,99,99)}else{rgb(99,199,255)};fill(hdc,&RECT{left:r.left,top:r.top,right:r.left+3,bottom:r.bottom},accent);
        SetTextColor(hdc,rgb(238,242,247));draw(hdc,&row.label,RECT{left:r.left+10,top:r.top+2,right:r.right-78,bottom:r.top+21},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);
        if let Some(target)=&row.target{SetTextColor(hdc,rgb(159,221,255));draw(hdc,target,RECT{left:r.left+10,top:r.top+19,right:r.right-78,bottom:r.bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}
        let timer=if row.persistent||row.expires_unix_ms<=0{"LIVE".to_string()}else{format!("{:.1}s",((row.expires_unix_ms-now).max(0)as f64)/1000.0)};SetTextColor(hdc,accent);draw(hdc,&timer,RECT{left:r.right-72,top:r.top,right:r.right-8,bottom:r.bottom},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);
    }
}

unsafe fn collapse(hwnd:HWND,state:&mut State){
    let mut r:RECT=std::mem::zeroed();GetWindowRect(hwnd,&mut r);state.expanded=r;let work=monitor_work(hwnd);let side=layout_side(state);
    let w=(r.right-r.left).max(100);let h=(r.bottom-r.top).max(80);
    let(x,y,nw,nh)=match side.as_str(){"left"=>(work.left,r.top.clamp(work.top,work.bottom-COLLAPSED),COLLAPSED,h.min(work.bottom-work.top)),"top"=>(r.left.clamp(work.left,work.right-COLLAPSED),work.top,w.min(work.right-work.left),COLLAPSED),"bottom"=>(r.left.clamp(work.left,work.right-COLLAPSED),work.bottom-COLLAPSED,w.min(work.right-work.left),COLLAPSED),_=>(work.right-COLLAPSED,r.top.clamp(work.top,work.bottom-COLLAPSED),COLLAPSED,h.min(work.bottom-work.top))};
    state.collapsed=true;SetWindowPos(hwnd,HWND_TOPMOST,x,y,nw,nh,SWP_NOACTIVATE);InvalidateRect(hwnd,null(),0);
}
unsafe fn expand_state(hwnd:HWND,state:&mut State){let r=state.expanded;state.collapsed=false;SetWindowPos(hwnd,HWND_TOPMOST,r.left,r.top,(r.right-r.left).max(100),(r.bottom-r.top).max(80),SWP_NOACTIVATE);InvalidateRect(hwnd,null(),0);}

unsafe fn save_bounds(hwnd:HWND,state:&mut State){if state.collapsed{return;}let mut r:RECT=std::mem::zeroed();if GetWindowRect(hwnd,&mut r)==0{return;}state.expanded=r;if let Ok(mut f)=state.features.write(){let l=if state.kind==Kind::Dps{&mut f.dps}else{&mut f.mechanics};l.x=r.left;l.y=r.top;l.width=(r.right-r.left).max(1);l.height=(r.bottom-r.top).max(1);let snap=f.clone();drop(f);let _=feature_settings::save(&state.paths,&snap);}}
fn layout_side(state:&State)->String{state.features.read().map(|f|if state.kind==Kind::Dps{f.dps.collapse_side.clone()}else{f.mechanics.collapse_side.clone()}).unwrap_or_else(|_|"Right".into()).to_ascii_lowercase()}

#[repr(C)]struct MonitorInfo{cb_size:u32,rc_monitor:RECT,rc_work:RECT,flags:u32}
#[link(name="user32")]extern "system"{fn MonitorFromWindow(hwnd:HWND,flags:u32)->*mut c_void;fn GetMonitorInfoW(monitor:*mut c_void,info:*mut MonitorInfo)->i32;}
unsafe fn monitor_work(hwnd:HWND)->RECT{let m=MonitorFromWindow(hwnd,2);let mut i=MonitorInfo{cb_size:std::mem::size_of::<MonitorInfo>()as u32,rc_monitor:std::mem::zeroed(),rc_work:std::mem::zeroed(),flags:0};if !m.is_null()&&GetMonitorInfoW(m,&mut i)!=0{i.rc_work}else{RECT{left:0,top:0,right:1920,bottom:1080}}}

unsafe fn with_state<F:FnOnce(&mut State)>(hwnd:HWND,f:F){let p=GetWindowLongPtrW(hwnd,GWLP_USERDATA)as *mut State;if !p.is_null(){f(&mut*p);}}
unsafe fn fill(hdc:isize,r:&RECT,c:u32){let b=CreateSolidBrush(c);if !b.is_null(){FillRect(hdc,r,b);DeleteObject(b);}}
unsafe fn draw(hdc:isize,text:&str,mut r:RECT,flags:u32){let w=wide(text);DrawTextW(hdc,w.as_ptr(),-1,&mut r,flags);}
fn compact(v:f64)->String{let a=v.abs();if a>=1_000_000_000.0{format!("{:.2}B",v/1_000_000_000.0)}else if a>=1_000_000.0{format!("{:.2}M",v/1_000_000.0)}else if a>=1_000.0{format!("{:.1}K",v/1_000.0)}else{format!("{:.0}",v)}}
fn format_time(ms:u64)->String{let s=ms/1000;format!("{}:{:02}",s/60,s%60)}
fn now_ms()->i64{SystemTime::now().duration_since(UNIX_EPOCH).map(|d|d.as_millis().min(i64::MAX as u128)as i64).unwrap_or(0)}
fn rgb(r:u8,g:u8,b:u8)->u32{u32::from(r)|(u32::from(g)<<8)|(u32::from(b)<<16)}
fn wide(s:&str)->Vec<u16>{s.encode_utf16().chain(std::iter::once(0)).collect()}
