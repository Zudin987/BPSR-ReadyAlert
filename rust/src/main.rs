#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
#[path = "capture_v160.rs"]
mod capture;
mod capture_supervisor;
#[path = "chat.rs"]
mod chat_legacy;
mod chat {
    pub use crate::chat_legacy::{ChatRuntime, should_hide_globally};
    pub(crate) use crate::chat_legacy::{matches_expression, validate_expression};
    use crate::{model::ChatMessage, settings::AppSettings};

    pub fn should_hide_in_overlay(settings: &AppSettings, message: &ChatMessage) -> bool {
        if should_hide_globally(settings, message) { return true; }
        let tab = settings.chat.active_tab();
        let level = message.sender_level.saturating_abs();
        if !tab.channels.contains(&message.channel) || level < tab.min_level { return true; }
        let hay = format!("{} {}", message.sender_name, message.text);
        if !tab.show_if_matches.trim().is_empty() && !matches_expression(&hay, &tab.show_if_matches) { return true; }
        if !tab.hide_if_matches.trim().is_empty() && matches_expression(&hay, &tab.hide_if_matches) { return true; }
        false
    }
}
#[path = "feature_settings_v170.rs"]
mod feature_settings;
mod feature_overlays_impl {
    include!(concat!(env!("OUT_DIR"), "/feature_overlays_v170_fixed.rs"));
}
mod feature_overlays {
    pub use crate::feature_overlays_impl::*;
    use windows_sys::Win32::{Foundation::HWND, Graphics::Gdi::InvalidateRect};
    pub unsafe fn tick(hwnd: HWND) {
        if !hwnd.is_null() { InvalidateRect(hwnd, std::ptr::null(), 0); }
    }
}
mod game_filter;
mod logging;
#[path = "model_v170.rs"]
mod model;
mod npcap;
#[path = "overlay_v150.rs"]
mod overlay;
mod paths;
mod proto;
mod settings;
mod settings_cleanup_v160;
mod settings_repaint_hotfix;
mod settings_ui;
#[path = "telemetry_adapter_v170.rs"]
mod telemetry;
#[path = "tray_v160.rs"]
mod tray;
#[path = "win_v160.rs"]
mod win;

use crate::{chat::ChatRuntime, model::PlayerIdentity};
use std::{
    sync::{atomic::AtomicBool, mpsc, Arc, RwLock},
    time::Duration,
};

fn main() {
    if std::env::args().any(|x| x.eq_ignore_ascii_case("--build-smoke-test")) {
        std::process::exit(match smoke_test() { Ok(()) => 0, Err(err) => { eprintln!("{err}"); 1 } });
    }

    let paths = match paths::AppPaths::create() {
        Ok(p) => p,
        Err(err) => { win::message_box("BPSR Ready Alert", &format!("Could not create app data folder.\n\n{err}"), true); return; }
    };
    logging::init(paths.log.clone());
    logging::write(format!("startup: native-rust version={}", env!("CARGO_PKG_VERSION")));

    let guard = match win::SingleInstance::acquire() {
        Ok(g) => g,
        Err(err) => { logging::write(format!("startup: mutex failed {err}")); win::message_box("BPSR Ready Alert", &err, true); logging::shutdown(); return; }
    };
    if !guard.is_owner() {
        win::message_box("BPSR Ready Alert", "BPSR Ready Alert is already running in the system tray.", false);
        logging::shutdown();
        return;
    }

    let mut loaded = settings::load(&paths);
    loaded.auto_launch_resonance_logs = false;
    loaded.resonance_logs_path.clear();
    loaded.chat.bold_message_text = false;
    loaded.chat.text_shadow = true;
    loaded.chat.show_separators = false;
    loaded.normalize();
    let _ = settings::save(&paths, &loaded);
    let settings = Arc::new(RwLock::new(loaded));

    let api = match npcap::PcapApi::load() {
        Ok(api) => api,
        Err(err) => {
            logging::write(format!("startup: Npcap missing {err}"));
            win::message_box(
                "BPSR Ready Alert - Npcap Required",
                &format!("BPSR Ready Alert could not load Npcap.\n\nInstall or repair Npcap.\n\nDetails: {err}"),
                true,
            );
            logging::shutdown();
            return;
        }
    };

    let identity: Arc<RwLock<Option<PlayerIdentity>>> = Arc::new(RwLock::new(None));
    let (tx, rx) = mpsc::channel();
    let chat_runtime = ChatRuntime::start(paths.clone(), settings.clone(), identity.clone(), tx.clone());
    let stop = Arc::new(AtomicBool::new(false));
    let capture_thread = capture_supervisor::spawn(
        api.clone(), settings.clone(), identity, chat_runtime, tx, stop.clone(),
    );
    let settings_repaint_thread = settings_repaint_hotfix::start(stop.clone());
    let settings_cleanup_thread = settings_cleanup_v160::start(stop.clone());

    if let Err(err) = win::run_ui(settings, paths, rx, stop.clone(), api) {
        logging::write(format!("startup/ui: {err}"));
        win::message_box("BPSR Ready Alert - Error", &err, true);
    }
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    let _ = settings_cleanup_thread.join();
    let _ = settings_repaint_thread.join();
    let _ = capture_thread.join();
    drop(guard);
    logging::write("shutdown: complete");
    logging::shutdown();
}

fn smoke_test() -> Result<(), String> {
    let mut frame = vec![0u8; 22];
    frame[0..4].copy_from_slice(&22u32.to_be_bytes());
    frame[4..6].copy_from_slice(&2u16.to_be_bytes());
    frame[6..14].copy_from_slice(&proto::CHAT_SERVICE.to_be_bytes());
    if frame.len() != 22 { return Err("frame fixture failed".into()); }
    let mut settings = settings::AppSettings::default();
    settings.normalize();
    if settings.chat.tabs.len() < 4 || settings.chat.local_chat_log_retention_hours != 168 { return Err("settings defaults failed".into()); }
    let mut features = feature_settings::FeatureSettings::default();
    features.normalize();
    if !features.dps_overlay_enabled || !features.mechanics_overlay_enabled { return Err("feature overlay defaults failed".into()); }
    if features.mechanic_attributes.tracked != vec![feature_settings::ATTR_LUCK, feature_settings::ATTR_HASTE, feature_settings::ATTR_MASTERY] { return Err("mechanic tracked-attribute defaults failed".into()); }
    if features.dps.opacity < 25 || features.mechanics.opacity < 25 { return Err("feature opacity normalization failed".into()); }
    if !settings.speech_translation.tts_for(3) || settings.speech_translation.tts_for(1) { return Err("TTS channel defaults failed".into()); }
    std::thread::sleep(Duration::from_millis(1));
    Ok(())
}
