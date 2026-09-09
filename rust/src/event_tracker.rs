use crate::{logging, model::DpsRow};
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

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
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
        let mut next_id = 1u32;
        for rule in &mut self.rules {
            rule.event_id = rule.event_id.max(0);
            rule.hold_seconds = rule.hold_seconds.clamp(1, 30);
            rule.label = rule.label.trim().chars().take(MAX_LABEL_CHARS).collect();
            if rule.rule_id == 0 || !seen.insert(rule.rule_id) {
                while seen.contains(&next_id) { next_id = next_id.saturating_add(1); }
                rule.rule_id = next_id;
                seen.insert(next_id);
            }
            next_id = next_id.max(rule.rule_id.saturating_add(1));
        }
    }
    pub fn next_rule_id(&self) -> u32 {
        self.rules.iter().map(|r| r.rule_id).max().unwrap_or(0).saturating_add(1).max(1)
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
}

#[derive(Debug, Default)]
struct RuntimeState {
    local_uid: i64,
    party_uids: HashSet<i64>,
    rule_states: HashMap<u32, RuleState>,
    /// Key is (target uid, buff instance uuid).
    buff_instances: HashMap<(i64, i32), BuffInstance>,
}

struct Runtime {
    path: PathBuf,
    settings: RwLock<TrackerSettings>,
    state: Mutex<RuntimeState>,
}

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

pub fn init(root: &Path) {
    let path = root.join("event_tracker.json");
    let settings = load_path(&path);
    let _ = RUNTIME.set(Runtime {
        path,
        settings: RwLock::new(settings),
        state: Mutex::new(RuntimeState::default()),
    });
}

pub fn current_settings() -> TrackerSettings {
    RUNTIME.get()
        .and_then(|runtime| runtime.settings.read().ok().map(|value| value.clone()))
        .unwrap_or_default()
}

pub fn replace_settings(mut settings: TrackerSettings) -> io::Result<()> {
    settings.normalize();
    let Some(runtime) = RUNTIME.get() else { return Err(io::Error::other("event tracker is not initialized")); };
    save_path(&runtime.path, &settings)?;
    if let Ok(mut slot) = runtime.settings.write() { *slot = settings.clone(); }
    if let Ok(mut state) = runtime.state.lock() {
        let valid: HashSet<u32> = settings.rules.iter().map(|rule| rule.rule_id).collect();
        state.rule_states.retain(|rule_id, _| valid.contains(rule_id));
    }
    Ok(())
}

pub fn update_roster(rows: &[DpsRow]) {
    let Some(runtime) = RUNTIME.get() else { return; };
    let Ok(mut state) = runtime.state.lock() else { return; };
    state.local_uid = rows.iter().find(|row| row.is_local).map(|row| row.uid).unwrap_or(0);
    state.party_uids.clear();
    for row in rows {
        if row.is_party || row.is_local { state.party_uids.insert(row.uid); }
    }
}

pub fn observe_skill(skill_id: i32, source_uid: i64, target_uid: i64) {
    if skill_id <= 0 { return; }
    let Some(runtime) = RUNTIME.get() else { return; };
    let settings = runtime.settings.read().ok().map(|value| value.clone()).unwrap_or_default();
    if !settings.enabled { return; }
    let now = now_ms();
    let Ok(mut state) = runtime.state.lock() else { return; };
    for rule in settings.rules.iter().filter(|rule| rule.enabled && rule.kind == TrackerKind::Skill && rule.event_id == skill_id) {
        if !scope_matches(rule.scope, source_uid, target_uid, &state) { continue; }
        let detail = match (source_uid > 0, target_uid > 0) {
            (true, true) => format!("{} → {}", scope_uid_label(source_uid, &state), scope_uid_label(target_uid, &state)),
            (true, false) => format!("from {}", scope_uid_label(source_uid, &state)),
            (false, true) => format!("on {}", scope_uid_label(target_uid, &state)),
            _ => "observed".into(),
        };
        let item = state.rule_states.entry(rule.rule_id).or_default();
        item.count = item.count.saturating_add(1);
        item.last_seen_unix_ms = now;
        item.expires_unix_ms = now.saturating_add(i64::from(rule.hold_seconds) * 1000);
        item.detail = detail;
    }
}

/// Observe a buff instance. `removed` can omit `base_id`; removal is keyed by
/// target uid + instance uuid. A zero expiry means the game supplied no finite
/// timer, so ReadyAlert shows the buff as active without inventing a countdown.
pub fn observe_buff(target_uid: i64, buff_uuid: i32, base_id: i32, removed: bool, expires_unix_ms: i64) {
    if target_uid <= 0 || buff_uuid == 0 { return; }
    let Some(runtime) = RUNTIME.get() else { return; };
    let settings = runtime.settings.read().ok().map(|value| value.clone()).unwrap_or_default();
    let now = now_ms();
    let Ok(mut state) = runtime.state.lock() else { return; };
    let key = (target_uid, buff_uuid);
    if removed {
        state.buff_instances.remove(&key);
        return;
    }
    if base_id <= 0 { return; }
    let is_new = !state.buff_instances.contains_key(&key);
    state.buff_instances.insert(key, BuffInstance { base_id, target_uid, expires_unix_ms });
    if !settings.enabled { return; }
    for rule in settings.rules.iter().filter(|rule| rule.enabled && rule.kind == TrackerKind::Buff && rule.event_id == base_id) {
        if !scope_matches(rule.scope, 0, target_uid, &state) { continue; }
        let item = state.rule_states.entry(rule.rule_id).or_default();
        if is_new { item.count = item.count.saturating_add(1); }
        item.last_seen_unix_ms = now;
        item.expires_unix_ms = expires_unix_ms;
        let detail = format!("on {}", scope_uid_label(target_uid, &state));
        state.rule_states.get_mut(&rule.rule_id).unwrap().detail = detail;
    }
}

pub fn clear_scene() {
    let Some(runtime) = RUNTIME.get() else { return; };
    if let Ok(mut state) = runtime.state.lock() {
        *state = RuntimeState::default();
    }
}

pub fn rows() -> Vec<TrackerDisplayRow> {
    let Some(runtime) = RUNTIME.get() else { return Vec::new(); };
    let settings = runtime.settings.read().ok().map(|value| value.clone()).unwrap_or_default();
    if !settings.enabled { return Vec::new(); }
    let now = now_ms();
    let Ok(mut state) = runtime.state.lock() else { return Vec::new(); };
    state.buff_instances.retain(|_, instance| instance.expires_unix_ms <= 0 || instance.expires_unix_ms > now);

    let mut out = Vec::new();
    for rule in settings.rules.iter().filter(|rule| rule.enabled && rule.event_id > 0) {
        match rule.kind {
            TrackerKind::Skill => {
                let Some(item) = state.rule_states.get(&rule.rule_id) else { continue; };
                if item.expires_unix_ms <= now { continue; }
                out.push(display_row(rule, item, now, true));
            }
            TrackerKind::Buff => {
                let matching: Vec<_> = state.buff_instances.values()
                    .filter(|instance| instance.base_id == rule.event_id && scope_matches(rule.scope, 0, instance.target_uid, &state))
                    .collect();
                if matching.is_empty() { continue; }
                let finite_expiry = matching.iter().filter_map(|instance| (instance.expires_unix_ms > 0).then_some(instance.expires_unix_ms)).max().unwrap_or(0);
                let fallback = RuleState::default();
                let item = state.rule_states.get(&rule.rule_id).unwrap_or(&fallback);
                let mut row = display_row(rule, item, now, true);
                row.remaining_ms = (finite_expiry > 0).then_some(finite_expiry.saturating_sub(now));
                row.active = true;
                if matching.len() > 1 { row.detail = format!("{} active", matching.len()); }
                out.push(row);
            }
        }
    }
    out.sort_by(|a, b| b.active.cmp(&a.active).then_with(|| b.last_seen_unix_ms.cmp(&a.last_seen_unix_ms)).then_with(|| a.label.cmp(&b.label)));
    out.truncate(settings.max_visible);
    out
}

fn display_row(rule: &TrackerRule, item: &RuleState, now: i64, active: bool) -> TrackerDisplayRow {
    TrackerDisplayRow {
        rule_id: rule.rule_id,
        label: rule_label(rule),
        detail: item.detail.clone(),
        count: item.count,
        active,
        remaining_ms: (item.expires_unix_ms > 0).then_some(item.expires_unix_ms.saturating_sub(now)),
        last_seen_unix_ms: item.last_seen_unix_ms,
    }
}

pub fn rule_label(rule: &TrackerRule) -> String {
    if !rule.label.trim().is_empty() { rule.label.trim().to_string() }
    else { format!("{} {}", rule.kind.label(), rule.event_id) }
}

fn scope_matches(scope: TrackerScope, source_uid: i64, target_uid: i64, state: &RuntimeState) -> bool {
    match scope {
        TrackerScope::Any => true,
        TrackerScope::SelfOnly => state.local_uid > 0 && (source_uid == state.local_uid || target_uid == state.local_uid),
        TrackerScope::Party => {
            (source_uid > 0 && state.party_uids.contains(&source_uid))
                || (target_uid > 0 && state.party_uids.contains(&target_uid))
        }
    }
}

fn scope_uid_label(uid: i64, state: &RuntimeState) -> String {
    if uid <= 0 { return "unknown".into(); }
    if uid == state.local_uid { "you".into() }
    else if state.party_uids.contains(&uid) { "party".into() }
    else { format!("UID {uid}") }
}

fn load_path(path: &Path) -> TrackerSettings {
    if let Ok(text) = fs::read_to_string(path) {
        match serde_json::from_str::<TrackerSettings>(&text) {
            Ok(mut value) => { value.normalize(); return value; }
            Err(err) => logging::write(format!("event tracker: settings load failed: {err}")),
        }
    }
    let mut value = TrackerSettings::default();
    value.normalize();
    let _ = save_path(path, &value);
    value
}

fn save_path(path: &Path, settings: &TrackerSettings) -> io::Result<()> {
    let mut value = settings.clone();
    value.normalize();
    let tmp = path.with_extension("json.new");
    fs::write(&tmp, serde_json::to_string_pretty(&value).map_err(io::Error::other)?)?;
    if path.exists() { fs::remove_file(path)?; }
    fs::rename(tmp, path)
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis().min(i64::MAX as u128) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_normalize_caps_rules_and_repairs_ids() {
        let mut value = TrackerSettings { enabled: true, max_visible: 99, rules: vec![
            TrackerRule { rule_id: 1, event_id: -4, hold_seconds: 0, label: "  test  ".into(), ..TrackerRule::default() },
            TrackerRule { rule_id: 1, event_id: 55, hold_seconds: 99, ..TrackerRule::default() },
        ]};
        value.normalize();
        assert_eq!(value.max_visible, 12);
        assert_eq!(value.rules[0].event_id, 0);
        assert_eq!(value.rules[0].label, "test");
        assert_eq!(value.rules[0].hold_seconds, 1);
        assert_ne!(value.rules[0].rule_id, value.rules[1].rule_id);
        assert_eq!(value.rules[1].hold_seconds, 30);
    }

    #[test]
    fn default_tracker_has_no_surprise_rules() {
        let value = TrackerSettings::default();
        assert!(value.enabled);
        assert!(value.rules.is_empty());
        assert_eq!(value.max_visible, 6);
    }

    #[test]
    fn label_falls_back_to_kind_and_id() {
        let rule = TrackerRule { kind: TrackerKind::Skill, event_id: 1234, ..TrackerRule::default() };
        assert_eq!(rule_label(&rule), "Skill ID 1234");
    }
}
