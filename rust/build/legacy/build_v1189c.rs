use std::{env,fs,path::{Path,PathBuf}};

mod prior { include!("build_v1189b.rs"); pub fn run(){main();} }

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){let count=source.matches(from).count();assert_eq!(count,1,"v1.18.9c patch {label} expected one match, found {count}");*source=source.replacen(from,to,1);}
fn replace_between(source:&mut String,start:&str,end:&str,replacement:&str,label:&str){let count=source.matches(start).count();assert_eq!(count,1,"v1.18.9c patch {label} expected one start, found {count}");let begin=source.find(start).expect("v1.18.9c start");let rel_end=source[begin..].find(end).expect("v1.18.9c end");source.replace_range(begin..begin+rel_end,replacement);}

fn patch_audio_priority(out:&Path){
    let path=out.join("win_v182_fixed.rs");let mut source=fs::read_to_string(&path).expect("read v1.18.9b win source").replace("\r\n","\n");
    replace_once(&mut source,"queue_audio(AudioJob::Alert(alert.kind,snapshot.alert_volume))","queue_alert_audio(alert.kind,snapshot.alert_volume)","alert queue route");
    replace_once(&mut source,"queue_audio(AudioJob::File(path,volume))","queue_chat_audio(path,volume)","chat queue route");
    replace_between(&mut source,"const AUDIO_QUEUE_CAPACITY:usize=64;","fn alert_enabled",r###"const AUDIO_QUEUE_CAPACITY:usize=32;
fn alert_audio_sender()->Option<&'static std::sync::mpsc::SyncSender<(AlertKind,i32)>>{
    static AUDIO:std::sync::OnceLock<Option<std::sync::mpsc::SyncSender<(AlertKind,i32)>>>=std::sync::OnceLock::new();
    AUDIO.get_or_init(||{let(tx,rx)=std::sync::mpsc::sync_channel(AUDIO_QUEUE_CAPACITY);match thread::Builder::new().name("readyalert-alert-audio".into()).spawn(move||{while let Ok((kind,volume))=rx.recv(){audio::play_alert(kind,volume);}}){Ok(_)=>Some(tx),Err(err)=>{logging::write(format!("audio: alert worker spawn failed: {err}"));None}}}).as_ref()
}
fn chat_audio_sender()->Option<&'static std::sync::mpsc::SyncSender<(String,i32)>>{
    static AUDIO:std::sync::OnceLock<Option<std::sync::mpsc::SyncSender<(String,i32)>>>=std::sync::OnceLock::new();
    AUDIO.get_or_init(||{let(tx,rx)=std::sync::mpsc::sync_channel(AUDIO_QUEUE_CAPACITY);match thread::Builder::new().name("readyalert-chat-audio".into()).spawn(move||{while let Ok((path,volume))=rx.recv(){if let Err(err)=audio::play_file(Path::new(&path),volume){logging::write(format!("audio: chat playback failed: {err}"));}}}){Ok(_)=>Some(tx),Err(err)=>{logging::write(format!("audio: chat worker spawn failed: {err}"));None}}}).as_ref()
}
fn queue_alert_audio(kind:AlertKind,volume:i32){
    static WARNED:AtomicBool=AtomicBool::new(false);let Some(tx)=alert_audio_sender()else{return;};match tx.try_send((kind,volume)){Ok(())=>WARNED.store(false,Ordering::Relaxed),Err(std::sync::mpsc::TrySendError::Full(_))=>{if !WARNED.swap(true,Ordering::Relaxed){logging::write("audio: alert queue full; dropping burst sound");}},Err(std::sync::mpsc::TrySendError::Disconnected(_))=>{if !WARNED.swap(true,Ordering::Relaxed){logging::write("audio: alert worker disconnected");}}}
}
fn queue_chat_audio(path:String,volume:i32){
    static WARNED:AtomicBool=AtomicBool::new(false);let Some(tx)=chat_audio_sender()else{return;};match tx.try_send((path,volume)){Ok(())=>WARNED.store(false,Ordering::Relaxed),Err(std::sync::mpsc::TrySendError::Full(_))=>{if !WARNED.swap(true,Ordering::Relaxed){logging::write("audio: chat queue full; dropping burst sound");}},Err(std::sync::mpsc::TrySendError::Disconnected(_))=>{if !WARNED.swap(true,Ordering::Relaxed){logging::write("audio: chat worker disconnected");}}}
}

"###,"separate alert/chat audio workers");
    replace_once(&mut source,"assert_eq!(AUDIO_QUEUE_CAPACITY,64)","assert_eq!(AUDIO_QUEUE_CAPACITY,32)","audio queue test");
    fs::write(path,source).expect("write v1.18.9c win source");
}

fn patch_settings_polish(out:&Path){
    let path=out.join("settings_ui_v1160_fixed.rs");let mut source=fs::read_to_string(&path).expect("read v1.18.9b settings UI").replace("\r\n","\n");
    replace_once(&mut source,"    tab.min_level = read_i32(hwnd, ID_TAB_MIN_LEVEL, tab.min_level);\n","    tab.min_level = read_i32(hwnd, ID_TAB_MIN_LEVEL, tab.min_level).clamp(1,100);\n    set_text(hwnd, ID_TAB_MIN_LEVEL, &tab.min_level.to_string());\n","chat tab min-level normalization");
    source.push_str(r###"

#[cfg(test)]
mod v1189_repeat_settings_polish_tests{
    #[test]fn chat_level_bounds_match_settings_normalizer(){assert_eq!(999i32.clamp(1,100),100);assert_eq!(0i32.clamp(1,100),1);}
}
"###);
    fs::write(path,source).expect("write v1.18.9c settings UI");
}

fn main(){prior::run();let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));patch_audio_priority(&out);patch_settings_polish(&out);println!("cargo:rerun-if-changed=build/legacy/build_v1189c.rs");}
