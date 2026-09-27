use std::{env, fs, path::{Path, PathBuf}};

mod previous {
    include!("build_v1410_event_discovery.rs");
    pub fn run() { main(); }
}

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){
    let count=source.matches(from).count();
    assert_eq!(count,1,"v1411 {label} expected one anchor, found {count}");
    *source=source.replacen(from,to,1);
}

fn patch_tracker(out:&Path){
    let path=out.join("event_tracker_v1321_generated.rs");
    let mut source=fs::read_to_string(&path).expect("read v1410 event tracker").replace("\r\n","\n");

    replace_once(&mut source,
        "        let row=self.discovery.entry((kind,event_id)).or_default();\n        row.count=row.count.saturating_add(1);row.last_seen_unix_ms=now;row.detail=detail;row.suggested_scope=suggested_scope;",
        "        let row=self.discovery.entry((kind,event_id)).or_default();\n        let duplicate=row.last_seen_unix_ms>0&&now.saturating_sub(row.last_seen_unix_ms)<=200;\n        if !duplicate{row.count=row.count.saturating_add(1);}\n        row.last_seen_unix_ms=now;row.detail=detail;row.suggested_scope=suggested_scope;",
        "discovery duplicate suppression");

    replace_once(&mut source,
        "            let item = self.rule_states.entry(rule.rule_id).or_default();\n            item.count = item.count.saturating_add(1);\n            item.last_seen_unix_ms = now;",
        "            let item = self.rule_states.entry(rule.rule_id).or_default();\n            let duplicate=item.last_seen_unix_ms>0&&now.saturating_sub(item.last_seen_unix_ms)<=200;\n            if !duplicate{item.count=item.count.saturating_add(1);}\n            item.last_seen_unix_ms = now;",
        "skill event duplicate suppression");

    replace_once(&mut source,
        "                    row.detail = format!(\"{} • {} hits\", row.detail, item.count);",
        "                    row.detail = format!(\"{} • {} events\", row.detail, item.count);",
        "skill tracker event wording");

    replace_once(&mut source,
        "        rows.sort_by(|a,b|b.last_seen_unix_ms.cmp(&a.last_seen_unix_ms).then_with(||a.event_id.cmp(&b.event_id)));",
r#"        let scope_rank=|scope:TrackerScope|match scope{TrackerScope::SelfOnly=>0u8,TrackerScope::Party=>1,TrackerScope::Any=>2};
        rows.sort_by(|a,b|scope_rank(a.suggested_scope).cmp(&scope_rank(b.suggested_scope))
            .then_with(||b.last_seen_unix_ms.cmp(&a.last_seen_unix_ms))
            .then_with(||a.event_id.cmp(&b.event_id)));"#,
        "self first discovery ordering");

    replace_once(&mut source,
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
r#"pub fn rule_label(rule: &TrackerRule) -> String {
    if !rule.label.trim().is_empty() { return rule.label.trim().to_string(); }
    if let Some(name)=discovery_event_name(rule.kind,rule.event_id){return name.to_string();}
    if rule.kind==TrackerKind::Attribute {
        if let Some((_,name))=crate::feature_settings::ATTRIBUTE_CATALOG.iter().find(|(id,_)|*id==rule.event_id){
            return (*name).to_string();
        }
    }
    let event_id=if rule.kind==TrackerKind::Skill{(rule.event_id as u32).to_string()}else{rule.event_id.to_string()};
    format!("{} {event_id}",rule.kind.label())
}"#,
        "automatic rule names");

    source.push_str(r#"

#[cfg(test)]
mod v1411_event_tracker_polish_tests {
    use super::*;

    #[test]
    fn known_manual_rules_get_names_without_custom_labels(){
        let skill=TrackerRule{kind:TrackerKind::Skill,event_id:1241,..TrackerRule::default()};
        assert_eq!(rule_label(&skill),"Frostbeam");
        let attr=TrackerRule{kind:TrackerKind::Attribute,event_id:crate::feature_settings::ATTR_LUCKY,..TrackerRule::default()};
        assert_eq!(rule_label(&attr),"Luck");
    }

    #[test]
    fn rapid_cast_and_hit_are_counted_as_one_event(){
        let mut state=RuntimeState::default();
        state.local_uid=42;
        state.discovery_active=true;
        let settings=TrackerSettings{rules:vec![TrackerRule{kind:TrackerKind::Skill,event_id:1241,scope:TrackerScope::SelfOnly,..TrackerRule::default()}],..TrackerSettings::default()};
        state.skill(&settings,1241,42,0,1_000);
        state.skill(&settings,1241,42,0,1_120);
        assert_eq!(state.discovery_rows()[0].count,1);
        let rows=state.rows(&settings,1_130);
        assert_eq!(rows[0].count,1);
        assert!(rows[0].detail.contains("events"));
    }

    #[test]
    fn discovery_prefers_self_events_over_world_noise(){
        let mut state=RuntimeState::default();
        state.local_uid=42;
        state.discovery_active=true;
        state.discover_event(TrackerKind::Skill,100,0,0,2_000);
        state.discover_event(TrackerKind::Skill,101,42,0,1_000);
        let rows=state.discovery_rows();
        assert_eq!(rows[0].event_id,101);
        assert_eq!(rows[0].suggested_scope,TrackerScope::SelfOnly);
    }
}
"#);
    fs::write(&path,source).expect("write v1411 event tracker");
}

fn patch_telemetry(out:&Path){
    let path=out.join("telemetry_v170_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read telemetry for v1411").replace("\r\n","\n");

    replace_once(&mut source,
r#"            if let Some(skill) = attr_varint(attrs, ATTR_SKILL_ID).map(|value| value as i32) {
                if let Some((label, duration, priority)) = focused_skill_rule_for_scene(self.current_scene_id, skill) {"#,
r#"            if let Some(skill) = attr_varint(attrs, ATTR_SKILL_ID).map(|value| value as i32) {
                // Tracked skills and discovery listen to actual skill activation too, so utility,
                // movement and support skills can be found even with no damage hit.
                crate::event_tracker::observe_skill(skill, if entity_kind(uuid)==ENTITY_PLAYER{uuid>>16}else{0}, 0);
                if let Some((label, duration, priority)) = focused_skill_rule_for_scene(self.current_scene_id, skill) {"#,
        "non damage skill discovery");

    fs::write(&path,source).expect("write v1411 telemetry");
}

fn patch_ui(out:&Path){
    let path=out.join("event_tracker_ui_v1160_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read tracker UI for v1411").replace("\r\n","\n");

    replace_once(&mut source,
        "Easy setup: start listening, trigger the buff or combat skill in game, then track the event that appears.",
        "Easy setup: Start listening clears old results. Trigger one action in game, choose it below, then Track selected saves it immediately.",
        "beginner instructions");

    replace_once(&mut source,
r#"    if let Some(index)=state.working.rules.iter().position(|r|r.kind==found.kind&&r.event_id==found.event_id){
        state.selected=Some(index);refresh_list(hwnd,state);load_selected(hwnd,state);return;
    }"#,
r#"    if let Some(index)=state.working.rules.iter().position(|r|r.kind==found.kind&&r.event_id==found.event_id){
        state.selected=Some(index);refresh_list(hwnd,state);load_selected(hwnd,state);
        event_tracker::set_discovery_active(false);refresh_discovery(hwnd,state);return;
    }"#,
        "pause when already tracked");

    replace_once(&mut source,
r#"    state.working.rules.push(TrackerRule{rule_id,enabled:true,kind:found.kind,event_id:found.event_id,label,scope:found.suggested_scope,hold_seconds:6});
    state.selected=Some(state.working.rules.len()-1);refresh_list(hwnd,state);load_selected(hwnd,state);refresh_discovery(hwnd,state);
}"#,
r#"    state.working.rules.push(TrackerRule{rule_id,enabled:true,kind:found.kind,event_id:found.event_id,label,scope:found.suggested_scope,hold_seconds:6});
    state.selected=Some(state.working.rules.len()-1);refresh_list(hwnd,state);load_selected(hwnd,state);
    if apply_settings(hwnd,state){event_tracker::set_discovery_active(false);}
    refresh_discovery(hwnd,state);
}"#,
        "save tracked selection immediately");

    source.push_str(r#"

#[cfg(test)]
mod v1411_event_tracker_ui_polish_tests {
    use super::*;
    #[test]
    fn discovery_action_is_a_real_save_action(){
        assert_ne!(ID_DISCOVERY_ADD,ID_APPLY);
    }
}
"#);

    fs::write(&path,source).expect("write v1411 tracker UI");
}

fn main(){
    previous::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_tracker(&out);
    patch_telemetry(&out);
    patch_ui(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1411_event_tracker_polish.rs");
}
