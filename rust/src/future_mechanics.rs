use crate::{model::MechanicRow, proto};
use std::{
    collections::{HashMap, HashSet},
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

const SYNC_NEAR_ENTITIES: u32 = 0x06;
const SYNC_SCENE_EVENTS: u32 = 0x08;
const SYNC_NEAR_DELTA_INFO: u32 = 0x2d;
const SYNC_TO_ME_DELTA_INFO: u32 = 0x2e;
const ATTR_SCENE_BASIC_ID: u64 = 0x155;
const EVENT_NOTICE_TIP: i32 = 1;
const EVENT_BOSS_DBM: i32 = 29;
const DEFAULT_TRANSIENT_MS: i64 = 6_000;
const MAX_TRANSIENT_MS: i64 = 3_600_000;
const MAX_RULES: usize = 256;
const OVERRIDE_FILE: &str = "mechanics_override.tsv";
const DIAGNOSTIC_FILE: &str = "unknown_mechanics.log";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TriggerKind {
    Dbm,
    Skill,
    Attribute,
    NoticeTip,
}

#[derive(Clone, Debug)]
struct Rule {
    scene_id: i32,
    kind: TriggerKind,
    event_id: i32,
    label: String,
    duration_ms: i64,
    priority: u8,
}

pub struct Runtime {
    root: Option<PathBuf>,
    rules: Vec<Rule>,
    current_scene_id: i32,
    local_uid: i64,
    rows: HashMap<String, MechanicRow>,
    attribute_values: HashMap<(i32, i64), i64>,
    notice_seen: HashSet<String>,
}

impl Runtime {
    pub fn new() -> Self {
        let root = crate::paths::AppPaths::create().ok().map(|paths| paths.root);
        if let Some(root) = root.as_ref() {
            ensure_override_template(root.join(OVERRIDE_FILE));
        }
        let rules = root
            .as_ref()
            .map(|root| load_rules(root.join(OVERRIDE_FILE)))
            .unwrap_or_default();
        Self {
            root,
            rules,
            current_scene_id: 0,
            local_uid: 0,
            rows: HashMap::new(),
            attribute_values: HashMap::new(),
            notice_seen: HashSet::new(),
        }
    }

    pub fn observe_notify(&mut self, service: u64, method: u32, body: &[u8]) -> bool {
        let mut changed = self.prune_expired();
        if service != proto::WORLD_SERVICE {
            return changed;
        }

        match method {
            proto::ENTER_SCENE_METHOD => {
                changed |= !self.rows.is_empty();
                self.rows.clear();
                self.attribute_values.clear();
                self.notice_seen.clear();
                self.current_scene_id = 0;
                self.local_uid = 0;
                if let Some(info) = proto::get_len_field(body, 1) {
                    if let Some(scene_attrs) = proto::get_len_field(info, 1) {
                        self.current_scene_id = scene_id_from_attrs(scene_attrs).unwrap_or(0);
                    }
                    if let Some(player) = proto::get_len_field(info, 2) {
                        let uuid = signed(proto::get_varint_field(player, 1).unwrap_or(0));
                        self.local_uid = player_uid(uuid);
                        changed |= self.observe_attrs(uuid, proto::get_len_field(player, 3));
                    }
                }
            }
            SYNC_NEAR_ENTITIES => {
                for entity in proto::len_fields(body, 1) {
                    let uuid = signed(proto::get_varint_field(entity, 1).unwrap_or(0));
                    changed |= self.observe_attrs(uuid, proto::get_len_field(entity, 3));
                }
            }
            SYNC_NEAR_DELTA_INFO => {
                for delta in proto::len_fields(body, 1) {
                    changed |= self.observe_delta(delta, 0);
                }
            }
            SYNC_TO_ME_DELTA_INFO => {
                if let Some(wrapper) = proto::get_len_field(body, 1) {
                    if let Some(delta) = proto::get_len_field(wrapper, 1) {
                        changed |= self.observe_delta(delta, self.local_uid);
                    }
                }
            }
            SYNC_SCENE_EVENTS => {
                changed |= self.observe_scene_events(body);
            }
            _ => {}
        }
        changed
    }

    pub fn rows(&mut self) -> Vec<MechanicRow> {
        self.prune_expired();
        self.rows.values().cloned().collect()
    }

    fn observe_scene_events(&mut self, body: &[u8]) -> bool {
        let Some(list) = proto::get_len_field(body, 1) else { return false; };
        let mut changed = false;
        for event in proto::len_fields(list, 2) {
            let event_type = proto::get_varint_field(event, 1).unwrap_or(0) as i32;
            let ints = repeated_varints(event, 2);
            match event_type {
                EVENT_BOSS_DBM => {
                    let Some(effect_id) = ints.first().copied().map(|value| value as i32).filter(|id| *id > 0) else { continue; };
                    let seconds = ints.get(1).copied().map(|value| value as i32).unwrap_or(0).max(0);
                    let insertion = ints.get(2).copied().map(|value| value as i32).unwrap_or(0);
                    changed |= self.observe_dbm(effect_id, seconds, insertion);
                }
                EVENT_NOTICE_TIP => {
                    let tip_id = ints.first().copied().map(|value| value as i32).unwrap_or(0);
                    let text = event_strings(event).join(" | ");
                    self.log_notice(tip_id, &text);
                    if tip_id > 0 {
                        if let Some(rule) = self.find_rule(TriggerKind::NoticeTip, tip_id).cloned() {
                            let target = (!text.is_empty()).then_some(text);
                            changed |= self.upsert_rule_row(&rule, format!("future:notice:{}:{tip_id}", self.current_scene_id), target);
                        }
                    }
                }
                _ => {}
            }
        }
        changed
    }

    fn observe_dbm(&mut self, effect_id: i32, duration_seconds: i32, insertion: i32) -> bool {
        let rule = self.find_rule(TriggerKind::Dbm, effect_id).cloned();
        let base_id = effect_id / 100;
        let label = rule
            .as_ref()
            .map(|rule| rule.label.clone())
            .or_else(|| crate::telemetry::game_names_v1300::skill_name(effect_id).map(str::to_owned))
            .or_else(|| (base_id > 0).then(|| crate::telemetry::game_names_v1300::skill_name(base_id)).flatten().map(str::to_owned))
            .unwrap_or_else(|| format!("Boss mechanic #{effect_id}"));
        let packet_ms = i64::from(duration_seconds).saturating_mul(1_000);
        let duration_ms = rule
            .as_ref()
            .map(|rule| rule.duration_ms)
            .filter(|duration| *duration > 0)
            .unwrap_or(packet_ms)
            .clamp(1, MAX_TRANSIENT_MS);
        let priority = rule.as_ref().map(|rule| rule.priority).unwrap_or(3);
        let now = now_ms();
        let key = format!("future:dbm:{}:{effect_id}:{insertion}", self.current_scene_id);
        self.rows.insert(key.clone(), MechanicRow {
            key,
            label,
            target: None,
            created_unix_ms: now,
            expires_unix_ms: now.saturating_add(duration_ms.max(DEFAULT_TRANSIENT_MS.min(duration_ms))),
            persistent: false,
            priority,
        });
        true
    }

    fn observe_delta(&mut self, delta: &[u8], fallback_uid: i64) -> bool {
        let uuid = signed(proto::get_varint_field(delta, 1).unwrap_or(0));
        let target_uid = if uuid != 0 { player_uid(uuid) } else { fallback_uid.max(0) };
        let mut changed = self.observe_attrs_with_uid(target_uid, proto::get_len_field(delta, 2));
        if let Some(effects) = proto::get_len_field(delta, 7) {
            for damage in proto::len_fields(effects, 2) {
                let skill_id = proto::get_varint_field(damage, 12).unwrap_or(0) as i32;
                if skill_id <= 0 {
                    continue;
                }
                if let Some(rule) = self.find_rule(TriggerKind::Skill, skill_id).cloned() {
                    let target = (target_uid > 0).then(|| format!("UID {target_uid}"));
                    changed |= self.upsert_rule_row(
                        &rule,
                        format!("future:skill:{}:{skill_id}:{target_uid}", self.current_scene_id),
                        target,
                    );
                }
            }
        }
        changed
    }

    fn observe_attrs(&mut self, uuid: i64, attrs: Option<&[u8]>) -> bool {
        self.observe_attrs_with_uid(player_uid(uuid), attrs)
    }

    fn observe_attrs_with_uid(&mut self, target_uid: i64, attrs: Option<&[u8]>) -> bool {
        let Some(attrs) = attrs else { return false; };
        let mut changed = false;
        for attr in proto::len_fields(attrs, 2) {
            let id = proto::get_varint_field(attr, 1).unwrap_or(0);
            if id == 0 || id > i32::MAX as u64 {
                continue;
            }
            let Some(raw) = proto::get_len_field(attr, 2) else { continue; };
            let Some(value) = raw_varint(raw) else { continue; };
            let attr_id = id as i32;
            let value = value as i64;
            changed |= crate::event_tracker::observe_attribute(attr_id, target_uid, value);

            if let Some(rule) = self.find_rule(TriggerKind::Attribute, attr_id).cloned() {
                let key = (attr_id, target_uid);
                let value_changed = self.attribute_values.get(&key).is_none_or(|old| *old != value);
                self.attribute_values.insert(key, value);
                if value_changed {
                    let target = if target_uid > 0 {
                        Some(format!("UID {target_uid}: {value}"))
                    } else {
                        Some(format!("value {value}"))
                    };
                    changed |= self.upsert_rule_row(
                        &rule,
                        format!("future:attr:{}:{attr_id}:{target_uid}", self.current_scene_id),
                        target,
                    );
                }
            }
        }
        changed
    }

    fn upsert_rule_row(&mut self, rule: &Rule, key: String, target: Option<String>) -> bool {
        let now = now_ms();
        let duration = if rule.duration_ms > 0 { rule.duration_ms } else { DEFAULT_TRANSIENT_MS };
        self.rows.insert(key.clone(), MechanicRow {
            key,
            label: rule.label.clone(),
            target,
            created_unix_ms: now,
            expires_unix_ms: now.saturating_add(duration.clamp(1, MAX_TRANSIENT_MS)),
            persistent: false,
            priority: rule.priority,
        });
        true
    }

    fn find_rule(&self, kind: TriggerKind, event_id: i32) -> Option<&Rule> {
        self.rules
            .iter()
            .find(|rule| rule.kind == kind && rule.event_id == event_id && rule.scene_id == self.current_scene_id)
            .or_else(|| self.rules.iter().find(|rule| rule.kind == kind && rule.event_id == event_id && rule.scene_id == 0))
    }

    fn prune_expired(&mut self) -> bool {
        let now = now_ms();
        let before = self.rows.len();
        self.rows.retain(|_, row| row.persistent || row.expires_unix_ms <= 0 || row.expires_unix_ms > now);
        before != self.rows.len()
    }

    fn log_notice(&mut self, tip_id: i32, text: &str) {
        let clean = sanitize_text(text, 180);
        let key = format!("{}:{tip_id}:{clean}", self.current_scene_id);
        if !self.notice_seen.insert(key) {
            return;
        }
        if self.notice_seen.len() > 1_024 {
            self.notice_seen.clear();
        }
        let Some(root) = self.root.as_ref() else { return; };
        let path = root.join(DIAGNOSTIC_FILE);
        let line = format!("{}\t{}\tN\t{}\t{}\n", now_ms(), self.current_scene_id, tip_id, clean.replace('\t', " "));
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
            let _ = file.write_all(line.as_bytes());
        }
    }
}

fn ensure_override_template(path: PathBuf) {
    let template = concat!(
        "# BPSR ReadyAlert runtime mechanic overrides. Restart ReadyAlert after editing.\n",
        "# Format: SCENE<TAB>KIND<TAB>ID<TAB>LABEL<TAB>DURATION_MS<TAB>PRIORITY\n",
        "# SCENE 0 = any scene. KIND: D Boss DBM, S combat skill, A attribute, N NoticeTip.\n",
        "# D uses the authoritative game packet duration when DURATION_MS is 0.\n",
        "# Other kinds default to a 6000 ms display when DURATION_MS is 0. PRIORITY is 0-3.\n",
        "# Example: 13031<TAB>D<TAB>12345601<TAB>Incoming mechanic<TAB>0<TAB>3\n",
    );
    match OpenOptions::new().create_new(true).write(true).open(&path) {
        Ok(mut file) => { let _ = file.write_all(template.as_bytes()); }
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(err) => crate::logging::write(format!("future mechanics: could not create {}: {err}", path.display())),
    }
}

fn load_rules(path: PathBuf) -> Vec<Rule> {
    let Ok(text) = fs::read_to_string(&path) else { return Vec::new(); };
    let mut rules = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim_end_matches('\r');
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != 6 {
            crate::logging::write(format!("future mechanics: ignored {} line {} with {} fields", path.display(), index + 1, fields.len()));
            continue;
        }
        let parsed = (|| {
            let scene_id = fields[0].trim().parse::<i32>().ok()?.max(0);
            let kind = parse_kind(fields[1])?;
            let event_id = fields[2].trim().parse::<i32>().ok()?.max(0);
            if event_id == 0 { return None; }
            let label = sanitize_text(fields[3], 80);
            if label.is_empty() { return None; }
            let duration_ms = fields[4].trim().parse::<i64>().ok()?.clamp(0, MAX_TRANSIENT_MS);
            let priority = fields[5].trim().parse::<u8>().ok()?.min(3);
            Some(Rule { scene_id, kind, event_id, label, duration_ms, priority })
        })();
        if let Some(rule) = parsed {
            rules.push(rule);
            if rules.len() >= MAX_RULES { break; }
        } else {
            crate::logging::write(format!("future mechanics: ignored invalid {} line {}", path.display(), index + 1));
        }
    }
    if !rules.is_empty() {
        crate::logging::write(format!("future mechanics: loaded {} runtime override(s)", rules.len()));
    }
    rules
}

fn parse_kind(raw: &str) -> Option<TriggerKind> {
    match raw.trim().to_ascii_uppercase().as_str() {
        "D" => Some(TriggerKind::Dbm),
        "S" => Some(TriggerKind::Skill),
        "A" => Some(TriggerKind::Attribute),
        "N" => Some(TriggerKind::NoticeTip),
        _ => None,
    }
}

fn scene_id_from_attrs(attrs: &[u8]) -> Option<i32> {
    for attr in proto::len_fields(attrs, 2) {
        if proto::get_varint_field(attr, 1) != Some(ATTR_SCENE_BASIC_ID) {
            continue;
        }
        let raw = proto::get_len_field(attr, 2)?;
        let value = raw_varint(raw)?;
        return i32::try_from(value).ok().filter(|value| *value > 0);
    }
    None
}

fn repeated_varints(data: &[u8], wanted: u32) -> Vec<u64> {
    let mut values = Vec::new();
    let mut p = 0usize;
    while p < data.len() {
        let Some(key) = proto::read_varint(data, &mut p) else { break; };
        let field = (key >> 3) as u32;
        let wire = (key & 7) as u8;
        match wire {
            0 => {
                let Some(value) = proto::read_varint(data, &mut p) else { break; };
                if field == wanted { values.push(value); }
            }
            1 => {
                let Some(end) = p.checked_add(8) else { break; };
                if end > data.len() { break; }
                p = end;
            }
            2 => {
                let Some(raw_len) = proto::read_varint(data, &mut p) else { break; };
                let Ok(len) = usize::try_from(raw_len) else { break; };
                let Some(end) = p.checked_add(len) else { break; };
                if end > data.len() { break; }
                if field == wanted {
                    let mut q = p;
                    while q < end {
                        let Some(value) = proto::read_varint(&data[..end], &mut q) else { break; };
                        values.push(value);
                    }
                }
                p = end;
            }
            5 => {
                let Some(end) = p.checked_add(4) else { break; };
                if end > data.len() { break; }
                p = end;
            }
            _ => break,
        }
    }
    values
}

fn event_strings(event: &[u8]) -> Vec<String> {
    proto::len_fields(event, 5)
        .into_iter()
        .filter_map(|raw| std::str::from_utf8(raw).ok())
        .map(|text| sanitize_text(text, 120))
        .filter(|text| !text.is_empty())
        .collect()
}

fn sanitize_text(text: &str, max_chars: usize) -> String {
    text.replace(['\r', '\n', '\0'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_chars)
        .collect()
}

fn raw_varint(raw: &[u8]) -> Option<u64> {
    let mut position = 0usize;
    proto::read_varint(raw, &mut position)
}

fn player_uid(uuid: i64) -> i64 {
    if uuid > 0 && (uuid & 0xffff) == 10 { uuid >> 16 } else { 0 }
}

fn signed(raw: u64) -> i64 { raw as i64 }

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| i64::try_from(duration.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put_varint(mut value: u64, out: &mut Vec<u8>) {
        loop {
            let mut byte = (value & 0x7f) as u8;
            value >>= 7;
            if value != 0 { byte |= 0x80; }
            out.push(byte);
            if value == 0 { break; }
        }
    }

    #[test]
    fn repeated_varints_accepts_unpacked_and_packed_fields() {
        let mut data = vec![0x10, 0x1d];
        data.extend([0x12, 0x03, 0x81, 0x01, 0x05]);
        assert_eq!(repeated_varints(&data, 2), vec![29, 129, 5]);
    }

    #[test]
    fn parser_accepts_runtime_trigger_kinds() {
        assert_eq!(parse_kind("D"), Some(TriggerKind::Dbm));
        assert_eq!(parse_kind("s"), Some(TriggerKind::Skill));
        assert_eq!(parse_kind("A"), Some(TriggerKind::Attribute));
        assert_eq!(parse_kind("N"), Some(TriggerKind::NoticeTip));
        assert_eq!(parse_kind("x"), None);
    }

    #[test]
    fn scene_attribute_parser_reads_protocol_scene_id() {
        let mut raw = Vec::new();
        put_varint(6615, &mut raw);
        let mut attr = vec![0x08];
        put_varint(ATTR_SCENE_BASIC_ID, &mut attr);
        attr.push(0x12);
        put_varint(raw.len() as u64, &mut attr);
        attr.extend(raw);
        let mut attrs = vec![0x12];
        put_varint(attr.len() as u64, &mut attrs);
        attrs.extend(attr);
        assert_eq!(scene_id_from_attrs(&attrs), Some(6615));
    }
}
