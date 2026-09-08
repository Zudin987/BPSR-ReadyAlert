use crate::{
    capture,
    chat::ChatRuntime,
    logging,
    model::{AppEvent, PlayerIdentity},
    npcap::PcapApi,
    settings::AppSettings,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
        Arc, RwLock,
    },
    thread,
    time::Duration,
};

/// Owns the normal capture worker and restarts only that worker when the user changes
/// the explicit Npcap preference. The packet parser/capture implementation stays in
/// capture.rs; this is deliberately a tiny control-plane wrapper so adapter switching
/// never requires restarting the whole ReadyAlert process.
pub fn spawn(
    api: Arc<PcapApi>,
    settings: Arc<RwLock<AppSettings>>,
    identity: Arc<RwLock<Option<PlayerIdentity>>>,
    chat_runtime: ChatRuntime,
    tx: Sender<AppEvent>,
    stop: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::Builder::new()
        .name("readyalert-capture-supervisor".into())
        .spawn(move || {
            let mut preference = adapter_preference(&settings);
            let mut child_stop = Arc::new(AtomicBool::new(false));
            let mut child = capture::spawn(
                api.clone(),
                settings.clone(),
                identity.clone(),
                chat_runtime.clone(),
                tx.clone(),
                child_stop.clone(),
            );

            while !stop.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(250));
                let next = adapter_preference(&settings);
                if next == preference {
                    continue;
                }

                logging::write(format!(
                    "capture: adapter preference changed; restarting worker old='{}' new='{}'",
                    preference, next
                ));
                child_stop.store(true, Ordering::Relaxed);
                let _ = child.join();
                if stop.load(Ordering::Relaxed) {
                    break;
                }

                preference = next;
                child_stop = Arc::new(AtomicBool::new(false));
                child = capture::spawn(
                    api.clone(),
                    settings.clone(),
                    identity.clone(),
                    chat_runtime.clone(),
                    tx.clone(),
                    child_stop.clone(),
                );
            }

            child_stop.store(true, Ordering::Relaxed);
            let _ = child.join();
            logging::write("capture-supervisor: stopped");
        })
        .expect("spawn capture supervisor")
}

fn adapter_preference(settings: &Arc<RwLock<AppSettings>>) -> String {
    settings
        .read()
        .map(|s| s.npcap_device_name.trim().to_ascii_lowercase())
        .unwrap_or_default()
}
