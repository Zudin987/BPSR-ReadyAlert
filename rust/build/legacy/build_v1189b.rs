use std::{env,fs,path::{Path,PathBuf}};

mod prior { include!("build_v1189.rs"); pub fn run(){main();} }

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){let count=source.matches(from).count();assert_eq!(count,1,"v1.18.9b patch {label} expected one match, found {count}");*source=source.replacen(from,to,1);}
fn insert_before_once(source:&mut String,anchor:&str,insertion:&str,label:&str){let count=source.matches(anchor).count();assert_eq!(count,1,"v1.18.9b patch {label} expected one anchor, found {count}");let at=source.find(anchor).expect("v1.18.9b insertion anchor");source.insert_str(at,insertion);}

fn patch_scale_persistence(out:&Path){
    let path=out.join("feature_overlays_v170_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read v1.18.9 generated feature overlays").replace("\r\n","\n");
    replace_once(&mut source,
        r###"fn load_scale_file(paths:&AppPaths)->OverlayScaleFile{let path=scale_file_path(paths);let mut value=std::fs::read_to_string(path).ok().and_then(|text|serde_json::from_str::<OverlayScaleFile>(&text).ok()).unwrap_or_default();normalize_scale_slot(&mut value.dps);normalize_scale_slot(&mut value.mechanics);value}
fn scale_slot(paths:&AppPaths,kind:Kind)->OverlayScaleSlot{let file=load_scale_file(paths);if kind==Kind::Dps{file.dps}else{file.mechanics}}
fn save_scale_slot(paths:&AppPaths,kind:Kind,mut slot:OverlayScaleSlot)->std::io::Result<()>{
    normalize_scale_slot(&mut slot);let mut file=load_scale_file(paths);if kind==Kind::Dps{file.dps=slot}else{file.mechanics=slot}
    let path=scale_file_path(paths);let temp=path.with_extension("json.new");let json=serde_json::to_string_pretty(&file).map_err(std::io::Error::other)?;std::fs::write(&temp,json)?;
    if path.exists(){std::fs::remove_file(&path)?;}std::fs::rename(temp,path)
}
"###,
        r###"fn normalize_scale_file(value:&mut OverlayScaleFile){normalize_scale_slot(&mut value.dps);normalize_scale_slot(&mut value.mechanics);}
fn read_scale_file(path:&std::path::Path)->Result<OverlayScaleFile,String>{let text=std::fs::read_to_string(path).map_err(|e|e.to_string())?;let mut value=serde_json::from_str::<OverlayScaleFile>(&text).map_err(|e|e.to_string())?;normalize_scale_file(&mut value);Ok(value)}
fn write_scale_file(path:&std::path::Path,file:&OverlayScaleFile)->std::io::Result<()>{
    let temp=path.with_extension("json.new");let backup=path.with_extension("json.bak");let json=serde_json::to_vec_pretty(file).map_err(std::io::Error::other)?;std::fs::write(&temp,&json)?;
    let _:OverlayScaleFile=serde_json::from_slice(&std::fs::read(&temp)?).map_err(std::io::Error::other)?;
    if path.exists(){if read_scale_file(path).is_ok(){std::fs::copy(path,&backup)?;}std::fs::remove_file(path)?;}std::fs::rename(&temp,path)
}
fn load_scale_file(paths:&AppPaths)->OverlayScaleFile{
    let path=scale_file_path(paths);let backup=path.with_extension("json.bak");let pending=path.with_extension("json.new");
    for(candidate,label)in[(&path,"primary"),(&backup,"backup"),(&pending,"pending")]{if !candidate.exists(){continue;}match read_scale_file(candidate){Ok(value)=>{if label!="primary"{crate::logging::write(format!("overlay scale: recovered from {label}"));if let Err(err)=write_scale_file(&path,&value){crate::logging::write(format!("overlay scale: recovery save failed: {err}"));}}return value;}Err(err)=>crate::logging::write(format!("overlay scale: load failed {}: {err}",candidate.display())),}}
    OverlayScaleFile::default()
}
fn scale_slot(paths:&AppPaths,kind:Kind)->OverlayScaleSlot{let file=load_scale_file(paths);if kind==Kind::Dps{file.dps}else{file.mechanics}}
fn save_scale_slot(paths:&AppPaths,kind:Kind,mut slot:OverlayScaleSlot)->std::io::Result<()>{normalize_scale_slot(&mut slot);let mut file=load_scale_file(paths);if kind==Kind::Dps{file.dps=slot}else{file.mechanics=slot}write_scale_file(&scale_file_path(paths),&file)}
"###,
        "crash-safe overlay scale persistence");
    source.push_str(r###"

#[cfg(test)]
mod v1189_repeat_scale_tests{
    use super::*;
    #[test]
    fn corrupt_scale_primary_recovers_backup_and_repairs_primary(){
        let root=std::env::temp_dir().join(format!("readyalert-scale-repeat-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));std::fs::create_dir_all(&root).unwrap();
        let paths=AppPaths{settings:root.join("settings.json"),log:root.join("readyalert.log"),chat_logs:root.join("ChatLogs"),root:root.clone()};
        let path=scale_file_path(&paths);let mut file=OverlayScaleFile::default();file.dps.percent=150;write_scale_file(&path,&file).unwrap();write_scale_file(&path,&file).unwrap();std::fs::write(&path,b"{ broken").unwrap();
        let recovered=load_scale_file(&paths);assert_eq!(recovered.dps.percent,150);assert!(read_scale_file(&path).is_ok());assert!(path.with_extension("json.bak").exists());std::fs::remove_dir_all(root).unwrap();
    }
}
"###);
    fs::write(path,source).expect("write v1.18.9b feature overlays");
}

fn patch_win_runtime(out:&Path){
    let path=out.join("win_v182_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read v1.18.9 generated win source").replace("\r\n","\n");
    replace_once(&mut source,
        r###"    state.chat_overlay=overlay::create(instance,state.settings.clone(),hwnd,state.paths.clone())?;
    state.dps_overlay=feature_overlays::create_dps(instance,hwnd,state.features.clone(),state.paths.clone())?;
    state.mechanics_overlay=feature_overlays::create_mechanics(instance,hwnd,state.features.clone(),state.paths.clone())?;
    add_tray(hwnd,&state.capture_status)?;apply_visibility(&mut state);SetTimer(hwnd,TIMER_ID,50,None);
    let mut msg:MSG=std::mem::zeroed();while GetMessageW(&mut msg,null_mut(),0,0)>0{if crate::ui::dialog_message(&msg){continue;}TranslateMessage(&msg);DispatchMessageW(&msg);}state.stop.store(true,Ordering::Relaxed);drop(state);Ok(())
"###,
        r###"    let setup=(||->Result<(),String>{
        state.chat_overlay=overlay::create(instance,state.settings.clone(),hwnd,state.paths.clone())?;
        state.dps_overlay=feature_overlays::create_dps(instance,hwnd,state.features.clone(),state.paths.clone())?;
        state.mechanics_overlay=feature_overlays::create_mechanics(instance,hwnd,state.features.clone(),state.paths.clone())?;
        add_tray(hwnd,&state.capture_status)?;apply_visibility(&mut state);
        if SetTimer(hwnd,TIMER_ID,50,None)==0{return Err(format!("SetTimer failed: {}",GetLastError()));}Ok(())
    })();
    if let Err(err)=setup{if IsWindow(hwnd)!=0{DestroyWindow(hwnd);}return Err(err);}
    let mut msg:MSG=std::mem::zeroed();loop{let status=GetMessageW(&mut msg,null_mut(),0,0);if status==0{break;}if status<0{let err=GetLastError();if IsWindow(hwnd)!=0{DestroyWindow(hwnd);}state.stop.store(true,Ordering::Relaxed);drop(state);return Err(format!("GetMessageW failed: {err}"));}if crate::ui::dialog_message(&msg){continue;}TranslateMessage(&msg);DispatchMessageW(&msg);}state.stop.store(true,Ordering::Relaxed);drop(state);Ok(())
"###,
        "UI setup and message-loop error handling");

    replace_once(&mut source,
        r###"    AppEvent::Alert(alert)=>{let snapshot=state.settings.read().map(|s|s.clone()).unwrap_or_default();let enabled=alert.kind==AlertKind::Error||alert_enabled(&snapshot,alert.kind);if enabled{if alert.kind!=AlertKind::Error{let volume=snapshot.alert_volume;thread::spawn(move||audio::play_alert(alert.kind,volume));}if snapshot.desktop_notification||alert.kind==AlertKind::Error{balloon(hwnd,&alert.title,&alert.message,alert.kind==AlertKind::Error);}}}
"###,
        r###"    AppEvent::Alert(alert)=>{let snapshot=state.settings.read().map(|s|s.clone()).unwrap_or_default();let enabled=alert.kind==AlertKind::Error||alert_enabled(&snapshot,alert.kind);if enabled{if alert.kind!=AlertKind::Error&&snapshot.alert_volume>0{queue_audio(AudioJob::Alert(alert.kind,snapshot.alert_volume));}if snapshot.desktop_notification||alert.kind==AlertKind::Error{balloon(hwnd,&alert.title,&alert.message,alert.kind==AlertKind::Error);}}}
"###,
        "bounded alert audio queue");

    replace_once(&mut source,
        r###"fn maybe_chat_sound(settings:&AppSettings,message:&ChatMessage){let path=if message.channel==5&&settings.chat.private_sound_enabled{Some(settings.chat.private_sound_path.clone())}else{settings.chat.highlight_sound_rules.iter().find(|r|r.enabled&&!r.match_text.trim().is_empty()&&chat::matches_expression(&message.text,&r.match_text)).map(|r|r.sound_path.clone())};if let Some(path)=path.filter(|p|!p.trim().is_empty()){let volume=settings.chat.chat_sound_volume;thread::spawn(move||{let _=audio::play_file(Path::new(&path),volume);});}}
"###,
        r###"fn maybe_chat_sound(settings:&AppSettings,message:&ChatMessage){let volume=settings.chat.chat_sound_volume.clamp(0,100);if volume<=0{return;}let path=if message.channel==5&&settings.chat.private_sound_enabled{Some(settings.chat.private_sound_path.clone())}else{settings.chat.highlight_sound_rules.iter().find(|r|r.enabled&&!r.match_text.trim().is_empty()&&chat::matches_expression(&message.text,&r.match_text)).map(|r|r.sound_path.clone())};if let Some(path)=path.filter(|p|!p.trim().is_empty()){queue_audio(AudioJob::File(path,volume));}}
"###,
        "bounded chat audio queue");

    insert_before_once(&mut source,
        "fn alert_enabled(settings:&AppSettings,kind:AlertKind)->bool",
        r###"const AUDIO_QUEUE_CAPACITY:usize=64;
enum AudioJob{Alert(AlertKind,i32),File(String,i32)}
fn audio_sender()->Option<&'static std::sync::mpsc::SyncSender<AudioJob>>{
    static AUDIO:std::sync::OnceLock<Option<std::sync::mpsc::SyncSender<AudioJob>>>=std::sync::OnceLock::new();
    AUDIO.get_or_init(||{let(tx,rx)=std::sync::mpsc::sync_channel(AUDIO_QUEUE_CAPACITY);match thread::Builder::new().name("readyalert-audio".into()).spawn(move||{while let Ok(job)=rx.recv(){match job{AudioJob::Alert(kind,volume)=>audio::play_alert(kind,volume),AudioJob::File(path,volume)=>if let Err(err)=audio::play_file(Path::new(&path),volume){logging::write(format!("audio: chat playback failed: {err}"));}}}}){Ok(_)=>Some(tx),Err(err)=>{logging::write(format!("audio: worker spawn failed: {err}"));None}}}).as_ref()
}
fn queue_audio(job:AudioJob){
    static QUEUE_WARNED:AtomicBool=AtomicBool::new(false);let Some(tx)=audio_sender()else{return;};match tx.try_send(job){Ok(())=>{QUEUE_WARNED.store(false,Ordering::Relaxed);},Err(std::sync::mpsc::TrySendError::Full(_))=>{if !QUEUE_WARNED.swap(true,Ordering::Relaxed){logging::write("audio: queue full; dropping burst sound");}},Err(std::sync::mpsc::TrySendError::Disconnected(_))=>{if !QUEUE_WARNED.swap(true,Ordering::Relaxed){logging::write("audio: worker disconnected");}}}
}

"###,
        "audio worker");

    source.push_str(r###"

#[cfg(test)]
mod v1189_repeat_win_tests{
    use super::*;
    #[test]fn audio_burst_queue_is_bounded(){assert_eq!(AUDIO_QUEUE_CAPACITY,64);}
}
"###);
    fs::write(path,source).expect("write v1.18.9b win source");
}

fn patch_tracker_ui(out:&Path){
    let path=out.join("event_tracker_ui_v1160_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read v1.18.9 event tracker UI").replace("\r\n","\n");
    replace_once(&mut source,"const ES_AUTOHSCROLL: u32 = 0x0080;\n","const ES_AUTOHSCROLL: u32 = 0x0080;\nconst ES_NUMBER: u32 = 0x2000;\n","tracker numeric edit style");
    replace_once(&mut source,
        r###"unsafe fn positive_number(hwnd: HWND, id: i32, max: i32, message: &str) -> Option<i32> {
    if let Ok(n)=get_text(hwnd,id).trim().parse::<i32>() { if (1..=max).contains(&n) {return Some(n);} }
    message_error(hwnd,message);crate::ui::SetFocus(GetDlgItem(hwnd,id));None
}
"###,
        r###"unsafe fn positive_number(hwnd: HWND, id: i32, max: i32, message: &str) -> Option<i32> {
    if let Ok(n)=get_text(hwnd,id).trim().parse::<i32>() { if (1..=max).contains(&n) {return Some(n);} }
    message_error(hwnd,message);let edit=GetDlgItem(hwnd,id);crate::ui::SetFocus(edit);SendMessageW(edit,0x00B1,0,-1isize);None
}
"###,
        "tracker invalid number focus/select");
    replace_once(&mut source,
        r###"unsafe fn create_edit(hwnd: HWND, id: i32, x: i32, y: i32, w: i32, h: i32) -> HWND {
    create_control(hwnd, "EDIT", "", id, x, y, w, h, WS_CHILD | WS_VISIBLE | WS_TABSTOP | ES_AUTOHSCROLL, 0)
}
"###,
        r###"unsafe fn create_edit(hwnd: HWND, id: i32, x: i32, y: i32, w: i32, h: i32) -> HWND {
    let numeric=matches!(id,ID_MAX_VISIBLE|ID_EVENT_ID|ID_HOLD);create_control(hwnd, "EDIT", "", id, x, y, w, h, WS_CHILD | WS_VISIBLE | WS_TABSTOP | ES_AUTOHSCROLL | if numeric{ES_NUMBER}else{0}, 0)
}
"###,
        "tracker numeric controls");
    source.push_str(r###"

#[cfg(test)]
mod v1189_repeat_tracker_ui_tests{
    use super::*;
    #[test]fn number_edit_style_is_native_numeric(){assert_eq!(ES_NUMBER,0x2000);}
}
"###);
    fs::write(path,source).expect("write v1.18.9b tracker UI");
}

fn main(){prior::run();let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));patch_scale_persistence(&out);patch_win_runtime(&out);patch_tracker_ui(&out);println!("cargo:rerun-if-changed=build/legacy/build_v1189b.rs");}
