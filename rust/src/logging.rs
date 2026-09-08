use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{mpsc::{self, SyncSender, TrySendError}, Mutex, OnceLock},
    thread::{self, JoinHandle},
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_LOG_BYTES: u64 = 2 * 1024 * 1024;
const MAX_QUEUE: usize = 2048;

enum LogMessage {
    Line(String),
    Stop,
}

struct LoggerState {
    tx: SyncSender<LogMessage>,
    thread: Option<JoinHandle<()>>,
}

static LOGGER: OnceLock<Mutex<Option<LoggerState>>> = OnceLock::new();

pub fn init(path: PathBuf) {
    let slot = LOGGER.get_or_init(|| Mutex::new(None));
    let mut guard = slot.lock().unwrap_or_else(|e| e.into_inner());
    if guard.is_some() {
        return;
    }
    let (tx, rx) = mpsc::sync_channel(MAX_QUEUE);
    let writer = thread::Builder::new()
        .name("readyalert-log".into())
        .spawn(move || {
            rotate_if_needed(&path);
            while let Ok(message) = rx.recv() {
                match message {
                    LogMessage::Line(line) => {
                        rotate_if_needed(&path);
                        if let Some(parent) = path.parent() {
                            let _ = fs::create_dir_all(parent);
                        }
                        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
                            let _ = writeln!(file, "{} {}", unix_millis(), line.replace('\0', ""));
                        }
                    }
                    LogMessage::Stop => break,
                }
            }
        })
        .ok();
    *guard = Some(LoggerState { tx, thread: writer });
}

pub fn write(message: impl Into<String>) {
    let Some(slot) = LOGGER.get() else { return; };
    let guard = slot.lock().unwrap_or_else(|e| e.into_inner());
    let Some(state) = guard.as_ref() else { return; };
    match state.tx.try_send(LogMessage::Line(message.into())) {
        Ok(()) | Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {}
    }
}

pub fn shutdown() {
    let Some(slot) = LOGGER.get() else { return; };
    let mut guard = slot.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(mut state) = guard.take() {
        let _ = state.tx.send(LogMessage::Stop);
        if let Some(thread) = state.thread.take() {
            let _ = thread.join();
        }
    }
}

fn unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or_default()
}

fn rotate_if_needed(path: &Path) {
    let Ok(meta) = fs::metadata(path) else { return; };
    if meta.len() <= MAX_LOG_BYTES { return; }
    let old = path.with_extension("log.old");
    let _ = fs::remove_file(&old);
    let _ = fs::rename(path, old);
}
