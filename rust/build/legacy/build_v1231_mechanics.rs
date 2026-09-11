use std::{env, fs, path::{Path, PathBuf}};

mod prior {
    include!("build_v1231.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.23.1 scene/mechanics patch {label} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn replace_count(source: &mut String, from: &str, to: &str, expected: usize, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, expected, "v1.23.1 scene/mechanics patch {label} expected {expected} matches, found {count}");
    *source = source.replace(from, to);
}

fn patch_telemetry(out: &Path) {
    let path = out.join("telemetry_v170_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .expect("read v1.23.1 telemetry")
        .replace("\r\n", "\n");

    // CN obtains the dungeon/raid scene from EnterSceneInfo.sceneAttrs (field 1),
    // attribute 0x155. Keep the scene in telemetry so dungeon-specific ids never
    // fire globally in an unrelated map.
    replace_once(
        &mut source,
        "const ATTR_PROFESSION_ID: u64 = 0xdc;",
        "const ATTR_PROFESSION_ID: u64 = 0xdc;\nconst ATTR_SCENE_BASIC_ID: u64 = 0x155;",
        "scene attribute id",
    );
    replace_once(
        &mut source,
        "    local_uid: i64,\n    encounter_started: Option<Instant>,",
        "    local_uid: i64,\n    current_scene_id: i32,\n    encounter_started: Option<Instant>,",
        "scene runtime field",
    );
    replace_once(
        &mut source,
        "            local_uid: 0,\n            encounter_started: None,",
        "            local_uid: 0,\n            current_scene_id: 0,\n            encounter_started: None,",
        "scene runtime init",
    );
    replace_once(
        &mut source,
        "    fn reset_scene(&mut self) {\n",
        "    fn reset_scene(&mut self) {\n        self.current_scene_id = 0;\n",
        "clear scene id",
    );
    replace_once(
        &mut source,
        "        if let Some(info) = proto::get_len_field(body, 1) {\n            if let Some(player) = proto::get_len_field(info, 2) {",
        r#"        if let Some(info) = proto::get_len_field(body, 1) {
            if let Some(scene_attrs) = proto::get_len_field(info, 1) {
                if let Some(scene_id) = scene_id_from_attrs(scene_attrs) {
                    self.current_scene_id = scene_id;
                    crate::logging::write(format!(
                        "mechanics: entered scene {scene_id} ({})",
                        mechanic_scene_name(scene_id).unwrap_or("unsupported")
                    ));
                }
            }
            if let Some(player) = proto::get_len_field(info, 2) {"#,
        "parse EnterScene scene id",
    );

    // Gate every dungeon mechanic source by the current CN scene. This is the
    // missing link that makes Raid/dungeon entry deterministic instead of relying
    // on globally unique monster, skill, or buff ids.
    replace_once(
        &mut source,
        "if let Some((label, duration, priority)) = monster_rule(monster_id) {",
        "if let Some((label, duration, priority)) = monster_rule_for_scene(self.current_scene_id, monster_id) {",
        "scene gate monster mechanics",
    );
    replace_once(
        &mut source,
        "if let Some((label, duration, priority)) = skill_rule(skill) {",
        "if let Some((label, duration, priority)) = skill_rule_for_scene(self.current_scene_id, skill) {",
        "scene gate skill mechanics",
    );
    replace_once(
        &mut source,
        "let Some((label, default_duration, priority)) = buff_rule(base_id) else { continue; };",
        "let Some((base_label, default_duration, priority)) = buff_rule_for_scene(self.current_scene_id, base_id) else { continue; };\n            let label = scene_buff_label(base_id, base_label, info, &play_effect_ids);",
        "scene gate buff mechanics",
    );
    replace_once(
        &mut source,
        "mechanic_row(key.clone(), label, self.display_target(host), duration, duration == 0, priority),",
        "mechanic_row(key.clone(), &label, self.display_target(host), duration, duration == 0, priority),",
        "dynamic mechanic label",
    );

    // Capture PlayEffect ids from the same BuffEffect packet. CN uses these for
    // mechanics such as Tina's Wudi Slash order and S4 Wasteland pair patterns.
    replace_once(
        &mut source,
        "        let buff_uuid = proto::get_varint_field(effect, 2).unwrap_or(0) as i32;\n",
        "        let buff_uuid = proto::get_varint_field(effect, 2).unwrap_or(0) as i32;\n        let play_effect_ids = buff_play_effect_ids(effect);\n",
        "decode mechanic PlayEffect ids",
    );

    // Buff snapshots are authoritative state too. Previously ReadyAlert only
    // created mechanic rows from deltas, so entering a dungeon mid-mechanic could
    // miss an already-active marker. Feed both snapshot and delta AddBuff paths
    // through the same scene-aware observer; the delta's richer label overwrites
    // the same row key immediately when PlayEffect metadata is available.
    replace_count(
        &mut source,
        "            self.observe_special_buff(host, buff_uuid, base_id, info);",
        "            self.observe_special_buff(host, buff_uuid, base_id, info);\n            self.observe_scene_mechanic_buff(host, buff_uuid, base_id, info);",
        2,
        "observe mechanics from snapshot and delta",
    );
    replace_once(
        &mut source,
        "        let mut snapshot_revive_blocks=HashSet::new();\n        let player_consumable_snapshot=entity_kind(host)==ENTITY_PLAYER;",
        "        let mut snapshot_revive_blocks=HashSet::new();\n        let mut snapshot_scene_instances=HashSet::new();\n        let player_consumable_snapshot=entity_kind(host)==ENTITY_PLAYER;",
        "scene snapshot instance set",
    );
    replace_once(
        &mut source,
        "            if player_consumable_snapshot&&base_id==REVIVE_BLOCK_BUFF_ID{snapshot_revive_blocks.insert(buff_uuid);}\n            if player_consumable_snapshot&&consumable_info(base_id).is_some(){snapshot_consumables.insert(buff_uuid);}",
        "            if player_consumable_snapshot&&base_id==REVIVE_BLOCK_BUFF_ID{snapshot_revive_blocks.insert(buff_uuid);}\n            if buff_rule_for_scene(self.current_scene_id,base_id).is_some(){snapshot_scene_instances.insert(buff_uuid);}\n            if player_consumable_snapshot&&consumable_info(base_id).is_some(){snapshot_consumables.insert(buff_uuid);}",
        "collect scene mechanic snapshot instances",
    );
    replace_once(
        &mut source,
        "        crate::event_tracker::sync_buff_instances(host, &tracker_instances);",
        r#"        let stale_scene:Vec<(i64,i32)>=self.buff_instances.keys()
            .filter(|(entry_host,buff_uuid)|*entry_host==host&&!snapshot_scene_instances.contains(buff_uuid))
            .copied().collect();
        let mut stale_scene_changed=false;
        for key in stale_scene {
            if let Some(row_key)=self.buff_instances.remove(&key) {
                stale_scene_changed|=self.mechanics.remove(&row_key).is_some();
            }
        }
        if stale_scene_changed { self.emit_mechanics(false); }
        crate::event_tracker::sync_buff_instances(host, &tracker_instances);"#,
        "reconcile mechanic snapshot removals",
    );

    replace_once(
        &mut source,
        "    fn observe_consumable(&mut self, host: i64, buff_uuid: i32, base_id: i32, info: &[u8]) {",
        r#"    fn observe_scene_mechanic_buff(&mut self, host: i64, buff_uuid: i32, base_id: i32, info: &[u8]) {
        let Some((base_label, default_duration, priority)) = buff_rule_for_scene(self.current_scene_id, base_id) else { return; };
        let now = now_ms();
        let observed_expiry = observed_buff_expiry(info);
        let expires_unix_ms = if observed_expiry > 0 {
            observed_expiry
        } else if default_duration > 0 {
            now.saturating_add(default_duration.min(i64::MAX as u64) as i64)
        } else {
            0
        };
        let label = scene_buff_label(base_id, base_label, info, &[]);
        let key = format!("buff:{host}:{buff_uuid}:{base_id}");
        self.mechanics.insert(key.clone(), MechanicRow {
            key: key.clone(),
            label,
            target: self.display_target(host),
            created_unix_ms: now,
            expires_unix_ms,
            persistent: expires_unix_ms <= 0,
            priority,
        });
        self.buff_instances.insert((host, buff_uuid), key);
        self.emit_mechanics(false);
    }

    fn observe_consumable(&mut self, host: i64, buff_uuid: i32, base_id: i32, info: &[u8]) {"#,
        "snapshot scene mechanic observer",
    );

    // ReadyAlert had most CN callouts already, but these visible frontend rows
    // were still missing or over-aggregated.
    replace_once(
        &mut source,
        "        829115 | 829116 | 829304 | 829305 => (\"Share\", 10_000, 3),\n        829306 | 829307 => (\"Decay\", 10_000, 3),\n        829308 | 829309 => (\"Spread\", 10_000, 3),",
        "        829115 | 829304 => (\"Share\", 10_000, 3),\n        829116 | 829305 => (\"Mirage Share\", 10_000, 3),\n        829306 => (\"Decay\", 10_000, 3),\n        829307 => (\"Mirage Decay\", 10_000, 3),\n        829308 => (\"Spread\", 10_000, 3),\n        829309 => (\"Mirage Spread\", 10_000, 3),",
        "preserve Raid mirage labels",
    );
    replace_once(
        &mut source,
        "        829323 | 829324 | 829326 => (\"Kill mark\", 10_000, 3),\n        829314 => (\"Pinball cast\", 6_000, 3),\n        883602 => (\"Dual mark - ICE\", 10_000, 3),",
        "        829323 => (\"Divine Trick Kill mark\", 10_000, 3),\n        829324 => (\"Kill mark\", 10_000, 3),\n        829326 => (\"Mirage Divine Trick Kill mark\", 10_000, 3),\n        829314 => (\"Pinball cast\", 6_000, 3),\n        829372 => (\"Preset return x1\", 10_000, 3),\n        829373 => (\"Preset return x2\", 10_000, 3),\n        829374 => (\"Preset return x3\", 10_000, 3),\n        522602 => (\"Matrix callout\", 10_000, 3),\n        883602 => (\"Dual mark - ICE\", 10_000, 3),",
        "missing CN Raid return and Reef matrix callouts",
    );

    // Install the pinned CN scene inventory and helper logic immediately before
    // the existing compact rule tables. Config-only ids remain recognized by the
    // parity helpers even when CN uses them only for map coloring/state rather
    // than an info-bar row.
    replace_once(
        &mut source,
        "fn monster_rule(id: i32) -> Option<(&'static str, u64, u8)> {",
        r#"fn scene_id_from_attrs(attrs:&[u8])->Option<i32>{
    attr_varint(attrs,ATTR_SCENE_BASIC_ID)
        .and_then(|value|i32::try_from(value).ok())
        .filter(|value|*value>0)
}

fn mechanic_scene_name(scene_id:i32)->Option<&'static str>{
    Some(match scene_id{
        6513|6514|6515=>"S3 Cursed Tomb",
        1150|1151|1152=>"S3 Giant Tower",
        13021|13022|13023=>"S3 Raid",
        6563|6564|6565=>"S3 Sea-Ringed Reef",
        1631|1632|1633=>"S3 Tina Mindrealm",
        6615=>"S4 Wasteland Court",
        _=>return None,
    })
}

fn scene_tracks_buff(scene_id:i32,id:i32)->bool{
    match scene_id{
        6513|6514|6515=>matches!(id,884101|884102|884103|884104|884106|884122|884129|884141|884162|884163|884166|884168|884169|884170),
        1150|1151|1152=>matches!(id,821076),
        13021|13022|13023=>matches!(id,829104|829105|829106|829115|829116|829214|829215|829217|829226|829227|829228|829245|829304|829305|829306|829307|829308|829309|829314|829316|829318|829323|829324|829326|829327|829328|829329|829330|829331|829332|829372|829373|829374),
        6563|6564|6565=>matches!(id,883707|883708|883709|883710|883714|883601|883602|883603|883605|883631|522602|883633|883634),
        1631|1632|1633=>matches!(id,510571|841519|841509),
        6615=>matches!(id,884609|884610|884614|884615|884616|884641|884659|884660|884661|884664),
        _=>false,
    }
}

fn scene_tracks_monster(scene_id:i32,id:i32)->bool{
    match scene_id{
        6513|6514|6515=>matches!(id,33901|33904|33905|33908|33909|33921|33922),
        1150|1151|1152=>matches!(id,2106|2107|1150|1151|1152),
        13021|13022|13023=>matches!(id,103100|103107|103108|103106|103207|103208|103308|103200|103300|103301|103302|103303|103309|103310|103311|10310062|10310063|10310064|3543|10330051),
        6563|6564|6565=>matches!(id,4601|4603|4604|4605|4639|3340219|3340220|3340227|3340228|1000),
        1631|1632|1633=>matches!(id,33701|300086|300089),
        6615=>matches!(id,4701|4711|4702|470131|884606|884607|884640|884642|884668|884669|884670|884671),
        _=>false,
    }
}

fn scene_tracks_skill(scene_id:i32,id:i32)->bool{
    match scene_id{
        6513|6514|6515=>matches!(id,3390117|3390118|3390123|3390124),
        1150|1151|1152=>id==111103,
        13021|13022|13023=>matches!(id,10310062|10310063|10310064),
        6563|6564|6565=>id==3340245,
        1631|1632|1633=>false,
        6615=>matches!(id,470125|470132|470119|470112|470113),
        _=>false,
    }
}

fn monster_rule_for_scene(scene_id:i32,id:i32)->Option<(&'static str,u64,u8)>{
    scene_tracks_monster(scene_id,id).then(||monster_rule(id)).flatten()
}
fn skill_rule_for_scene(scene_id:i32,id:i32)->Option<(&'static str,u64,u8)>{
    scene_tracks_skill(scene_id,id).then(||skill_rule(id)).flatten()
}
fn buff_rule_for_scene(scene_id:i32,id:i32)->Option<(&'static str,u64,u8)>{
    scene_tracks_buff(scene_id,id).then(||buff_rule(id)).flatten()
}

fn buff_play_effect_ids(effect:&[u8])->Vec<i32>{
    let mut ids=Vec::new();
    for logic in proto::len_fields(effect,5){
        // CN EBuffEffectLogicPbType::PlayEffect == 0. Missing enum field also
        // decodes to the protobuf default 0.
        if proto::get_varint_field(logic,1).unwrap_or(0)!=0{continue;}
        let Some(raw)=proto::get_len_field(logic,2)else{continue;};
        // BuffEffectLogicPlayEffect.effect_id == field 1.
        ids.push(proto::get_varint_field(raw,1).unwrap_or(0).min(i32::MAX as u64)as i32);
    }
    ids
}

fn scene_buff_label(id:i32,base:&str,info:&[u8],effect_ids:&[i32])->String{
    match id{
        841509=>effect_ids.first().map(|order|format!("Wudi Slash #{}",order.saturating_add(1))).unwrap_or_else(||base.to_string()),
        829324=>{
            let layer=proto::get_varint_field(info,8).unwrap_or(0);
            if layer>0{format!("Kill mark x{layer}")}else{base.to_string()}
        }
        884659=>s4_pair_mark_label(effect_ids).unwrap_or_else(||base.to_string()),
        _=>base.to_string(),
    }
}

fn s4_pair_mark_label(effect_ids:&[i32])->Option<String>{
    if effect_ids.len()<4{return None;}
    let mut colors=['?';3];
    for(index,value)in effect_ids.iter().take(3).enumerate(){
        let(expected_slot,color)=match *value{
            1=>(1,'B'),2=>(2,'B'),3=>(3,'B'),4=>(1,'W'),5=>(2,'W'),6=>(3,'W'),_=>return None,
        };
        if expected_slot!=index+1{return None;}
        colors[index]=color;
    }
    let lock=effect_ids[3].checked_sub(6)? as usize;
    if !(1..=3).contains(&lock){return None;}
    let target=colors[lock-1];
    let mut pattern=String::new();
    for(index,color)in colors.iter().enumerate(){
        if index+1==lock{pattern.push('[');pattern.push(*color);pattern.push(']');}else{pattern.push(*color);}
    }
    if colors.iter().all(|color|*color==target){
        Some(format!("Pair matched {pattern}"))
    }else{
        Some(format!("Pair mark {pattern} -> {}",if target=='W'{"WHITE"}else{"BLACK"}))
    }
}

fn monster_rule(id: i32) -> Option<(&'static str, u64, u8)> {"#,
        "CN scene parity helpers",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1231_cn_scene_parity_tests {
    use super::*;

    fn put_varint(mut value:u64,out:&mut Vec<u8>){
        loop{
            let mut byte=(value&0x7f)as u8;value>>=7;
            if value!=0{byte|=0x80;}out.push(byte);
            if value==0{break;}
        }
    }
    fn field_varint(field:u32,value:u64)->Vec<u8>{
        let mut out=Vec::new();put_varint(((field as u64)<<3)as u64,&mut out);put_varint(value,&mut out);out
    }
    fn field_bytes(field:u32,value:&[u8])->Vec<u8>{
        let mut out=Vec::new();put_varint(((field as u64)<<3|2)as u64,&mut out);put_varint(value.len()as u64,&mut out);out.extend_from_slice(value);out
    }

    #[test]
    fn enter_scene_extracts_cn_raid_scene_id(){
        let mut attr=field_varint(1,ATTR_SCENE_BASIC_ID);
        attr.extend(field_bytes(2,&field_varint(0,13023)[1..]));
        let attrs=field_bytes(2,&attr);
        let info=field_bytes(1,&attrs);
        let body=field_bytes(1,&info);
        let(tx,_rx)=std::sync::mpsc::channel();
        let mut runtime=TelemetryRuntime::new(tx);
        runtime.handle_enter_scene(&body);
        assert_eq!(runtime.current_scene_id,13023);
        assert_eq!(mechanic_scene_name(runtime.current_scene_id),Some("S3 Raid"));
    }

    #[test]
    fn all_cn_registered_scene_ids_are_recognized(){
        for scene in [6513,6514,6515,1150,1151,1152,13021,13022,13023,6563,6564,6565,1631,1632,1633,6615]{
            assert!(mechanic_scene_name(scene).is_some(),"missing scene {scene}");
        }
        assert!(mechanic_scene_name(0).is_none());
    }

    #[test]
    fn cn_backend_buff_filter_inventory_is_complete(){
        let cases:&[(i32,&[i32])]=&[
            (6513,&[884101,884102,884103,884104,884106,884122,884129,884141,884162,884163,884166,884168,884169,884170]),
            (1150,&[821076]),
            (13023,&[829104,829105,829106,829115,829116,829214,829215,829217,829226,829227,829228,829245,829304,829305,829306,829307,829308,829309,829314,829316,829318,829323,829324,829326,829327,829328,829329,829330,829331,829332,829372,829373,829374]),
            (6563,&[883707,883708,883709,883710,883714,883601,883602,883603,883605,883631,522602,883633,883634]),
            (1631,&[510571,841519,841509]),
            (6615,&[884609,884610,884614,884615,884616,884641,884659,884660,884661,884664]),
        ];
        for(scene,ids)in cases{for id in *ids{assert!(scene_tracks_buff(*scene,*id),"scene {scene} missing buff {id}");}}
    }

    #[test]
    fn visible_cn_callouts_have_readyalert_rows(){
        let cases:&[(i32,&[i32])]=&[
            (6513,&[884101,884102,884103,884106,884122,884129,884141,884162,884163,884168,884169,884170]),
            (1150,&[821076]),
            (13023,&[829104,829105,829106,829115,829116,829214,829215,829217,829226,829227,829228,829245,829304,829305,829306,829307,829308,829309,829314,829316,829323,829324,829326,829327,829328,829329,829330,829331,829332,829372,829373,829374]),
            (6563,&[522602,883602,883603,883633,883634]),
            (1631,&[510571,841519,841509]),
            (6615,&[884609,884610,884614,884615,884616,884641,884659,884660,884661,884664]),
        ];
        for(scene,ids)in cases{for id in *ids{assert!(buff_rule_for_scene(*scene,*id).is_some(),"scene {scene} missing visible buff row {id}");}}
    }

    #[test]
    fn scene_gate_rejects_cross_dungeon_ids(){
        assert!(monster_rule_for_scene(13023,10330051).is_some());
        assert!(monster_rule_for_scene(6513,10330051).is_none());
        assert!(skill_rule_for_scene(1150,111103).is_some());
        assert!(skill_rule_for_scene(13023,111103).is_none());
        assert!(buff_rule_for_scene(6563,522602).is_some());
        assert!(buff_rule_for_scene(13023,522602).is_none());
    }

    #[test]
    fn mechanic_snapshot_creates_active_row_on_entry(){
        let(tx,_rx)=std::sync::mpsc::channel();
        let mut runtime=TelemetryRuntime::new(tx);
        runtime.current_scene_id=13023;
        runtime.local_uid=1;
        let mut info=field_varint(1,77);
        info.extend(field_varint(2,829304));
        info.extend(field_varint(11,9000));
        runtime.observe_scene_mechanic_buff(canonical_player_uuid(1),77,829304,&info);
        assert!(runtime.mechanics.values().any(|row|row.label=="Share"));
    }

    #[test]
    fn play_effect_metadata_restores_cn_order_and_pair_pattern(){
        let play=field_varint(1,2);
        let mut logic=field_varint(1,0);
        logic.extend(field_bytes(2,&play));
        let effect=field_bytes(5,&logic);
        assert_eq!(buff_play_effect_ids(&effect),vec![2]);
        assert_eq!(scene_buff_label(841509,"Wudi Slash",&[],&[2]),"Wudi Slash #3");
        assert_eq!(s4_pair_mark_label(&[1,5,3,8]).as_deref(),Some("Pair mark B[W]B -> WHITE"));
    }
}
"#);

    fs::write(path, source).expect("write v1.23.1 scene-aware telemetry");
}

fn main() {
    prior::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_telemetry(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1231_mechanics.rs");
}
