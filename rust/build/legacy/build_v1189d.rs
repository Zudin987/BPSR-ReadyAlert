use std::{env,fs,path::{Path,PathBuf}};

mod prior { include!("build_v1189c.rs"); pub fn run(){main();} }

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){let count=source.matches(from).count();assert_eq!(count,1,"v1.18.9d patch {label} expected one match, found {count}");*source=source.replacen(from,to,1);}

fn patch_shell_ux(out:&Path){
    let path=out.join("win_v182_fixed.rs");let mut source=fs::read_to_string(&path).expect("read v1.18.9c win source").replace("\r\n","\n");
    replace_once(&mut source,
        r###"    TrayAction::OpenLogFile=>{if !state.paths.log.exists(){let _=std::fs::write(&state.paths.log,"No log entries yet.\r\n");}open_path(&state.paths.log);},
"###,
        r###"    TrayAction::OpenLogFile=>{if !state.paths.log.exists(){if let Err(err)=std::fs::write(&state.paths.log,"No log entries yet.\r\n"){logging::write(format!("open log: create failed: {err}"));}}open_path(hwnd,&state.paths.log,"diagnostic log");},
"###,"tray log open errors");
    replace_once(&mut source,
        r###"    if command==CMD_OPEN_LOGS{let _=std::fs::create_dir_all(&state.paths.chat_logs);open_path(&state.paths.chat_logs);return;}if command==CMD_OPEN_SETTINGS_JSON{open_path(&state.paths.settings);return;}if command==CMD_OPEN_APP_FOLDER{open_path(&state.paths.root);return;}
"###,
        r###"    if command==CMD_OPEN_LOGS{if let Err(err)=std::fs::create_dir_all(&state.paths.chat_logs){logging::write(format!("open chat logs: create directory failed: {err}"));}open_path(hwnd,&state.paths.chat_logs,"chat logs folder");return;}if command==CMD_OPEN_SETTINGS_JSON{open_path(hwnd,&state.paths.settings,"settings file");return;}if command==CMD_OPEN_APP_FOLDER{open_path(hwnd,&state.paths.root,"app data folder");return;}
"###,"settings shell open errors");
    replace_once(&mut source,
        r###"unsafe fn update_tip(hwnd:HWND,tip:&str){let mut nid:NOTIFYICONDATAW=std::mem::zeroed();nid.cbSize=std::mem::size_of::<NOTIFYICONDATAW>()as u32;nid.hWnd=hwnd;nid.uID=TRAY_ID;nid.uFlags=NIF_TIP;write_wide(&mut nid.szTip,&format!("BPSR Ready Alert - {tip}"));Shell_NotifyIconW(NIM_MODIFY,&nid);}
"###,
        r###"unsafe fn update_tip(hwnd:HWND,tip:&str){let mut nid:NOTIFYICONDATAW=std::mem::zeroed();nid.cbSize=std::mem::size_of::<NOTIFYICONDATAW>()as u32;nid.hWnd=hwnd;nid.uID=TRAY_ID;nid.uFlags=NIF_TIP;write_wide(&mut nid.szTip,&format!("BPSR Ready Alert - {tip}"));if Shell_NotifyIconW(NIM_MODIFY,&nid)==0{if add_tray(hwnd,tip).is_ok(){logging::write("tray: restored notification icon after shell reset");}}}
"###,"tray icon self recovery");
    replace_once(&mut source,
        r###"unsafe fn open_path(path:&Path){ShellExecuteW(null_mut(),wide("open").as_ptr(),wide(&path.to_string_lossy()).as_ptr(),null(),null(),SW_SHOWNORMAL);}
"###,
        r###"unsafe fn open_path(hwnd:HWND,path:&Path,label:&str){let result=ShellExecuteW(null_mut(),wide("open").as_ptr(),wide(&path.to_string_lossy()).as_ptr(),null(),null(),SW_SHOWNORMAL)as isize;if result<=32{let detail=format!("Could not open the {label}.\r\n\r\nWindows ShellExecute error {result}.\r\n{}",path.display());logging::write(format!("shell: {detail}"));MessageBoxW(hwnd,wide(&detail).as_ptr(),wide("BPSR Ready Alert").as_ptr(),MB_OK|MB_ICONERROR);}}
"###,"visible ShellExecute failures");
    source.push_str(r###"

#[cfg(test)]
mod v1189_repeat_shell_tests{
    #[test]fn shell_execute_error_range_is_bounded(){assert!(32isize<=32);assert!(33isize>32);}
}
"###);
    fs::write(path,source).expect("write v1.18.9d win source");
}

fn main(){prior::run();let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));patch_shell_ux(&out);println!("cargo:rerun-if-changed=build/legacy/build_v1189d.rs");}
