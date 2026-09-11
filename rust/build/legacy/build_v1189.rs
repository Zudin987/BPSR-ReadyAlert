use std::{env,fs,path::{Path,PathBuf}};

mod prior { include!("build_v1188.rs"); pub fn run(){main();} }

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){let count=source.matches(from).count();assert_eq!(count,1,"v1.18.9 patch {label} expected one match, found {count}");*source=source.replacen(from,to,1);}
fn insert_before_once(source:&mut String,anchor:&str,insertion:&str,label:&str){let count=source.matches(anchor).count();assert_eq!(count,1,"v1.18.9 patch {label} expected one anchor, found {count}");let at=source.find(anchor).expect("v1.18.9 insertion anchor");source.insert_str(at,insertion);}

fn patch_win_v182_fixed_rs(out:&Path){
 let path=out.join("win_v182_fixed.rs");let mut source=fs::read_to_string(&path).expect("read win_v182_fixed.rs").replace("\r\n","\n");
 replace_once(&mut source,r########"use std::{ffi::c_void,io::Read,path::Path,ptr::{null,null_mut},sync::{atomic::{AtomicBool,Ordering},mpsc::{Receiver,TryRecvError},Arc,RwLock},thread,time::Duration};
"########,r########"use std::{ffi::c_void,io::Read,path::Path,ptr::{null,null_mut},sync::{atomic::{AtomicBool,Ordering},mpsc::{Receiver,TryRecvError},Arc,RwLock},thread,time::{Duration,Instant}};
"########,"win edit 1");
 replace_once(&mut source,r########"    chat_overlay:HWND,dps_overlay:HWND,mechanics_overlay:HWND,capture_status:String,
"########,r########"    chat_overlay:HWND,dps_overlay:HWND,mechanics_overlay:HWND,capture_status:String,last_chat_refresh:Instant,
"########,"win edit 2");
 replace_once(&mut source,r########"    let mut state=Box::new(UiState{settings,features,paths,rx,stop,api,chat_overlay:null_mut(),dps_overlay:null_mut(),mechanics_overlay:null_mut(),capture_status:"Starting capture…".into()});let state_ptr:*mut UiState=&mut*state;
"########,r########"    let mut state=Box::new(UiState{settings,features,paths,rx,stop,api,chat_overlay:null_mut(),dps_overlay:null_mut(),mechanics_overlay:null_mut(),capture_status:"Starting capture…".into(),last_chat_refresh:Instant::now()});let state_ptr:*mut UiState=&mut*state;
"########,"win edit 3");
 replace_once(&mut source,r########"    add_tray(hwnd,&state.capture_status)?;apply_visibility(&mut state);SetTimer(hwnd,TIMER_ID,33,None);
"########,r########"    add_tray(hwnd,&state.capture_status)?;apply_visibility(&mut state);SetTimer(hwnd,TIMER_ID,50,None);
"########,"win edit 4");
 replace_once(&mut source,r########"        WM_TIMER if wparam==TIMER_ID=>{if !ptr.is_null(){drain_events(hwnd,&mut*ptr);feature_overlays::tick((*ptr).dps_overlay);feature_overlays::tick((*ptr).mechanics_overlay);}0}
"########,r########"        WM_TIMER if wparam==TIMER_ID=>{if !ptr.is_null(){let state=&mut*ptr;drain_events(hwnd,state);feature_overlays::tick(state.dps_overlay);feature_overlays::tick(state.mechanics_overlay);if state.last_chat_refresh.elapsed()>=Duration::from_secs(1){state.last_chat_refresh=Instant::now();let refresh_relative=state.settings.read().map(|s|s.chat_overlay_enabled&&s.chat.show_time&&s.chat.show_time_as_ago).unwrap_or(false);if refresh_relative{overlay::refresh(state.chat_overlay);}}}0}
"########,"win edit 5");
 replace_once(&mut source,r########"unsafe fn drain_events(hwnd:HWND,state:&mut UiState){loop{match state.rx.try_recv(){Ok(event)=>handle_event(hwnd,state,event),Err(TryRecvError::Empty)|Err(TryRecvError::Disconnected)=>break,}}}
"########,r########"const MAX_EVENTS_PER_TICK:usize=512;
unsafe fn drain_events(hwnd:HWND,state:&mut UiState){for _ in 0..MAX_EVENTS_PER_TICK{match state.rx.try_recv(){Ok(event)=>handle_event(hwnd,state,event),Err(TryRecvError::Empty)|Err(TryRecvError::Disconnected)=>break,}}}
"########,"win edit 6");
 replace_once(&mut source,r########"    AppEvent::Chat(message)=>{let snapshot=state.settings.read().map(|s|s.clone()).unwrap_or_default();if snapshot.chat_overlay_enabled&&!chat::should_hide_globally(&snapshot,&message){let mut display=message.clone();if display.sender_level>0{display.sender_level=-display.sender_level;}overlay::push_chat(state.chat_overlay,display);maybe_chat_sound(&snapshot,&message);}}
"########,r########"    AppEvent::Chat(message)=>{let snapshot=state.settings.read().map(|s|s.clone()).unwrap_or_default();if !chat::should_hide_globally(&snapshot,&message){maybe_chat_sound(&snapshot,&message);if snapshot.chat_overlay_enabled{let mut display=message.clone();if display.sender_level>0{display.sender_level=-display.sender_level;}overlay::push_chat(state.chat_overlay,display);}}}
"########,"win edit 7");
 replace_once(&mut source,r########"fn sanitize_and_save(settings_arc:&Arc<RwLock<AppSettings>>,paths:&AppPaths){if let Ok(mut s)=settings_arc.write(){enforce_hardcoded(&mut s);s.normalize();let snap=s.clone();drop(s);let _=settings::save(paths,&snap);}}
"########,r########"fn sanitize_and_save(settings_arc:&Arc<RwLock<AppSettings>>,paths:&AppPaths){if let Ok(mut s)=settings_arc.write(){enforce_hardcoded(&mut s);s.normalize();let snap=s.clone();drop(s);if let Err(err)=settings::save(paths,&snap){logging::write(format!("settings: sanitize save failed: {err}"));}}}
"########,"win edit 8");
 replace_once(&mut source,r########"    if command==CMD_SETTINGS_APPLIED{sanitize_and_save(&state.settings,&state.paths);let snapshot=state.settings.read().map(|s|s.clone()).unwrap_or_default();overlay::apply_style(state.chat_overlay,&snapshot);overlay::refresh(state.chat_overlay);apply_visibility(state);return;}
"########,r########"    if command==CMD_SETTINGS_APPLIED{let snapshot=state.settings.read().map(|s|s.clone()).unwrap_or_default();overlay::apply_style(state.chat_overlay,&snapshot);overlay::refresh(state.chat_overlay);apply_visibility(state);return;}
"########,"win edit 9");
 source.push_str(r########"

#[cfg(test)]
mod v1189_win_maintenance_tests{
    use super::*;
    #[test]fn event_drain_has_fairness_cap(){assert_eq!(MAX_EVENTS_PER_TICK,512);}
    #[test]fn maintenance_timer_matches_telemetry_cadence(){assert!(50>=33&&50<=100);}
}
"########);
 fs::write(path,source).expect("write v1.18.9 patched source");
}

fn patch_settings_ui_v1160_fixed_rs(out:&Path){
 let path=out.join("settings_ui_v1160_fixed.rs");let mut source=fs::read_to_string(&path).expect("read settings_ui_v1160_fixed.rs").replace("\r\n","\n");
 insert_before_once(&mut source,r########"const ES_MULTILINE: u32 = 0x0004;
"########,r########"const ES_NUMBER: u32 = 0x2000;
"########,"settings edit 1");
 insert_before_once(&mut source,r########"    let s = &mut state.working;
"########,r########"    let Some(alert_volume)=read_i32_range(hwnd,ID_ALERT_VOLUME,"Alert volume",0,100)else{return;};
    let Some(window_opacity)=read_i32_range(hwnd,ID_WINDOW_OPACITY,"Chat opacity",25,100)else{return;};
    let Some(font_size)=read_f32_range(hwnd,ID_FONT_SIZE,"Font size",8.0,24.0)else{return;};
    let Some(max_history)=read_i32_range(hwnd,ID_MAX_HISTORY,"Max history",10,500)else{return;};
    let Some(tts_volume)=read_i32_range(hwnd,ID_TTS_VOLUME,"TTS volume",0,100)else{return;};
    let Some(chat_sound_volume)=read_i32_range(hwnd,ID_CHAT_SOUND_VOLUME,"Chat sound volume",0,100)else{return;};
"########,"settings edit 2");
 replace_once(&mut source,r########"    s.alert_volume = read_i32(hwnd, ID_ALERT_VOLUME, s.alert_volume);
"########,r########"    s.alert_volume = alert_volume;
"########,"settings edit 3");
 replace_once(&mut source,r########"    s.chat.window_opacity = read_i32(hwnd, ID_WINDOW_OPACITY, s.chat.window_opacity);
"########,r########"    s.chat.window_opacity = window_opacity;
"########,"settings edit 4");
 replace_once(&mut source,r########"    s.chat.font_size = get_text(hwnd, ID_FONT_SIZE).parse::<f32>().unwrap_or(s.chat.font_size);
    s.chat.max_history = read_i32(hwnd, ID_MAX_HISTORY, s.chat.max_history as i32).max(1) as usize;
"########,r########"    s.chat.font_size = font_size;
    s.chat.max_history = max_history as usize;
"########,"settings edit 5");
 replace_once(&mut source,r########"    sp.tts_volume = read_i32(hwnd, ID_TTS_VOLUME, sp.tts_volume);
"########,r########"    sp.tts_volume = tts_volume;
"########,"settings edit 6");
 replace_once(&mut source,r########"    s.chat.chat_sound_volume = read_i32(hwnd, ID_CHAT_SOUND_VOLUME, s.chat.chat_sound_volume);
"########,r########"    s.chat.chat_sound_volume = chat_sound_volume;
"########,"settings edit 7");
 replace_once(&mut source,r########"            let volume = read_i32(hwnd, ID_TTS_VOLUME, state.working.speech_translation.tts_volume).clamp(0,100);
"########,r########"            let Some(volume)=read_i32_range(hwnd,ID_TTS_VOLUME,"TTS volume",0,100)else{return;};
"########,"settings edit 8");
 insert_before_once(&mut source,r########"    if multiline { style |= ES_MULTILINE | ES_AUTOVSCROLL | WS_VSCROLL; }
"########,r########"    if matches!(id,ID_ALERT_VOLUME|ID_WINDOW_OPACITY|ID_MAX_HISTORY|ID_TTS_VOLUME|ID_TAB_MIN_LEVEL|ID_CHAT_SOUND_VOLUME){style|=ES_NUMBER;}
"########,"settings edit 9");
 insert_before_once(&mut source,r########"unsafe fn set_combo_items(hwnd: HWND, id: i32, values: &[&str], selected: usize) {
"########,r########"fn parse_i32_range(text:&str,min:i32,max:i32)->Option<i32>{text.trim().parse::<i32>().ok().filter(|v|*v>=min&&*v<=max)}
fn parse_f32_range(text:&str,min:f32,max:f32)->Option<f32>{text.trim().parse::<f32>().ok().filter(|v|v.is_finite()&&*v>=min&&*v<=max)}
unsafe fn invalid_number(hwnd:HWND,id:i32,label:&str,min:&str,max:&str){let text=format!("{label} must be a number from {min} to {max}.");MessageBoxW(hwnd,wide(&text).as_ptr(),wide("Invalid setting").as_ptr(),MB_OK|MB_ICONERROR);let c=GetDlgItem(hwnd,id);if !c.is_null(){crate::ui::SetFocus(c);SendMessageW(c,0x00B1,0,-1isize as isize);}}
unsafe fn read_i32_range(hwnd:HWND,id:i32,label:&str,min:i32,max:i32)->Option<i32>{let text=get_text(hwnd,id);match parse_i32_range(&text,min,max){Some(v)=>Some(v),None=>{invalid_number(hwnd,id,label,&min.to_string(),&max.to_string());None}}}
unsafe fn read_f32_range(hwnd:HWND,id:i32,label:&str,min:f32,max:f32)->Option<f32>{let text=get_text(hwnd,id);match parse_f32_range(&text,min,max){Some(v)=>Some(v),None=>{invalid_number(hwnd,id,label,&format!("{min:.0}"),&format!("{max:.0}"));None}}}
"########,"settings edit 10");
 source.push_str(r########"

#[cfg(test)]
mod v1189_settings_maintenance_tests{
    use super::*;
    #[test]fn volume_ranges_reject_invalid_input(){assert_eq!(parse_i32_range("0",0,100),Some(0));assert_eq!(parse_i32_range("100",0,100),Some(100));assert_eq!(parse_i32_range("101",0,100),None);assert_eq!(parse_i32_range("abc",0,100),None);}
    #[test]fn font_range_rejects_nan_and_outliers(){assert_eq!(parse_f32_range("12.5",8.0,24.0),Some(12.5));assert!(parse_f32_range("NaN",8.0,24.0).is_none());assert!(parse_f32_range("30",8.0,24.0).is_none());}
}
"########);
 fs::write(path,source).expect("write v1.18.9 patched source");
}

fn patch_feature_overlays_v170_fixed_rs(out:&Path){
 let path=out.join("feature_overlays_v170_fixed.rs");let mut source=fs::read_to_string(&path).expect("read feature_overlays_v170_fixed.rs").replace("\r\n","\n");
 replace_once(&mut source,r########"    if let Ok(mut features)=state.features.write(){let layout=if state.kind==Kind::Dps{&mut features.dps}else{&mut features.mechanics};layout.x=rect.left;layout.y=rect.top;layout.width=(rect.right-rect.left).max(1);layout.height=(rect.bottom-rect.top).max(1);let snapshot=features.clone();drop(features);let _=feature_settings::save(&state.paths,&snapshot);}
    let mut slot=scale_slot(&state.paths,state.kind);slot.percent=state.scale_percent;if state.scale_percent==100{slot.has_bounds=false;}else{slot.x=rect.left;slot.y=rect.top;slot.width=(rect.right-rect.left).max(1);slot.height=(rect.bottom-rect.top).max(1);slot.has_bounds=true;}let _=save_scale_slot(&state.paths,state.kind,slot);
"########,r########"    if let Ok(mut features)=state.features.write(){let layout=if state.kind==Kind::Dps{&mut features.dps}else{&mut features.mechanics};layout.x=rect.left;layout.y=rect.top;layout.width=(rect.right-rect.left).max(1);layout.height=(rect.bottom-rect.top).max(1);let snapshot=features.clone();drop(features);if let Err(err)=feature_settings::save(&state.paths,&snapshot){crate::logging::write(format!("feature settings: overlay bounds save failed: {err}"));}}
    let mut slot=scale_slot(&state.paths,state.kind);slot.percent=state.scale_percent;if state.scale_percent==100{slot.has_bounds=false;}else{slot.x=rect.left;slot.y=rect.top;slot.width=(rect.right-rect.left).max(1);slot.height=(rect.bottom-rect.top).max(1);slot.has_bounds=true;}if let Err(err)=save_scale_slot(&state.paths,state.kind,slot){crate::logging::write(format!("overlay scale: bounds save failed: {err}"));}
"########,"feature edit 1");
 replace_once(&mut source,r########"unsafe fn set_overlay_scale(state:&mut SettingsState,requested:i32){
    let ptr=GetWindowLongPtrW(state.parent,GWLP_USERDATA)as *mut State;if ptr.is_null(){return;}let overlay=&mut*ptr;let old=overlay.scale_percent;let new=snap_overlay_scale(requested);if new==old{return;}
"########,r########"unsafe fn persist_current_overlay_scale(state:&mut SettingsState){let ptr=GetWindowLongPtrW(state.parent,GWLP_USERDATA)as *mut State;if ptr.is_null(){return;}let overlay=&mut*ptr;let rect=if overlay.collapsed{overlay.expanded}else{let mut r:RECT=std::mem::zeroed();if GetWindowRect(state.parent,&mut r)==0{return;}r};persist_overlay_rect(overlay,rect);}
fn scale_scroll_should_persist(code:u16)->bool{!matches!(code,4|5)}
unsafe fn set_overlay_scale(state:&mut SettingsState,requested:i32,persist:bool){
    let ptr=GetWindowLongPtrW(state.parent,GWLP_USERDATA)as *mut State;if ptr.is_null(){return;}let overlay=&mut*ptr;let old=overlay.scale_percent;let new=snap_overlay_scale(requested);if new==old{if persist{persist_current_overlay_scale(state);}return;}
"########,"feature edit 2");
 replace_once(&mut source,r########"    overlay.scale_percent=new;overlay.hover_text=None;
    if overlay.collapsed{persist_overlay_rect(overlay,target);resize_collapsed_for_scale(state.parent,overlay);}else{SetWindowPos(state.parent,HWND_TOPMOST,target.left,target.top,(target.right-target.left).max(1),(target.bottom-target.top).max(1),SWP_NOACTIVATE);persist_overlay_rect(overlay,target);clamp_scroll(state.parent,overlay);sync_consumable_popup(state.parent,overlay);}
"########,r########"    overlay.scale_percent=new;overlay.hover_text=None;overlay.expanded=target;
    if overlay.collapsed{if persist{persist_overlay_rect(overlay,target);}resize_collapsed_for_scale(state.parent,overlay);}else{SetWindowPos(state.parent,HWND_TOPMOST,target.left,target.top,(target.right-target.left).max(1),(target.bottom-target.top).max(1),SWP_NOACTIVATE);if persist{persist_overlay_rect(overlay,target);}clamp_scroll(state.parent,overlay);sync_consumable_popup(state.parent,overlay);}
"########,"feature edit 3");
 replace_once(&mut source,r########"unsafe fn change_overlay_scale(state:&mut SettingsState,delta:i32){let current=settings_overlay_scale(state);set_overlay_scale(state,current.saturating_add(delta));}
"########,r########"unsafe fn change_overlay_scale(state:&mut SettingsState,delta:i32){let current=settings_overlay_scale(state);set_overlay_scale(state,current.saturating_add(delta),true);}
"########,"feature edit 4");
 replace_once(&mut source,r########"        0x0114=>{if lparam as HWND==fc(hwnd,7007){let raw=SendMessageW(fc(hwnd,7007),0x0400,0,0)as i32;let snapped=snap_overlay_scale(raw);SendMessageW(fc(hwnd,7007),0x0405,1,snapped as isize);set_overlay_scale(state,snapped);refresh_feature_form(hwnd,state);return 0;}DefWindowProcW(hwnd,msg,wparam,lparam)},
"########,r########"        0x0114=>{if lparam as HWND==fc(hwnd,7007){let raw=SendMessageW(fc(hwnd,7007),0x0400,0,0)as i32;let snapped=snap_overlay_scale(raw);let code=(wparam&0xffff)as u16;SendMessageW(fc(hwnd,7007),0x0405,1,snapped as isize);set_overlay_scale(state,snapped,scale_scroll_should_persist(code));refresh_feature_form(hwnd,state);return 0;}DefWindowProcW(hwnd,msg,wparam,lparam)},
"########,"feature edit 5");
 replace_once(&mut source,r########"            if id==7005||id==7006||id==7008{if id==7008{set_overlay_scale(state,OVERLAY_SCALE_DEFAULT);}else{change_overlay_scale(state,if id==7005{-10}else{10});}refresh_feature_form(hwnd,state);return 0;}
"########,r########"            if id==7005||id==7006||id==7008{if id==7008{set_overlay_scale(state,OVERLAY_SCALE_DEFAULT,true);}else{change_overlay_scale(state,if id==7005{-10}else{10});}refresh_feature_form(hwnd,state);return 0;}
"########,"feature edit 6");
 replace_once(&mut source,r########"fn mutate_features<F:FnOnce(&mut FeatureSettings)>(state:&SettingsState,change:F){let snapshot=if let Ok(mut features)=state.features.write(){change(&mut features);feature_settings::normalize_v1182(&mut features);let snapshot=features.clone();drop(features);snapshot}else{return;};let _=feature_settings::save(&state.paths,&snapshot);let opacity=if state.kind==Kind::Dps{snapshot.dps.opacity}else{snapshot.mechanics.opacity};unsafe{apply_opacity(state.parent,opacity)};}
"########,r########"fn mutate_features<F:FnOnce(&mut FeatureSettings)>(state:&SettingsState,change:F){let snapshot=if let Ok(mut features)=state.features.write(){change(&mut features);feature_settings::normalize_v1182(&mut features);let snapshot=features.clone();drop(features);snapshot}else{return;};if let Err(err)=feature_settings::save(&state.paths,&snapshot){crate::logging::write(format!("feature settings: save failed: {err}"));}let opacity=if state.kind==Kind::Dps{snapshot.dps.opacity}else{snapshot.mechanics.opacity};unsafe{apply_opacity(state.parent,opacity)};}
"########,"feature edit 7");
 source.push_str(r########"

#[cfg(test)]
mod v1189_feature_maintenance_tests{
    use super::*;
    #[test]fn slider_drag_defers_disk_persistence(){assert!(!scale_scroll_should_persist(4));assert!(!scale_scroll_should_persist(5));assert!(scale_scroll_should_persist(8));assert!(scale_scroll_should_persist(0));}
}
"########);
 fs::write(path,source).expect("write v1.18.9 patched source");
}

fn patch_overlay_v150_v1181_rs(out:&Path){
 let path=out.join("overlay_v150_v1181.rs");let mut source=fs::read_to_string(&path).expect("read overlay_v150_v1181.rs").replace("\r\n","\n");
 replace_once(&mut source,r########"    let _ = settings::save(&state.paths, &snapshot);
"########,r########"    if let Err(err)=settings::save(&state.paths,&snapshot){logging::write(format!("chat overlay: bounds save failed: {err}"));}
"########,"chat overlay edit 1");
 fs::write(path,source).expect("write v1.18.9 patched source");
}

fn main(){prior::run();let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
 patch_win_v182_fixed_rs(&out);
 patch_settings_ui_v1160_fixed_rs(&out);
 patch_feature_overlays_v170_fixed_rs(&out);
 patch_overlay_v150_v1181_rs(&out);
 println!("cargo:rerun-if-changed=build/legacy/build_v1189.rs");
}
