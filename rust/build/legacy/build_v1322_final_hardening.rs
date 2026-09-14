use std::{env, fs, path::{Path, PathBuf}};

mod previous {
    include!("build_v1321_regression_fix.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.32.2 final hardening {label:?} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn patch_future_mechanics(out: &Path) {
    let path = out.join("future_mechanics_v1321_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .expect("read generated v1.32.1 future mechanics for v1.32.2")
        .replace("\r\n", "\n");

    replace_once(
        &mut source,
        r#"const DIAGNOSTIC_FILE: &str = "unknown_mechanics.log";"#,
        r#"const DIAGNOSTIC_FILE: &str = "unknown_mechanics.log";
const MAX_DIAGNOSTIC_BYTES: u64 = 4 * 1024 * 1024;"#,
        "mechanics diagnostic size cap",
    );

    replace_once(
        &mut source,
        r#"        let uuid = signed(proto::get_varint_field(delta, 1).unwrap_or(0));
        let target_uid = if uuid != 0 { player_uid(uuid) } else { fallback_uid.max(0) };
        let mut changed = self.observe_attrs_with_uid(target_uid, proto::get_len_field(delta, 2));"#,
        r#"        let uuid = signed(proto::get_varint_field(delta, 1).unwrap_or(0));
        let target_uid = if uuid != 0 { player_uid(uuid) } else { fallback_uid.max(0) };
        let entity_id = if uuid != 0 { uuid } else if target_uid > 0 { (target_uid << 16) | 10 } else { 0 };
        let mut changed = self.observe_attrs_with_identity(entity_id, target_uid, proto::get_len_field(delta, 2));"#,
        "delta raw entity identity",
    );

    replace_once(
        &mut source,
        r#"                let skill_id = proto::get_varint_field(damage, 12).unwrap_or(0) as i32;
                if skill_id <= 0 {
                    continue;
                }"#,
        r#"                let skill_id = proto::get_varint_field(damage, 12).unwrap_or(0) as i32;
                if skill_id == 0 {
                    continue;
                }"#,
        "high-bit observed skill ids",
    );

    replace_once(
        &mut source,
        r#"    fn observe_attrs(&mut self, uuid: i64, attrs: Option<&[u8]>) -> bool {
        self.observe_attrs_with_uid(player_uid(uuid), attrs)
    }

    fn observe_attrs_with_uid(&mut self, target_uid: i64, attrs: Option<&[u8]>) -> bool {"#,
        r#"    fn observe_attrs(&mut self, uuid: i64, attrs: Option<&[u8]>) -> bool {
        self.observe_attrs_with_identity(uuid, player_uid(uuid), attrs)
    }

    fn observe_attrs_with_identity(&mut self, entity_id: i64, target_uid: i64, attrs: Option<&[u8]>) -> bool {"#,
        "attribute raw entity identity",
    );

    replace_once(
        &mut source,
        r#"            changed |= crate::event_tracker::observe_attribute(attr_id, target_uid, value);"#,
        r#"            changed |= crate::event_tracker::observe_attribute_entity(attr_id, entity_id, target_uid, value);"#,
        "attribute tracker entity identity",
    );

    replace_once(
        &mut source,
        r#"                let key = (attr_id, target_uid);"#,
        r#"                let key = (attr_id, entity_id);"#,
        "mechanic attribute dedup key",
    );

    replace_once(
        &mut source,
        r#"                        format!("future:attr:{}:{attr_id}:{target_uid}", self.current_scene_id),"#,
        r#"                        format!("future:attr:{}:{attr_id}:{entity_id}", self.current_scene_id),"#,
        "mechanic attribute row key",
    );

    replace_once(
        &mut source,
        r#"            let kind = parse_kind(fields[1])?;
            let event_id = fields[2].trim().parse::<i32>().ok()?.max(0);
            if event_id == 0 { return None; }"#,
        r#"            let kind = parse_kind(fields[1])?;
            let event_id = parse_event_id(kind, fields[2])?;"#,
        "runtime override event id normalization",
    );

    replace_once(
        &mut source,
        r#"fn parse_kind(raw: &str) -> Option<TriggerKind> {"#,
        r#"fn parse_event_id(kind: TriggerKind, raw: &str) -> Option<i32> {
    let raw = raw.trim();
    if kind == TriggerKind::Skill {
        if let Ok(id) = raw.parse::<i32>() {
            return (id != 0).then_some(id);
        }
        return raw.parse::<u32>().ok().map(|id| id as i32).filter(|id| *id != 0);
    }
    raw.parse::<i32>().ok().filter(|id| *id > 0)
}

fn parse_kind(raw: &str) -> Option<TriggerKind> {"#,
        "runtime override skill id parser",
    );

    replace_once(
        &mut source,
        r#"        let path = root.join(DIAGNOSTIC_FILE);
        let line = format!("{}\t{}\tN\t{}\t{}\n", now_ms(), self.current_scene_id, tip_id, clean.replace('\t', " "));
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
            let _ = file.write_all(line.as_bytes());
        }"#,
        r#"        let path = root.join(DIAGNOSTIC_FILE);
        let line = format!("{}\t{}\tN\t{}\t{}\n", now_ms(), self.current_scene_id, tip_id, clean.replace('\t', " "));
        rotate_diagnostic_if_needed(&path, line.len() as u64);
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
            let _ = file.write_all(line.as_bytes());
        }"#,
        "mechanics diagnostic rotation call",
    );

    replace_once(
        &mut source,
        r#"fn ensure_override_template(path: PathBuf) {"#,
        r#"fn should_rotate_diagnostic(current_bytes: u64, incoming_bytes: u64) -> bool {
    current_bytes.saturating_add(incoming_bytes) > MAX_DIAGNOSTIC_BYTES
}

fn rotate_diagnostic_if_needed(path: &PathBuf, incoming_bytes: u64) {
    let current = fs::metadata(path).map(|meta| meta.len()).unwrap_or(0);
    if !should_rotate_diagnostic(current, incoming_bytes) {
        return;
    }
    let rotated = path.with_extension("log.1");
    let _ = fs::remove_file(&rotated);
    if let Err(err) = fs::rename(path, &rotated) {
        crate::logging::write(format!(
            "future mechanics: could not rotate {}: {err}",
            path.display()
        ));
    }
}

fn ensure_override_template(path: PathBuf) {"#,
        "mechanics diagnostic rotation helper",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1322_final_hardening_tests {
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

    fn len_field(field: u32, payload: &[u8], out: &mut Vec<u8>) {
        put_varint(u64::from(field) << 3 | 2, out);
        put_varint(payload.len() as u64, out);
        out.extend_from_slice(payload);
    }

    fn attrs(attr_id: i32, value: i64) -> Vec<u8> {
        let mut raw = Vec::new();
        put_varint(value as u64, &mut raw);
        let mut attr = Vec::new();
        put_varint(1 << 3, &mut attr);
        put_varint(attr_id as u64, &mut attr);
        len_field(2, &raw, &mut attr);
        let mut attrs = Vec::new();
        len_field(2, &attr, &mut attrs);
        attrs
    }

    fn runtime() -> Runtime {
        Runtime {
            root: None,
            rules: Vec::new(),
            current_scene_id: 0,
            local_uid: 0,
            rows: HashMap::new(),
            attribute_values: HashMap::new(),
            notice_seen: HashSet::new(),
        }
    }

    #[test]
    fn two_npcs_with_same_attribute_keep_distinct_identity() {
        let mut runtime = runtime();
        runtime.rules.push(Rule {
            scene_id: 0,
            kind: TriggerKind::Attribute,
            event_id: 777,
            label: "NPC state".into(),
            duration_ms: 1_000,
            priority: 2,
        });
        let data = attrs(777, 5);
        let npc_a = 0x0001_0001_i64;
        let npc_b = 0x0002_0001_i64;

        assert!(runtime.observe_attrs(npc_a, Some(&data)));
        assert!(runtime.observe_attrs(npc_b, Some(&data)));
        assert_eq!(runtime.attribute_values.len(), 2);
        assert_eq!(runtime.rows.len(), 2);
        assert!(!runtime.observe_attrs(npc_a, Some(&data)));
    }

    #[test]
    fn unsigned_high_bit_skill_id_normalizes_to_wire_i32() {
        assert_eq!(parse_event_id(TriggerKind::Skill, "4294967290"), Some(-6));
        assert_eq!(parse_event_id(TriggerKind::Skill, "-6"), Some(-6));
        assert_eq!(parse_event_id(TriggerKind::Attribute, "4294967290"), None);
    }

    #[test]
    fn mechanics_diagnostic_rotation_has_four_mib_cap() {
        assert!(!should_rotate_diagnostic(MAX_DIAGNOSTIC_BYTES - 1, 1));
        assert!(should_rotate_diagnostic(MAX_DIAGNOSTIC_BYTES, 1));
    }
}
"#);

    fs::write(path, source).expect("write v1.32.2-hardened future mechanics");
}

fn patch_event_tracker(out: &Path) {
    let path = out.join("event_tracker_v1321_generated.rs");
    let mut source = fs::read_to_string(&path)
        .expect("read generated v1.32.1 event tracker for v1.32.2")
        .replace("\r\n", "\n");

    replace_once(
        &mut source,
        r#"            rule.event_id = rule.event_id.max(0);"#,
        r#"            if rule.kind != TrackerKind::Skill {
                rule.event_id = rule.event_id.max(0);
            }"#,
        "tracker high-bit skill normalization",
    );

    replace_once(
        &mut source,
        r#"        if !settings.enabled || skill_id <= 0 { return; }"#,
        r#"        if !settings.enabled || skill_id == 0 { return; }"#,
        "tracker high-bit observed skill",
    );

    replace_once(
        &mut source,
        r#"    fn attribute(&mut self, settings: &TrackerSettings, attr_id: i32, target_uid: i64, value: i64, now: i64) -> bool {"#,
        r#"    fn attribute(&mut self, settings: &TrackerSettings, attr_id: i32, entity_id: i64, target_uid: i64, value: i64, now: i64) -> bool {"#,
        "tracker attribute entity parameter",
    );

    replace_once(
        &mut source,
        r#"            let key = (rule.rule_id, target_uid);"#,
        r#"            let key = (rule.rule_id, entity_id);"#,
        "tracker attribute entity dedup",
    );

    replace_once(
        &mut source,
        r#"        for rule in settings.rules.iter().filter(|r| r.enabled && r.event_id > 0) {"#,
        r#"        for rule in settings.rules.iter().filter(|r| r.enabled && (r.event_id > 0 || (r.kind == TrackerKind::Skill && r.event_id < 0))) {"#,
        "tracker row visibility for high-bit skills",
    );

    replace_once(
        &mut source,
        r#"pub fn observe_attribute(attr_id: i32, target_uid: i64, value: i64) -> bool {
    if let Some(r) = RUNTIME.get() { if let (Ok(settings), Ok(mut s)) = (r.settings.read(), r.state.lock()) {
        return s.attribute(&settings, attr_id, target_uid, value, now_ms());
    } }
    false
}"#,
        r#"pub fn observe_attribute(attr_id: i32, target_uid: i64, value: i64) -> bool {
    observe_attribute_entity(attr_id, target_uid, target_uid, value)
}
pub fn observe_attribute_entity(attr_id: i32, entity_id: i64, target_uid: i64, value: i64) -> bool {
    if let Some(r) = RUNTIME.get() { if let (Ok(settings), Ok(mut s)) = (r.settings.read(), r.state.lock()) {
        return s.attribute(&settings, attr_id, entity_id, target_uid, value, now_ms());
    } }
    false
}"#,
        "tracker raw entity observer",
    );

    replace_once(
        &mut source,
        r#"pub fn remove_entity(host: i64) {
    if let Some(r) = RUNTIME.get() { if let Ok(mut s) = r.state.lock() { s.buff_instances.retain(|(h, _), _| *h != host); } }
}"#,
        r#"pub fn remove_entity(host: i64) {
    if let Some(r) = RUNTIME.get() { if let Ok(mut s) = r.state.lock() {
        s.buff_instances.retain(|(h, _), _| *h != host);
        s.attribute_values.retain(|(_, entity_id), _| *entity_id != host);
    } }
}"#,
        "tracker entity cleanup",
    );

    replace_once(
        &mut source,
        r#"pub fn rule_label(rule: &TrackerRule) -> String {
    if !rule.label.trim().is_empty() { rule.label.trim().to_string() } else { format!("{} {}", rule.kind.label(), rule.event_id) }
}"#,
        r#"pub fn rule_label(rule: &TrackerRule) -> String {
    if !rule.label.trim().is_empty() {
        rule.label.trim().to_string()
    } else {
        let event_id = if rule.kind == TrackerKind::Skill {
            (rule.event_id as u32).to_string()
        } else {
            rule.event_id.to_string()
        };
        format!("{} {event_id}", rule.kind.label())
    }
}"#,
        "tracker unsigned skill label",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1322_final_hardening_tests {
    use super::*;

    fn attribute_settings() -> TrackerSettings {
        TrackerSettings {
            enabled: true,
            max_visible: 6,
            rules: vec![TrackerRule {
                rule_id: 1,
                enabled: true,
                kind: TrackerKind::Attribute,
                event_id: 777,
                label: "NPC state".into(),
                scope: TrackerScope::Any,
                hold_seconds: 6,
            }],
        }
    }

    #[test]
    fn two_npcs_with_same_attribute_do_not_share_dedup_state() {
        let mut state = RuntimeState::default();
        let settings = attribute_settings();
        assert!(state.attribute(&settings, 777, 1001, 0, 5, 1_000));
        assert!(state.attribute(&settings, 777, 2001, 0, 5, 1_001));
        assert!(!state.attribute(&settings, 777, 1001, 0, 5, 1_002));
        assert_eq!(state.attribute_values.len(), 2);
        assert_eq!(state.rule_states.get(&1).map(|row| row.count), Some(2));
    }

    #[test]
    fn high_bit_skill_id_survives_normalization_and_matches() {
        let mut settings = TrackerSettings {
            enabled: true,
            max_visible: 6,
            rules: vec![TrackerRule {
                rule_id: 1,
                enabled: true,
                kind: TrackerKind::Skill,
                event_id: -6,
                label: String::new(),
                scope: TrackerScope::Any,
                hold_seconds: 6,
            }],
        };
        settings.normalize();
        assert_eq!(settings.rules[0].event_id, -6);

        let mut state = RuntimeState::default();
        state.skill(&settings, -6, 7, 0, 1_000);
        assert_eq!(state.rule_states.get(&1).map(|row| row.count), Some(1));
        assert_eq!(state.rows(&settings, 1_001).len(), 1);
        assert!(rule_label(&settings.rules[0]).ends_with("4294967290"));
    }
}
"#);

    fs::write(path, source).expect("write v1.32.2-hardened event tracker");
}

fn patch_tracker_ui(out: &Path) {
    let path = out.join("event_tracker_ui_v1160_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .expect("read generated event tracker UI for v1.32.2")
        .replace("\r\n", "\n");

    replace_once(
        &mut source,
        r#"        dirty |= get_text(hwnd,ID_EVENT_ID)!=r.event_id.to_string() || get_text(hwnd,ID_LABEL)!=r.label"#,
        r#"        dirty |= get_text(hwnd,ID_EVENT_ID)!=event_id_text(r.kind, r.event_id) || get_text(hwnd,ID_LABEL)!=r.label"#,
        "tracker UI unsigned skill dirty check",
    );

    replace_once(
        &mut source,
        r#"        let text = format!("[{status}] {} {}  |  {}  |  {}", rule.kind.label(), rule.event_id, event_tracker::rule_label(rule), rule.scope.label());"#,
        r#"        let text = format!("[{status}] {} {}  |  {}  |  {}", rule.kind.label(), event_id_text(rule.kind, rule.event_id), event_tracker::rule_label(rule), rule.scope.label());"#,
        "tracker UI unsigned skill list",
    );

    replace_once(
        &mut source,
        r#"    set_text(hwnd, ID_EVENT_ID, &rule.event_id.to_string());"#,
        r#"    set_text(hwnd, ID_EVENT_ID, &event_id_text(rule.kind, rule.event_id));"#,
        "tracker UI unsigned skill editor",
    );

    replace_once(
        &mut source,
        r#"    let Some(id)=positive_number(hwnd,ID_EVENT_ID,i32::MAX,"Enter a positive numeric Buff, Skill or Attribute ID.") else{return false;};
    let kind=match combo_sel(hwnd,ID_KIND){1=>TrackerKind::Skill,2=>TrackerKind::Attribute,_=>TrackerKind::Buff};"#,
        r#"    let kind=match combo_sel(hwnd,ID_KIND){1=>TrackerKind::Skill,2=>TrackerKind::Attribute,_=>TrackerKind::Buff};
    let Some(id)=parse_event_id_text(kind,&get_text(hwnd,ID_EVENT_ID)) else{
        message_error(hwnd,"Enter a positive Buff/Attribute ID or a non-zero 32-bit Skill ID.");
        let edit=GetDlgItem(hwnd,ID_EVENT_ID);crate::ui::SetFocus(edit);SendMessageW(edit,0x00B1,0,-1isize);
        return false;
    };"#,
        "tracker UI high-bit skill parser",
    );

    replace_once(
        &mut source,
        r#"unsafe fn set_editor_enabled(hwnd: HWND, enabled: bool) {"#,
        r#"fn event_id_text(kind: TrackerKind, id: i32) -> String {
    if kind == TrackerKind::Skill { (id as u32).to_string() } else { id.to_string() }
}

fn parse_event_id_text(kind: TrackerKind, raw: &str) -> Option<i32> {
    let raw = raw.trim();
    if kind == TrackerKind::Skill {
        return raw.parse::<u32>().ok().map(|id| id as i32).filter(|id| *id != 0);
    }
    raw.parse::<i32>().ok().filter(|id| *id > 0)
}

unsafe fn set_editor_enabled(hwnd: HWND, enabled: bool) {"#,
        "tracker UI skill id helpers",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1322_skill_id_ui_tests {
    use super::*;

    #[test]
    fn high_bit_skill_id_round_trips_as_unsigned_text() {
        assert_eq!(parse_event_id_text(TrackerKind::Skill, "4294967290"), Some(-6));
        assert_eq!(event_id_text(TrackerKind::Skill, -6), "4294967290");
        assert_eq!(parse_event_id_text(TrackerKind::Attribute, "4294967290"), None);
    }
}
"#);

    fs::write(path, source).expect("write v1.32.2-hardened event tracker UI");
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_future_mechanics(&out);
    patch_event_tracker(&out);
    patch_tracker_ui(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1322_final_hardening.rs");
}
