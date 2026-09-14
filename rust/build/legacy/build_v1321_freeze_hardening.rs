use std::{env, fs, path::{Path, PathBuf}};

mod previous {
    include!("build_v1320_season4_future_proof_fix.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.32.1 freeze hardening {label:?} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn patch_event_tracker(manifest: &Path, out: &Path) {
    let input = manifest.join("src/event_tracker.rs");
    let mut source = fs::read_to_string(&input)
        .expect("read event_tracker.rs for v1.32.1")
        .replace("\r\n", "\n");

    replace_once(&mut source, "const MAX_RULES: usize = 24;", "const MAX_RULES: usize = 64;", "tracker rule capacity");
    replace_once(
        &mut source,
        "pub enum TrackerKind {\n    Buff,\n    Skill,\n}",
        "pub enum TrackerKind {\n    Buff,\n    Skill,\n    Attribute,\n}",
        "attribute tracker enum",
    );
    replace_once(
        &mut source,
        "match self { Self::Buff => \"Buff ID\", Self::Skill => \"Skill ID\" }",
        "match self { Self::Buff => \"Buff ID\", Self::Skill => \"Skill ID\", Self::Attribute => \"Attribute ID\" }",
        "attribute tracker label",
    );
    replace_once(
        &mut source,
        "    rule_states: HashMap<u32, RuleState>,\n    // Raw host UUID + instance ID keeps NPCs and players distinct.",
        "    rule_states: HashMap<u32, RuleState>,\n    // Last value per Attribute rule/target. This is intentionally bounded by the\n    // configured rule set rather than every attribute observed in the world.\n    attribute_values: HashMap<(u32, i64), i64>,\n    // Raw host UUID + instance ID keeps NPCs and players distinct.",
        "attribute tracker state",
    );
    replace_once(
        &mut source,
        "    fn buff(&mut self, settings: &TrackerSettings, host: i64, target_uid: i64, instance_id: i32, base_id: i32, removed: bool, expiry: i64, now: i64) {",
        r#"    fn attribute(&mut self, settings: &TrackerSettings, attr_id: i32, target_uid: i64, value: i64, now: i64) -> bool {
        if !settings.enabled || attr_id <= 0 { return false; }
        let mut changed = false;
        for rule in settings.rules.iter().filter(|r| r.enabled && r.kind == TrackerKind::Attribute && r.event_id == attr_id) {
            if !scope_matches(rule.scope, 0, target_uid, self) { continue; }
            let key = (rule.rule_id, target_uid);
            if self.attribute_values.get(&key).is_some_and(|old| *old == value) { continue; }
            self.attribute_values.insert(key, value);
            let detail = if target_uid > 0 { format!("{} = {value}", scope_uid_label(target_uid, self)) } else { format!("value = {value}") };
            let item = self.rule_states.entry(rule.rule_id).or_default();
            item.count = item.count.saturating_add(1);
            item.last_seen_unix_ms = now;
            item.expires_unix_ms = now.saturating_add(i64::from(rule.hold_seconds) * 1000);
            item.detail = detail;
            changed = true;
        }
        changed
    }

    fn buff(&mut self, settings: &TrackerSettings, host: i64, target_uid: i64, instance_id: i32, base_id: i32, removed: bool, expiry: i64, now: i64) {"#,
        "attribute tracker observer",
    );
    replace_once(
        &mut source,
        r#"                TrackerKind::Buff => {
                    let matching: Vec<_> = self.buff_instances.values().filter(|b|
                        b.base_id == rule.event_id && scope_matches(rule.scope, 0, b.target_uid, self)).collect();"#,
        r#"                TrackerKind::Attribute => {
                    if item.expires_unix_ms <= now { continue; }
                    row.remaining_ms = Some(item.expires_unix_ms - now);
                    row.detail = format!("{} • {} changes", row.detail, item.count);
                }
                TrackerKind::Buff => {
                    let matching: Vec<_> = self.buff_instances.values().filter(|b|
                        b.base_id == rule.event_id && scope_matches(rule.scope, 0, b.target_uid, self)).collect();"#,
        "attribute tracker display row",
    );
    replace_once(
        &mut source,
        "        self.buff_instances.retain(|_, b| new.enabled && new.rules.iter().any(|r|\n            r.enabled && r.kind == TrackerKind::Buff && r.event_id == b.base_id));",
        "        self.buff_instances.retain(|_, b| new.enabled && new.rules.iter().any(|r|\n            r.enabled && r.kind == TrackerKind::Buff && r.event_id == b.base_id));\n        self.attribute_values.retain(|(rule_id, _), _| new.enabled && new.rules.iter().any(|r|\n            r.rule_id == *rule_id && r.enabled && r.kind == TrackerKind::Attribute));",
        "attribute tracker settings reconcile",
    );
    replace_once(
        &mut source,
        "pub fn observe_buff(host: i64, target_uid: i64, instance: i32, base_id: i32, removed: bool, expiry: i64) {",
        r#"pub fn observe_attribute(attr_id: i32, target_uid: i64, value: i64) -> bool {
    if let Some(r) = RUNTIME.get() { if let (Ok(settings), Ok(mut s)) = (r.settings.read(), r.state.lock()) {
        return s.attribute(&settings, attr_id, target_uid, value, now_ms());
    } }
    false
}
pub fn observe_buff(host: i64, target_uid: i64, instance: i32, base_id: i32, removed: bool, expiry: i64) {"#,
        "attribute tracker public observer",
    );
    replace_once(
        &mut source,
        "        s.local_uid = 0; s.rule_states.clear(); s.buff_instances.clear();",
        "        s.local_uid = 0; s.rule_states.clear(); s.attribute_values.clear(); s.buff_instances.clear();",
        "attribute tracker scene clear",
    );
    replace_once(
        &mut source,
        "pub fn reset_counts() {\n    if let Some(r) = RUNTIME.get() { if let Ok(mut s) = r.state.lock() { s.rule_states.clear(); } }\n}",
        "pub fn reset_counts() {\n    if let Some(r) = RUNTIME.get() { if let Ok(mut s) = r.state.lock() { s.rule_states.clear(); s.attribute_values.clear(); } }\n}",
        "attribute tracker count reset",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1321_attribute_tracker_tests {
    use super::*;

    #[test]
    fn tracker_accepts_sixty_four_rules_and_attribute_kind() {
        let mut settings = TrackerSettings::default();
        settings.rules = (1..=80).map(|rule_id| TrackerRule {
            rule_id,
            kind: TrackerKind::Attribute,
            event_id: 11_440,
            ..TrackerRule::default()
        }).collect();
        settings.normalize();
        assert_eq!(settings.rules.len(), 64);
        assert_eq!(TrackerKind::Attribute.label(), "Attribute ID");
    }

    #[test]
    fn attribute_tracker_only_refreshes_when_value_changes() {
        let mut state = RuntimeState::default();
        let settings = TrackerSettings {
            rules: vec![TrackerRule { kind: TrackerKind::Attribute, event_id: 11_440, ..TrackerRule::default() }],
            ..TrackerSettings::default()
        };
        assert!(state.attribute(&settings, 11_440, 123, 55, 1_000));
        assert!(!state.attribute(&settings, 11_440, 123, 55, 1_001));
        assert!(state.attribute(&settings, 11_440, 123, 56, 1_002));
        let rows = state.rows(&settings, 1_003);
        assert_eq!(rows.len(), 1);
        assert!(rows[0].detail.contains("56"));
    }
}
"#);

    fs::write(out.join("event_tracker_v1321_generated.rs"), source)
        .expect("write generated v1.32.1 event tracker");
    println!("cargo:rerun-if-changed={}", input.display());
}

fn patch_tracker_ui(out: &Path) {
    let path = out.join("event_tracker_ui_v1160_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .expect("read generated Event Tracker UI for v1.32.1")
        .replace("\r\n", "\n");

    replace_once(&mut source, "create_static(hwnd,\"Rules (up to 24)\",18,120,300,24);", "create_static(hwnd,\"Rules (up to 64)\",18,120,300,24);", "tracker UI rule label");
    replace_once(&mut source, "state.working.rules.len()>=24", "state.working.rules.len()>=64", "tracker UI add limit");
    replace_once(&mut source, "state.working.rules.len()<24", "state.working.rules.len()<64", "tracker UI add enable limit");
    replace_once(&mut source, "set_combo_items(hwnd,ID_KIND,&[\"Buff ID\",\"Skill ID\"],0);", "set_combo_items(hwnd,ID_KIND,&[\"Buff ID\",\"Skill ID\",\"Attribute ID\"],0);", "tracker UI kind list");
    replace_once(&mut source, "create_static(hwnd,\"Skill display seconds (1–30)\",x,408,250,20);", "create_static(hwnd,\"Skill / attribute display seconds (1–30)\",x,408,280,20);", "tracker UI hold label");
    replace_once(
        &mut source,
        "create_static(hwnd,\"Buff: game countdown or ACTIVE. Skill: observed damage/heal hits; display time is not a cooldown. New rules start observing after Apply.\",18,504,690,42);",
        "create_static(hwnd,\"Buff: game countdown or ACTIVE. Skill: observed combat hits. Attribute: changed values. Display time is not a cooldown.\",18,504,690,42);",
        "tracker UI help text",
    );
    replace_once(&mut source, "ID_KIND => { EnableWindow(GetDlgItem(hwnd,ID_HOLD),(combo_sel(hwnd,ID_KIND)==1) as i32); }", "ID_KIND => { EnableWindow(GetDlgItem(hwnd,ID_HOLD),(combo_sel(hwnd,ID_KIND)!=0) as i32); }", "tracker UI hold enable");
    replace_once(
        &mut source,
        "            || get_check(hwnd,ID_RULE_ENABLED)!=r.enabled || (combo_sel(hwnd,ID_KIND)==1)!=(r.kind==TrackerKind::Skill)\n            || combo_sel(hwnd,ID_SCOPE)!=match r.scope {TrackerScope::Any=>0,TrackerScope::SelfOnly=>1,TrackerScope::Party=>2}\n            || (r.kind==TrackerKind::Skill && get_text(hwnd,ID_HOLD)!=r.hold_seconds.to_string());",
        "            || get_check(hwnd,ID_RULE_ENABLED)!=r.enabled || combo_sel(hwnd,ID_KIND)!=tracker_kind_index(r.kind)\n            || combo_sel(hwnd,ID_SCOPE)!=match r.scope {TrackerScope::Any=>0,TrackerScope::SelfOnly=>1,TrackerScope::Party=>2}\n            || (r.kind!=TrackerKind::Buff && get_text(hwnd,ID_HOLD)!=r.hold_seconds.to_string());",
        "tracker UI dirty kind handling",
    );
    replace_once(
        &mut source,
        "unsafe fn load_selected(hwnd: HWND, state: &UiState) {",
        "fn tracker_kind_index(kind: TrackerKind) -> isize { match kind { TrackerKind::Buff=>0, TrackerKind::Skill=>1, TrackerKind::Attribute=>2 } }\n\nunsafe fn load_selected(hwnd: HWND, state: &UiState) {",
        "tracker UI kind helper",
    );
    replace_once(
        &mut source,
        "    SendMessageW(GetDlgItem(hwnd, ID_KIND), CB_SETCURSEL, if rule.kind == TrackerKind::Buff { 0 } else { 1 }, 0);",
        "    SendMessageW(GetDlgItem(hwnd, ID_KIND), CB_SETCURSEL, tracker_kind_index(rule.kind) as usize, 0);",
        "tracker UI load kind",
    );
    replace_once(&mut source, "    EnableWindow(GetDlgItem(hwnd,ID_HOLD),(rule.kind==TrackerKind::Skill) as i32);", "    EnableWindow(GetDlgItem(hwnd,ID_HOLD),(rule.kind!=TrackerKind::Buff) as i32);", "tracker UI load hold");
    replace_once(
        &mut source,
        "    let Some(id)=positive_number(hwnd,ID_EVENT_ID,i32::MAX,\"Enter a positive numeric Buff or Skill ID.\") else{return false;};\n    let kind=if combo_sel(hwnd,ID_KIND)==1 {TrackerKind::Skill}else{TrackerKind::Buff};\n    let hold=if kind==TrackerKind::Skill {\n        let Some(n)=positive_number(hwnd,ID_HOLD,30,\"Skill display seconds must be from 1 to 30.\") else{return false;};n as u8\n    }else{state.working.rules[index].hold_seconds};",
        "    let Some(id)=positive_number(hwnd,ID_EVENT_ID,i32::MAX,\"Enter a positive numeric Buff, Skill or Attribute ID.\") else{return false;};\n    let kind=match combo_sel(hwnd,ID_KIND){1=>TrackerKind::Skill,2=>TrackerKind::Attribute,_=>TrackerKind::Buff};\n    let hold=if kind!=TrackerKind::Buff {\n        let Some(n)=positive_number(hwnd,ID_HOLD,30,\"Skill / attribute display seconds must be from 1 to 30.\") else{return false;};n as u8\n    }else{state.working.rules[index].hold_seconds};",
        "tracker UI save attribute kind",
    );

    fs::write(path, source).expect("write v1.32.1 Event Tracker UI");
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    patch_event_tracker(&manifest, &out);
    patch_tracker_ui(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1321_freeze_hardening.rs");
}
