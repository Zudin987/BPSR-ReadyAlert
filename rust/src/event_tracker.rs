use crate::logging;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs, io,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_RULES: usize = 24;
const MAX_LABEL_CHARS: usize = 40;
const DEFAULT_HOLD_SECONDS: u8 = 6;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackerKind {
    Buff,
    Skill,
}
impl TrackerKind {
    pub const fn label(self) -> &'static str {
        match self { Self::Buff => "Buff ID", Self::Skill => "Skill ID" }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackerScope {
    Any,
    SelfOnly,
    Party,
}
impl TrackerScope {
    pub const fn label(self) -> &'static str {
        match self { Self::Any => "Any", Self::SelfOnly => "Self", Self::Party => "Party" }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TrackerRule {
    pub rule_id: u32,
    pub enabled: bool,
    pub kind: TrackerKind,
    pub event_id: i32,
    pub label: String,
    pub scope: TrackerScope,
    /// Only used for transient Skill ID rows. Buff timers use the game packet.
    pub hold_seconds: u8,
}
impl Default for TrackerRule {
    fn default() -> Self {
        Self {
            rule_id: 1,
            enabled: true,
            kind: TrackerKind::Buff,
            event_id: 0,
            label: String::new(),
            scope: TrackerScope::Any,
            hold_seconds: DEFAULT_HOLD_SECONDS,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TrackerSettings {
    pub enabled: bool,
    pub max_visible: usize,
    pub rules: Vec<TrackerRule>,
}
impl Default for TrackerSettings {
    fn default() -> Self { Self { enabled: true, max_visible: 6, rules: Vec::new() } }
}
impl TrackerSettings {
    pub fn normalize(&mut self) {
        self.max_visible = self.max_visible.clamp(1, 12);
        self.rules.truncate(MAX_RULES);
        let mut seen = HashSet::new();
        for rule in &mut self.rules {
            rule.event_id = rule.event_id.max(0);
            rule.hold_seconds = rule.hold_seconds.clamp(1, 30);
            rule.label = rule.label.trim().chars().filter(|c| !c.is_control()).take(MAX_LABEL_CHARS).collect();
            if rule.rule_id == 0 || seen.contains(&rule.rule_id) {
                rule.rule_id = (1..=MAX_RULES as u32 + 1).find(|id| !seen.contains(id)).unwrap();
            }
            seen.insert(rule.rule_id);
        }
    }
    pub fn next_rule_id(&self) -> u32 {
        (1..=MAX_RULES as u32 + 1).find(|id| !self.rules.iter().any(|r| r.rule_id == *id)).unwrap()
    }

}

#[derive(Clone, Debug, Default)]
pub struct TrackerDisplayRow {
    pub rule_id: u32,
    pub label: String,
    pub detail: String,
    pub count: u64,
    pub active: bool,
    pub remaining_ms: Option<i64>,
    pub last_seen_unix_ms: i64,
}

#[derive(Clone, Debug, Default)]
struct RuleState {
    count: u64,
    last_seen_unix_ms: i64,
    expires_unix_ms: i64,
    detail: String,
}

#[derive(Clone, Copy, Debug)]
struct BuffInstance {
    base_id: i32,
    target_uid: i64,
    expires_unix_ms: i64,
    last_seen_unix_ms: i64,
}

const MAX_BUFF_INSTANCES: usize = 2048;

#[derive(Debug, Default)]
struct RuntimeState {
    local_uid: i64,
    party_uids: HashSet<i64>,
    rule_states: HashMap<u32, RuleState>,
    // Raw host UUID + instance ID keeps NPCs and players distinct.
    buff_instances: HashMap<(i64, i32), BuffInstance>,
}

impl RuntimeState {
    fn skill(&mut self, settings: &TrackerSettings, skill_id: i32, source: i64, target: i64, now: i64) {
        if !settings.enabled || skill_id <= 0 { return; }
        for rule in settings.rules.iter().filter(|r| r.enabled && r.kind == TrackerKind::Skill && r.event_id == skill_id) {
            if !scope_matches(rule.scope, source, target, self) { continue; }
            let detail = match (source > 0, target > 0) {
                (true, true) => format!("{} → {}", scope_uid_label(source, self), scope_uid_label(target, self)),
                (true, false) => format!("from {}", scope_uid_label(source, self)),
                (false, true) => format!("on {}", scope_uid_label(target, self)),
                _ => "observed".into(),
            };
            let item = self.rule_states.entry(rule.rule_id).or_default();
            item.count = item.count.saturating_add(1);
            item.last_seen_unix_ms = now;
            item.expires_unix_ms = now.saturating_add(i64::from(rule.hold_seconds) * 1000);
            item.detail = detail;
        }
    }

    fn buff(&mut self, settings: &TrackerSettings, host: i64, target_uid: i64, instance_id: i32, base_id: i32, removed: bool, expiry: i64, now: i64) {
        if host == 0 || instance_id == 0 { return; }
        let key = (host, instance_id);
        if removed { self.buff_instances.remove(&key); return; }
        self.buff_instances.retain(|_, b| b.expires_unix_ms <= 0 || b.expires_unix_ms > now);
        // Store only IDs that the user elected to track. Unrelated world buffs
        // do not accumulate, even when the overlay is hidden.
        if !settings.enabled || base_id <= 0 || (expiry > 0 && expiry <= now)
            || !settings.rules.iter().any(|r| r.enabled && r.kind == TrackerKind::Buff && r.event_id == base_id) { return; }
        let is_new = self.buff_instances.get(&key).is_none_or(|b| b.base_id != base_id);
        if is_new && self.buff_instances.len() >= MAX_BUFF_INSTANCES {
            if let Some(oldest) = self.buff_instances.iter().min_by_key(|(_, b)| b.last_seen_unix_ms).map(|(key, _)| *key) {
                self.buff_instances.remove(&oldest);
            }
        }
        self.buff_instances.insert(key, BuffInstance { base_id, target_uid, expires_unix_ms: expiry, last_seen_unix_ms: now });
        for rule in settings.rules.iter().filter(|r| r.enabled && r.kind == TrackerKind::Buff && r.event_id == base_id) {
            if !scope_matches(rule.scope, 0, target_uid, self) { continue; }
            let detail = if target_uid > 0 { format!("on {}", scope_uid_label(target_uid, self)) } else { "on NPC".into() };
            let item = self.rule_states.entry(rule.rule_id).or_default();
            if is_new { item.count = item.count.saturating_add(1); }
            item.last_seen_unix_ms = now;
            item.expires_unix_ms = expiry;
            item.detail = detail;
        }
    }

    fn rows(&mut self, settings: &TrackerSettings, now: i64) -> Vec<TrackerDisplayRow> {
        if !settings.enabled { return Vec::new(); }
        self.buff_instances.retain(|_, b| b.expires_unix_ms <= 0 || b.expires_unix_ms > now);
        let mut out = Vec::new();
        for rule in settings.rules.iter().filter(|r| r.enabled && r.event_id > 0) {
            let fallback = RuleState::default();
            let item = self.rule_states.get(&rule.rule_id).unwrap_or(&fallback);
            let mut row = TrackerDisplayRow {
                rule_id: rule.rule_id, label: rule_label(rule), detail: item.detail.clone(), count: item.count,
                active: true, remaining_ms: None, last_seen_unix_ms: item.last_seen_unix_ms,
            };
            match rule.kind {
                TrackerKind::Skill => {
                    if item.expires_unix_ms <= now { continue; }
                    row.remaining_ms = Some(item.expires_unix_ms - now);
                    row.detail = format!("{} • {} hits", row.detail, item.count);
                }
                TrackerKind::Buff => {
                    let matching: Vec<_> = self.buff_instances.values().filter(|b|
                        b.base_id == rule.event_id && scope_matches(rule.scope, 0, b.target_uid, self)).collect();
                    if matching.is_empty() { continue; }
                    // An untimed instance keeps the rule active after finite
                    // instances end; do not promise a misleading countdown.
                    if matching.iter().all(|b| b.expires_unix_ms > 0) {
                        row.remaining_ms = matching.iter().map(|b| b.expires_unix_ms - now).max();
                    }
                    row.last_seen_unix_ms = matching.iter().map(|b| b.last_seen_unix_ms).max().unwrap_or(0);
                    row.detail = if matching.len() > 1 { format!("{} active", matching.len()) }
                        else if matching[0].target_uid > 0 { format!("on {}", scope_uid_label(matching[0].target_uid, self)) }
                        else { "on NPC".into() };
                }
            }
            out.push(row);
        }
        // Stable rule order prevents rows jumping under the pointer on each hit.
        out.truncate(settings.max_visible);
        out
    }

    fn reconcile(&mut self, old: &TrackerSettings, new: &TrackerSettings) {
        self.rule_states.retain(|id, _| new.enabled && new.rules.iter().any(|r|
            r.rule_id == *id && r.enabled && old.rules.iter().any(|old| old == r)));
        self.buff_instances.retain(|_, b| new.enabled && new.rules.iter().any(|r|
            r.enabled && r.kind == TrackerKind::Buff && r.event_id == b.base_id));
    }
}

struct Runtime { path: PathBuf, settings: RwLock<TrackerSettings>, state: Mutex<RuntimeState> }
static RUNTIME: OnceLock<Runtime> = OnceLock::new();

pub fn init(root: &Path) {
    let path = root.join("event_tracker.json");
    let settings = load_path(&path);
    let _ = RUNTIME.set(Runtime { path, settings: RwLock::new(settings), state: Mutex::new(RuntimeState::default()) });
}

pub fn current_settings() -> TrackerSettings {
    RUNTIME.get().and_then(|r| r.settings.read().ok().map(|s| s.clone())).unwrap_or_default()
}

pub fn replace_settings(mut settings: TrackerSettings) -> io::Result<()> {
    settings.normalize();
    let runtime = RUNTIME.get().ok_or_else(|| io::Error::other("event tracker is not initialized"))?;
    // One lock order throughout: settings, then state. Publish only after saving.
    let mut slot = runtime.settings.write().map_err(|_| io::Error::other("tracker settings lock failed"))?;
    save_path(&runtime.path, &settings)?;
    if let Ok(mut state) = runtime.state.lock() { state.reconcile(&slot, &settings); }
    *slot = settings;
    Ok(())
}

pub fn set_local_uid(uid: i64) {
    if let Some(r) = RUNTIME.get() { if let Ok(mut s) = r.state.lock() { s.local_uid = uid; } }
}
pub fn set_party(uids: &HashSet<i64>) {
    if let Some(r) = RUNTIME.get() { if let Ok(mut s) = r.state.lock() { s.party_uids.clone_from(uids); } }
}
pub fn observe_skill(skill_id: i32, source_uid: i64, target_uid: i64) {
    if let Some(r) = RUNTIME.get() { if let (Ok(settings), Ok(mut s)) = (r.settings.read(), r.state.lock()) {
        s.skill(&settings, skill_id, source_uid, target_uid, now_ms());
    } }
}
pub fn observe_buff(host: i64, target_uid: i64, instance: i32, base_id: i32, removed: bool, expiry: i64) {
    if let Some(r) = RUNTIME.get() { if let (Ok(settings), Ok(mut s)) = (r.settings.read(), r.state.lock()) {
        s.buff(&settings, host, target_uid, instance, base_id, removed, expiry, now_ms());
    } }
}
pub fn remove_entity(host: i64) {
    if let Some(r) = RUNTIME.get() { if let Ok(mut s) = r.state.lock() { s.buff_instances.retain(|(h, _), _| *h != host); } }
}
pub fn sync_buff_instances(host: i64, instances: &HashSet<i32>) {
    if let Some(r) = RUNTIME.get() { if let Ok(mut s) = r.state.lock() {
        s.buff_instances.retain(|(h, id), _| *h != host || instances.contains(id));
    } }
}
pub fn clear_scene() {
    if let Some(r) = RUNTIME.get() { if let Ok(mut s) = r.state.lock() {
        s.local_uid = 0; s.rule_states.clear(); s.buff_instances.clear();
    } }
}
pub fn clear_session() {
    if let Some(r) = RUNTIME.get() { if let Ok(mut s) = r.state.lock() { *s = RuntimeState::default(); } }
}
pub fn reset_counts() {
    if let Some(r) = RUNTIME.get() { if let Ok(mut s) = r.state.lock() { s.rule_states.clear(); } }
}
pub fn rows() -> Vec<TrackerDisplayRow> {
    if let Some(r) = RUNTIME.get() { if let (Ok(settings), Ok(mut s)) = (r.settings.read(), r.state.lock()) {
        return s.rows(&settings, now_ms());
    } }
    Vec::new()
}
pub fn rule_label(rule: &TrackerRule) -> String {
    if !rule.label.trim().is_empty() { rule.label.trim().to_string() } else { format!("{} {}", rule.kind.label(), rule.event_id) }
}
fn scope_matches(scope: TrackerScope, source: i64, target: i64, state: &RuntimeState) -> bool {
    let is_self = state.local_uid > 0 && (source == state.local_uid || target == state.local_uid);
    match scope {
        TrackerScope::Any => true,
        TrackerScope::SelfOnly => is_self,
        TrackerScope::Party => is_self || (source > 0 && state.party_uids.contains(&source)) || (target > 0 && state.party_uids.contains(&target)),
    }
}
fn scope_uid_label(uid: i64, state: &RuntimeState) -> String {
    if uid == state.local_uid && uid > 0 { "you".into() } else { format!("UID {uid}") }
}
fn load_path(path: &Path) -> TrackerSettings {
    for candidate in [path.to_path_buf(), path.with_extension("json.bak"), path.with_extension("json.new")] {
        if let Ok(text) = fs::read_to_string(&candidate) {
            match serde_json::from_str::<TrackerSettings>(&text) {
                Ok(mut value) => { value.normalize(); return value; }
                Err(err) => logging::write(format!("event tracker: settings load failed: {err}")),
            }
        }
    }
    TrackerSettings::default()
}
fn save_path(path: &Path, settings: &TrackerSettings) -> io::Result<()> {
    let tmp = path.with_extension("json.new");
    let backup = path.with_extension("json.bak");
    fs::write(&tmp, serde_json::to_vec_pretty(settings).map_err(io::Error::other)?)?;
    let _: TrackerSettings = serde_json::from_slice(&fs::read(&tmp)?).map_err(io::Error::other)?;
    if path.exists() {
        if fs::read(path).ok().and_then(|s| serde_json::from_slice::<TrackerSettings>(&s).ok()).is_some() { fs::copy(path, backup)?; }
        fs::remove_file(path)?;
    }
    fs::rename(tmp, path)
}
fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis().min(i64::MAX as u128) as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    fn config(kind: TrackerKind, scope: TrackerScope) -> TrackerSettings {
        TrackerSettings { rules: vec![TrackerRule { event_id: 55, kind, scope, ..TrackerRule::default() }], ..TrackerSettings::default() }
    }
    #[test]
    fn ids_are_unique_even_after_u32_max_and_labels_are_safe() {
        let mut s = config(TrackerKind::Buff, TrackerScope::Any);
        s.rules = vec![TrackerRule { rule_id: u32::MAX, label: "  hi\nthere  ".into(), ..TrackerRule::default() }; 30];
        s.max_visible = 99; s.normalize();
        assert_eq!(s.rules.len(), 24); assert_eq!(s.max_visible, 12);
        assert_eq!(s.rules.iter().map(|r| r.rule_id).collect::<HashSet<_>>().len(), 24);
        assert_eq!(s.rules[0].label, "hithere");
        assert!(!s.rules.iter().any(|r| r.rule_id == s.next_rule_id()));
    }
    #[test]
    fn refresh_is_not_an_application_and_removal_keeps_other_hosts() {
        let mut s = RuntimeState::default(); let c = config(TrackerKind::Buff, TrackerScope::Any);
        s.buff(&c, 100, 7, 1, 55, false, 9000, 1000);
        s.buff(&c, 100, 7, 1, 55, false, 10000, 2000);
        assert_eq!(s.rows(&c, 3000)[0].count, 1);
        assert_eq!(s.rows(&c, 3000)[0].remaining_ms, Some(7000));
        s.buff(&c, 200, 8, 1, 55, false, 0, 3000);
        assert_eq!(s.rows(&c, 4000)[0].remaining_ms, None);
        s.buff(&c, 200, 8, 1, 0, true, 0, 4000);
        assert_eq!(s.rows(&c, 4000)[0].remaining_ms, Some(6000));
        assert!(s.rows(&c, 10000).is_empty());
    }
    #[test]
    fn local_scope_works_before_any_combat_snapshot_and_party_leave_takes_effect() {
        let mut s = RuntimeState { local_uid: 7, ..RuntimeState::default() };
        let c = config(TrackerKind::Skill, TrackerScope::SelfOnly);
        s.skill(&c, 55, 7, 0, 1000); s.skill(&c, 55, 99, 0, 2000);
        assert_eq!(s.rows(&c, 2000)[0].count, 1);
        assert!(s.rows(&c, 7000).is_empty());
        let c = config(TrackerKind::Buff, TrackerScope::Party);
        s.party_uids.insert(8); s.buff(&c, 200, 8, 1, 55, false, 0, 1000);
        assert_eq!(s.rows(&c, 2000).len(), 1);
        s.party_uids.clear(); assert!(s.rows(&c, 2000).is_empty());
    }
    #[test]
    fn unrelated_and_expired_buffs_are_ignored_and_storage_is_bounded() {
        let mut s = RuntimeState::default(); let c = config(TrackerKind::Buff, TrackerScope::Any);
        s.buff(&c, 100, 7, 1, 99, false, 0, 1000);
        s.buff(&c, 100, 7, 1, 55, false, 900, 1000);
        assert!(s.buff_instances.is_empty());
        for id in 1..=MAX_BUFF_INSTANCES as i32 + 5 { s.buff(&c, 100, 7, id, 55, false, 0, 1000 + id as i64); }
        assert_eq!(s.buff_instances.len(), MAX_BUFF_INSTANCES);
    }
    #[test]
    fn changing_skill_id_does_not_relabel_old_hits() {
        let mut s = RuntimeState::default(); let old = config(TrackerKind::Skill, TrackerScope::Any);
        s.skill(&old, 55, 7, 0, 1000); let mut new = old.clone(); new.rules[0].event_id = 66;
        s.reconcile(&old, &new); assert!(s.rows(&new, 2000).is_empty());
    }
    #[test]
    fn corrupt_primary_recovers_rules_from_backup() {
        let dir = std::env::temp_dir().join(format!("tracker-{}-{}", std::process::id(), now_ms()));
        fs::create_dir_all(&dir).unwrap(); let path = dir.join("event_tracker.json");
        let c = config(TrackerKind::Buff, TrackerScope::SelfOnly);
        save_path(&path, &c).unwrap(); save_path(&path, &c).unwrap();
        fs::write(&path, "broken").unwrap(); assert_eq!(load_path(&path), c);
        save_path(&path, &c).unwrap(); assert_eq!(load_path(&path), c);
        fs::remove_dir_all(dir).unwrap();
    }
}
