// Time-based persistence for the DPS meter's native History store.
// Included inside the generated overlay module by the final build stage.
use super::*;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Cursor, Read},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const HISTORY_SCHEMA: u32 = 1;
const MAX_DECODED_BYTES: u64 = 16 * 1024 * 1024;
const MAX_COMPRESSED_BYTES: usize = 8 * 1024 * 1024;
const DEFAULT_DAYS: u16 = 7;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(default)]
struct RetentionPolicy {
    days: u16,
}
impl Default for RetentionPolicy {
    fn default() -> Self { Self { days: DEFAULT_DAYS } }
}

fn normalize_days(days: u16) -> u16 {
    match days { 0 | 1 | 3 | 7 => days, _ => DEFAULT_DAYS }
}

fn policy_path(paths: &AppPaths) -> PathBuf {
    paths.root.join("meter-history-retention.json")
}

pub fn load_days(paths: &AppPaths) -> u16 {
    let path = policy_path(paths);
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<RetentionPolicy>(&text).ok())
        .map(|policy| normalize_days(policy.days))
        .unwrap_or(DEFAULT_DAYS)
}

pub fn save_days(paths: &AppPaths, days: u16) -> io::Result<()> {
    let days = normalize_days(days);
    let path = policy_path(paths);
    let pending = path.with_extension("json.new");
    let bytes = serde_json::to_vec_pretty(&RetentionPolicy { days }).map_err(io::Error::other)?;
    fs::write(&pending, bytes)?;
    if path.exists() { fs::remove_file(&path)?; }
    fs::rename(&pending, &path)?;
    prune_expired(&paths.root, days)
}

fn history_dir(root: &Path) -> PathBuf { root.join("History") }

fn history_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else { return Vec::new(); };
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.file_name().and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("encounter-") && name.ends_with(".json.zst")))
        .collect()
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis()
        .try_into().unwrap_or(i64::MAX)
}

fn file_time_ms(path: &Path) -> Option<i64> {
    let name = path.file_name()?.to_str()?;
    let digits = name.strip_prefix("encounter-")?.strip_suffix(".json.zst")?;
    let nanos = digits.parse::<u64>().ok()?;
    i64::try_from(nanos / 1_000_000).ok()
}

fn cutoff_ms(days: u16) -> Option<i64> {
    let days = normalize_days(days);
    if days == 0 { return None; }
    Some(now_ms().saturating_sub(i64::from(days) * 86_400_000))
}

pub fn prune_expired(root: &Path, days: u16) -> io::Result<()> {
    let Some(cutoff) = cutoff_ms(days) else { return Ok(()); };
    let dir = history_dir(root);
    for path in history_files(&dir) {
        if file_time_ms(&path).is_some_and(|ended| ended < cutoff) {
            if let Err(err) = fs::remove_file(&path) {
                crate::logging::write(format!("history retention: prune failed {}: {err}", path.display()));
            }
        }
    }
    Ok(())
}

pub fn archive(root: &Path, snapshot: &DpsSnapshot, days: u16) -> io::Result<Option<HistoryEncounter>> {
    if !history::has_combat_data(snapshot) { return Ok(None); }
    let dir = history_dir(root);
    fs::create_dir_all(&dir)?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let id = u64::try_from(now.as_nanos()).unwrap_or(u64::MAX);
    let ended_unix_ms = i64::try_from(now.as_millis()).unwrap_or(i64::MAX);
    let target_name = snapshot.target.as_ref()
        .map(|target| target.name.trim())
        .filter(|name| !name.is_empty())
        .unwrap_or("Encounter")
        .to_string();
    let record = HistoryEncounter {
        schema: HISTORY_SCHEMA,
        id,
        ended_unix_ms,
        target_name,
        context: crate::encounter_context::snapshot(),
        snapshot: snapshot.clone(),
    };
    let json = serde_json::to_vec(&record).map_err(io::Error::other)?;
    if json.len() as u64 > MAX_DECODED_BYTES {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "history payload too large"));
    }
    let compressed = zstd::stream::encode_all(Cursor::new(json), 3)?;
    if compressed.len() > MAX_COMPRESSED_BYTES {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "history file too large"));
    }
    let final_path = dir.join(format!("encounter-{id:020}.json.zst"));
    let pending = final_path.with_extension("zst.new");
    fs::write(&pending, compressed)?;
    fs::rename(&pending, &final_path)?;
    prune_expired(root, days)?;
    Ok(Some(record))
}

fn load_one(path: &Path) -> io::Result<HistoryEncounter> {
    let compressed = fs::read(path)?;
    if compressed.len() > MAX_COMPRESSED_BYTES {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "history file too large"));
    }
    let decoder = zstd::stream::read::Decoder::new(Cursor::new(compressed))?;
    let mut bytes = Vec::new();
    decoder.take(MAX_DECODED_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_DECODED_BYTES {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "history payload too large"));
    }
    serde_json::from_slice(&bytes).map_err(io::Error::other)
}

pub fn load(root: &Path, days: u16) -> Vec<HistoryEncounter> {
    if let Err(err) = prune_expired(root, days) {
        crate::logging::write(format!("history retention: startup prune failed: {err}"));
    }
    let cutoff = cutoff_ms(days);
    let mut paths = history_files(&history_dir(root));
    paths.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
    let mut out = Vec::with_capacity(paths.len().min(256));
    for path in paths {
        if cutoff.is_some_and(|cutoff| file_time_ms(&path).is_some_and(|ended| ended < cutoff)) {
            continue;
        }
        match load_one(&path) {
            Ok(record) if record.schema == HISTORY_SCHEMA && history::has_combat_data(&record.snapshot) => out.push(record),
            Ok(_) => crate::logging::write(format!("history retention: ignored incompatible/empty {}", path.display())),
            Err(err) => crate::logging::write(format!("history retention: failed to load {}: {err}", path.display())),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn supported_retention_choices_are_stable() {
        assert_eq!(normalize_days(0), 0);
        assert_eq!(normalize_days(1), 1);
        assert_eq!(normalize_days(3), 3);
        assert_eq!(normalize_days(7), 7);
        assert_eq!(normalize_days(99), 7);
    }
}
