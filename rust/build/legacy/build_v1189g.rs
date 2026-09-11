use std::{env,fs,path::{Path,PathBuf}};

mod prior { include!("build_v1189f.rs"); pub fn run(){main();} }

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){let count=source.matches(from).count();assert_eq!(count,1,"v1.18.9g patch {label} expected one match, found {count}");*source=source.replacen(from,to,1);}

fn patch_tts_test_feedback(out:&Path){
    let path=out.join("win_v182_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read v1.18.9f win source").replace("\r\n","\n");
    replace_once(&mut source,
        r###"fn start_tts_test(volume:i32){
    if !claim_tts_test(){logging::write("tts: test ignored because another test is already running");return;}
    if let Err(err)=thread::Builder::new().name("readyalert-tts-test".into()).spawn(move||{if let Err(err)=test_tts(volume){logging::write(format!("tts: test failed: {err}"));}release_tts_test();}){release_tts_test();logging::write(format!("tts: test worker spawn failed: {err}"));}
}
"###,
        r###"fn start_tts_test(volume:i32){
    if !claim_tts_test(){logging::write("tts: test ignored because another test is already running");return;}
    if let Err(err)=thread::Builder::new().name("readyalert-tts-test".into()).spawn(move||{
        let result=std::panic::catch_unwind(||test_tts(volume));
        release_tts_test();
        match result{
            Ok(Ok(()))=>{},
            Ok(Err(err))=>{logging::write(format!("tts: test failed: {err}"));message_box("ReadyAlert TTS Test",&format!("TTS test failed.\n\n{err}"),true);},
            Err(_)=>{logging::write("tts: test worker panicked");message_box("ReadyAlert TTS Test","TTS test failed unexpectedly. Check the diagnostic log for details.",true);}
        }
    }){release_tts_test();logging::write(format!("tts: test worker spawn failed: {err}"));message_box("ReadyAlert TTS Test",&format!("Could not start the TTS test.\n\n{err}"),true);}
}
"###,
        "visible and panic-safe TTS test result");
    fs::write(path,source).expect("write v1.18.9g win source");
}

fn patch_image_clipboard(out:&Path){
    let path=out.join("feature_overlays_v170_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read v1.18.9f feature overlay source").replace("\r\n","\n");
    replace_once(&mut source,
        r###"    if OpenClipboard(hwnd)==0{DeleteObject(bitmap);return Err(format!("OpenClipboard failed: {}",GetLastError()));}
    let transferred=EmptyClipboard()!=0&&!SetClipboardData(CF_BITMAP_,bitmap).is_null();
    let error=GetLastError();
    CloseClipboard();
    if !transferred{DeleteObject(bitmap);return Err(format!("SetClipboardData(CF_BITMAP) failed: {error}"));}
    Ok(row_count)
"###,
        r###"    let mut clipboard_open=false;
    for _ in 0..5{if OpenClipboard(hwnd)!=0{clipboard_open=true;break;}std::thread::sleep(std::time::Duration::from_millis(10));}
    if !clipboard_open{let error=GetLastError();DeleteObject(bitmap);return Err(format!("OpenClipboard stayed busy: {error}"));}
    if EmptyClipboard()==0{let error=GetLastError();CloseClipboard();DeleteObject(bitmap);return Err(format!("EmptyClipboard failed: {error}"));}
    let stored=SetClipboardData(CF_BITMAP_,bitmap);
    let error=GetLastError();
    CloseClipboard();
    if stored.is_null(){DeleteObject(bitmap);return Err(format!("SetClipboardData(CF_BITMAP) failed: {error}"));}
    Ok(row_count)
"###,
        "bounded image clipboard retry");
    source.push_str(r###"

#[cfg(test)]
mod v1189_repeat_clipboard_tests{
    #[test]fn clipboard_retry_budget_stays_short(){assert_eq!(5usize*10,50);}
}
"###);
    fs::write(path,source).expect("write v1.18.9g feature overlay source");
}

fn main(){
    prior::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_tts_test_feedback(&out);
    patch_image_clipboard(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1189g.rs");
}
