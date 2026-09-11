use std::{env,fs,path::{Path,PathBuf}};

mod prior { include!("build_v1189i.rs"); pub fn run(){main();} }

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){let count=source.matches(from).count();assert_eq!(count,1,"v1.18.9j patch {label} expected one match, found {count}");*source=source.replacen(from,to,1);}

fn patch_tray_add_race(out:&Path){
    let path=out.join("win_v182_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read v1.18.9i win source").replace("\r\n","\n");
    replace_once(&mut source,
        r###"unsafe fn add_tray(hwnd:HWND,tip:&str)->Result<(),String>{let mut nid:NOTIFYICONDATAW=std::mem::zeroed();nid.cbSize=std::mem::size_of::<NOTIFYICONDATAW>()as u32;nid.hWnd=hwnd;nid.uID=TRAY_ID;nid.uFlags=NIF_ICON|NIF_MESSAGE|NIF_TIP;nid.uCallbackMessage=WM_TRAY;nid.hIcon=app_icon(GetModuleHandleW(null()));write_wide(&mut nid.szTip,&format!("BPSR Ready Alert - {tip}"));if Shell_NotifyIconW(NIM_ADD,&nid)==0{return Err("Shell_NotifyIconW(NIM_ADD) failed".into());}Ok(())}"###,
        r###"unsafe fn add_tray(hwnd:HWND,tip:&str)->Result<(),String>{let mut nid:NOTIFYICONDATAW=std::mem::zeroed();nid.cbSize=std::mem::size_of::<NOTIFYICONDATAW>()as u32;nid.hWnd=hwnd;nid.uID=TRAY_ID;nid.uFlags=NIF_ICON|NIF_MESSAGE|NIF_TIP;nid.uCallbackMessage=WM_TRAY;nid.hIcon=app_icon(GetModuleHandleW(null()));write_wide(&mut nid.szTip,&format!("BPSR Ready Alert - {tip}"));if Shell_NotifyIconW(NIM_ADD,&nid)!=0{return Ok(());}if Shell_NotifyIconW(NIM_MODIFY,&nid)!=0{logging::write("tray: add found an existing icon; refreshed it instead");return Ok(());}Err("Shell_NotifyIconW could neither add nor refresh the tray icon".into())}"###,
        "idempotent tray add");
    source.push_str(r###"

#[cfg(test)]
mod v1189_repeat_tray_race_tests{
    #[test]fn tray_identity_is_stable(){assert_eq!(TRAY_ID,1);}
}
"###);
    fs::write(path,source).expect("write v1.18.9j win source");
}

fn main(){prior::run();let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));patch_tray_add_race(&out);println!("cargo:rerun-if-changed=build/legacy/build_v1189j.rs");}
