use crate::logging;
use std::{fs, path::{Path, PathBuf}, sync::atomic::{AtomicU64, Ordering}, time::{SystemTime, UNIX_EPOCH}};
use windows_sys::Win32::Media::Multimedia::mciSendStringW;

static NEXT_ALIAS: AtomicU64 = AtomicU64::new(1);

static READY_WAV: &[u8] = include_bytes!("../../src/BPSR.ReadyAlert/Assets/ReadyCheck.wav");
static QUEUE_WAV: &[u8] = include_bytes!("../../src/BPSR.ReadyAlert/Assets/Queue.wav");
static INVITE_WAV: &[u8] = include_bytes!("../../src/BPSR.ReadyAlert/Assets/PartyInvite.wav");
static REQUEST_WAV: &[u8] = include_bytes!("../../src/BPSR.ReadyAlert/Assets/PartyRequest.wav");

pub fn play_alert(kind: crate::model::AlertKind, volume: i32) {
    let bytes = match kind {
        crate::model::AlertKind::Queue => QUEUE_WAV,
        crate::model::AlertKind::Ready => READY_WAV,
        crate::model::AlertKind::PartyInvite => INVITE_WAV,
        crate::model::AlertKind::PartyRequest => REQUEST_WAV,
        crate::model::AlertKind::Error => return,
    };
    if let Err(err) = play_bytes(bytes, "wav", volume) {
        logging::write(format!("audio: alert playback failed: {err}"));
    }
}

pub fn play_mp3(bytes: &[u8], volume: i32) -> Result<(), String> {
    play_bytes(bytes, "mp3", volume)
}

pub fn play_file(path: &Path, volume: i32) -> Result<(), String> {
    let extension = path.extension().and_then(|x| x.to_str()).unwrap_or("wav");
    let bytes = fs::read(path).map_err(|e| format!("read custom sound {}: {e}", path.display()))?;
    play_bytes(&bytes, extension, volume)
}

fn play_bytes(bytes: &[u8], extension: &str, volume: i32) -> Result<(), String> {
    if volume <= 0 || bytes.is_empty() { return Ok(()); }
    let id = NEXT_ALIAS.fetch_add(1, Ordering::Relaxed);
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    let path: PathBuf = std::env::temp_dir().join(format!("bpsr-readyalert-{stamp}-{id}.{extension}"));
    fs::write(&path, bytes).map_err(|e| format!("write temp audio: {e}"))?;
    let alias = format!("ra{id}");
    let quoted = path.to_string_lossy().replace('"', "");
    let result = (|| {
        mci(&format!("open \"{quoted}\" alias {alias}"))?;
        let mci_volume = volume.clamp(0, 100) * 10;
        let _ = mci(&format!("setaudio {alias} volume to {mci_volume}"));
        mci(&format!("play {alias} wait"))?;
        Ok(())
    })();
    let _ = mci(&format!("close {alias}"));
    let _ = fs::remove_file(path);
    result
}

fn mci(command: &str) -> Result<(), String> {
    let wide: Vec<u16> = command.encode_utf16().chain(std::iter::once(0)).collect();
    let code = unsafe { mciSendStringW(wide.as_ptr(), std::ptr::null_mut(), 0, std::ptr::null_mut()) };
    if code == 0 { Ok(()) } else { Err(format!("MCI error {code} for {command}")) }
}
