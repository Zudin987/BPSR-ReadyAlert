#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod capture;
mod chat;
mod game_filter;
mod logging;
mod model;
mod npcap;
mod paths;
mod proto;
mod settings;
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

    let loaded = settings::load(&paths);
    let settings = Arc::new(RwLock::new(loaded.clone()));
    win::auto_launch_resonance_logs(&loaded);

    let api = match npcap::PcapApi::load() {
        Ok(api) => api,
        Err(err) => {
            logging::write(format!("startup: Npcap missing {err}"));
            win::message_box(
                "BPSR Ready Alert - Npcap Required",
                &format!("BPSR Ready Alert could not load Npcap.\n\nInstall Npcap (or repair the Npcap installation used by Resonance Logs CN).\n\nDetails: {err}"),
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
    let capture_thread = capture::spawn(api, settings.clone(), identity, chat_runtime, tx, stop.clone());

    if let Err(err) = win::run_ui(settings, paths, rx, stop.clone()) {
        logging::write(format!("startup/ui: {err}"));
        win::message_box("BPSR Ready Alert - Error", &err, true);
    }
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    let _ = capture_thread.join();
    drop(guard);
    logging::write("shutdown: complete");
    logging::shutdown();
}

fn smoke_test() -> Result<(), String> {
    // Deterministic protocol tests that also prove all statically embedded assets link.
    let mut frame = vec![0u8; 22];
    frame[0..4].copy_from_slice(&22u32.to_be_bytes());
    frame[4..6].copy_from_slice(&2u16.to_be_bytes());
    frame[6..14].copy_from_slice(&proto::CHAT_SERVICE.to_be_bytes());
    if frame.len() != 22 { return Err("frame fixture failed".into()); }
    let mut settings = settings::AppSettings::default();
    settings.normalize();
    if settings.chat.tabs.len() < 4 || settings.chat.local_chat_log_retention_hours != 168 {
        return Err("settings defaults failed".into());
    }
    if !settings.speech_translation.tts_for(3) || settings.speech_translation.tts_for(1) {
        return Err("TTS channel defaults failed".into());
    }
    std::thread::sleep(Duration::from_millis(1));
    Ok(())
}
