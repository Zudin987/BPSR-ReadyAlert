use crate::{
    history::HistoryEncounter,
    model::{DpsRow, DpsSnapshot},
};
use serde::Serialize;
use std::{
    fs, io,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const MIN_PB_ENCOUNTER_MS: u64 = 10_000;
const EXPORT_SCHEMA: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ViewMode {
    Damage,
    Heal,
    Tank,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExportFormat {
    Csv,
    Json,
}

#[derive(Clone, Copy, Debug)]
pub struct PersonalBest {
    pub current_rate: f64,
    pub previous_best_rate: f64,
    pub delta_percent: f64,
    pub samples: usize,
    pub is_new_record: bool,
}

impl ViewMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Damage => "Damage",
            Self::Heal => "Healing",
            Self::Tank => "Taken",
        }
    }

    fn rate_label(self) -> &'static str {
        match self {
            Self::Damage => "DPS",
            Self::Heal => "HPS",
            Self::Tank => "DTPS",
        }
    }
}

impl ExportFormat {
    fn extension(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Json => "json",
        }
    }
}

/// Compare the local player's current encounter rate against matching saved
/// encounters. Tank is intentionally excluded: a higher DTPS is not a "best".
pub fn personal_best(
    records: &[HistoryEncounter],
    current: &DpsSnapshot,
    exclude_history_id: Option<u64>,
    mode: ViewMode,
) -> Option<PersonalBest> {
    if mode == ViewMode::Tank || current.encounter_ms < MIN_PB_ENCOUNTER_MS {
        return None;
    }

    let target = normalized_target(current)?;
    let current_row = current.rows.iter().find(|row| row.is_local)?;
    let current_rate = row_rate(current_row, current.encounter_ms, mode);
    if !current_rate.is_finite() || current_rate <= 0.0 {
        return None;
    }

    let mut previous_best_rate = 0.0_f64;
    let mut samples = 0usize;
    for record in records {
        if exclude_history_id.is_some_and(|id| id == record.id)
            || record.snapshot.encounter_ms < MIN_PB_ENCOUNTER_MS
            || normalized_target(&record.snapshot).as_deref() != Some(target.as_str())
        {
            continue;
        }
        let Some(row) = record.snapshot.rows.iter().find(|row| row.is_local) else {
            continue;
        };
        if !same_spec(current_row, row) {
            continue;
        }
        let rate = row_rate(row, record.snapshot.encounter_ms, mode);
        if rate.is_finite() && rate > 0.0 {
            previous_best_rate = previous_best_rate.max(rate);
            samples = samples.saturating_add(1);
        }
    }

    if samples == 0 || previous_best_rate <= 0.0 {
        return None;
    }
    let delta_percent = (current_rate - previous_best_rate) * 100.0 / previous_best_rate;
    Some(PersonalBest {
        current_rate,
        previous_best_rate,
        delta_percent,
        samples,
        is_new_record: current_rate > previous_best_rate,
    })
}

/// Compact, chat-friendly summary of the currently viewed meter tab.
pub fn encounter_summary(snapshot: &DpsSnapshot, mode: ViewMode) -> String {
    let target = snapshot
        .target
        .as_ref()
        .map(|target| target.name.trim())
        .filter(|name| !name.is_empty())
        .unwrap_or("Encounter");
    let mut rows: Vec<&DpsRow> = snapshot
        .rows
        .iter()
        .filter(|row| row_metric(row, mode) > 0)
        .collect();
    rows.sort_by(|a, b| row_metric(b, mode).cmp(&row_metric(a, mode)));
    let total: i64 = rows.iter().map(|row| row_metric(row, mode)).sum();

    let mut out = format!(
        "BPSR ReadyAlert v{} | {} | {} | {}\n",
        env!("CARGO_PKG_VERSION"),
        target,
        format_duration(snapshot.encounter_ms),
        mode.label(),
    );
    if rows.is_empty() {
        out.push_str("No contribution data recorded.\n");
        return out;
    }

    for (index, row) in rows.into_iter().take(10).enumerate() {
        let metric = row_metric(row, mode);
        let share = if total > 0 {
            metric as f64 * 100.0 / total as f64
        } else {
            0.0
        };
        let death = if row.deaths > 0 {
            format!(" | D{}", row.deaths)
        } else {
            String::new()
        };
        out.push_str(&format!(
            "{}. {} - {} {} | {} | {:.1}%{}\n",
            index + 1,
            row_identity(row),
            compact(row_rate(row, snapshot.encounter_ms, mode)),
            mode.rate_label(),
            compact(metric as f64),
            share,
            death,
        ));
    }
    out
}

pub fn export_encounter(
    root: &Path,
    snapshot: &DpsSnapshot,
    format: ExportFormat,
) -> io::Result<PathBuf> {
    let dir = root.join("Exports");
    fs::create_dir_all(&dir)?;
    let target = snapshot
        .target
        .as_ref()
        .map(|target| target.name.as_str())
        .unwrap_or("Encounter");
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let base = format!("ReadyAlert-{}-{}", sanitize_filename(target), now.as_millis());
    let path = unique_export_path(&dir, &base, format.extension());
    let bytes = match format {
        ExportFormat::Csv => encounter_csv(snapshot).into_bytes(),
        ExportFormat::Json => encounter_json(snapshot)?.into_bytes(),
    };
    fs::write(&path, bytes)?;
    Ok(path)
}

pub fn encounter_csv(snapshot: &DpsSnapshot) -> String {
    let target = snapshot
        .target
        .as_ref()
        .map(|target| target.name.as_str())
        .unwrap_or("Encounter");
    let seconds = snapshot.encounter_ms as f64 / 1000.0;
    let mut out = String::from("\u{feff}target,encounter_seconds,name,uid,spec,is_local,is_party,damage,dps,active_dps,boss_damage,healing,hps,active_hps,effective_healing,overhealing,damage_taken,dtps,active_dtps,absorbed_damage,deaths\r\n");
    for row in &snapshot.rows {
        let fields = [
            csv_escape(target),
            format!("{seconds:.3}"),
            csv_escape(&row.name),
            row.uid.to_string(),
            csv_escape(&row.subprofession_name),
            row.is_local.to_string(),
            row.is_party.to_string(),
            row.damage.to_string(),
            format!("{:.3}", row_rate(row, snapshot.encounter_ms, ViewMode::Damage)),
            format!("{:.3}", row.active_dps),
            row.boss_damage.to_string(),
            row.healing.to_string(),
            format!("{:.3}", row_rate(row, snapshot.encounter_ms, ViewMode::Heal)),
            format!("{:.3}", row.active_hps),
            row.effective_healing.to_string(),
            row.overhealing.to_string(),
            row.damage_taken.to_string(),
            format!("{:.3}", row_rate(row, snapshot.encounter_ms, ViewMode::Tank)),
            format!("{:.3}", row.active_dtps),
            row.absorbed_damage.to_string(),
            row.deaths.to_string(),
        ];
        out.push_str(&fields.join(","));
        out.push_str("\r\n");
    }
    out
}

#[derive(Serialize)]
struct JsonEncounterExport<'a> {
    format: &'static str,
    schema: u32,
    readyalert_version: &'static str,
    exported_unix_ms: i64,
    snapshot: &'a DpsSnapshot,
}

pub fn encounter_json(snapshot: &DpsSnapshot) -> io::Result<String> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let exported_unix_ms = i64::try_from(now.as_millis()).unwrap_or(i64::MAX);
    serde_json::to_string_pretty(&JsonEncounterExport {
        format: "BPSR ReadyAlert Encounter Export",
        schema: EXPORT_SCHEMA,
        readyalert_version: env!("CARGO_PKG_VERSION"),
        exported_unix_ms,
        snapshot,
    })
    .map_err(io::Error::other)
}

#[cfg(windows)]
mod clipboard {
    use std::{
        ffi::c_void,
        ptr::{copy_nonoverlapping, null_mut},
        thread,
        time::Duration,
    };

    const CF_UNICODETEXT: u32 = 13;
    const GMEM_MOVEABLE: u32 = 0x0002;

    #[link(name = "user32")]
    extern "system" {
        fn OpenClipboard(owner: *mut c_void) -> i32;
        fn EmptyClipboard() -> i32;
        fn SetClipboardData(format: u32, memory: *mut c_void) -> *mut c_void;
        fn CloseClipboard() -> i32;
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GlobalAlloc(flags: u32, bytes: usize) -> *mut c_void;
        fn GlobalLock(memory: *mut c_void) -> *mut c_void;
        fn GlobalUnlock(memory: *mut c_void) -> i32;
        fn GlobalFree(memory: *mut c_void) -> *mut c_void;
    }

    struct ClipboardGuard;
    impl Drop for ClipboardGuard {
        fn drop(&mut self) {
            unsafe { CloseClipboard(); }
        }
    }

    pub fn copy_text(text: &str) -> Result<(), String> {
        // Clipboard ownership is occasionally held for a few milliseconds by
        // another desktop app. A short bounded retry avoids making Copy flaky.
        let mut opened = false;
        for _ in 0..5 {
            if unsafe { OpenClipboard(null_mut()) } != 0 {
                opened = true;
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        if !opened {
            return Err("OpenClipboard failed".into());
        }
        let _guard = ClipboardGuard;

        unsafe {
            if EmptyClipboard() == 0 {
                return Err("EmptyClipboard failed".into());
            }
            let mut wide: Vec<u16> = text.encode_utf16().collect();
            wide.push(0);
            let bytes = wide.len().saturating_mul(std::mem::size_of::<u16>());
            let memory = GlobalAlloc(GMEM_MOVEABLE, bytes);
            if memory.is_null() {
                return Err("GlobalAlloc failed".into());
            }
            let locked = GlobalLock(memory);
            if locked.is_null() {
                GlobalFree(memory);
                return Err("GlobalLock failed".into());
            }
            copy_nonoverlapping(wide.as_ptr().cast::<u8>(), locked.cast::<u8>(), bytes);
            let _ = GlobalUnlock(memory);
            if SetClipboardData(CF_UNICODETEXT, memory).is_null() {
                GlobalFree(memory);
                return Err("SetClipboardData failed".into());
            }
            // Ownership transfers to the clipboard after SetClipboardData.
            Ok(())
        }
    }
}

#[cfg(windows)]
pub use clipboard::copy_text;

#[cfg(not(windows))]
pub fn copy_text(_text: &str) -> Result<(), String> {
    Err("Clipboard sharing is only supported on Windows".into())
}

fn row_metric(row: &DpsRow, mode: ViewMode) -> i64 {
    match mode {
        ViewMode::Damage => row.damage,
        ViewMode::Heal => row.healing,
        ViewMode::Tank => row.damage_taken,
    }
    .max(0)
}

fn row_rate(row: &DpsRow, encounter_ms: u64, mode: ViewMode) -> f64 {
    let value = row_metric(row, mode);
    if value <= 0 {
        return 0.0;
    }
    let seconds = (encounter_ms.max(1) as f64 / 1000.0).max(0.001);
    value as f64 / seconds
}

fn same_spec(a: &DpsRow, b: &DpsRow) -> bool {
    a.profession_id == b.profession_id
        && (a.subprofession_id <= 0
            || b.subprofession_id <= 0
            || a.subprofession_id == b.subprofession_id)
}

fn normalized_target(snapshot: &DpsSnapshot) -> Option<String> {
    let name = snapshot.target.as_ref()?.name.trim();
    if name.is_empty() {
        return None;
    }
    Some(
        name.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase(),
    )
}

fn row_identity(row: &DpsRow) -> String {
    let spec = row.subprofession_name.trim();
    if spec.is_empty() {
        row.name.clone()
    } else {
        format!("{}-{}", row.name, spec)
    }
}

fn compact(value: f64) -> String {
    let abs = value.abs();
    if abs >= 1_000_000_000.0 {
        format!("{:.2}B", value / 1_000_000_000.0)
    } else if abs >= 1_000_000.0 {
        format!("{:.2}M", value / 1_000_000.0)
    } else if abs >= 1_000.0 {
        format!("{:.1}K", value / 1_000.0)
    } else {
        format!("{value:.0}")
    }
}

fn format_duration(ms: u64) -> String {
    let seconds = ms / 1000;
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

fn csv_escape(value: &str) -> String {
    if value
        .chars()
        .any(|ch| matches!(ch, ',' | '"' | '\r' | '\n'))
    {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn sanitize_filename(value: &str) -> String {
    let mut out = String::with_capacity(value.len().min(48));
    let mut last_sep = false;
    let mut count = 0usize;
    for ch in value.chars() {
        if count >= 48 {
            break;
        }
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            count += 1;
            last_sep = false;
        } else if (ch.is_whitespace() || matches!(ch, '-' | '_'))
            && !out.is_empty()
            && !last_sep
        {
            out.push('_');
            count += 1;
            last_sep = true;
        }
    }
    while out.ends_with('_') {
        out.pop();
    }
    if out.is_empty() {
        "Encounter".into()
    } else {
        out
    }
}

fn unique_export_path(dir: &Path, base: &str, extension: &str) -> PathBuf {
    let first = dir.join(format!("{base}.{extension}"));
    if !first.exists() {
        return first;
    }
    for suffix in 2..=999 {
        let candidate = dir.join(format!("{base}-{suffix}.{extension}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    dir.join(format!("{base}-overflow.{extension}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DpsRow, TargetSnapshot};

    fn snapshot(target: &str, ms: u64, damage: i64, profession: i32, spec: i32) -> DpsSnapshot {
        DpsSnapshot {
            encounter_ms: ms,
            total_damage: damage,
            target: Some(TargetSnapshot {
                entity_uuid: 1,
                name: target.into(),
                hp: 1,
                max_hp: 1,
                enrage_remaining_ms: None,
            }),
            rows: vec![DpsRow {
                uid: 42,
                name: "Tester".into(),
                profession_id: profession,
                subprofession_id: spec,
                subprofession_name: "Smite".into(),
                damage,
                is_local: true,
                is_party: true,
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    fn record(id: u64, snap: DpsSnapshot) -> HistoryEncounter {
        HistoryEncounter {
            schema: 1,
            id,
            ended_unix_ms: id as i64,
            target_name: "Dummy".into(),
            snapshot: snap,
        }
    }

    #[test]
    fn pb_matches_target_and_spec_and_excludes_selected_record() {
        let records = vec![
            record(1, snapshot("Training Dummy", 20_000, 2_000_000, 4, 41)),
            record(2, snapshot("Training Dummy", 20_000, 2_400_000, 4, 41)),
            record(3, snapshot("Other Boss", 20_000, 8_000_000, 4, 41)),
            record(4, snapshot("Training Dummy", 20_000, 9_000_000, 5, 51)),
        ];
        let current = snapshot(" Training   Dummy ", 20_000, 2_600_000, 4, 41);
        let pb = personal_best(&records, &current, Some(2), ViewMode::Damage).unwrap();
        assert_eq!(pb.samples, 1);
        assert!((pb.previous_best_rate - 100_000.0).abs() < 0.1);
        assert!((pb.current_rate - 130_000.0).abs() < 0.1);
        assert!(pb.is_new_record);
    }

    #[test]
    fn pb_rejects_short_or_tank_runs() {
        let records = vec![record(1, snapshot("Dummy", 20_000, 2_000_000, 4, 41))];
        assert!(personal_best(
            &records,
            &snapshot("Dummy", 5_000, 1_000_000, 4, 41),
            None,
            ViewMode::Damage
        )
        .is_none());
        assert!(personal_best(
            &records,
            &snapshot("Dummy", 20_000, 3_000_000, 4, 41),
            None,
            ViewMode::Tank
        )
        .is_none());
    }

    #[test]
    fn csv_quotes_names_and_has_bom() {
        let mut snap = snapshot("Boss, Prime", 10_000, 1000, 4, 41);
        snap.rows[0].name = "A \"Name\"".into();
        let csv = encounter_csv(&snap);
        assert!(csv.starts_with('\u{feff}'));
        assert!(csv.contains("\"Boss, Prime\""));
        assert!(csv.contains("\"A \"\"Name\"\"\""));
    }

    #[test]
    fn filename_sanitizer_is_safe_and_bounded() {
        assert_eq!(sanitize_filename("  Boss: Prime / S4  "), "Boss_Prime_S4");
        assert_eq!(sanitize_filename("***"), "Encounter");
        assert!(sanitize_filename(&"A".repeat(100)).len() <= 48);
    }
}
