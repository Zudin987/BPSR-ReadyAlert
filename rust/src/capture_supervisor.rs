use crate::{
    capture,
    chat::ChatRuntime,
    logging,
    model::{AppEvent, PlayerIdentity},
    npcap::PcapApi,
    settings::AppSettings,
};
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
        Arc, RwLock,
    },
    thread,
    time::{Duration, Instant},
};

const SUPERVISOR_POLL: Duration = Duration::from_millis(250);
const SPAWN_RETRY: Duration = Duration::from_secs(1);

/// Owns the normal capture worker and restarts only that worker when the user changes
/// the explicit Npcap preference. It also detects an unexpectedly terminated worker
/// so ReadyAlert cannot remain alive in the tray with packet capture silently dead.
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
            let mut child = start_child(
                &api,
                &settings,
                &identity,
                &chat_runtime,
                &tx,
                &child_stop,
            );
            let mut last_spawn_attempt = Instant::now();

            while !stop.load(Ordering::Relaxed) {
                thread::sleep(SUPERVISOR_POLL);

                if child.as_ref().is_some_and(thread::JoinHandle::is_finished) {
                    let handle = child.take().expect("finished capture handle");
                    match handle.join() {
                        Ok(()) => logging::write("capture-supervisor: worker exited unexpectedly; restarting"),
                        Err(_) => logging::write("capture-supervisor: worker panicked; restarting"),
                    }
                    if stop.load(Ordering::Relaxed) { break; }
                    child_stop = Arc::new(AtomicBool::new(false));
                    child = start_child(&api, &settings, &identity, &chat_runtime, &tx, &child_stop);
                    last_spawn_attempt = Instant::now();
                }

                let next = adapter_preference(&settings);
                if next != preference {
                    logging::write(format!(
                        "capture: adapter preference changed; restarting worker old='{}' new='{}'",
                        preference, next
                    ));
                    child_stop.store(true, Ordering::Relaxed);
                    join_child(child.take(), "adapter switch");
                    if stop.load(Ordering::Relaxed) { break; }

                    preference = next;
                    child_stop = Arc::new(AtomicBool::new(false));
                    child = start_child(&api, &settings, &identity, &chat_runtime, &tx, &child_stop);
                    last_spawn_attempt = Instant::now();
                    continue;
                }

                // Thread creation can fail under severe resource pressure. Do not
                // panic the supervisor; retry at a bounded cadence instead.
                if child.is_none() && last_spawn_attempt.elapsed() >= SPAWN_RETRY {
                    child_stop = Arc::new(AtomicBool::new(false));
                    child = start_child(&api, &settings, &identity, &chat_runtime, &tx, &child_stop);
                    last_spawn_attempt = Instant::now();
                }
            }

            child_stop.store(true, Ordering::Relaxed);
            join_child(child.take(), "shutdown");
            logging::write("capture-supervisor: stopped");
        })
        .expect("spawn capture supervisor")
}

fn start_child(
    api: &Arc<PcapApi>,
    settings: &Arc<RwLock<AppSettings>>,
    identity: &Arc<RwLock<Option<PlayerIdentity>>>,
    chat_runtime: &ChatRuntime,
    tx: &Sender<AppEvent>,
    stop: &Arc<AtomicBool>,
) -> Option<thread::JoinHandle<()>> {
    match catch_unwind(AssertUnwindSafe(|| {
        capture::spawn(
            api.clone(),
            settings.clone(),
            identity.clone(),
            chat_runtime.clone(),
            tx.clone(),
            stop.clone(),
        )
    })) {
        Ok(handle) => Some(handle),
        Err(_) => {
            logging::write("capture-supervisor: could not create capture worker; retrying shortly");
            None
        }
    }
}

fn join_child(child: Option<thread::JoinHandle<()>>, reason: &str) {
    if let Some(handle) = child {
        if handle.join().is_err() {
            logging::write(format!("capture-supervisor: capture worker panicked during {reason}"));
        }
    }
}

fn adapter_preference(settings: &Arc<RwLock<AppSettings>>) -> String {
    settings
        .read()
        .map(|s| s.npcap_device_name.trim().to_ascii_lowercase())
        .unwrap_or_default()
}
