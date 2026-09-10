use std::{
    fs::{self, File, OpenOptions},
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
            let mut output = open_log(&path);
            while let Ok(message) = rx.recv() {
                match message {
                    LogMessage::Line(line) => {
                        let bytes = format_log_line(&line);
                        let needs_rotate = output
                            .as_ref()
                            .is_some_and(|(_, size)| size.saturating_add(bytes.len() as u64) > MAX_LOG_BYTES);
                        if needs_rotate {
                            output.take();
                            rotate_existing(&path);
                        }
                        if output.is_none() {
                            output = open_log(&path);
                        }
                        if let Some((file, size)) = output.as_mut() {
                            match file.write_all(bytes.as_bytes()) {
                                Ok(()) => *size = size.saturating_add(bytes.len() as u64),
                                Err(_) => output = None,
                            }
                        }
                    }
                    LogMessage::Stop => {
                        if let Some((file, _)) = output.as_mut() { let _ = file.flush(); }
                        break;
                    }
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

fn format_log_line(line: &str) -> String {
    format!("{} {}\n", unix_millis(), line.replace('\0', ""))
}

fn open_log(path: &Path) -> Option<(File, u64)> {
    if let Some(parent) = path.parent() { fs::create_dir_all(parent).ok()?; }
    let file = OpenOptions::new().create(true).append(true).open(path).ok()?;
    let size = file.metadata().map(|m| m.len()).unwrap_or(0);
    Some((file, size))
}

fn rotate_existing(path: &Path) {
    if !path.exists() { return; }
    let old = path.with_extension("log.old");
    let _ = fs::remove_file(&old);
    let _ = fs::rename(path, old);
}

fn rotate_if_needed(path: &Path) {
    let Ok(meta) = fs::metadata(path) else { return; };
    if meta.len() <= MAX_LOG_BYTES { return; }
    rotate_existing(path);
}

#[cfg(test)]
mod v1189_logging_tests {
    use super::*;

    #[test]
    fn log_line_removes_nul_and_ends_with_newline() {
        let line = format_log_line("a\0b");
        assert!(!line.contains('\0'));
        assert!(line.ends_with(" a b\n"));
    }
}
