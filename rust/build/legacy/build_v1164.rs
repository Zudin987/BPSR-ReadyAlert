use std::{env, fs, path::{Path, PathBuf}};

mod prior {
    include!("build_v1163_ui.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.16.4 patch {label} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn patch_overlay(out: &Path) {
    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read generated v1.16.4 overlay");

    replace_once(
        &mut source,
        r#"fn paint_consumable_status(hdc:HDC,label:&str,status:Option<&ConsumableStatus>,r:RECT){unsafe{SetTextColor(hdc,rgb(215,224,235));let name=status.map(|s|s.name.as_str()).unwrap_or("None");draw(hdc,&format!("{label}: {name}"),RECT{left:r.left,top:r.top,right:r.right-94,bottom:r.bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);if let Some(status)=status{if status.expires_unix_ms>0{let left=status.expires_unix_ms.saturating_sub(now_ms()).max(0)as u64;SetTextColor(hdc,rgb(255,70,70));draw(hdc,&format_countdown(left),RECT{left:r.right-90,top:r.top,right:r.right,bottom:r.bottom},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}}}}"#,
        r#"fn paint_consumable_status(hdc:HDC,label:&str,status:Option<&ConsumableStatus>,r:RECT){unsafe{let now=now_ms();let active=status.filter(|s|s.expires_unix_ms<=0||s.expires_unix_ms>now);SetTextColor(hdc,rgb(215,224,235));let name=active.map(|s|s.name.as_str()).unwrap_or("None");draw(hdc,&format!("{label}: {name}"),RECT{left:r.left,top:r.top,right:r.right-94,bottom:r.bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);if let Some(status)=active{if status.expires_unix_ms>0{let left=status.expires_unix_ms.saturating_sub(now).max(0)as u64;SetTextColor(hdc,rgb(255,70,70));draw(hdc,&format_countdown(left),RECT{left:r.right-90,top:r.top,right:r.right,bottom:r.bottom},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}}}}"#,
        "paint expired food/serum as inactive immediately",
    );

    fs::write(path, source).expect("write v1.16.4 overlay");
}

fn patch_telemetry(out: &Path) {
    let path = out.join("telemetry_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read generated v1.16.4 telemetry");

    replace_once(
        &mut source,
        "            let mut player_skill_entries: Option<Vec<(i32,i32,i32)>> = None;",
        "            let mut player_skill_entries: Option<Vec<(i32,i32,i32)>> = None;\n            let mut mechanic_attr_changed = false;",
        "track local mechanics attribute changes",
    );

    replace_once(
        &mut source,
        r#"                        _ if is_trackable_attr(id as i32) => {
                            if let Some(value) = value {
                                player.attrs.insert(id as i32, value);
                            }
                        }"#,
        r#"                        _ if is_trackable_attr(id as i32) => {
                            if let Some(value) = value {
                                let attr_id=id as i32;
                                if player.attrs.get(&attr_id).copied()!=Some(value){
                                    player.attrs.insert(attr_id,value);
                                    mechanic_attr_changed=true;
                                }
                            }
                        }"#,
        "detect changed mechanics panel values",
    );

    replace_once(
        &mut source,
        "            if let Some(entries)=player_skill_entries.as_deref(){self.apply_player_skill_list(uid,entries);}",
        "            if let Some(entries)=player_skill_entries.as_deref(){self.apply_player_skill_list(uid,entries);}\n            if uid==self.local_uid&&mechanic_attr_changed{self.emit_mechanics(true);}",
        "emit live mechanics stats on local attr deltas",
    );

    replace_once(
        &mut source,
        "        let mut marker_source = None;\n        let mut tracker_instances = HashSet::new();",
        "        let mut marker_source = None;\n        let mut tracker_instances = HashSet::new();\n        let local_consumable_snapshot=entity_kind(host)==ENTITY_PLAYER&&host>>16==self.local_uid;\n        let mut snapshot_consumables=HashSet::new();",
        "stage authoritative local consumable snapshot",
    );

    replace_once(
        &mut source,
        "            let base_id = proto::get_varint_field(info, 2).unwrap_or(0) as i32;\n            if base_id == FANTASY_MARKER_BUFF_ID {",
        "            let base_id = proto::get_varint_field(info, 2).unwrap_or(0) as i32;\n            if local_consumable_snapshot&&consumable_info(base_id).is_some(){snapshot_consumables.insert(buff_uuid);}\n            if base_id == FANTASY_MARKER_BUFF_ID {",
        "remember consumables present in full buff snapshot",
    );

    replace_once(
        &mut source,
        "        crate::event_tracker::sync_buff_instances(host, &tracker_instances);\n        marker_source",
        r#"        if local_consumable_snapshot{
            let stale:Vec<(i64,i32)>=self.consumable_instances.keys().filter(|(entry_host,buff_uuid)|*entry_host==host&&!snapshot_consumables.contains(buff_uuid)).copied().collect();
            if !stale.is_empty(){
                for key in stale{self.consumable_instances.remove(&key);}
                self.refresh_local_consumables(host);
                self.emit_mechanics(true);
            }
        }
        crate::event_tracker::sync_buff_instances(host, &tracker_instances);
        marker_source"#,
        "clear stale food/serum from authoritative snapshots",
    );

    replace_once(
        &mut source,
        "    fn observe_consumable(&mut self, host: i64, buff_uuid: i32, base_id: i32, info: &[u8]) {",
        r#"    fn refresh_local_consumables(&mut self,host:i64){
        let now=now_ms();
        let latest=|kind:ConsumableKind|self.consumable_instances.iter()
            .filter(|((entry_host,_),(entry_kind,status))|*entry_host==host&&*entry_kind==kind&&(status.expires_unix_ms<=0||status.expires_unix_ms>now))
            .map(|(_,(_,status))|status)
            .max_by_key(|status|status.expires_unix_ms)
            .cloned();
        self.food=latest(ConsumableKind::Food);
        self.serum=latest(ConsumableKind::Serum);
    }

    fn observe_consumable(&mut self, host: i64, buff_uuid: i32, base_id: i32, info: &[u8]) {"#,
        "add consumable snapshot refresh helper",
    );

    replace_once(
        &mut source,
        r#"        match kind {
            ConsumableKind::Food => self.food = Some(status),
            ConsumableKind::Serum => self.serum = Some(status),
        }
        self.emit_mechanics(false);
    }

    fn display_target"#,
        r#"        match kind {
            ConsumableKind::Food => self.food = Some(status),
            ConsumableKind::Serum => self.serum = Some(status),
        }
        // Consumables are sparse events; never let the general 50 ms mechanics
        // throttle swallow the only add/update notification.
        self.emit_mechanics(true);
    }

    fn display_target"#,
        "force live food/serum updates",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1164_mechanics_live_tests {
    use super::*;
    use std::sync::mpsc;

    fn push_varint(mut value:u64,out:&mut Vec<u8>){while value>=0x80{out.push((value as u8&0x7f)|0x80);value>>=7;}out.push(value as u8);}
    fn attr_list(id:i32,value:i64)->Vec<u8>{
        let mut raw=Vec::new();push_varint(value as u64,&mut raw);
        let mut attr=vec![0x08];push_varint(id as u64,&mut attr);attr.push(0x12);push_varint(raw.len()as u64,&mut attr);attr.extend(raw);
        let mut attrs=vec![0x12];push_varint(attr.len()as u64,&mut attrs);attrs.extend(attr);attrs
    }

    #[test]
    fn local_panel_attribute_delta_emits_mechanics_immediately(){
        let(tx,rx)=mpsc::channel();let mut runtime=TelemetryRuntime::new(tx);runtime.local_uid=42;runtime.last_mechanic_emit=Instant::now();
        let attrs=attr_list(crate::feature_settings::ATTR_CAST_SPEED,4_130);
        runtime.update_entity_from_attrs(canonical_player_uuid(42),Some(&attrs));
        let event=rx.try_recv().expect("forced mechanics snapshot");
        let AppEvent::Mechanics(snapshot)=event else{panic!("mechanics event expected")};
        let value=snapshot.tracked_attributes.iter().find(|a|a.attr_id==crate::feature_settings::ATTR_CAST_SPEED).map(|a|a.value);
        assert_eq!(value,Some(4_130));
        runtime.update_entity_from_attrs(canonical_player_uuid(42),Some(&attrs));
        assert!(rx.try_recv().is_err(),"unchanged attr must not spam mechanics snapshots");
    }

    #[test]
    fn consumable_add_bypasses_mechanics_throttle(){
        let(tx,rx)=mpsc::channel();let mut runtime=TelemetryRuntime::new(tx);runtime.local_uid=42;runtime.last_mechanic_emit=Instant::now();
        let host=canonical_player_uuid(42);runtime.observe_consumable(host,77,2_010_003,&[]);
        let event=rx.try_recv().expect("food mechanics snapshot");
        let AppEvent::Mechanics(snapshot)=event else{panic!("mechanics event expected")};
        assert_eq!(snapshot.food.as_ref().map(|x|x.buff_id),Some(2_010_003));
    }

    #[test]
    fn full_local_buff_snapshot_clears_stale_consumable(){
        let(tx,rx)=mpsc::channel();let mut runtime=TelemetryRuntime::new(tx);runtime.local_uid=42;
        let host=canonical_player_uuid(42);runtime.observe_consumable(host,77,2_010_003,&[]);while rx.try_recv().is_ok(){}
        runtime.handle_buff_snapshot(host,&[]);
        assert!(runtime.food.is_none());assert!(runtime.consumable_instances.is_empty());
        let event=rx.try_recv().expect("cleared food mechanics snapshot");
        let AppEvent::Mechanics(snapshot)=event else{panic!("mechanics event expected")};assert!(snapshot.food.is_none());
    }
}
"#);

    fs::write(path, source).expect("write v1.16.4 telemetry");
}

fn main() {
    prior::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_overlay(&out);
    patch_telemetry(&out);
    println!("cargo:rerun-if-changed=build_v1164.rs");
}
