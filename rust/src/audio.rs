use crate::logging;
use std::{fs, path::{Path, PathBuf}, sync::atomic::{AtomicU64, Ordering}, time::{SystemTime, UNIX_EPOCH}};
use windows_sys::Win32::Media::Multimedia::mciSendStringW;

static NEXT_ALIAS: AtomicU64 = AtomicU64::new(1);

static READY_WAV: &[u8] = include_bytes!("../../src/BPSR.ReadyAlert/Assets/ReadyCheck.wav");
static QUEUE_WAV: &[u8] = include_bytes!("../../src/BPSR.ReadyAlert/Assets/Queue.wav");
static INVITE_WAV: &[u8] = include_bytes!("../../src/BPSR.ReadyAlert/Assets/PartyInvite.wav");
static REQUEST_WAV: &[u8] = include_bytes!("../../src/BPSR.ReadyAlert/Assets/PartyRequest.wav");
static CHAT_FALLBACK_WAV: &[u8] = include_bytes!("../../src/BPSR.ReadyAlert/Assets/LetsDoThis.wav");

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

/// Custom file playback is currently used by keyword/private chat notifications.
/// Match v1.3.6's resilient policy: a missing, unreadable or MCI-incompatible custom
/// file falls back to the bundled LetsDoThis sound instead of losing the alert.
pub fn play_file(path: &Path, volume: i32) -> Result<(), String> {
    if volume <= 0 {
        return Ok(());
    }
    let custom = (|| {
        let extension = path.extension().and_then(|x| x.to_str()).unwrap_or("wav");
        let bytes = fs::read(path).map_err(|e| format!("read custom sound {}: {e}", path.display()))?;
        play_bytes(&bytes, extension, volume)
    })();
    match custom {
        Ok(()) => Ok(()),
        Err(err) => {
            logging::write(format!(
                "audio: custom chat sound failed {}; using bundled fallback: {err}",
                path.display()
            ));
            play_bytes(CHAT_FALLBACK_WAV, "wav", volume)
        }
    }
}

fn play_bytes(bytes: &[u8], extension: &str, volume: i32) -> Result<(), String> {
    let volume = volume.clamp(0, 100);
    if volume == 0 || bytes.is_empty() { return Ok(()); }

    let extension = extension.trim_start_matches('.').to_ascii_lowercase();
    let mut scaled_wav = None;
    let mut needs_mci_volume = volume < 100;
    if extension == "wav" && volume < 100 {
        let mut scaled = bytes.to_vec();
        match scale_wav_pcm_in_place(&mut scaled, volume) {
            Ok(()) => {
                scaled_wav = Some(scaled);
                needs_mci_volume = false;
            }
            Err(err) => logging::write(format!("audio: WAV software volume unavailable; trying MCI volume: {err}")),
        }
    } else if extension == "wav" {
        needs_mci_volume = false;
    }
    let payload = scaled_wav.as_deref().unwrap_or(bytes);

    let id = NEXT_ALIAS.fetch_add(1, Ordering::Relaxed);
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    let path: PathBuf = std::env::temp_dir().join(format!("bpsr-readyalert-{stamp}-{id}.{extension}"));
    fs::write(&path, payload).map_err(|e| format!("write temp audio: {e}"))?;
    let alias = format!("ra{id}");
    let quoted = path.to_string_lossy().replace('"', "");
    let result = (|| {
        mci(&format!("open \"{quoted}\" alias {alias}"))?;
        if needs_mci_volume {
            let mci_volume = volume * 10;
            mci(&format!("setaudio {alias} volume to {mci_volume}"))?;
        }
        mci(&format!("play {alias} wait"))?;
        Ok(())
    })();
    let _ = mci(&format!("close {alias}"));
    let _ = fs::remove_file(path);
    result
}

/// Apply per-alert volume directly to ordinary PCM/IEEE-float WAV samples. This
/// avoids relying on MCI `setaudio`, which is not consistently implemented by
/// the wave-audio device. The RIFF container and sample format are preserved.
fn scale_wav_pcm_in_place(bytes: &mut [u8], volume: i32) -> Result<(), String> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("not a RIFF/WAVE file".into());
    }
    let volume = volume.clamp(0, 100);
    if volume == 100 { return Ok(()); }

    let mut format_tag = None;
    let mut bits_per_sample = None;
    let mut data_range = None;
    let mut cursor = 12usize;
    while cursor.saturating_add(8) <= bytes.len() {
        let size = u32::from_le_bytes(bytes[cursor + 4..cursor + 8].try_into().unwrap()) as usize;
        let start = cursor + 8;
        let end = start.checked_add(size).ok_or("WAV chunk size overflow")?;
        if end > bytes.len() { return Err("truncated WAV chunk".into()); }
        match &bytes[cursor..cursor + 4] {
            b"fmt " if size >= 16 => {
                let mut tag = u16::from_le_bytes([bytes[start], bytes[start + 1]]);
                let bits = u16::from_le_bytes([bytes[start + 14], bytes[start + 15]]);
                if tag == 0xfffe && size >= 40 {
                    // WAVE_FORMAT_EXTENSIBLE stores the base format tag in the
                    // first two bytes of the sub-format GUID.
                    tag = u16::from_le_bytes([bytes[start + 24], bytes[start + 25]]);
                }
                format_tag = Some(tag);
                bits_per_sample = Some(bits);
            }
            b"data" => data_range = Some((start, end)),
            _ => {}
        }
        cursor = end.saturating_add(size & 1);
    }

    let tag = format_tag.ok_or("WAV fmt chunk missing")?;
    let bits = bits_per_sample.ok_or("WAV sample size missing")?;
    let (start, end) = data_range.ok_or("WAV data chunk missing")?;
    let data = &mut bytes[start..end];

    match (tag, bits) {
        (1, 8) => {
            for sample in data {
                let centered = i32::from(*sample) - 128;
                *sample = (128 + centered * volume / 100).clamp(0, 255) as u8;
            }
        }
        (1, 16) => {
            for sample in data.chunks_exact_mut(2) {
                let value = i16::from_le_bytes([sample[0], sample[1]]) as i32;
                sample.copy_from_slice(&((value * volume / 100) as i16).to_le_bytes());
            }
        }
        (1, 24) => {
            for sample in data.chunks_exact_mut(3) {
                let raw = i32::from(sample[0]) | (i32::from(sample[1]) << 8) | (i32::from(sample[2]) << 16);
                let value = if raw & 0x0080_0000 != 0 { raw | !0x00ff_ffff } else { raw };
                let scaled = ((i64::from(value) * i64::from(volume)) / 100).clamp(-8_388_608, 8_388_607) as i32;
                sample[0] = scaled as u8;
                sample[1] = (scaled >> 8) as u8;
                sample[2] = (scaled >> 16) as u8;
            }
        }
        (1, 32) => {
            for sample in data.chunks_exact_mut(4) {
                let value = i32::from_le_bytes(sample.try_into().unwrap()) as i64;
                let scaled = (value * i64::from(volume) / 100).clamp(i32::MIN as i64, i32::MAX as i64) as i32;
                sample.copy_from_slice(&scaled.to_le_bytes());
            }
        }
        (3, 32) => {
            let factor = volume as f32 / 100.0;
            for sample in data.chunks_exact_mut(4) {
                let value = f32::from_le_bytes(sample.try_into().unwrap());
                sample.copy_from_slice(&(value * factor).to_le_bytes());
            }
        }
        _ => return Err(format!("unsupported WAV format tag={tag} bits={bits}")),
    }
    Ok(())
}

fn mci(command: &str) -> Result<(), String> {
    let wide: Vec<u16> = command.encode_utf16().chain(std::iter::once(0)).collect();
    let code = unsafe { mciSendStringW(wide.as_ptr(), std::ptr::null_mut(), 0, std::ptr::null_mut()) };
    if code == 0 { Ok(()) } else { Err(format!("MCI error {code} for {command}")) }
}

#[cfg(test)]
mod v1189_audio_tests {
    use super::*;

    fn pcm16_wav(samples: &[i16]) -> Vec<u8> {
        let data_len = (samples.len() * 2) as u32;
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + data_len).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&44_100u32.to_le_bytes());
        out.extend_from_slice(&(44_100u32 * 2).to_le_bytes());
        out.extend_from_slice(&2u16.to_le_bytes());
        out.extend_from_slice(&16u16.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&data_len.to_le_bytes());
        for sample in samples { out.extend_from_slice(&sample.to_le_bytes()); }
        out
    }

    #[test]
    fn pcm16_wav_volume_scales_samples() {
        let mut wav = pcm16_wav(&[10_000, -10_000, 0]);
        scale_wav_pcm_in_place(&mut wav, 50).unwrap();
        let data = &wav[44..];
        assert_eq!(i16::from_le_bytes([data[0], data[1]]), 5_000);
        assert_eq!(i16::from_le_bytes([data[2], data[3]]), -5_000);
        assert_eq!(i16::from_le_bytes([data[4], data[5]]), 0);
    }

    #[test]
    fn invalid_wav_is_rejected_instead_of_corrupted() {
        let mut data = b"not a wav".to_vec();
        assert!(scale_wav_pcm_in_place(&mut data, 50).is_err());
    }
}
