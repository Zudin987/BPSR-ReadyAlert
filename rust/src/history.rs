use crate::{logging, model::DpsSnapshot};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Cursor, Read},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const HISTORY_SCHEMA: u32 = 1;
const MAX_DECODED_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryEncounter {
    pub schema: u32,
    pub id: u64,
    pub ended_unix_ms: i64,
    pub target_name: String,
    pub snapshot: DpsSnapshot,
}

pub fn has_combat_data(snapshot: &DpsSnapshot) -> bool {
    snapshot.encounter_ms > 0
        && (snapshot.total_damage > 0
            || snapshot.total_healing > 0
            || snapshot.total_damage_taken > 0)
}

pub fn encounter_rolled(previous: &DpsSnapshot, next: &DpsSnapshot) -> bool {
    if !has_combat_data(previous) {
        return false;
    }
    if !has_combat_data(next) {
        return true;
    }

    // Wipe/new-pull transitions in the compact telemetry core may produce the
    // first hit of the next pull without an intermediate empty snapshot. A
    // decreasing encounter clock or any decreasing cumulative total is therefore
    // authoritative evidence that this is a new segment.
    next.encounter_ms + 250 < previous.encounter_ms
        || next.total_damage < previous.total_damage
        || next.total_healing < previous.total_healing
        || next.total_damage_taken < previous.total_damage_taken
}

pub fn archive(root: &Path, snapshot: &DpsSnapshot, limit: usize) -> io::Result<Option<HistoryEncounter>> {
    if !has_combat_data(snapshot) {
        return Ok(None);
    }

    let dir = history_dir(root);
    fs::create_dir_all(&dir)?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let id = u64::try_from(now.as_nanos()).unwrap_or(u64::MAX);
    let ended_unix_ms = i64::try_from(now.as_millis()).unwrap_or(i64::MAX);
    let target_name = snapshot
        .target
        .as_ref()
        .map(|target| target.name.trim())
        .filter(|name| !name.is_empty())
        .unwrap_or("Encounter")
        .to_string();
    let record = HistoryEncounter {
        schema: HISTORY_SCHEMA,
        id,
        ended_unix_ms,
        target_name,
        snapshot: snapshot.clone(),
    };

    let json = serde_json::to_vec(&record).map_err(io::Error::other)?;
    let compressed = zstd::stream::encode_all(Cursor::new(json), 3)?;
    let final_path = dir.join(format!("encounter-{id:020}.json.zst"));
    let pending = final_path.with_extension("zst.new");
    fs::write(&pending, compressed)?;
    fs::rename(&pending, &final_path)?;
    prune(&dir, limit)?;
    Ok(Some(record))
}

pub fn load_recent(root: &Path, limit: usize) -> Vec<HistoryEncounter> {
    let dir = history_dir(root);
    let mut paths = history_files(&dir);
    paths.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
    paths.truncate(limit.clamp(1, 200));

    let mut out = Vec::with_capacity(paths.len());
    for path in paths {
        match load_one(&path) {
            Ok(record) if record.schema == HISTORY_SCHEMA && has_combat_data(&record.snapshot) => out.push(record),
            Ok(_) => logging::write(format!("history: ignored incompatible/empty {}", path.display())),
            Err(err) => logging::write(format!("history: failed to load {}: {err}", path.display())),
        }
    }
    out
}

fn load_one(path: &Path) -> io::Result<HistoryEncounter> {
    let compressed = fs::read(path)?;
    if compressed.len() > 8 * 1024 * 1024 {
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

fn prune(dir: &Path, limit: usize) -> io::Result<()> {
    let mut paths = history_files(dir);
    paths.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
    for path in paths.into_iter().skip(limit.clamp(1, 200)) {
        if let Err(err) = fs::remove_file(&path) {
            logging::write(format!("history: prune failed {}: {err}", path.display()));
        }
    }
    Ok(())
}

fn history_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else { return Vec::new(); };
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("encounter-") && name.ends_with(".json.zst"))
        })
        .collect()
}

fn history_dir(root: &Path) -> PathBuf {
    root.join("History")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DpsRow, TargetSnapshot};

    fn temp_root(label: &str) -> PathBuf {
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("readyalert-history-{label}-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn sample(ms: u64, damage: i64) -> DpsSnapshot {
        DpsSnapshot {
            encounter_ms: ms,
            total_damage: damage,
            target: Some(TargetSnapshot { entity_uuid: 1, name: "Test Boss".into(), hp: 50, max_hp: 100, enrage_remaining_ms: None }),
            rows: vec![DpsRow { uid: 42, name: "Tester".into(), damage, dps: damage as f64 / (ms as f64 / 1000.0), active_dps: damage as f64, is_local: true, is_party: true, ..Default::default() }],
            ..Default::default()
        }
    }

    #[test]
    fn rollover_detects_empty_and_decreasing_totals() {
        let old = sample(10_000, 1_000_000);
        assert!(encounter_rolled(&old, &DpsSnapshot::default()));
        assert!(encounter_rolled(&old, &sample(500, 10_000)));
        assert!(!encounter_rolled(&old, &sample(11_000, 1_100_000)));
    }

    #[test]
    fn archive_roundtrip_and_prune() {
        let root = temp_root("roundtrip");
        for i in 0..4 {
            archive(&root, &sample(1_000 + i, 100 + i as i64), 3).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        let records = load_recent(&root, 10);
        assert_eq!(records.len(), 3);
        assert_eq!(records[0].target_name, "Test Boss");
        assert_eq!(records[0].snapshot.rows[0].uid, 42);
        fs::remove_dir_all(root).unwrap();
    }
}
