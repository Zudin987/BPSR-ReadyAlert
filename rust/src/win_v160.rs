use crate::{
    audio, chat, feature_overlays, feature_settings::{self,FeatureSettings}, logging, overlay,
    settings_ui, tray::{self,TrayAction},
    model::{AlertKind,AppEvent,ChatMessage},
    npcap::{CaptureHandle,PcapApi}, paths::AppPaths, settings::{self,AppSettings},
};
use std::{ffi::c_void,io::Read,path::Path,ptr::{null,null_mut},sync::{atomic::{AtomicBool,Ordering},mpsc::{Receiver,TryRecvError},Arc,RwLock},thread,time::Duration};
use windows_sys::Win32::{
    Foundation::{CloseHandle,GetLastError,ERROR_ALREADY_EXISTS,HANDLE,HINSTANCE,HWND,LPARAM,LRESULT,WPARAM},
    System::{LibraryLoader::GetModuleHandleW,Threading::{CreateMutexW,ReleaseMutex}},
    UI::{Shell::{ShellExecuteW,Shell_NotifyIconW,NIF_ICON,NIF_INFO,NIF_MESSAGE,NIF_TIP,NIIF_ERROR,NIIF_INFO,NIM_ADD,NIM_DELETE,NIM_MODIFY,NOTIFYICONDATAW},WindowsAndMessaging::{CreateWindowExW,DefWindowProcW,DestroyWindow,DispatchMessageW,GetMessageW,GetWindowLongPtrW,IsWindow,LoadCursorW,LoadIconW,MessageBoxW,PostQuitMessage,RegisterClassW,SetForegroundWindow,SetTimer,SetWindowLongPtrW,ShowWindow,TranslateMessage,CREATESTRUCTW,GWLP_USERDATA,IDC_ARROW,IDI_APPLICATION,MB_ICONERROR,MB_ICONINFORMATION,MB_OK,MSG,SW_HIDE,SW_SHOWNORMAL,SW_SHOW,WM_APP,WM_COMMAND,WM_DESTROY,WM_LBUTTONDBLCLK,WM_NCCREATE,WM_RBUTTONUP,WM_TIMER,WNDCLASSW,WS_OVERLAPPED,HICON}},
};

const WM_TRAY:u32=WM_APP+17;const TRAY_ID:u32=1;const TIMER_ID:usize=1;
const CMD_OPEN_SETTINGS:u32=1010;const CMD_OPEN_LOGS:u32=1011;const CMD_TTS_TOGGLE:u32=1013;const CMD_ADD_TAB:u32=1014;const CMD_SETTINGS_APPLIED:u32=1020;const CMD_TTS_TEST:u32=1021;const CMD_OPEN_SETTINGS_JSON:u32=1022;const CMD_OPEN_APP_FOLDER:u32=1023;const CMD_EXIT:u32=1099;const CMD_TAB_BASE:u32=2000;const MAX_MENU_TABS:usize=50;

pub struct SingleInstance{handle:HANDLE,owned:bool}
impl SingleInstance{pub fn acquire()->Result<Self,String>{let name=wide("Local\\BPSR-ReadyAlert-Rust-v1");let handle=unsafe{CreateMutexW(null(),1,name.as_ptr())};if handle.is_null(){return Err(format!("CreateMutexW failed: {}",unsafe{GetLastError()}));}let already=unsafe{GetLastError()}==ERROR_ALREADY_EXISTS;Ok(Self{handle,owned:!already})}pub fn is_owner(&self)->bool{self.owned}}
impl Drop for SingleInstance{fn drop(&mut self){unsafe{if self.owned{let _=ReleaseMutex(self.handle);}let _=CloseHandle(self.handle);}}}

struct UiState{
    settings:Arc<RwLock<AppSettings>>,features:Arc<RwLock<FeatureSettings>>,paths:AppPaths,rx:Receiver<AppEvent>,stop:Arc<AtomicBool>,api:Arc<PcapApi>,
    chat_overlay:HWND,dps_overlay:HWND,mechanics_overlay:HWND,capture_status:String,
}

pub fn run_ui(settings:Arc<RwLock<AppSettings>>,paths:AppPaths,rx:Receiver<AppEvent>,stop:Arc<AtomicBool>,api:Arc<PcapApi>)->Result<(),String>{unsafe{
    sanitize_and_save(&settings,&paths);
    let features=Arc::new(RwLock::new(feature_settings::load(&paths)));
    let instance=GetModuleHandleW(null());let main_class=wide("BPSRReadyAlertRustMainV160");let wc=WNDCLASSW{lpfnWndProc:Some(main_wnd_proc),hInstance:instance,hIcon:app_icon(instance),hCursor:LoadCursorW(null_mut(),IDC_ARROW),lpszClassName:main_class.as_ptr(),..std::mem::zeroed()};
    if RegisterClassW(&wc)==0&&GetLastError()!=1410{return Err(format!("RegisterClassW(main) failed: {}",GetLastError()));}
    let mut state=Box::new(UiState{settings,features,paths,rx,stop,api,chat_overlay:null_mut(),dps_overlay:null_mut(),mechanics_overlay:null_mut(),capture_status:"Starting capture…".into()});let state_ptr:*mut UiState=&mut*state;
    let hwnd=CreateWindowExW(0,main_class.as_ptr(),wide("BPSR Ready Alert").as_ptr(),WS_OVERLAPPED,0,0,0,0,null_mut(),null_mut(),instance,state_ptr.cast::<c_void>());if hwnd.is_null(){return Err(format!("CreateWindowExW(main) failed: {}",GetLastError()));}
    state.chat_overlay=overlay::create(instance,state.settings.clone(),hwnd,state.paths.clone())?;
    state.dps_overlay=feature_overlays::create_dps(instance,hwnd,state.features.clone(),state.paths.clone())?;
    state.mechanics_overlay=feature_overlays::create_mechanics(instance,hwnd,state.features.clone(),state.paths.clone())?;
    add_tray(hwnd,&state.capture_status)?;apply_visibility(&mut state);SetTimer(hwnd,TIMER_ID,50,None);
    let mut msg:MSG=std::mem::zeroed();while GetMessageW(&mut msg,null_mut(),0,0)>0{TranslateMessage(&msg);DispatchMessageW(&msg);}state.stop.store(true,Ordering::Relaxed);drop(state);Ok(())
}}

unsafe extern "system" fn main_wnd_proc(hwnd:HWND,msg:u32,wparam:WPARAM,lparam:LPARAM)->LRESULT{
    if msg==WM_NCCREATE{let cs=lparam as *const CREATESTRUCTW;if !cs.is_null(){SetWindowLongPtrW(hwnd,GWLP_USERDATA,(*cs).lpCreateParams as isize);}}
    let ptr=GetWindowLongPtrW(hwnd,GWLP_USERDATA)as *mut UiState;
    match msg{
        WM_TIMER if wparam==TIMER_ID=>{if !ptr.is_null(){drain_events(hwnd,&mut*ptr);feature_overlays::tick((*ptr).mechanics_overlay);}0}
        WM_TRAY=>{if !ptr.is_null(){let state=&mut*ptr;let mouse=lparam as u32;if mouse==WM_RBUTTONUP{let base=state.settings.read().map(|s|s.clone()).unwrap_or_default();let features=state.features.read().map(|s|s.clone()).unwrap_or_default();let action=tray::show(hwnd,&base,&features,&state.api);handle_tray_action(hwnd,state,action);}else if mouse==WM_LBUTTONDBLCLK{handle_command(hwnd,state,CMD_OPEN_SETTINGS,0);}}0}
        WM_COMMAND=>{if !ptr.is_null(){handle_command(hwnd,&mut*ptr,(wparam as u32)&0xffff,lparam);}0}
        WM_DESTROY=>{if !ptr.is_null(){let state=&mut*ptr;state.stop.store(true,Ordering::Relaxed);for w in[state.chat_overlay,state.dps_overlay,state.mechanics_overlay]{if !w.is_null()&&IsWindow(w)!=0{DestroyWindow(w);}}}remove_tray(hwnd);PostQuitMessage(0);0}
        _=>DefWindowProcW(hwnd,msg,wparam,lparam),
    }
}

unsafe fn drain_events(hwnd:HWND,state:&mut UiState){loop{match state.rx.try_recv(){Ok(event)=>handle_event(hwnd,state,event),Err(TryRecvError::Empty)|Err(TryRecvError::Disconnected)=>break,}}}
unsafe fn handle_event(hwnd:HWND,state:&mut UiState,event:AppEvent){match event{
    AppEvent::Alert(alert)=>{let snapshot=state.settings.read().map(|s|s.clone()).unwrap_or_default();let enabled=alert.kind==AlertKind::Error||alert_enabled(&snapshot,alert.kind);if enabled{if alert.kind!=AlertKind::Error{let volume=snapshot.alert_volume;thread::spawn(move||audio::play_alert(alert.kind,volume));}if snapshot.desktop_notification||alert.kind==AlertKind::Error{balloon(hwnd,&alert.title,&alert.message,alert.kind==AlertKind::Error);}}}
    AppEvent::Chat(message)=>{let snapshot=state.settings.read().map(|s|s.clone()).unwrap_or_default();if snapshot.chat_overlay_enabled&&!chat::should_hide_globally(&snapshot,&message){let mut display=message.clone();if display.sender_level>0{display.sender_level=-display.sender_level;}overlay::push_chat(state.chat_overlay,display);maybe_chat_sound(&snapshot,&message);}}
    AppEvent::Translation{sequence_id,text,source_language}=>{let snapshot=state.settings.read().map(|s|s.clone()).unwrap_or_default();if snapshot.chat_overlay_enabled&&snapshot.speech_translation.show_translation_in_overlay{overlay::set_translation(state.chat_overlay,sequence_id,text,source_language);}}
    AppEvent::Identity(identity)=>logging::write(format!("identity: {} uid={}",identity.name,identity.uid)),
    AppEvent::CaptureStatus(status)=>{state.capture_status=status;update_tip(hwnd,&state.capture_status);}
    AppEvent::Dps(snapshot)=>feature_overlays::update_dps(state.dps_overlay,snapshot),
    AppEvent::Mechanics(snapshot)=>feature_overlays::update_mechanics(state.mechanics_overlay,snapshot),
}}

fn alert_enabled(settings:&AppSettings,kind:AlertKind)->bool{match kind{AlertKind::Queue=>settings.queue_pop_alert,AlertKind::Ready=>settings.ready_check_alert,AlertKind::PartyInvite=>settings.party_invite_alert,AlertKind::PartyRequest=>settings.party_request_alert,AlertKind::Error=>true}}
fn maybe_chat_sound(settings:&AppSettings,message:&ChatMessage){let path=if message.channel==5&&settings.chat.private_sound_enabled{Some(settings.chat.private_sound_path.clone())}else{settings.chat.highlight_sound_rules.iter().find(|r|r.enabled&&!r.match_text.trim().is_empty()&&chat::matches_expression(&message.text,&r.match_text)).map(|r|r.sound_path.clone())};if let Some(path)=path.filter(|p|!p.trim().is_empty()){let volume=settings.chat.chat_sound_volume;thread::spawn(move||{let _=audio::play_file(Path::new(&path),volume);});}}

unsafe fn handle_tray_action(hwnd:HWND,state:&mut UiState,action:TrayAction){match action{
    TrayAction::None=>{},TrayAction::Exit=>{DestroyWindow(hwnd);},TrayAction::OpenSettings=>handle_command(hwnd,state,CMD_OPEN_SETTINGS,0),TrayAction::OpenChatLogs=>handle_command(hwnd,state,CMD_OPEN_LOGS,0),TrayAction::OpenAppFolder=>handle_command(hwnd,state,CMD_OPEN_APP_FOLDER,0),
    TrayAction::OpenLogFile=>{if !state.paths.log.exists(){let _=std::fs::write(&state.paths.log,"No log entries yet.\r\n");}open_path(&state.paths.log);},
    TrayAction::SetAlertVolume(volume)=>update_settings(state,|s|s.alert_volume=volume.clamp(0,100)),TrayAction::SelectAdapter(device)=>select_adapter(state,device),
    TrayAction::ToggleSoundAlerts=>update_settings(state,|s|{let all=s.queue_pop_alert&&s.ready_check_alert&&s.party_invite_alert&&s.party_request_alert;let next=!all;s.queue_pop_alert=next;s.ready_check_alert=next;s.party_invite_alert=next;s.party_request_alert=next;}),
    TrayAction::ToggleDesktop=>update_settings(state,|s|s.desktop_notification=!s.desktop_notification),
    TrayAction::ToggleChat=>{update_settings(state,|s|s.chat_overlay_enabled=!s.chat_overlay_enabled);apply_visibility(state);},
    TrayAction::ToggleDps=>{update_features(state,|f|f.dps_overlay_enabled=!f.dps_overlay_enabled);apply_visibility(state);},
    TrayAction::ToggleMechanics=>{update_features(state,|f|f.mechanics_overlay_enabled=!f.mechanics_overlay_enabled);apply_visibility(state);},
}}

fn update_settings<F:FnOnce(&mut AppSettings)>(state:&UiState,update:F){let snapshot=if let Ok(mut s)=state.settings.write(){update(&mut s);enforce_hardcoded(&mut s);s.normalize();let snap=s.clone();drop(s);snap}else{return;};if let Err(err)=settings::save(&state.paths,&snapshot){logging::write(format!("settings: save failed: {err}"));}}
fn update_features<F:FnOnce(&mut FeatureSettings)>(state:&UiState,update:F){let snapshot=if let Ok(mut f)=state.features.write(){update(&mut f);f.normalize();let snap=f.clone();drop(f);snap}else{return;};if let Err(err)=feature_settings::save(&state.paths,&snapshot){logging::write(format!("feature settings: save failed: {err}"));}}
fn sanitize_and_save(settings_arc:&Arc<RwLock<AppSettings>>,paths:&AppPaths){if let Ok(mut s)=settings_arc.write(){enforce_hardcoded(&mut s);s.normalize();let snap=s.clone();drop(s);let _=settings::save(paths,&snap);}}
fn enforce_hardcoded(s:&mut AppSettings){s.auto_launch_resonance_logs=false;s.resonance_logs_path.clear();s.chat.bold_message_text=false;s.chat.text_shadow=true;s.chat.show_separators=false;}

unsafe fn apply_visibility(state:&mut UiState){let base=state.settings.read().map(|s|s.clone()).unwrap_or_default();let features=state.features.read().map(|s|s.clone()).unwrap_or_default();if base.chat_overlay_enabled{overlay::expand_if_collapsed(state.chat_overlay);ShowWindow(state.chat_overlay,SW_SHOW);}else{ShowWindow(state.chat_overlay,SW_HIDE);}if features.dps_overlay_enabled{feature_overlays::expand(state.dps_overlay);ShowWindow(state.dps_overlay,SW_SHOW);}else{ShowWindow(state.dps_overlay,SW_HIDE);}if features.mechanics_overlay_enabled{feature_overlays::expand(state.mechanics_overlay);ShowWindow(state.mechanics_overlay,SW_SHOW);}else{ShowWindow(state.mechanics_overlay,SW_HIDE);}}

fn select_adapter(state:&UiState,device:Option<String>){let next=device.unwrap_or_default();let current=state.settings.read().map(|s|s.npcap_device_name.clone()).unwrap_or_default();if current.eq_ignore_ascii_case(&next){return;}if !next.is_empty(){match CaptureHandle::open(state.api.clone(),&next){Ok(handle)=>{logging::write(format!("capture: adapter preflight ok datalink={} device={}",handle.datalink,next));drop(handle);},Err(err)=>{logging::write(format!("capture: adapter preflight rejected device={next}: {err}"));message_box("BPSR Ready Alert - Network Adapter",&format!("Could not activate that Npcap adapter. ReadyAlert kept the current adapter.\n\n{err}"),true);return;}}}update_settings(state,|s|s.npcap_device_name=next.clone());logging::write(format!("capture: adapter preference updated to {}",if next.is_empty(){"Auto"}else{next.as_str()}));}

unsafe fn handle_command(hwnd:HWND,state:&mut UiState,command:u32,lparam:LPARAM){
    if command==CMD_EXIT{DestroyWindow(hwnd);return;}if command==feature_overlays::CMD_HIDE_DPS{update_features(state,|f|f.dps_overlay_enabled=false);ShowWindow(state.dps_overlay,SW_HIDE);return;}if command==feature_overlays::CMD_HIDE_MECHANICS{update_features(state,|f|f.mechanics_overlay_enabled=false);ShowWindow(state.mechanics_overlay,SW_HIDE);return;}
    if command==CMD_OPEN_LOGS{let _=std::fs::create_dir_all(&state.paths.chat_logs);open_path(&state.paths.chat_logs);return;}if command==CMD_OPEN_SETTINGS_JSON{open_path(&state.paths.settings);return;}if command==CMD_OPEN_APP_FOLDER{open_path(&state.paths.root);return;}
    if command==CMD_OPEN_SETTINGS{let instance=GetModuleHandleW(null());if let Err(err)=settings_ui::show(instance,hwnd,state.settings.clone(),state.paths.clone(),1,false){logging::write(format!("settings ui: {err}"));message_box("ReadyAlert Settings",&err,true);}return;}
    if command==CMD_ADD_TAB{let instance=GetModuleHandleW(null());if let Err(err)=settings_ui::show(instance,hwnd,state.settings.clone(),state.paths.clone(),4,true){logging::write(format!("settings ui: {err}"));message_box("ReadyAlert Settings",&err,true);}return;}
    if command==CMD_TTS_TEST{let volume=(lparam as i32).clamp(0,100);thread::spawn(move||{if let Err(err)=test_tts(volume){logging::write(format!("tts: test failed: {err}"));}});return;}
    if command==CMD_SETTINGS_APPLIED{sanitize_and_save(&state.settings,&state.paths);let snapshot=state.settings.read().map(|s|s.clone()).unwrap_or_default();overlay::apply_style(state.chat_overlay,&snapshot);overlay::refresh(state.chat_overlay);apply_visibility(state);return;}
    if command==CMD_TTS_TOGGLE{update_settings(state,|s|s.speech_translation.tts_enabled=!s.speech_translation.tts_enabled);overlay::refresh(state.chat_overlay);return;}
    if command>=CMD_TAB_BASE&&command<CMD_TAB_BASE+MAX_MENU_TABS as u32{let idx=(command-CMD_TAB_BASE)as usize;update_settings(state,|s|{if let Some(tab)=s.chat.tabs.get(idx){s.chat.last_selected_tab_id=tab.id;}});overlay::refresh(state.chat_overlay);}
}

fn test_tts(volume:i32)->Result<(),String>{if volume<=0{return Err("TTS volume is muted at 0%.".into());}let sample="ReadyAlert text to speech test.";let url=format!("https://translate.google.com/translate_tts?ie=UTF-8&client=tw-ob&tl=en&total=1&idx=0&textlen={}&q={}",sample.chars().count(),urlencoding::encode(sample));let agent=ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(4)).timeout_read(Duration::from_secs(8)).timeout_write(Duration::from_secs(8)).build();let response=agent.get(&url).set("Accept","audio/mpeg,*/*;q=0.8").call().map_err(|e|e.to_string())?;let content_type=response.header("content-type").unwrap_or("").to_ascii_lowercase();if !content_type.starts_with("audio/"){return Err(format!("unexpected TTS content-type {content_type}"));}let mut bytes=Vec::new();response.into_reader().take(4*1024*1024+1).read_to_end(&mut bytes).map_err(|e|e.to_string())?;if bytes.len()<200||bytes.len()>4*1024*1024{return Err(format!("invalid TTS payload {} bytes",bytes.len()));}audio::play_mp3(&bytes,volume)}

unsafe fn add_tray(hwnd:HWND,tip:&str)->Result<(),String>{let mut nid:NOTIFYICONDATAW=std::mem::zeroed();nid.cbSize=std::mem::size_of::<NOTIFYICONDATAW>()as u32;nid.hWnd=hwnd;nid.uID=TRAY_ID;nid.uFlags=NIF_ICON|NIF_MESSAGE|NIF_TIP;nid.uCallbackMessage=WM_TRAY;nid.hIcon=app_icon(GetModuleHandleW(null()));write_wide(&mut nid.szTip,&format!("BPSR Ready Alert - {tip}"));if Shell_NotifyIconW(NIM_ADD,&nid)==0{return Err("Shell_NotifyIconW(NIM_ADD) failed".into());}Ok(())}
unsafe fn remove_tray(hwnd:HWND){let mut nid:NOTIFYICONDATAW=std::mem::zeroed();nid.cbSize=std::mem::size_of::<NOTIFYICONDATAW>()as u32;nid.hWnd=hwnd;nid.uID=TRAY_ID;Shell_NotifyIconW(NIM_DELETE,&nid);}
unsafe fn update_tip(hwnd:HWND,tip:&str){let mut nid:NOTIFYICONDATAW=std::mem::zeroed();nid.cbSize=std::mem::size_of::<NOTIFYICONDATAW>()as u32;nid.hWnd=hwnd;nid.uID=TRAY_ID;nid.uFlags=NIF_TIP;write_wide(&mut nid.szTip,&format!("BPSR Ready Alert - {tip}"));Shell_NotifyIconW(NIM_MODIFY,&nid);}
unsafe fn balloon(hwnd:HWND,title:&str,message:&str,error:bool){let mut nid:NOTIFYICONDATAW=std::mem::zeroed();nid.cbSize=std::mem::size_of::<NOTIFYICONDATAW>()as u32;nid.hWnd=hwnd;nid.uID=TRAY_ID;nid.uFlags=NIF_INFO;nid.dwInfoFlags=if error{NIIF_ERROR}else{NIIF_INFO};write_wide(&mut nid.szInfoTitle,title);write_wide(&mut nid.szInfo,message);Shell_NotifyIconW(NIM_MODIFY,&nid);}
unsafe fn app_icon(instance:HINSTANCE)->HICON{let icon=LoadIconW(instance,1usize as *const u16);if icon.is_null(){LoadIconW(null_mut(),IDI_APPLICATION)}else{icon}}
pub fn message_box(title:&str,text:&str,error:bool){unsafe{MessageBoxW(null_mut(),wide(text).as_ptr(),wide(title).as_ptr(),MB_OK|if error{MB_ICONERROR}else{MB_ICONINFORMATION});}}
unsafe fn open_path(path:&Path){ShellExecuteW(null_mut(),wide("open").as_ptr(),wide(&path.to_string_lossy()).as_ptr(),null(),null(),SW_SHOWNORMAL);}
fn wide(text:&str)->Vec<u16>{text.encode_utf16().chain(std::iter::once(0)).collect()}
fn write_wide<const N:usize>(target:&mut[u16;N],text:&str){target.fill(0);for(dst,src)in target.iter_mut().take(N.saturating_sub(1)).zip(text.encode_utf16()){*dst=src;}}
