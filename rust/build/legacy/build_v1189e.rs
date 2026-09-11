use std::{env,fs,path::{Path,PathBuf}};

mod prior { include!("build_v1189d.rs"); pub fn run(){main();} }

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){let count=source.matches(from).count();assert_eq!(count,1,"v1.18.9e patch {label} expected one match, found {count}");*source=source.replacen(from,to,1);}

fn patch_win_resilience(out:&Path){
    let path=out.join("win_v182_fixed.rs");let mut source=fs::read_to_string(&path).expect("read v1.18.9d win source").replace("\r\n","\n");
    replace_once(&mut source,
        "sync::{atomic::{AtomicBool,Ordering},mpsc::{Receiver,TryRecvError},Arc,RwLock}",
        "sync::{atomic::{AtomicBool,Ordering},mpsc::{Receiver,TryRecvError},Arc,OnceLock,RwLock}",
        "OnceLock import");
    replace_once(&mut source,
        r###"#[link(name="user32")]extern "system"{fn RegisterHotKey(hwnd:HWND,id:i32,modifiers:u32,vk:u32)->i32;fn UnregisterHotKey(hwnd:HWND,id:i32)->i32;}"###,
        r###"#[link(name="user32")]extern "system"{fn RegisterHotKey(hwnd:HWND,id:i32,modifiers:u32,vk:u32)->i32;fn UnregisterHotKey(hwnd:HWND,id:i32)->i32;fn RegisterWindowMessageW(name:*const u16)->u32;}"###,
        "TaskbarCreated registration");
    replace_once(&mut source,
        "const CMD_OPEN_SETTINGS:u32=1010;const CMD_OPEN_LOGS:u32=1011;",
        r###"static TTS_TEST_BUSY:AtomicBool=AtomicBool::new(false);
fn claim_tts_test()->bool{TTS_TEST_BUSY.compare_exchange(false,true,Ordering::AcqRel,Ordering::Acquire).is_ok()}
fn release_tts_test(){TTS_TEST_BUSY.store(false,Ordering::Release);}
fn taskbar_created_message()->u32{static MESSAGE:OnceLock<u32>=OnceLock::new();*MESSAGE.get_or_init(||unsafe{RegisterWindowMessageW(wide("TaskbarCreated").as_ptr())})}

const CMD_OPEN_SETTINGS:u32=1010;const CMD_OPEN_LOGS:u32=1011;"###,
        "maintenance statics");
    replace_once(&mut source,
        r###"    let ptr=GetWindowLongPtrW(hwnd,GWLP_USERDATA)as *mut UiState;
    match msg{"###,
        r###"    let ptr=GetWindowLongPtrW(hwnd,GWLP_USERDATA)as *mut UiState;
    let shell_message=taskbar_created_message();if shell_message!=0&&msg==shell_message{if !ptr.is_null(){let state=&*ptr;match add_tray(hwnd,&state.capture_status){Ok(())=>logging::write("tray: restored notification icon after Explorer restart"),Err(err)=>logging::write(format!("tray: Explorer restart restore failed: {err}")),}}return 0;}
    match msg{"###,
        "TaskbarCreated handler");
    replace_once(&mut source,
        r###"    if command==CMD_TTS_TEST{let volume=(lparam as i32).clamp(0,100);thread::spawn(move||{if let Err(err)=test_tts(volume){logging::write(format!("tts: test failed: {err}"));}});return;}"###,
        r###"    if command==CMD_TTS_TEST{start_tts_test((lparam as i32).clamp(0,100));return;}"###,
        "bounded TTS test command");
    replace_once(&mut source,
        "fn test_tts(volume:i32)->Result<(),String>{",
        r###"fn start_tts_test(volume:i32){
    if !claim_tts_test(){logging::write("tts: test ignored because another test is already running");return;}
    if let Err(err)=thread::Builder::new().name("readyalert-tts-test".into()).spawn(move||{if let Err(err)=test_tts(volume){logging::write(format!("tts: test failed: {err}"));}release_tts_test();}){release_tts_test();logging::write(format!("tts: test worker spawn failed: {err}"));}
}

fn test_tts(volume:i32)->Result<(),String>{"###,
        "single-flight TTS test worker");
    source.push_str(r###"

#[cfg(test)]
mod v1189_repeat_runtime_tests{
    use super::*;
    #[test]fn tts_test_is_single_flight(){release_tts_test();assert!(claim_tts_test());assert!(!claim_tts_test());release_tts_test();}
}
"###);
    fs::write(path,source).expect("write v1.18.9e win source");
}

fn patch_chat_clipboard(out:&Path){
    let path=out.join("overlay_v150_v1181.rs");let mut source=fs::read_to_string(&path).expect("read v1.18.9d chat overlay source").replace("\r\n","\n");
    replace_once(&mut source,
        r###"unsafe fn copy_to_clipboard(hwnd: HWND, text: &str) -> bool {
    if text.is_empty() || OpenClipboard(hwnd) == 0 { return false; }
    let mut wide_text: Vec<u16> = text.encode_utf16().collect();
    wide_text.push(0);
    let bytes = wide_text.len() * std::mem::size_of::<u16>();
    let memory = GlobalAlloc(GMEM_MOVEABLE, bytes);
    if memory.is_null() {
        CloseClipboard();
        return false;
    }
    let target = GlobalLock(memory) as *mut u16;
    if target.is_null() {
        let _ = GlobalFree(memory);
        CloseClipboard();
        return false;
    }
    std::ptr::copy_nonoverlapping(wide_text.as_ptr(), target, wide_text.len());
    let _ = GlobalUnlock(memory);
    let _ = EmptyClipboard();
    let stored = SetClipboardData(CF_UNICODETEXT, memory);
    if stored.is_null() { let _ = GlobalFree(memory); }
    CloseClipboard();
    !stored.is_null()
}
"###,
        r###"unsafe fn copy_to_clipboard(hwnd: HWND, text: &str) -> bool {
    if text.is_empty() { return false; }
    let mut opened=false;
    for _ in 0..5 { if OpenClipboard(hwnd)!=0 {opened=true;break;} std::thread::sleep(std::time::Duration::from_millis(10)); }
    if !opened { logging::write("chat: clipboard stayed busy after bounded retries"); return false; }
    let mut wide_text: Vec<u16> = text.encode_utf16().collect();
    wide_text.push(0);
    let bytes = wide_text.len() * std::mem::size_of::<u16>();
    let memory = GlobalAlloc(GMEM_MOVEABLE, bytes);
    if memory.is_null() { CloseClipboard(); logging::write("chat: clipboard allocation failed"); return false; }
    let target = GlobalLock(memory) as *mut u16;
    if target.is_null() { let _ = GlobalFree(memory); CloseClipboard(); logging::write("chat: clipboard lock failed"); return false; }
    std::ptr::copy_nonoverlapping(wide_text.as_ptr(), target, wide_text.len());
    let _ = GlobalUnlock(memory);
    if EmptyClipboard()==0 { let _=GlobalFree(memory); CloseClipboard(); logging::write("chat: EmptyClipboard failed"); return false; }
    let stored = SetClipboardData(CF_UNICODETEXT, memory);
    if stored.is_null() { let _ = GlobalFree(memory); logging::write("chat: SetClipboardData failed"); }
    CloseClipboard();
    !stored.is_null()
}
"###,
        "clipboard retry and error handling");
    fs::write(path,source).expect("write v1.18.9e chat overlay source");
}

fn main(){prior::run();let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));patch_win_resilience(&out);patch_chat_clipboard(&out);println!("cargo:rerun-if-changed=build/legacy/build_v1189e.rs");}
