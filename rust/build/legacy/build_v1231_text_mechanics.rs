use std::{env, fs, path::{Path, PathBuf}};

mod prior {
    include!("build_v1231_mechanics.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.23.1 text-mechanics patch {label} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn patch_telemetry(out: &Path) {
    let path = out.join("telemetry_v170_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .expect("read v1.23.1 scene-aware telemetry")
        .replace("\r\n", "\n");

    // CN has a full spatial minimap. ReadyAlert intentionally does not. Keep the
    // complete CN scene inventory for parity audits, but only render mechanics
    // whose useful instruction survives as text/timer/target information.
    replace_once(
        &mut source,
        "fn monster_rule_for_scene(scene_id:i32,id:i32)->Option<(&'static str,u64,u8)>{\n    scene_tracks_monster(scene_id,id).then(||monster_rule(id)).flatten()\n}\nfn skill_rule_for_scene(scene_id:i32,id:i32)->Option<(&'static str,u64,u8)>{\n    scene_tracks_skill(scene_id,id).then(||skill_rule(id)).flatten()\n}\nfn buff_rule_for_scene(scene_id:i32,id:i32)->Option<(&'static str,u64,u8)>{\n    scene_tracks_buff(scene_id,id).then(||buff_rule(id)).flatten()\n}",
        r#"fn monster_rule_for_scene(scene_id:i32,id:i32)->Option<(&'static str,u64,u8)>{
    scene_tracks_monster(scene_id,id).then(||monster_rule(id)).flatten()
}
fn skill_rule_for_scene(scene_id:i32,id:i32)->Option<(&'static str,u64,u8)>{
    scene_tracks_skill(scene_id,id).then(||skill_rule(id)).flatten()
}
fn buff_rule_for_scene(scene_id:i32,id:i32)->Option<(&'static str,u64,u8)>{
    scene_tracks_buff(scene_id,id).then(||buff_rule(id)).flatten()
}

fn map_only_monster_mechanic(scene_id:i32,id:i32)->bool{
    match scene_id{
        // Correct-portal value is its location/color on CN's map; a generic
        // "correct portal" line cannot tell ReadyAlert users which portal.
        1150|1151|1152=>id==2106,
        // Reef wave lanes and Tina pizza sectors depend on live position/facing.
        6563|6564|6565=>matches!(id,3340219|3340220),
        1631|1632|1633=>matches!(id,300086|300089),
        _=>false,
    }
}

fn map_only_skill_mechanic(scene_id:i32,id:i32)->bool{
    // Reef pizza skill 3340245 only becomes actionable after CN draws the live
    // facing-dependent danger sectors. Do not emit an incomplete text warning.
    matches!(scene_id,6563|6564|6565)&&id==3340245
}

fn map_only_buff_mechanic(scene_id:i32,id:i32)->bool{
    match scene_id{
        // Cursed Tomb blue/gold tower-complete buffs are minimap color state;
        // activation/tower timers remain as normal ReadyAlert text rows.
        6513|6514|6515=>matches!(id,884102|884103),
        // Preset Return requires CN's source-position -> floor-cell mapping.
        13021|13022|13023=>matches!(id,829372|829373|829374),
        // Reef orange/purple markers only rotate/color the spatial pizza sectors.
        6563|6564|6565=>matches!(id,883633|883634),
        // Wasteland swap/penalty state is consumed by CN's orb/map renderer;
        // player pair pattern + settle rows carry the useful text mechanic.
        6615=>matches!(id,884661|884664),
        _=>false,
    }
}

fn text_monster_rule_for_scene(scene_id:i32,id:i32)->Option<(&'static str,u64,u8)>{
    if map_only_monster_mechanic(scene_id,id){None}else{monster_rule_for_scene(scene_id,id)}
}
fn text_skill_rule_for_scene(scene_id:i32,id:i32)->Option<(&'static str,u64,u8)>{
    if map_only_skill_mechanic(scene_id,id){None}else{skill_rule_for_scene(scene_id,id)}
}
fn text_buff_rule_for_scene(scene_id:i32,id:i32)->Option<(&'static str,u64,u8)>{
    if map_only_buff_mechanic(scene_id,id){None}else{buff_rule_for_scene(scene_id,id)}
}"#,
        "text-only mechanic filters",
    );

    replace_once(
        &mut source,
        "if let Some((label, duration, priority)) = monster_rule_for_scene(self.current_scene_id, monster_id) {",
        "if let Some((label, duration, priority)) = text_monster_rule_for_scene(self.current_scene_id, monster_id) {",
        "filter map-only monster mechanics",
    );
    replace_once(
        &mut source,
        "if let Some((label, duration, priority)) = skill_rule_for_scene(self.current_scene_id, skill) {",
        "if let Some((label, duration, priority)) = text_skill_rule_for_scene(self.current_scene_id, skill) {",
        "filter map-only skill mechanics",
    );
    replace_once(
        &mut source,
        "let Some((base_label, default_duration, priority)) = buff_rule_for_scene(self.current_scene_id, base_id) else { continue; };",
        "let Some((base_label, default_duration, priority)) = text_buff_rule_for_scene(self.current_scene_id, base_id) else { continue; };",
        "filter map-only delta buff mechanics",
    );
    replace_once(
        &mut source,
        "            if buff_rule_for_scene(self.current_scene_id,base_id).is_some(){snapshot_scene_instances.insert(buff_uuid);}",
        "            if text_buff_rule_for_scene(self.current_scene_id,base_id).is_some(){snapshot_scene_instances.insert(buff_uuid);}",
        "filter map-only snapshot inventory",
    );
    replace_once(
        &mut source,
        "        let Some((base_label, default_duration, priority)) = buff_rule_for_scene(self.current_scene_id, base_id) else { return; };",
        "        let Some((base_label, default_duration, priority)) = text_buff_rule_for_scene(self.current_scene_id, base_id) else { return; };",
        "filter map-only snapshot rows",
    );

    // BuffInfo itself carries logic_effect at field 13. The previous stage only
    // looked at outer BuffEffect logic, which misses PlayEffect metadata nested
    // inside AddBuff. Parse the nested payload so Tina slash order and S4 pair
    // patterns work from both snapshots and live AddBuff packets.
    replace_once(
        &mut source,
        "            let label = scene_buff_label(base_id, base_label, info, &play_effect_ids);",
        "            let nested_effect_ids=buff_info_play_effect_ids(info);\n            let label_effect_ids=if nested_effect_ids.is_empty(){&play_effect_ids}else{&nested_effect_ids};\n            let label = scene_buff_label(base_id, base_label, info, label_effect_ids);",
        "nested AddBuff PlayEffect metadata",
    );
    replace_once(
        &mut source,
        "        let label = scene_buff_label(base_id, base_label, info, &[]);",
        "        let effect_ids=buff_info_play_effect_ids(info);\n        let label = scene_buff_label(base_id, base_label, info, &effect_ids);",
        "snapshot PlayEffect metadata",
    );

    // CN updates S4 pair slot/lock values with BuffEventCustomize (1002). Handle
    // that edge explicitly instead of ignoring the event because it has no nested
    // AddBuff record.
    replace_once(
        &mut source,
        "        let play_effect_ids = buff_play_effect_ids(effect);\n        if event_type == 2 {",
        "        let play_effect_ids = buff_play_effect_ids(effect);\n        if event_type==1002{self.refresh_scene_mechanic_customize(host,buff_uuid,effect);return;}\n        if event_type == 2 {",
        "live BuffEventCustomize handling",
    );
    replace_once(
        &mut source,
        "    fn observe_scene_mechanic_buff(&mut self, host: i64, buff_uuid: i32, base_id: i32, info: &[u8]) {",
        r#"    fn refresh_scene_mechanic_customize(&mut self,host:i64,buff_uuid:i32,effect:&[u8]){
        if self.current_scene_id!=6615{return;}
        let Some(row_key)=self.buff_instances.get(&(host,buff_uuid)).cloned()else{return;};
        if !row_key.ends_with(":884659"){return;}
        let values=buff_customize_values(effect);
        let Some(label)=s4_pair_mark_label(&values)else{return;};
        let changed=if let Some(row)=self.mechanics.get_mut(&row_key){
            if row.label!=label{row.label=label;true}else{false}
        }else{false};
        if changed{self.emit_mechanics(false);}
    }

    fn observe_scene_mechanic_buff(&mut self, host: i64, buff_uuid: i32, base_id: i32, info: &[u8]) {"#,
        "S4 customize row refresh",
    );

    replace_once(
        &mut source,
        "fn buff_play_effect_ids(effect:&[u8])->Vec<i32>{",
        r#"fn buff_info_play_effect_ids(info:&[u8])->Vec<i32>{
    let mut ids=Vec::new();
    for logic in proto::len_fields(info,13){
        if proto::get_varint_field(logic,1).unwrap_or(0)!=0{continue;}
        let Some(raw)=proto::get_len_field(logic,2)else{continue;};
        if let Some(value)=proto::get_varint_field(raw,1){
            ids.push(value.min(i32::MAX as u64)as i32);
        }
    }
    ids
}

fn buff_customize_values(effect:&[u8])->Vec<i32>{
    let mut values=Vec::new();
    for logic in proto::len_fields(effect,5){
        // CN skips BuffEffectStopAll (15): it is an init/refresh/settle wipe,
        // not one of the four pair slot/lock values.
        if proto::get_varint_field(logic,1).unwrap_or(0)==15{continue;}
        let Some(raw)=proto::get_len_field(logic,2)else{continue;};
        if let Some(value)=customize_value(raw){values.push(value);}
    }
    values
}

fn customize_value(raw:&[u8])->Option<i32>{
    if let Some(value)=proto::get_varint_field(raw,1).filter(|value|*value!=0){
        return Some(value.min(i32::MAX as u64)as i32);
    }
    match raw{
        [byte]=>Some(i32::from(*byte)),
        [low,high]=>Some(i32::from(i16::from_le_bytes([*low,*high]))),
        _=>None,
    }
}

fn buff_play_effect_ids(effect:&[u8])->Vec<i32>{"#,
        "nested/customize effect decoders",
    );

    // The original parity test intentionally enumerated every CN-known row
    // candidate. Rename it so it does not imply that map-only candidates are
    // supposed to render in ReadyAlert.
    replace_once(
        &mut source,
        "    fn visible_cn_callouts_have_readyalert_rows(){",
        "    fn cn_known_row_candidates_remain_available_for_parity_audit(){",
        "parity test naming",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1231_text_mechanics_tests {
    use super::*;

    fn put_varint(mut value:u64,out:&mut Vec<u8>){
        loop{
            let mut byte=(value&0x7f)as u8;value>>=7;
            if value!=0{byte|=0x80;}out.push(byte);
            if value==0{break;}
        }
    }
    fn field_varint(field:u32,value:u64)->Vec<u8>{
        let mut out=Vec::new();put_varint((field as u64)<<3,&mut out);put_varint(value,&mut out);out
    }
    fn field_bytes(field:u32,value:&[u8])->Vec<u8>{
        let mut out=Vec::new();put_varint(((field as u64)<<3)|2,&mut out);put_varint(value.len()as u64,&mut out);out.extend_from_slice(value);out
    }

    #[test]
    fn map_only_cn_mechanics_do_not_render_as_incomplete_text(){
        // Full CN inventory is still recognized...
        assert!(monster_rule_for_scene(1150,2106).is_some());
        assert!(monster_rule_for_scene(6563,3340219).is_some());
        assert!(monster_rule_for_scene(1631,300086).is_some());
        assert!(skill_rule_for_scene(6563,3340245).is_some());
        assert!(buff_rule_for_scene(6513,884102).is_some());
        assert!(buff_rule_for_scene(13023,829372).is_some());
        assert!(buff_rule_for_scene(6563,883633).is_some());
        assert!(buff_rule_for_scene(6615,884664).is_some());

        // ...but Tracker & Mech only emits mechanics that remain useful without
        // CN's spatial minimap renderer.
        assert!(text_monster_rule_for_scene(1150,2106).is_none());
        assert!(text_monster_rule_for_scene(6563,3340219).is_none());
        assert!(text_monster_rule_for_scene(1631,300086).is_none());
        assert!(text_skill_rule_for_scene(6563,3340245).is_none());
        assert!(text_buff_rule_for_scene(6513,884102).is_none());
        assert!(text_buff_rule_for_scene(13023,829372).is_none());
        assert!(text_buff_rule_for_scene(6563,883633).is_none());
        assert!(text_buff_rule_for_scene(6615,884664).is_none());

        // Target/order/timer mechanics stay enabled.
        assert!(text_buff_rule_for_scene(13023,829304).is_some());
        assert!(text_buff_rule_for_scene(6563,522602).is_some());
        assert!(text_buff_rule_for_scene(1631,841509).is_some());
        assert!(text_buff_rule_for_scene(6615,884659).is_some());
    }

    #[test]
    fn nested_buffinfo_play_effect_restores_slash_order(){
        let play=field_varint(1,2);
        let mut logic=field_varint(1,0);
        logic.extend(field_bytes(2,&play));
        let info=field_bytes(13,&logic);
        assert_eq!(buff_info_play_effect_ids(&info),vec![2]);
        assert_eq!(scene_buff_label(841509,"Wudi Slash",&info,&buff_info_play_effect_ids(&info)),"Wudi Slash #3");
    }

    #[test]
    fn customize_values_restore_live_s4_pair_pattern(){
        let mut effect=Vec::new();
        for value in [1u8,5,3,8]{
            let logic=field_bytes(2,&[value]);
            effect.extend(field_bytes(5,&logic));
        }
        assert_eq!(buff_customize_values(&effect),vec![1,5,3,8]);
        assert_eq!(s4_pair_mark_label(&buff_customize_values(&effect)).as_deref(),Some("Pair mark B[W]B -> WHITE"));
    }
}
"#);

    fs::write(path, source).expect("write v1.23.1 text-only scene mechanics");
}

fn main() {
    prior::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_telemetry(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1231_text_mechanics.rs");
}
