use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_pixel_strict_qa_regression.rs");
    pub fn run() { main(); }
}

fn replace_count(source: &mut String, from: &str, to: &str, expected: usize, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, expected, "v1.31.1 {label} expected {expected}, found {count}");
    *source = source.replace(from, to);
}

fn replace_required(source: &mut String, from: &str, to: &str, label: &str) {
    replace_count(source, from, to, 1, label);
}

fn replace_region(source: &mut String, start: &str, end: &str, replacement: &str, label: &str) {
    let start_at = source.find(start).unwrap_or_else(|| panic!("v1.31.1 {label}: start marker missing"));
    let end_at = source[start_at..]
        .find(end)
        .map(|offset| start_at + offset)
        .unwrap_or_else(|| panic!("v1.31.1 {label}: end marker missing"));
    source.replace_range(start_at..end_at, replacement);
}

fn patch_telemetry(out: &PathBuf) {
    let path = out.join("telemetry_v170_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read final telemetry source: {e}"))
        .replace("\r\n", "\n");

    // Party roster changes may update metadata and Party scope, but must not own
    // encounter/combat lifetime or clear accumulated meter rows.
    replace_count(&mut source, "            self.retain_team_rows();\n", "", 2, "remove party-driven combat retention");
    replace_region(
        &mut source,
        "    fn retain_team_rows(&mut self) {",
        "    fn scan_team_members(&mut self, data: &[u8]) {",
        "",
        "remove obsolete party row pruning helper",
    );
    replace_required(
        &mut source,
        "            crate::event_tracker::set_party(&self.team);\n            for uid in self.team.clone() {\n                self.combat.entry(uid).or_default();\n            }\n            self.emit_dps();",
        "            crate::event_tracker::set_party(&self.team);\n            self.emit_dps();",
        "party join does not create combat rows",
    );
    replace_required(
        &mut source,
        "            self.combat.entry(member.uid).or_default();\n",
        "",
        "team metadata does not create combat rows",
    );

    // actor_uuid is kept as current world-proximity evidence. Party packets do
    // not set it; near-world packets do, and disappearance clears it without
    // destroying the player's accumulated combat totals.
    replace_required(
        &mut source,
        "        self.entities.clear();\n        self.combat.clear();",
        "        self.entities.clear();\n        for player in self.players.values_mut() { player.actor_uuid = 0; }\n        self.combat.clear();",
        "scene clears stale proximity markers",
    );
    replace_required(
        &mut source,
        "            self.update_entity_from_attrs(uuid, proto::get_len_field(entity, 3));",
        "            if entity_kind(uuid) == ENTITY_PLAYER { let uid = uuid >> 16; if uid > 0 { self.players.entry(uid).or_default().actor_uuid = uuid; } }\n            self.update_entity_from_attrs(uuid, proto::get_len_field(entity, 3));",
        "near entity marks player visible",
    );
    replace_required(
        &mut source,
        "            self.entities.remove(&uuid);\n            self.enrage_instances.remove(&uuid);",
        "            self.entities.remove(&uuid);\n            if entity_kind(uuid) == ENTITY_PLAYER { let uid = uuid >> 16; if uid > 0 && uid != self.local_uid { if let Some(player) = self.players.get_mut(&uid) { player.actor_uuid = 0; } } }\n            self.enrage_instances.remove(&uuid);",
        "near disappearance clears player visibility",
    );
    replace_required(
        &mut source,
        "        if uuid == 0 {\n            return;\n        }\n\n        if let Some(attrs) = proto::get_len_field(delta, 2) {",
        "        if uuid == 0 {\n            return;\n        }\n        if entity_kind(uuid) == ENTITY_PLAYER { let uid = uuid >> 16; if uid > 0 { self.players.entry(uid).or_default().actor_uuid = uuid; } }\n\n        if let Some(attrs) = proto::get_len_field(delta, 2) {",
        "near delta refreshes player visibility",
    );

    let team_start = source.find("    fn handle_team(&mut self").expect("patched handle_team");
    let team_end = source[team_start..].find("    fn scan_team_members").map(|v| team_start + v).expect("patched scan_team_members");
    let team_block = &source[team_start..team_end];
    assert!(!team_block.contains("combat.entry"), "party handler still mutates combat rows");
    assert!(!source.contains("retain_team_rows"), "obsolete party combat pruning survived");
    assert!(source.contains("player.actor_uuid = 0"), "proximity disappearance contract missing");

    fs::write(path, source).unwrap_or_else(|e| panic!("write final telemetry source: {e}"));
}

fn patch_feature_overlay(out: &PathBuf) {
    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read final feature overlay: {e}"))
        .replace("\r\n", "\n");

    replace_region(
        &mut source,
        "#[derive(Clone)]\nenum MechanicDisplayRow{",
        "fn dps_compact_row_h",
        include_str!("v1311_mechanic_group_block.txt"),
        "mechanic/tracker grouping block",
    );
    replace_region(
        &mut source,
        "unsafe fn paint_mechanics",
        "fn mechanics_consumable_row_color",
        include_str!("v1311_mechanics_paint_block.txt"),
        "mechanics grouped-row painter",
    );
    replace_region(
        &mut source,
        "fn mechanics_scroll_metrics",
        "unsafe fn clamp_scroll",
        "fn mechanics_scroll_metrics(state:&State,bottom:i32)->(usize,usize,i32){let tracked=state.features.read().map(|f|f.mechanic_attributes.tracked.iter().filter(|id|state.mechanics.tracked_attributes.iter().any(|a|a.attr_id==**id)).count()).unwrap_or(0);let attr_rows=mechanic_attr_rows(tracked);let top=TOOLBAR_H+5+attr_rows as i32*MECH_ATTR_H+MECH_CONSUMABLE_H;let now=now_ms();let total=ordered_mechanic_rows(state,now).len();let visible=(((bottom-top)/MECH_ROW_H).max(0)as usize).min(MAX_MECHANIC_ROWS_VISIBLE);(total,visible,top)}\n",
        "group-aware mechanics scrollbar",
    );

    // Only rows with current world-proximity evidence are presented. Keep self
    // visible and suppress unresolved team placeholders injected by compatibility
    // layers before real nearby entity data exists.
    replace_required(
        &mut source,
        "fn meter_rows(state:&State)->Vec<&DpsRow>{let snapshot=view_snapshot(state);let settings=state.features.read().map(|f|f.clone()).unwrap_or_default();let mut rows=sorted_rows(snapshot,state.sort_mode);rows.retain(|row|{if settings.meter.always_show_self&&row.is_local{return true;}(!settings.meter.party_only||row.is_party)&&(!settings.meter.only_contributors||mode_metric(row,state.sort_mode)>0)});rows}",
        "fn meter_row_has_nearby_evidence(row:&DpsRow)->bool{if row.is_local{return true;}if row.actor_uuid==0{return false;}let unresolved=row.damage==0&&row.healing==0&&row.damage_taken==0&&row.skills.is_empty()&&row.name==format!(\"Player {}\",row.uid);!unresolved}\nfn meter_rows(state:&State)->Vec<&DpsRow>{let snapshot=view_snapshot(state);let settings=state.features.read().map(|f|f.clone()).unwrap_or_default();let mut rows=sorted_rows(snapshot,state.sort_mode);rows.retain(|row|{if !meter_row_has_nearby_evidence(row){return false;}if settings.meter.always_show_self&&row.is_local{return true;}(!settings.meter.party_only||row.is_party)&&(!settings.meter.only_contributors||mode_metric(row,state.sort_mode)>0)});rows}",
        "nearby-only meter rows",
    );

    let spec_start = source.find("fn spec_color(row:&DpsRow)->u32").expect("spec color helper");
    let text_on = source[spec_start..].find("fn text_on(color:u32)->u32").map(|v| spec_start + v).expect("text_on helper");
    let spec_block = source[spec_start..text_on].to_string();
    let replacement = format!("{}{}", spec_block, include_str!("v1311_class_accent_helpers.txt"));
    source.replace_range(spec_start..text_on, &replacement);

    replace_required(
        &mut source,
        "fill(hdc,&RECT{left:r.left,top:r.top,right:r.left+2,bottom:bar_top},if row.is_dead{crate::ui_modern::BPSR_DANGER}else{class_bg});SelectObject(hdc,secondary_font);SetTextColor(hdc,if row.is_local{crate::ui_modern::BPSR_DANGER}else{crate::ui_modern::BPSR_TEXT_SECONDARY});draw(hdc,&rank.to_string(),RECT{left:r.left+3,top:r.top,right:r.left+21,bottom:content_bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);",
        "fill(hdc,&RECT{left:r.left,top:r.top,right:r.left+2,bottom:bar_top},if row.is_dead{crate::ui_modern::BPSR_DANGER}else{spec_accent_color(row)});SelectObject(hdc,if row.is_local{primary_font}else{secondary_font});SetTextColor(hdc,if row.is_local{crate::ui_modern::BPSR_DANGER}else{crate::ui_modern::BPSR_TEXT_SECONDARY});draw(hdc,&rank.to_string(),RECT{left:r.left+3,top:r.top,right:r.left+21,bottom:content_bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);",
        "normal meter class rail and bold self rank",
    );
    replace_required(
        &mut source,
        "SetTextColor(hdc,crate::ui_modern::BPSR_TEXT_SECONDARY);draw(hdc,&secondary,RECT{left:layout.spec_left,top:r.top,right:layout.spec_right,bottom:content_bottom},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);",
        "paint_dps_secondary_colored(hdc,row,&secondary,RECT{left:layout.spec_left,top:r.top,right:layout.spec_right,bottom:content_bottom});",
        "light class/spec label",
    );
    replace_required(
        &mut source,
        "unsafe fn paint_compact_player(hdc:HDC,state:&State,row:&DpsRow,rank:usize,r:RECT,show_total:bool){let class_bg=spec_color(row);let bg=if row.is_dead{crate::ui_modern::tonal_danger_surface()}else{crate::ui_modern::tonal_class_surface(class_bg)};fill(hdc,&r,bg);if row.is_local{",
        "unsafe fn paint_compact_player(hdc:HDC,state:&State,row:&DpsRow,rank:usize,r:RECT,show_total:bool){let class_bg=spec_color(row);let bg=if row.is_dead{crate::ui_modern::tonal_danger_surface()}else{crate::ui_modern::tonal_class_surface(class_bg)};fill(hdc,&r,bg);fill(hdc,&RECT{left:r.left,top:r.top,right:r.left+2,bottom:r.bottom},if row.is_dead{crate::ui_modern::BPSR_DANGER}else{spec_accent_color(row)});if row.is_local{",
        "compact meter class rail",
    );
    replace_required(
        &mut source,
        "SelectObject(hdc,dps_secondary_font(state));SetTextColor(hdc,if row.is_local{crate::ui_modern::BPSR_DANGER}else{crate::ui_modern::BPSR_TEXT_SECONDARY});draw(hdc,&rank.to_string(),RECT{left:r.left+3,top:r.top,right:r.left+rank_w,bottom:r.bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);",
        "SelectObject(hdc,if row.is_local{dps_primary_font(state)}else{dps_secondary_font(state)});SetTextColor(hdc,if row.is_local{crate::ui_modern::BPSR_DANGER}else{crate::ui_modern::BPSR_TEXT_SECONDARY});draw(hdc,&rank.to_string(),RECT{left:r.left+3,top:r.top,right:r.left+rank_w,bottom:r.bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);",
        "compact bold self rank",
    );
    replace_required(
        &mut source,
        "fill(hdc,&r,bg);fill(hdc,&RECT{left:r.left,top:r.top,right:r.left+3,bottom:r.bottom},if row.is_dead{crate::ui_modern::BPSR_DANGER}else{class_bg});if row.is_local{",
        "fill(hdc,&r,bg);fill(hdc,&RECT{left:r.left,top:r.top,right:r.left+3,bottom:r.bottom},if row.is_dead{crate::ui_modern::BPSR_DANGER}else{spec_accent_color(row)});if row.is_local{",
        "raid meter class rail",
    );
    replace_required(
        &mut source,
        "SelectObject(hdc,dps_secondary_font(state));SetTextColor(hdc,base);draw(hdc,&rank.to_string(),RECT{left:r.left+3,top:r.top,right:r.left+rank_w,bottom:r.bottom-2},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);",
        "SelectObject(hdc,if row.is_local{dps_primary_font(state)}else{dps_secondary_font(state)});SetTextColor(hdc,if row.is_local{crate::ui_modern::BPSR_DANGER}else{base});draw(hdc,&rank.to_string(),RECT{left:r.left+3,top:r.top,right:r.left+rank_w,bottom:r.bottom-2},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);",
        "raid bold red self rank",
    );

    source.push_str(r#"
#[cfg(test)]
mod v1311_party_tracker_meter_tests{
    use super::*;
    #[test]fn requested_special_tracker_ids_have_stable_shared_row_order(){let ids=[55226,2_110_049,2_110_050,2_110_055,2_110_056,2_110_065];for(index,id)in ids.into_iter().enumerate(){let(_,order)=special_tracker_name(id).expect("special tracker");assert_eq!(order,index);}}
    #[test]fn same_mechanic_and_timer_combines_targets(){let now=1_000_i64;let make=|key:&str,target:&str,expiry:i64|crate::model::MechanicRow{key:key.into(),label:"Hit order #1".into(),target:Some(target.into()),expires_unix_ms:expiry,priority:3,..Default::default()};let rows=group_mechanics(vec![make("a","ValidStrike",3_000),make("b","AlterTI",3_080)],now);assert_eq!(rows.len(),1);assert_eq!(rows[0].targets.len(),2);}
    #[test]fn light_spec_accent_is_distinct_from_dark_row_surface(){let row=DpsRow{subprofession_name:"Smite".into(),..Default::default()};assert_ne!(spec_accent_color(&row),spec_color(&row));let other=DpsRow{subprofession_name:"Lifebind".into(),..Default::default()};assert_ne!(spec_accent_color(&row),spec_accent_color(&other));}
    #[test]fn unresolved_remote_party_placeholder_is_not_nearby(){let row=DpsRow{uid:77,actor_uuid:(77_i64<<16)|(10<<6),name:"Player 77".into(),..Default::default()};assert!(!meter_row_has_nearby_evidence(&row));let local=DpsRow{is_local:true,..Default::default()};assert!(meter_row_has_nearby_evidence(&local));}
}
"#);

    for required in ["55226", "2_110_049", "2_110_065", "TrackerGroup", "spec_accent_color", "meter_row_has_nearby_evidence", "paint_dps_secondary_colored"] {
        assert!(source.contains(required), "v1.31.1 feature contract missing {required}");
    }
    fs::write(path, source).unwrap_or_else(|e| panic!("write final feature overlay: {e}"));
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_telemetry(&out);
    patch_feature_overlay(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1311_party_tracker_meter.rs");
    println!("cargo:rerun-if-changed=build/legacy/v1311_mechanic_group_block.txt");
    println!("cargo:rerun-if-changed=build/legacy/v1311_mechanics_paint_block.txt");
    println!("cargo:rerun-if-changed=build/legacy/v1311_class_accent_helpers.txt");
}
