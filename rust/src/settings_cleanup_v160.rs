use std::{ptr::{null,null_mut},sync::{atomic::{AtomicBool,Ordering},Arc},thread,time::Duration};
use windows_sys::Win32::{Foundation::{BOOL,HWND,LPARAM},UI::WindowsAndMessaging::{EnumChildWindows,FindWindowW,GetDlgItem,GetWindowTextLengthW,GetWindowTextW,IsWindow,SetWindowTextW,ShowWindow,SW_HIDE}};

const SETTINGS_CLASS:&str="BPSRReadyAlertRustSettingsV151";
const RETIRED_IDS:[i32;6]=[3205,3210,3217,3218,3219,3330];

pub fn start(stop:Arc<AtomicBool>)->thread::JoinHandle<()>{thread::Builder::new().name("readyalert-settings-cleanup".into()).spawn(move||{let class=wide(SETTINGS_CLASS);while !stop.load(Ordering::Relaxed){unsafe{let hwnd=FindWindowW(class.as_ptr(),null());if !hwnd.is_null()&&IsWindow(hwnd)!=0{for id in RETIRED_IDS{let child=GetDlgItem(hwnd,id);if !child.is_null(){ShowWindow(child,SW_HIDE);}}EnumChildWindows(hwnd,Some(child_proc),0);}}thread::sleep(Duration::from_millis(200));}}).expect("spawn settings cleanup")}

unsafe extern "system" fn child_proc(hwnd:HWND,_:LPARAM)->BOOL{let len=GetWindowTextLengthW(hwnd);if len<=0{return 1;}let mut buf=vec![0u16;len as usize+1];let got=GetWindowTextW(hwnd,buf.as_mut_ptr(),buf.len()as i32);if got<=0{return 1;}let text=String::from_utf16_lossy(&buf[..got as usize]);match text.as_str(){"Auto-launch Resonance Logs CN"|"Enable Chat Overlay"|"Bold message text"|"Text shadow"|"Row separators"|"Resonance Logs CN executable path"=>{ShowWindow(hwnd,SW_HIDE);},"Npcap device name (blank = follow Resonance Logs / auto)"=>{SetWindowTextW(hwnd,wide("Npcap device name (blank = auto)").as_ptr());},_=>{}}1}
fn wide(text:&str)->Vec<u16>{text.encode_utf16().chain(std::iter::once(0)).collect()}
