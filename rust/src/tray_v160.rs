use crate::{feature_settings::FeatureSettings, npcap::PcapApi, settings::AppSettings};
use std::ptr::null;
use windows_sys::Win32::{
    Foundation::{HWND, POINT},
    UI::WindowsAndMessaging::{
        AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, PostMessageW, SetForegroundWindow,
        TrackPopupMenu, HMENU, MF_CHECKED, MF_POPUP, MF_SEPARATOR, MF_STRING,
        TPM_BOTTOMALIGN, TPM_LEFTALIGN, TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_NULL,
    },
};

const CMD_SOUND_ALERTS:u32=1001;
const CMD_DESKTOP:u32=1002;
const CMD_CHAT:u32=1003;
const CMD_DPS:u32=1004;
const CMD_MECH:u32=1005;
const CMD_EVENT_TRACKER:u32=1006;
const CMD_SETTINGS:u32=1010;
const CMD_CHAT_LOGS:u32=1011;
const CMD_APP_FOLDER:u32=1012;
const CMD_LOG_FILE:u32=1013;
const CMD_EXIT:u32=1099;
const CMD_VOLUME_BASE:u32=1300;
const CMD_ADAPTER_AUTO:u32=1400;
const CMD_ADAPTER_BASE:u32=1410;
const MAX_ADAPTERS:usize=128;

#[derive(Clone,Debug)]
pub enum TrayAction{
    None,
    ToggleSoundAlerts,
    ToggleDesktop,
    ToggleChat,
    ToggleDps,
    ToggleMechanics,
    OpenEventTracker,
    SetAlertVolume(i32),
    SelectAdapter(Option<String>),
    OpenSettings,
    OpenChatLogs,
    OpenAppFolder,
    OpenLogFile,
    Exit,
}

pub unsafe fn show(hwnd:HWND,settings:&AppSettings,features:&FeatureSettings,api:&PcapApi)->TrayAction{
    let menu=CreatePopupMenu();if menu.is_null(){return TrayAction::None;}
    let all_alerts=settings.queue_pop_alert&&settings.ready_check_alert&&settings.party_invite_alert&&settings.party_request_alert;
    append_check(menu,CMD_SOUND_ALERTS,"Alert sounds",all_alerts);
    append_check(menu,CMD_DESKTOP,"Desktop notifications",settings.desktop_notification);
    AppendMenuW(menu,MF_SEPARATOR,0,null());
    append_check(menu,CMD_CHAT,"Chat Overlay",settings.chat_overlay_enabled);
    append_check(menu,CMD_DPS,"DPS Meter",features.dps_overlay_enabled);
    append_check(menu,CMD_MECH,"Dungeon Mechanics",features.mechanics_overlay_enabled);
    append_string(menu,CMD_EVENT_TRACKER,"Event Tracker…");

    let adapter_menu=CreatePopupMenu();let devices=api.devices().unwrap_or_default();
    if !adapter_menu.is_null(){append_check(adapter_menu,CMD_ADAPTER_AUTO,"Automatic",settings.npcap_device_name.trim().is_empty());AppendMenuW(adapter_menu,MF_SEPARATOR,0,null());for(index,device)in devices.iter().take(MAX_ADAPTERS).enumerate(){let selected=!settings.npcap_device_name.trim().is_empty()&&settings.npcap_device_name.eq_ignore_ascii_case(&device.name);append_check(adapter_menu,CMD_ADAPTER_BASE+index as u32,&shorten(&device.description,64),selected);}AppendMenuW(menu,MF_POPUP,adapter_menu as usize,wide(&adapter_label(settings,&devices)).as_ptr());}

    let volume_menu=CreatePopupMenu();if !volume_menu.is_null(){for step in 0..=10{let volume=step*10;let label=if volume==0{"Mute".to_string()}else{format!("{volume}%")};append_check(volume_menu,CMD_VOLUME_BASE+step as u32,&label,settings.alert_volume==volume);}AppendMenuW(menu,MF_POPUP,volume_menu as usize,wide(&format!("Alert volume: {}%",settings.alert_volume.clamp(0,100))).as_ptr());}

    AppendMenuW(menu,MF_SEPARATOR,0,null());append_string(menu,CMD_SETTINGS,"Settings…");append_string(menu,CMD_CHAT_LOGS,"Open local archives");append_string(menu,CMD_APP_FOLDER,"Open app data folder");append_string(menu,CMD_LOG_FILE,"Open diagnostic log");AppendMenuW(menu,MF_SEPARATOR,0,null());append_string(menu,CMD_EXIT,"Exit ReadyAlert");
    let mut p:POINT=std::mem::zeroed();GetCursorPos(&mut p);SetForegroundWindow(hwnd);let command=TrackPopupMenu(menu,TPM_LEFTALIGN|TPM_BOTTOMALIGN|TPM_RIGHTBUTTON|TPM_RETURNCMD,p.x,p.y,0,hwnd,null());
    // Explorer notification-area menus require a message after TrackPopupMenu
    // returns; without it the menu can remain in an odd/stuck dismissal state.
    let _=PostMessageW(hwnd,WM_NULL,0,0);
    DestroyMenu(menu);
    match command as u32{
        CMD_SOUND_ALERTS=>TrayAction::ToggleSoundAlerts,CMD_DESKTOP=>TrayAction::ToggleDesktop,CMD_CHAT=>TrayAction::ToggleChat,CMD_DPS=>TrayAction::ToggleDps,CMD_MECH=>TrayAction::ToggleMechanics,CMD_EVENT_TRACKER=>TrayAction::OpenEventTracker,
        CMD_SETTINGS=>TrayAction::OpenSettings,CMD_CHAT_LOGS=>TrayAction::OpenChatLogs,CMD_APP_FOLDER=>TrayAction::OpenAppFolder,CMD_LOG_FILE=>TrayAction::OpenLogFile,CMD_EXIT=>TrayAction::Exit,CMD_ADAPTER_AUTO=>TrayAction::SelectAdapter(None),
        id if id>=CMD_ADAPTER_BASE&&id<CMD_ADAPTER_BASE+MAX_ADAPTERS as u32=>devices.get((id-CMD_ADAPTER_BASE)as usize).map(|d|TrayAction::SelectAdapter(Some(d.name.clone()))).unwrap_or(TrayAction::None),
        id if id>=CMD_VOLUME_BASE&&id<=CMD_VOLUME_BASE+10=>TrayAction::SetAlertVolume(((id-CMD_VOLUME_BASE)*10)as i32),_=>TrayAction::None,
    }
}

unsafe fn append_check(menu:HMENU,id:u32,text:&str,checked:bool){AppendMenuW(menu,MF_STRING|if checked{MF_CHECKED}else{0},id as usize,wide(text).as_ptr());}
unsafe fn append_string(menu:HMENU,id:u32,text:&str){AppendMenuW(menu,MF_STRING,id as usize,wide(text).as_ptr());}
fn adapter_label(settings:&AppSettings,devices:&[crate::npcap::NpcapDevice])->String{if settings.npcap_device_name.trim().is_empty(){return "Network adapter: Automatic".into();}let description=devices.iter().find(|d|d.name.eq_ignore_ascii_case(settings.npcap_device_name.trim())).map(|d|d.description.as_str()).unwrap_or(settings.npcap_device_name.trim());format!("Network adapter: {}",shorten(description,38))}
fn shorten(value:&str,max:usize)->String{if value.chars().count()<=max{return value.into();}let head:String=value.chars().take(max.saturating_sub(3)).collect();format!("{head}...")}
fn wide(text:&str)->Vec<u16>{text.encode_utf16().chain(std::iter::once(0)).collect()}
