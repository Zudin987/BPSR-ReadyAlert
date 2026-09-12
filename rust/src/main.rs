#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod chat_archive;
mod capture {
    include!(concat!(env!("OUT_DIR"), "/capture_v185.rs"));
}
mod capture_supervisor {
    include!(concat!(env!("OUT_DIR"), "/capture_supervisor_v1241.rs"));
}
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
mod event_tracker;
mod event_tracker_ui {
    include!(concat!(env!("OUT_DIR"), "/event_tracker_ui_v1160_fixed.rs"));
}
#[path = "feature_settings_v181.rs"]
mod feature_settings;
mod feature_overlays_impl {
    include!(concat!(env!("OUT_DIR"), "/feature_overlays_v170_fixed.rs"));
}
mod feature_overlays {
    pub use crate::feature_overlays_impl::*;
    use std::{cell::RefCell, collections::HashMap, time::{Duration, Instant}};
    use windows_sys::Win32::{Foundation::HWND, Graphics::Gdi::InvalidateRect, UI::WindowsAndMessaging::IsWindowVisible};

    thread_local! {
        static LAST_TICK_PAINT: RefCell<HashMap<isize, Instant>> = RefCell::new(HashMap::new());
    }

    /// Time-based overlay labels still need periodic refresh, but the native UI
    /// should not invalidate hidden windows or repaint on every host timer pulse.
    pub unsafe fn tick(hwnd: HWND) {
        if hwnd.is_null() || IsWindowVisible(hwnd) == 0 { return; }
        let now = Instant::now();
        let should_paint = LAST_TICK_PAINT.with(|last| {
            let mut last = last.borrow_mut();
            let key = hwnd as isize;
            match last.get_mut(&key) {
                Some(previous) if now.duration_since(*previous) < Duration::from_millis(125) => false,
                Some(previous) => { *previous = now; true }
                None => { last.insert(key, now); true }
            }
        });
        if should_paint { InvalidateRect(hwnd, std::ptr::null(), 0); }
    }
}
mod game_filter;
mod history;
mod logging;
#[path = "model_v170.rs"]
mod model;
mod npcap;
#[path = "overlay_v181.rs"]
mod overlay;
mod paths;
mod proto;
#[path = "settings_v181.rs"]
mod settings;
mod settings_ui {
    include!(concat!(env!("OUT_DIR"), "/settings_ui_v1160_fixed.rs"));
}
mod sharing;
mod ui;
mod ui_theme;
mod hotkeys;
#[path = "telemetry_v1110.rs"]
mod telemetry;
#[path = "tray_v160.rs"]
mod tray;
mod updater {
    include!(concat!(env!("OUT_DIR"), "/updater_v1241.rs"));
}
mod win {
    include!(concat!(env!("OUT_DIR"), "/win_v182_fixed.rs"));
}

use crate::{chat::ChatRuntime, model::PlayerIdentity};
use std::{
    sync::{atomic::AtomicBool, mpsc, Arc, RwLock},
    time::Duration,
};

fn main() {
    if updater::handle_special_args() {
        return;
    }
    updater::cleanup_stale_helper();

    if std::env::args().any(|x| x.eq_ignore_ascii_case("--build-smoke-test")) {
        std::process::exit(match smoke_test() { Ok(()) => 0, Err(err) => { eprintln!("{err}"); 1 } });
    }

    let paths = match paths::AppPaths::create() {
        Ok(p) => p,
        Err(err) => { win::message_box("BPSR ReadyAlert", &format!("Could not create app data folder.\n\n{err}"), true); return; }
    };

    // Establish ownership before initializing any subsystem that can read or write
    // persistent state. A second launch should not race the running instance's
    // event-tracker/settings/log files merely to discover that it must exit.
    let guard = match win::SingleInstance::acquire() {
        Ok(g) => g,
        Err(err) => { win::message_box("BPSR ReadyAlert", &err, true); return; }
    };
    if !guard.is_owner() {
        win::message_box("BPSR ReadyAlert", "BPSR ReadyAlert is already running in the system tray.", false);
        return;
    }

    logging::init(paths.log.clone());
    logging::write(format!("startup: native-rust version={}", env!("CARGO_PKG_VERSION")));
    event_tracker::init(&paths.root);

    let mut loaded = settings::load(&paths);
    loaded.normalize();
    let settings = Arc::new(RwLock::new(loaded));

    let api = match npcap::PcapApi::load() {
        Ok(api) => api,
        Err(err) => {
            logging::write(format!("startup: Npcap missing {err}"));
            win::message_box(
                "BPSR ReadyAlert - Npcap Required",
                &format!("BPSR ReadyAlert could not load Npcap.\n\nInstall or repair Npcap.\n\nDetails: {err}"),
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
    let capture_thread = match capture_supervisor::spawn(
        api.clone(), settings.clone(), identity, chat_runtime, tx, stop.clone(),
    ) {
        Ok(handle) => handle,
        Err(err) => {
            logging::write(format!("startup/capture-supervisor: {err}"));
            win::message_box("BPSR ReadyAlert - Error", &format!("Could not start packet capture supervision.\n\n{err}"), true);
            logging::shutdown();
            return;
        }
    };

    if let Err(err) = win::run_ui(settings, paths, rx, stop.clone(), api) {
        logging::write(format!("startup/ui: {err}"));
        win::message_box("BPSR ReadyAlert - Error", &err, true);
    }
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
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
    if features.mechanic_attributes.tracked != vec![feature_settings::ATTR_CRIT, feature_settings::ATTR_LUCK, feature_settings::ATTR_HASTE, feature_settings::ATTR_MASTERY] { return Err("mechanic tracked-attribute defaults failed".into()); }
    if feature_settings::format_attr_value(feature_settings::ATTR_LUCK, 4_816) != "48.16%" { return Err("mechanic percentage formatting failed".into()); }
    if features.dps.opacity < 25 || features.mechanics.opacity < 25 { return Err("feature opacity normalization failed".into()); }
    if !features.meter.show_active_rates || !features.meter.always_show_self || features.meter.history_limit < 10 { return Err("v1.9 meter defaults failed".into()); }
    let analysis = model::DpsRow::default();
    if analysis.boss_damage != 0 || analysis.effective_healing != 0 || analysis.overhealing != 0 || !analysis.death_recaps.is_empty() || !analysis.buff_uptimes.is_empty() { return Err("v1.10 analysis defaults failed".into()); }
    let skill = model::SkillBreakdown::default();
    if analysis.absorbed_damage != 0 || !analysis.absorbed_sources.is_empty() || skill.boss_damage != 0 || skill.effective_healing != 0 || skill.overhealing != 0 { return Err("v1.11 skill-analysis defaults failed".into()); }
    if skill.min_value != 0 { return Err("v1.12 skill min default failed".into()); }
    let share_preview = sharing::encounter_summary(&model::DpsSnapshot::default(), sharing::ViewMode::Damage);
    if !share_preview.contains("No contribution data recorded") { return Err("v1.13 sharing smoke test failed".into()); }
    let mut tracker = event_tracker::TrackerSettings::default();
    tracker.max_visible = 99;
    tracker.normalize();
    if tracker.max_visible != 12 || !tracker.rules.is_empty() { return Err("v1.14 tracker defaults failed".into()); }
    if model::imagine_tier_label(0) != "T0" || model::imagine_tier_label(5) != "T5" { return Err("v1.15 Imagine tier display failed".into()); }
    if !settings.speech_translation.tts_for(3) || settings.speech_translation.tts_for(1) { return Err("TTS channel defaults failed".into()); }
    if ui_theme::SETTINGS_W > 820 || ui_theme::SETTINGS_H > 450 || ui_theme::EVENT_H > 450 || ui_theme::MECH_SETTINGS_H > 450 { return Err("v1.17 fixed settings work-area budget failed".into()); }
    std::thread::sleep(Duration::from_millis(1));
    Ok(())
}
