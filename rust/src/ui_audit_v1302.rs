#![allow(dead_code)]
use std::{ffi::c_void,ptr::{null,null_mut}};
use windows_sys::Win32::{
 Foundation::{HWND,LPARAM,LRESULT,RECT,WPARAM},
 Graphics::Gdi::{DrawTextW,GetDC,GetStockObject,ReleaseDC,SelectObject,SetBkMode,SetTextColor,HDC,TRANSPARENT},
 System::LibraryLoader::GetModuleHandleW,
 UI::WindowsAndMessaging::{CreateWindowExW,DefWindowProcW,DestroyWindow,DispatchMessageW,GetClientRect,GetMessageW,GetWindowLongPtrW,IsDialogMessageW,IsWindow,LoadCursorW,MessageBoxW,PostQuitMessage,RegisterClassW,SendMessageW,SetForegroundWindow,SetWindowLongPtrW,ShowWindow,TranslateMessage,CREATESTRUCTW,CW_USEDEFAULT,GWLP_USERDATA,IDC_ARROW,MSG,SW_SHOW,WM_CLOSE,WM_COMMAND,WM_CREATE,WM_CTLCOLORSTATIC,WM_DRAWITEM,WM_ERASEBKGND,WM_NCCREATE,WM_NCDESTROY,WNDCLASSW,WS_CAPTION,WS_CHILD,WS_POPUP,WS_SYSMENU,WS_TABSTOP,WS_VISIBLE}
};

const CLASS:&str="BPSRReadyAlertAuditDialogV1302";
const IDOK:i32=1;const IDCANCEL:i32=2;const IDYES:i32=6;const IDNO:i32=7;
const MB_OKCANCEL:u32=1;const MB_YESNOCANCEL:u32=3;const MB_YESNO:u32=4;
const MB_ICONERROR:u32=0x10;const MB_ICONWARNING:u32=0x30;
const BS_OWNERDRAW:u32=0x000B;const DT_WORDBREAK:u32=0x10;const DT_CALCRECT:u32=0x400;const DT_NOPREFIX:u32=0x800;
const DWMWCP_ROUND:i32=2;const NULL_BRUSH_STOCK:i32=5;

#[link(name="dwmapi")]extern "system"{fn DwmSetWindowAttribute(hwnd:HWND,attribute:u32,value:*const c_void,size:u32)->i32;}
#[link(name="user32")]extern "system"{fn EnableWindow(hwnd:HWND,enable:i32)->i32;}

#[derive(Clone,Copy,Debug,Eq,PartialEq)]enum Flavor{Ok,OkCancel,YesNo,YesNoCancel}
#[derive(Clone,Copy,Debug,Eq,PartialEq)]enum Severity{Normal,Warning,Error}
#[derive(Clone)]struct Button{id:i32,label:String}
struct State{body:String,result:*mut i32,color:u32,flavor:Flavor,buttons:Vec<Button>,body_h:i32}

fn wide(s:&str)->Vec<u16>{s.encode_utf16().chain(std::iter::once(0)).collect()}
unsafe fn decode(p:*const u16)->String{if p.is_null(){return String::new();}let mut n=0;while *p.add(n)!=0&&n<32768{n+=1;}String::from_utf16_lossy(std::slice::from_raw_parts(p,n))}
fn flavor(flags:u32)->Flavor{match flags&0xf{MB_YESNOCANCEL=>Flavor::YesNoCancel,MB_YESNO=>Flavor::YesNo,MB_OKCANCEL=>Flavor::OkCancel,_=>Flavor::Ok}}
fn severity(flags:u32)->Severity{match flags&0xf0{MB_ICONWARNING=>Severity::Warning,MB_ICONERROR=>Severity::Error,_=>Severity::Normal}}
fn close_result(f:Flavor)->i32{match f{Flavor::YesNoCancel|Flavor::OkCancel=>IDCANCEL,Flavor::YesNo=>IDNO,Flavor::Ok=>IDOK}}
fn buttons(f:Flavor,title:&str,body:&str)->Vec<Button>{let t=title.to_ascii_lowercase();let b=body.to_ascii_lowercase();match f{
 Flavor::YesNoCancel=>if b.contains("save changes")||t.contains("unsaved"){vec![Button{id:IDYES,label:"Save".into()},Button{id:IDNO,label:"Don't Save".into()},Button{id:IDCANCEL,label:"Cancel".into()}]}else{vec![Button{id:IDYES,label:"Yes".into()},Button{id:IDNO,label:"No".into()},Button{id:IDCANCEL,label:"Cancel".into()}]},
 Flavor::YesNo=>if b.contains("discard unsaved"){vec![Button{id:IDYES,label:"Discard".into()},Button{id:IDNO,label:"Keep Editing".into()}]}else if t.contains("update")||b.contains("update"){vec![Button{id:IDYES,label:"Update".into()},Button{id:IDNO,label:"Later".into()}]}else{vec![Button{id:IDYES,label:"Yes".into()},Button{id:IDNO,label:"No".into()}]},
 Flavor::OkCancel=>vec![Button{id:IDOK,label:"OK".into()},Button{id:IDCANCEL,label:"Cancel".into()}],Flavor::Ok=>vec![Button{id:IDOK,label:"OK".into()}]}}

unsafe fn titlebar(hwnd:HWND,caption:u32,border:u32,text:u32){if hwnd.is_null(){return;}let on:i32=1;let rounded=DWMWCP_ROUND;for(a,p,s)in[(20,(&on as*const i32).cast::<c_void>(),4),(33,(&rounded as*const i32).cast::<c_void>(),4),(35,(&caption as*const u32).cast::<c_void>(),4),(34,(&border as*const u32).cast::<c_void>(),4),(36,(&text as*const u32).cast::<c_void>(),4)]{let _=DwmSetWindowAttribute(hwnd,a,p,s);}}
pub unsafe fn mist_titlebar(hwnd:HWND){titlebar(hwnd,crate::ui_modern::MIST_SIDEBAR,crate::ui_modern::MIST_BORDER,crate::ui_modern::BPSR_TEXT)}
pub unsafe fn dark_titlebar(hwnd:HWND){titlebar(hwnd,crate::ui_modern::DARK_SURFACE,crate::ui_modern::DARK_BORDER,crate::ui_modern::BPSR_TEXT)}

unsafe fn register()->bool{let instance=GetModuleHandleW(null());let c=wide(CLASS);let wc=WNDCLASSW{lpfnWndProc:Some(proc),hInstance:instance,hCursor:LoadCursorW(null_mut(),IDC_ARROW),hbrBackground:crate::ui_modern::mist_bg_brush(),lpszClassName:c.as_ptr(),..std::mem::zeroed()};RegisterClassW(&wc)!=0||windows_sys::Win32::Foundation::GetLastError()==1410}
unsafe fn child(parent:HWND,class:&str,text:&str,id:i32,x:i32,y:i32,w:i32,h:i32,extra:u32)->HWND{let hwnd=CreateWindowExW(0,wide(class).as_ptr(),wide(text).as_ptr(),WS_CHILD|WS_VISIBLE|if id!=0{WS_TABSTOP}else{0}|extra,x,y,w,h,parent,id as usize as _,GetModuleHandleW(null()),null_mut());if class.eq_ignore_ascii_case("BUTTON"){crate::ui_modern::theme_control(hwnd);}else if class.eq_ignore_ascii_case("STATIC")&&!hwnd.is_null(){SendMessageW(hwnd,0x30,crate::ui_modern::body_font()as usize,1);}hwnd}
unsafe fn body_height(owner:HWND,body:&str)->i32{let work=crate::ui::work_area(owner);let max=((work.bottom-work.top-36).clamp(215,620)-111).max(72);let dc=GetDC(null_mut());if dc.is_null(){return 104.min(max);}let old=SelectObject(dc,crate::ui_modern::body_font());let w=wide(body);let mut r=RECT{left:0,top:0,right:392,bottom:0};DrawTextW(dc,w.as_ptr(),-1,&mut r,DT_CALCRECT|DT_WORDBREAK|DT_NOPREFIX);SelectObject(dc,old);ReleaseDC(null_mut(),dc);(r.bottom-r.top+10).clamp(72,max)}
unsafe fn paint_bg(hwnd:HWND,hdc:HDC,s:&State){let mut r:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut r);crate::ui_modern::fill_mist_background(hdc,&r);crate::ui_modern::fill_round_rect(hdc,RECT{left:18,top:26,right:28,bottom:36},s.color,5);}

unsafe extern "system" fn proc(hwnd:HWND,msg:u32,wparam:WPARAM,lparam:LPARAM)->LRESULT{if msg==WM_NCCREATE{let cs=lparam as*const CREATESTRUCTW;if !cs.is_null(){SetWindowLongPtrW(hwnd,GWLP_USERDATA,(*cs).lpCreateParams as isize);}}let p=GetWindowLongPtrW(hwnd,GWLP_USERDATA)as*mut State;match msg{
 WM_CREATE=>{mist_titlebar(hwnd);if !p.is_null(){let s=&*p;child(hwnd,"STATIC",&s.body,0,38,20,396,s.body_h,0);let y=38+s.body_h;let bw=if s.buttons.len()>=3{96}else{112};let gap=10;let total=bw*s.buttons.len()as i32+gap*s.buttons.len().saturating_sub(1)as i32;let start=(466-total-18).max(24);let mut first:HWND=null_mut();for(i,b)in s.buttons.iter().enumerate(){let h=child(hwnd,"BUTTON",&b.label,b.id,start+i as i32*(bw+gap),y,bw,32,BS_OWNERDRAW);if first.is_null(){first=h;}}if !first.is_null(){crate::ui::SetFocus(first);}}0},
 WM_ERASEBKGND=>{if p.is_null(){0}else{paint_bg(hwnd,wparam as HDC,&*p);1}},
 WM_CTLCOLORSTATIC=>{let hdc=wparam as HDC;SetBkMode(hdc,TRANSPARENT as i32);SetTextColor(hdc,crate::ui_modern::BPSR_TEXT_SECONDARY);GetStockObject(NULL_BRUSH_STOCK)as LRESULT},
 WM_DRAWITEM=>{let id=(wparam&0xffff)as i32;let primary=!p.is_null()&&(*p).buttons.first().map(|b|b.id==id).unwrap_or(false);crate::ui_modern::draw_button(lparam as _,false,primary,false,false)},
 WM_COMMAND=>{let id=(wparam&0xffff)as i32;if !p.is_null(){let s=&mut*p;let v=if s.buttons.iter().any(|b|b.id==id){Some(id)}else if id==IDCANCEL{Some(close_result(s.flavor))}else{None};if let Some(v)=v{if !s.result.is_null(){*s.result=v;}DestroyWindow(hwnd);return 0;}}0},
 WM_CLOSE=>{if !p.is_null(){let s=&mut*p;if !s.result.is_null(){*s.result=close_result(s.flavor);}}DestroyWindow(hwnd);0},
 WM_NCDESTROY=>{SetWindowLongPtrW(hwnd,GWLP_USERDATA,0);if !p.is_null(){drop(Box::from_raw(p));}DefWindowProcW(hwnd,msg,wparam,lparam)},_=>DefWindowProcW(hwnd,msg,wparam,lparam)}}

pub unsafe fn run_modal_window(hwnd:HWND,owner:HWND){if hwnd.is_null(){return;}if !owner.is_null(){EnableWindow(owner,0);}ShowWindow(hwnd,SW_SHOW);SetForegroundWindow(hwnd);let mut msg:MSG=std::mem::zeroed();while IsWindow(hwnd)!=0{let got=GetMessageW(&mut msg,null_mut(),0,0);if got<=0{if got==0{PostQuitMessage(0);}break;}if IsDialogMessageW(hwnd,&msg)==0{TranslateMessage(&msg);DispatchMessageW(&msg);}}if !owner.is_null(){EnableWindow(owner,1);SetForegroundWindow(owner);}}

pub unsafe fn message_box_w(owner:HWND,text:*const u16,title:*const u16,flags:u32)->i32{let body=decode(text);let title=decode(title);let f=flavor(flags);if !register(){return MessageBoxW(owner,text,wide(&title).as_ptr(),flags);}let color=match severity(flags){Severity::Error=>crate::ui_modern::BPSR_DANGER,Severity::Warning=>crate::ui_modern::BPSR_WARNING,Severity::Normal=>crate::ui_modern::BPSR_INFO};let h=body_height(owner,&body);let mut result=close_result(f);let state=Box::new(State{buttons:buttons(f,&title,&body),body,result:&mut result,color,flavor:f,body_h:h});let ptr=Box::into_raw(state);let hwnd=CreateWindowExW(0,wide(CLASS).as_ptr(),wide(&title).as_ptr(),WS_POPUP|WS_CAPTION|WS_SYSMENU,CW_USEDEFAULT,CW_USEDEFAULT,466,h+113,owner,null_mut(),GetModuleHandleW(null()),ptr.cast::<c_void>());if hwnd.is_null(){drop(Box::from_raw(ptr));return MessageBoxW(owner,text,wide(&title).as_ptr(),flags);}crate::ui::fit_window(hwnd,owner,true);run_modal_window(hwnd,owner);result}

#[cfg(test)]mod tests{use super::*;#[test]fn warning_not_error(){assert_eq!(severity(MB_ICONWARNING),Severity::Warning);assert_eq!(severity(MB_ICONERROR),Severity::Error);}#[test]fn yes_no_cancel_has_cancel(){let b=buttons(Flavor::YesNoCancel,"Unsaved rules","Save changes before closing?");assert_eq!(b.iter().map(|x|x.id).collect::<Vec<_>>(),vec![IDYES,IDNO,IDCANCEL]);assert_eq!(close_result(Flavor::YesNoCancel),IDCANCEL);}#[test]fn settings_labels_are_clear(){let b=buttons(Flavor::YesNo,"ReadyAlert Settings","Discard unsaved settings changes?");assert_eq!(b[0].label,"Discard");assert_eq!(b[1].label,"Keep Editing");}#[test]fn pixel_corner_value_is_two(){assert_eq!(DWMWCP_ROUND,2);}}
