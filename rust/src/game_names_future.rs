#![allow(dead_code)]

use std::{
    collections::{HashMap, HashSet},
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

#[path = "game_names_v1300.rs"]
mod builtin;

const OVERRIDE_FILE: &str = "game_names_override.tsv";
const UNKNOWN_FILE: &str = "unknown_ids.log";
const VALID_KINDS: &[u8] = b"SBEDMTFGPCXAOR";

#[derive(Default)]
struct OverrideCatalog {
    names: HashMap<(u8, i32), &'static str>,
}

static OVERRIDES: OnceLock<OverrideCatalog> = OnceLock::new();

fn runtime_file(name: &str) -> Option<PathBuf> {
    crate::paths::AppPaths::create().ok().map(|paths| paths.root.join(name))
}

fn parse_kind(raw: &str) -> Option<u8> {
    let bytes = raw.trim().as_bytes();
    (bytes.len() == 1 && VALID_KINDS.contains(&bytes[0])).then_some(bytes[0])
}

fn parse_id(kind: u8, raw: &str) -> Option<i32> {
    if let Ok(id) = raw.trim().parse::<i32>() {
        return Some(id);
    }
    // Skill ids can be unsigned/composite on disk while telemetry stores the
    // low 32 bits in i32. Match the embedded catalog's normalization exactly.
    (kind == b'S')
        .then(|| raw.trim().parse::<u64>().ok().map(|id| id as i32))
        .flatten()
}

fn parse_override_text(text: &str) -> OverrideCatalog {
    let mut out = OverrideCatalog::default();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim_end_matches('\r');
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let mut fields = line.splitn(3, '\t');
        let Some(kind) = fields.next().and_then(parse_kind) else {
            crate::logging::write(format!("game-name override: ignored line {} with invalid kind", index + 1));
            continue;
        };
        let Some(id) = fields.next().and_then(|raw| parse_id(kind, raw)) else {
            crate::logging::write(format!("game-name override: ignored line {} with invalid id", index + 1));
            continue;
        };
        let Some(name) = fields.next().map(str::trim).filter(|name| !name.is_empty()) else {
            crate::logging::write(format!("game-name override: ignored line {} with empty name", index + 1));
            continue;
        };
        let leaked: &'static str = Box::leak(name.to_owned().into_boxed_str());
        out.names.insert((kind, id), leaked);
    }
    out
}

#[cfg(not(test))]
fn load_override_catalog() -> OverrideCatalog {
    let Some(path) = runtime_file(OVERRIDE_FILE) else { return OverrideCatalog::default(); };
    ensure_override_template(&path);
    match fs::read_to_string(&path) {
        Ok(text) => {
            let catalog = parse_override_text(&text);
            if !catalog.names.is_empty() {
                crate::logging::write(format!(
                    "game-name override: loaded {} mapping(s) from {}",
                    catalog.names.len(),
                    path.display()
                ));
            }
            catalog
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => OverrideCatalog::default(),
        Err(err) => {
            crate::logging::write(format!("game-name override: failed to read {}: {err}", path.display()));
            OverrideCatalog::default()
        }
    }
}

#[cfg(test)]
fn load_override_catalog() -> OverrideCatalog {
    OverrideCatalog::default()
}

#[cfg(not(test))]
fn ensure_override_template(path: &PathBuf) {
    let template = concat!(
        "# BPSR ReadyAlert runtime game-name overrides.\n",
        "# Loaded once at startup. Restart ReadyAlert after editing.\n",
        "# Format: KIND<TAB>ID<TAB>NAME\n",
        "# Kinds: S skill, B buff, E scene, D dungeon, M monster, T talent,\n",
        "#        F factor, G factor-grade item, P spec, C class, X modifier,\n",
        "#        A attribute, O objective, R recount row.\n",
        "# Runtime overrides take priority over embedded/global/Season 4 names.\n",
        "# Example: S<TAB>123456<TAB>Season 4 Skill Name\n",
    );
    match OpenOptions::new().create_new(true).write(true).open(path) {
        Ok(mut file) => {
            let _ = file.write_all(template.as_bytes());
        }
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(err) => crate::logging::write(format!(
            "game-name override: could not create template {}: {err}",
            path.display()
        )),
    }
}

fn overrides() -> &'static OverrideCatalog {
    OVERRIDES.get_or_init(load_override_catalog)
}

fn override_name(kind: u8, id: i32) -> Option<&'static str> {
    overrides().names.get(&(kind, id)).copied()
}

struct UnknownState {
    path: Option<PathBuf>,
    seen: HashSet<(u8, i32)>,
}

static UNKNOWN: OnceLock<Mutex<UnknownState>> = OnceLock::new();

#[cfg(not(test))]
fn load_unknown_state() -> UnknownState {
    let path = runtime_file(UNKNOWN_FILE);
    let mut seen = HashSet::new();
    if let Some(path) = path.as_ref() {
        if let Ok(text) = fs::read_to_string(path) {
            for line in text.lines() {
                let mut fields = line.split('\t');
                let _timestamp = fields.next();
                let kind = fields.next().and_then(parse_kind);
                let id = fields.next().and_then(|raw| kind.and_then(|k| parse_id(k, raw)));
                if let (Some(kind), Some(id)) = (kind, id) {
                    seen.insert((kind, id));
                }
            }
        }
    }
    UnknownState { path, seen }
}

#[cfg(test)]
fn load_unknown_state() -> UnknownState {
    UnknownState { path: None, seen: HashSet::new() }
}

fn unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn note_unknown(kind: u8, id: i32) {
    if id == 0 {
        return;
    }
    let slot = UNKNOWN.get_or_init(|| Mutex::new(load_unknown_state()));
    let Ok(mut state) = slot.lock() else { return; };
    if !state.seen.insert((kind, id)) {
        return;
    }
    let Some(path) = state.path.as_ref() else { return; };
    let line = format!("{}\t{}\t{}\n", unix_millis(), kind as char, id);
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = file.write_all(line.as_bytes());
    }
}

fn resolve(
    kind: u8,
    id: i32,
    builtin_lookup: impl FnOnce(i32) -> Option<&'static str>,
) -> Option<&'static str> {
    if let Some(name) = override_name(kind, id) {
        return Some(name);
    }
    let name = builtin_lookup(id);
    if name.is_none() {
        note_unknown(kind, id);
    }
    name
}

pub fn override_skill_name(id: i32) -> Option<&'static str> { override_name(b'S', id) }
pub fn override_buff_name(id: i32) -> Option<&'static str> { override_name(b'B', id) }
pub fn override_scene_name(id: i32) -> Option<&'static str> { override_name(b'E', id) }
pub fn override_dungeon_name(id: i32) -> Option<&'static str> { override_name(b'D', id) }
pub fn override_monster_name(id: i32) -> Option<&'static str> { override_name(b'M', id) }
pub fn override_talent_name(id: i32) -> Option<&'static str> { override_name(b'T', id) }
pub fn override_factor_name(id: i32) -> Option<&'static str> { override_name(b'F', id) }
pub fn override_factor_grade_item_name(id: i32) -> Option<&'static str> { override_name(b'G', id) }
pub fn override_spec_name(id: i32) -> Option<&'static str> { override_name(b'P', id) }
pub fn override_class_name(id: i32) -> Option<&'static str> { override_name(b'C', id) }
pub fn override_modifier_effect_name(id: i32) -> Option<&'static str> { override_name(b'X', id) }
pub fn override_attribute_name(id: i32) -> Option<&'static str> { override_name(b'A', id) }
pub fn override_objective_name(id: i32) -> Option<&'static str> { override_name(b'O', id) }
pub fn override_recount_name(id: i32) -> Option<&'static str> { override_name(b'R', id) }

pub fn skill_name(id: i32) -> Option<&'static str> { resolve(b'S', id, builtin::skill_name) }
pub fn buff_name(id: i32) -> Option<&'static str> { resolve(b'B', id, builtin::buff_name) }
pub fn scene_name(id: i32) -> Option<&'static str> { resolve(b'E', id, builtin::scene_name) }
pub fn dungeon_name(id: i32) -> Option<&'static str> { resolve(b'D', id, builtin::dungeon_name) }
pub fn monster_name(id: i32) -> Option<&'static str> { resolve(b'M', id, builtin::monster_name) }
pub fn talent_name(id: i32) -> Option<&'static str> { resolve(b'T', id, builtin::talent_name) }
pub fn factor_name(id: i32) -> Option<&'static str> { resolve(b'F', id, builtin::factor_name) }
pub fn factor_grade_item_name(id: i32) -> Option<&'static str> { resolve(b'G', id, builtin::factor_grade_item_name) }
pub fn spec_name(id: i32) -> Option<&'static str> { resolve(b'P', id, builtin::spec_name) }
pub fn class_name(id: i32) -> Option<&'static str> { resolve(b'C', id, builtin::class_name) }
pub fn modifier_effect_name(id: i32) -> Option<&'static str> { resolve(b'X', id, builtin::modifier_effect_name) }
pub fn attribute_name(id: i32) -> Option<&'static str> { resolve(b'A', id, builtin::attribute_name) }
pub fn objective_name(id: i32) -> Option<&'static str> { resolve(b'O', id, builtin::objective_name) }
pub fn recount_name(id: i32) -> Option<&'static str> { resolve(b'R', id, builtin::recount_name) }

/// Used by the Season 4 probe for protocol categories that are intentionally
/// not displayed. This records the numeric id without inventing a name.
pub fn note_unknown_protocol_id(kind: u8, id: i32) {
    if VALID_KINDS.contains(&kind) {
        note_unknown(kind, id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn override_parser_accepts_all_runtime_categories_and_last_value_wins() {
        let parsed = parse_override_text("S\t42\tFirst\nM\t7\tBoss\nS\t42\tSecond\n");
        assert_eq!(parsed.names.get(&(b'S', 42)).copied(), Some("Second"));
        assert_eq!(parsed.names.get(&(b'M', 7)).copied(), Some("Boss"));
    }

    #[test]
    fn override_parser_uses_same_unsigned_skill_normalization_as_embedded_catalog() {
        let raw = (i32::MAX as u64) + 5;
        let text = format!("S\t{raw}\tWide Skill\n");
        let parsed = parse_override_text(&text);
        assert_eq!(parsed.names.get(&(b'S', raw as i32)).copied(), Some("Wide Skill"));
    }

    #[test]
    fn malformed_override_rows_are_ignored_without_hiding_good_rows() {
        let parsed = parse_override_text("# comment\nQ\t1\tBad kind\nS\tnope\tBad id\nS\t2\t\nE\t6615\tDesolate Court Override\n");
        assert_eq!(parsed.names.len(), 1);
        assert_eq!(parsed.names.get(&(b'E', 6615)).copied(), Some("Desolate Court Override"));
    }
}
