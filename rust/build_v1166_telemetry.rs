use super::*;

pub fn patch(out:&Path){
    let path=out.join("telemetry_v170_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read generated v1.16.6 telemetry");
    replace_once(&mut source,"if !matches!(event_type, 1 | 3 | 4 | 5) {","if !matches!(event_type, 1 | 3 | 4 | 5 | 6) {","accept remove-layer buff events");
    replace_once(&mut source,
        r#"        for logic in proto::len_fields(effect, 5) {
            if proto::get_varint_field(logic, 1).unwrap_or(0) != 18 {
                continue;
            }
            let Some(info) = proto::get_len_field(logic, 2) else { continue; };
            let base_id = proto::get_varint_field(info, 2).unwrap_or(0) as i32;"#,
        r#"        for logic in proto::len_fields(effect, 5) {
            let logic_type=proto::get_varint_field(logic,1).unwrap_or(0) as i32;
            let Some(info)=proto::get_len_field(logic,2) else{continue;};
            // BuffEffectBuffChange (19) is the live stack/timer update path.
            if logic_type==19{self.observe_consumable_change(host,buff_uuid,info);continue;}
            if logic_type!=18{continue;}
            let base_id = proto::get_varint_field(info, 2).unwrap_or(0) as i32;"#,
        "consume live BuffChange payloads");
    replace_once(&mut source,
        "    fn observe_consumable(&mut self, host: i64, buff_uuid: i32, base_id: i32, info: &[u8]) {",
        r#"    fn observe_consumable_change(&mut self,host:i64,buff_uuid:i32,change:&[u8]){
        if entity_kind(host)!=ENTITY_PLAYER{return;}
        let Some((_kind,status))=self.consumable_instances.get_mut(&(host,buff_uuid))else{return;};
        // BuffChange fields: 1=layer, 2=duration, 3=create_time. Rebase the
        // countdown immediately; full snapshots remain authoritative correction.
        let duration=proto::get_varint_field(change,2).unwrap_or(0).min(24*60*60*1000)as i64;
        if duration>0{status.duration_ms=duration;status.expires_unix_ms=now_ms().saturating_add(duration);}
        if host>>16==self.local_uid{self.refresh_local_consumables(host);self.emit_mechanics(true);}
        self.emit_dps();
    }

    fn observe_consumable(&mut self, host: i64, buff_uuid: i32, base_id: i32, info: &[u8]) {"#,
        "add live consumable stack timer updater");
    let catalog=exact_consumable_catalog();
    replace_between(&mut source,"fn consumable_info(id: i32) -> Option<(ConsumableKind, &'static str)> {","fn monster_rule",&catalog,"replace heuristic consumables with uploaded exact catalog");
    // Old tests used heuristic-only ids; move them to exact catalog entries.
    replace_all_checked(&mut source,"2_010_003","2_032_011",7,"move Food regression fixtures to exact catalog");
    replace_all_checked(&mut source,"2_010_005","2_033_011",1,"move Serum regression fixture to exact catalog");
    source.push_str(r#"

#[cfg(test)]
mod v1166_consumable_change_tests{
    use super::*;use std::sync::mpsc;
    fn push_varint(mut v:u64,out:&mut Vec<u8>){while v>=0x80{out.push((v as u8&0x7f)|0x80);v>>=7;}out.push(v as u8);}
    fn buff_change(layer:i32,duration:i64,create:i64)->Vec<u8>{let mut out=vec![0x08];push_varint(layer.max(0)as u64,&mut out);out.push(0x10);push_varint(duration.max(0)as u64,&mut out);out.push(0x18);push_varint(create.max(0)as u64,&mut out);out}
    #[test]fn serum_buff_change_refreshes_timer_without_scene_change(){let(tx,_rx)=mpsc::channel();let mut runtime=TelemetryRuntime::new(tx);runtime.local_uid=42;let host=canonical_player_uuid(42);runtime.observe_consumable(host,77,2_033_011,&[]);let before=runtime.player_consumable(host,ConsumableKind::Serum).expect("serum");assert_eq!(before.expires_unix_ms,0);let change=buff_change(2,600_000,123);runtime.observe_consumable_change(host,77,&change);let after=runtime.player_consumable(host,ConsumableKind::Serum).expect("updated serum");assert_eq!(after.duration_ms,600_000);assert!(after.expires_unix_ms>now_ms());}
    #[test]fn uploaded_catalog_is_exact_and_replaces_legacy_guesses(){let count=(2_032_000..=2_034_000).filter(|id|consumable_info(*id).is_some()).count();assert_eq!(count,370);assert_eq!(consumable_info(2_032_065),Some((ConsumableKind::Food,"S1 ATK +75, +5%")));assert_eq!(consumable_info(2_033_011),Some((ConsumableKind::Serum,"S1 Fire +240")));assert!(consumable_info(2_010_003).is_none());}
}
"#);
    fs::write(path,source).expect("write v1.16.6 telemetry");
}
