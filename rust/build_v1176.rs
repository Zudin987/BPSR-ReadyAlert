use std::{env, fs, path::{Path, PathBuf}};
mod prior { include!("build_v1175.rs"); pub fn run(){ main(); } }

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){
    let count=source.matches(from).count();
    assert_eq!(count,1,"v1.17.6 patch {label} expected one match, found {count}");
    *source=source.replacen(from,to,1);
}

fn patch_telemetry(out:&Path){
    let path=out.join("telemetry_v170_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read v1.17.5 telemetry").replace("\r\n","\n");
    replace_once(&mut source,r####"const WIPE_BUFF_BASE_ID: i32 = 510_072;
const SEGMENT_BOUNDARY_DELAY: Duration = Duration::from_secs(3);"####,r####"const WIPE_BUFF_BASE_ID: i32 = 510_072;
// Exact enemy Power Seal / Power Sealed buffs shown by the game as Enrage.
const ENRAGE_BUFF_IDS: [i32; 4] = [501_706, 501_710, 501_712, 501_714];
const SEGMENT_BOUNDARY_DELAY: Duration = Duration::from_secs(3);"####,"constants");
    replace_once(&mut source,r####"    tracked_buff_instances: HashMap<(i64, i32), String>,
    revive_block_instances: HashMap<(i64, i32), i64>,
    food: Option<ConsumableStatus>,"####,r####"    tracked_buff_instances: HashMap<(i64, i32), String>,
    revive_block_instances: HashMap<(i64, i32), i64>,
    // host -> (buff_uuid, exact base_id, wall-clock expiry, total duration).
    // Power Seal is one authoritative Enrage timer per monster host.
    enrage_instances: HashMap<i64, (i32, i32, i64, i64)>,
    food: Option<ConsumableStatus>,"####,"state field");
    replace_once(&mut source,r####"            tracked_buff_instances: HashMap::new(),
            revive_block_instances: HashMap::new(),
            food: None,"####,r####"            tracked_buff_instances: HashMap::new(),
            revive_block_instances: HashMap::new(),
            enrage_instances: HashMap::new(),
            food: None,"####,"state init");
    replace_once(&mut source,r####"        self.tracked_buff_instances.clear();
        self.revive_block_instances.clear();
        self.food = None;"####,r####"        self.tracked_buff_instances.clear();
        self.revive_block_instances.clear();
        self.enrage_instances.clear();
        self.food = None;"####,"scene clear");
    replace_once(&mut source,r####"            self.entities.remove(&uuid);
            crate::event_tracker::remove_entity(uuid);
            if let Some(keys) = self.entity_mechanic_keys.remove(&uuid) {"####,r####"            self.entities.remove(&uuid);
            self.enrage_instances.remove(&uuid);
            crate::event_tracker::remove_entity(uuid);
            if let Some(keys) = self.entity_mechanic_keys.remove(&uuid) {"####,"despawn");
    replace_once(&mut source,r####"        } else {
            let old_state = self.entities.get(&uuid).map(|m| m.actor_state).unwrap_or_default();
            let mut actor_state_seen = false;
            {"####,r####"        } else {
            let old_state = self.entities.get(&uuid).map(|m| m.actor_state).unwrap_or_default();
            let mut actor_state_seen = false;
            let mut target_vitals_changed = false;
            {"####,"vitals flag");
    replace_once(&mut source,r####"                        x if x == ATTR_CURRENT_HP as u64 => {
                            if let Some(value) = raw_signed(raw) {
                                meta.hp = value;
                            }
                        }"####,r####"                        x if x == ATTR_CURRENT_HP as u64 => {
                            if let Some(value) = raw_signed(raw) {
                                if meta.hp != value { target_vitals_changed = true; }
                                meta.hp = value;
                            }
                        }"####,"hp");
    replace_once(&mut source,r####"                        x if x == ATTR_MAX_HP as u64 => {
                            if let Some(value) = raw_signed(raw) {
                                meta.max_hp = value;
                            }
                        }"####,r####"                        x if x == ATTR_MAX_HP as u64 => {
                            if let Some(value) = raw_signed(raw) {
                                if meta.max_hp != value { target_vitals_changed = true; }
                                meta.max_hp = value;
                            }
                        }"####,"max hp");
    replace_once(&mut source,r####"            if actor_state_seen {
                let new_state = self.entities.get(&uuid).map(|m| m.actor_state).unwrap_or_default();
                if old_state != ACTOR_STATE_DEAD && new_state == ACTOR_STATE_DEAD && uuid == self.last_target {
                    // Boss/add death is not an encounter boundary. Keep the accumulated
                    // meter until a real wipe, scene re-entry or manual reset.
                }
            }
        }
    }"####,r####"            if actor_state_seen {
                let new_state = self.entities.get(&uuid).map(|m| m.actor_state).unwrap_or_default();
                if old_state != ACTOR_STATE_DEAD && new_state == ACTOR_STATE_DEAD && uuid == self.last_target {
                    // Boss/add death is not an encounter boundary. Keep the accumulated
                    // meter until a real wipe, scene re-entry or manual reset.
                }
            }
            // Target HP is combat-critical UI. Push authoritative monster HP/max-HP
            // changes at up to 20 Hz even when the packet has no damage event.
            if target_vitals_changed
                && uuid == self.last_target
                && self.last_dps_emit.elapsed() >= Duration::from_millis(50)
            {
                self.emit_dps();
            }
        }
    }"####,"vitals emit");
    replace_once(&mut source,r####"        let target_kind = entity_kind(target_uuid);
        let now = Instant::now();

        // CN-style segment lifecycle: start on eligible outgoing player damage."####,r####"        let target_kind = entity_kind(target_uuid);
        let now = Instant::now();
        let mut target_changed = false;

        // CN-style segment lifecycle: start on eligible outgoing player damage."####,"target changed var");
    replace_once(&mut source,r####"        if !is_heal && target_kind == ENTITY_MONSTER && attacker_uid > 0 {
            self.prepare_segment(now);
            self.last_target = target_uuid;
        }"####,r####"        if !is_heal && target_kind == ENTITY_MONSTER && attacker_uid > 0 {
            self.prepare_segment(now);
            target_changed = self.last_target != target_uuid;
            self.last_target = target_uuid;
        }"####,"target changed set");
    replace_once(&mut source,r####"        if self.last_dps_emit.elapsed() >= Duration::from_millis(100) {
            self.emit_dps();
        }"####,r####"        if target_changed || self.last_dps_emit.elapsed() >= Duration::from_millis(50) {
            self.emit_dps();
        }"####,"dps cadence");
    replace_once(&mut source,r####"            hp: meta.hp,
            max_hp: meta.max_hp,
            enrage_remaining_ms: None,
        })"####,r####"            hp: meta.hp,
            max_hp: meta.max_hp,
            enrage_remaining_ms: self.enrage_remaining_ms(self.last_target),
        })"####,"snapshot enrage");
    replace_once(&mut source,r####"        let mut marker_source = None;
        let mut tracker_instances = HashSet::new();
        let player_consumable_snapshot=entity_kind(host)==ENTITY_PLAYER;
        let local_consumable_snapshot=player_consumable_snapshot&&host>>16==self.local_uid;
        let mut snapshot_consumables=HashSet::new();"####,r####"        let mut marker_source = None;
        let mut tracker_instances = HashSet::new();
        let player_consumable_snapshot=entity_kind(host)==ENTITY_PLAYER;
        let local_consumable_snapshot=player_consumable_snapshot&&host>>16==self.local_uid;
        let monster_enrage_snapshot=entity_kind(host)==ENTITY_MONSTER;
        let mut saw_enrage=false;
        let mut snapshot_consumables=HashSet::new();"####,"snapshot flags");
    replace_once(&mut source,r####"            let base_id = proto::get_varint_field(info, 2).unwrap_or(0) as i32;
            if player_consumable_snapshot&&consumable_info(base_id).is_some(){snapshot_consumables.insert(buff_uuid);}
            if base_id == FANTASY_MARKER_BUFF_ID {"####,r####"            let base_id = proto::get_varint_field(info, 2).unwrap_or(0) as i32;
            if player_consumable_snapshot&&consumable_info(base_id).is_some(){snapshot_consumables.insert(buff_uuid);}
            if monster_enrage_snapshot&&is_enrage_buff(base_id){saw_enrage=true;}
            if base_id == FANTASY_MARKER_BUFF_ID {"####,"snapshot detect");
    replace_once(&mut source,r####"            self.observe_consumable(host, buff_uuid, base_id, info);
            self.observe_special_buff(host, buff_uuid, base_id, info);
        }
        if player_consumable_snapshot{"####,r####"            self.observe_consumable(host, buff_uuid, base_id, info);
            self.observe_special_buff(host, buff_uuid, base_id, info);
            self.observe_enrage_buff(host, buff_uuid, base_id, info);
        }
        if monster_enrage_snapshot&&!saw_enrage{
            let removed=self.enrage_instances.remove(&host).is_some();
            if removed&&host==self.last_target{self.emit_dps();}
        }
        if player_consumable_snapshot{"####,"snapshot observe");
    replace_once(&mut source,r####"        if event_type == 2 {
            crate::event_tracker::observe_buff(host, 0, buff_uuid, 0, true, 0);
            let mut special_changed = false;"####,r####"        if event_type == 2 {
            crate::event_tracker::observe_buff(host, 0, buff_uuid, 0, true, 0);
            if self.clear_enrage_buff(host,buff_uuid)&&host==self.last_target{self.emit_dps();}
            let mut special_changed = false;"####,"remove");
    replace_once(&mut source,r####"            // BuffEffectBuffChange (19) is the live stack/timer update path.
            if logic_type==19{self.observe_consumable_change(host,buff_uuid,info);continue;}
            if logic_type!=18{continue;}
            let base_id = proto::get_varint_field(info, 2).unwrap_or(0) as i32;"####,r####"            // BuffEffectBuffChange (19) is the live stack/timer update path.
            if logic_type==19{
                self.observe_consumable_change(host,buff_uuid,info);
                self.observe_enrage_change(host,buff_uuid,info);
                continue;
            }
            if logic_type!=18{continue;}
            let base_id = proto::get_varint_field(info, 2).unwrap_or(0) as i32;
            self.observe_enrage_buff(host,buff_uuid,base_id,info);"####,"apply/change");
    replace_once(&mut source,r####"    fn observe_special_buff(&mut self, host: i64, buff_uuid: i32, base_id: i32, info: &[u8]) {"####,r####"    fn observe_enrage_buff(&mut self,host:i64,buff_uuid:i32,base_id:i32,info:&[u8]){
        if entity_kind(host)!=ENTITY_MONSTER||!is_enrage_buff(base_id){return;}
        let now=now_ms();
        let duration=observed_buff_duration(info);
        let create=proto::get_varint_field(info,6).map(signed).unwrap_or(0);
        let absolute_expiry=if duration>0&&create>=1_000_000_000_000{
            Some(create.saturating_add(duration))
        }else if duration>0&&create>=1_000_000_000{
            Some(create.saturating_mul(1000).saturating_add(duration))
        }else{None};
        let expiry=if let Some(expiry)=absolute_expiry{
            expiry
        }else if let Some((_,_,old_expiry,_))=self.enrage_instances.get(&host).copied()
            .filter(|(active_uuid,active_base,old_expiry,_)|*active_uuid==buff_uuid&&*active_base==base_id&&*old_expiry>now)
        {
            old_expiry
        }else if duration>0{
            now.saturating_add(duration)
        }else{0};
        let next=(buff_uuid,base_id,expiry,duration);
        let changed=self.enrage_instances.get(&host).copied()!=Some(next);
        self.enrage_instances.insert(host,next);
        if changed&&host==self.last_target{self.emit_dps();}
    }

    fn observe_enrage_change(&mut self,host:i64,buff_uuid:i32,change:&[u8]){
        if entity_kind(host)!=ENTITY_MONSTER{return;}
        let Some((active_uuid,_base_id,old_expiry,old_duration))=self.enrage_instances.get(&host).copied()else{return;};
        if active_uuid!=buff_uuid{return;}
        let duration=proto::get_varint_field(change,2).unwrap_or(0).min(24*60*60*1000)as i64;
        if duration<=0{return;}
        let create=proto::get_varint_field(change,3).map(signed).unwrap_or(0);
        let now=now_ms();
        let absolute_expiry=if create>=1_000_000_000_000{
            Some(create.saturating_add(duration))
        }else if create>=1_000_000_000{
            Some(create.saturating_mul(1000).saturating_add(duration))
        }else{None};
        let expiry=if old_expiry>now&&old_duration>0&&duration>old_duration{
            old_expiry.saturating_add(duration.saturating_sub(old_duration))
        }else if let Some(expiry)=absolute_expiry{
            expiry
        }else if old_expiry>now{
            old_expiry
        }else{
            now.saturating_add(duration)
        };
        if let Some(status)=self.enrage_instances.get_mut(&host){
            status.2=expiry;
            status.3=duration;
        }
        if host==self.last_target{self.emit_dps();}
    }

    fn clear_enrage_buff(&mut self,host:i64,buff_uuid:i32)->bool{
        let matches=self.enrage_instances.get(&host).is_some_and(|(active_uuid,_,_,_)|*active_uuid==buff_uuid);
        if matches{self.enrage_instances.remove(&host);true}else{false}
    }

    fn enrage_remaining_ms(&self,host:i64)->Option<i64>{
        let (_,_,expiry,_)=self.enrage_instances.get(&host).copied()?;
        if expiry<=0{return None;}
        let remaining=expiry.saturating_sub(now_ms());
        (remaining>0).then_some(remaining)
    }

    fn observe_special_buff(&mut self, host: i64, buff_uuid: i32, base_id: i32, info: &[u8]) {"####,"helpers");
    replace_once(&mut source,r####"fn observed_buff_duration(info:&[u8])->i64{proto::get_varint_field(info,11).unwrap_or(0).min(24*60*60*1000)as i64}"####,r####"fn is_enrage_buff(id:i32)->bool{ENRAGE_BUFF_IDS.contains(&id)}

fn observed_buff_duration(info:&[u8])->i64{proto::get_varint_field(info,11).unwrap_or(0).min(24*60*60*1000)as i64}"####,"is_enrage");
    source.push_str(r####"

#[cfg(test)]
mod v1176_target_enrage_tests {
    use super::*;

    fn push_varint(mut value:u64,out:&mut Vec<u8>){
        while value>=0x80 { out.push(((value as u8)&0x7f)|0x80); value>>=7; }
        out.push(value as u8);
    }
    fn field_varint(field:u8,value:u64,out:&mut Vec<u8>){
        push_varint(((field as u64)<<3)|0,out);
        push_varint(value,out);
    }
    fn buff_info(base_id:i32,duration_ms:i64)->Vec<u8>{
        let mut out=Vec::new();
        field_varint(2,base_id as u64,&mut out);
        field_varint(11,duration_ms as u64,&mut out);
        out
    }
    fn buff_change(duration_ms:i64)->Vec<u8>{
        let mut out=Vec::new();
        field_varint(2,duration_ms as u64,&mut out);
        out
    }

    #[test]
    fn exact_power_seal_ids_are_enrage_only() {
        for id in [501_706,501_710,501_712,501_714] { assert!(is_enrage_buff(id)); }
        for id in [501_705,501_707,501_709,501_711,501_713,501_715,510_072] { assert!(!is_enrage_buff(id)); }
    }

    #[test]
    fn target_snapshot_exposes_observed_power_seal_timer() {
        let (tx,_rx)=std::sync::mpsc::channel();
        let mut runtime=TelemetryRuntime::new(tx);
        let monster=(777_i64<<16)|(ENTITY_MONSTER<<6);
        runtime.last_target=monster;
        runtime.entities.entry(monster).or_default();
        runtime.observe_enrage_buff(monster,91,501_706,&buff_info(501_706,60_000));
        let remaining=runtime.target_snapshot().and_then(|t|t.enrage_remaining_ms).expect("enrage timer");
        assert!(remaining>58_000&&remaining<=60_000,"remaining={remaining}");
        assert!(runtime.clear_enrage_buff(monster,91));
        assert!(runtime.target_snapshot().and_then(|t|t.enrage_remaining_ms).is_none());
    }

    #[test]
    fn repeated_snapshot_and_change_do_not_restart_countdown() {
        let (tx,_rx)=std::sync::mpsc::channel();
        let mut runtime=TelemetryRuntime::new(tx);
        let monster=(778_i64<<16)|(ENTITY_MONSTER<<6);
        runtime.last_target=monster;
        runtime.entities.entry(monster).or_default();
        let info=buff_info(501_710,60_000);
        runtime.observe_enrage_buff(monster,92,501_710,&info);
        let first=runtime.enrage_instances.get(&monster).copied().expect("first").2;
        runtime.observe_enrage_buff(monster,92,501_710,&info);
        let second=runtime.enrage_instances.get(&monster).copied().expect("second").2;
        assert_eq!(second,first);
        runtime.observe_enrage_change(monster,92,&buff_change(60_000));
        let third=runtime.enrage_instances.get(&monster).copied().expect("third").2;
        assert_eq!(third,first);
    }
}
"####);
    fs::write(path,source).expect("write v1.17.6 telemetry");
}

fn patch_overlay(out:&Path){
    let path=out.join("feature_overlays_v170_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read v1.17.5 overlays").replace("\r\n","\n");
    replace_once(&mut source,r####"dps: DpsSnapshot,
mechanics: MechanicSnapshot,"####,r####"dps: DpsSnapshot,
dps_updated_unix_ms: i64,
mechanics: MechanicSnapshot,"####,"overlay state receipt timestamp");
    replace_once(&mut source,r####"dps:DpsSnapshot::default(),mechanics:MechanicSnapshot::default(),"####,r####"dps:DpsSnapshot::default(),dps_updated_unix_ms:0,mechanics:MechanicSnapshot::default(),"####,"overlay state receipt initialization");
    replace_once(&mut source,r####"pub unsafe fn update_dps(hwnd:HWND,snapshot:DpsSnapshot){with_state(hwnd,|state|{let rolled=history::encounter_rolled(&state.dps,&snapshot);if rolled{archive_live_snapshot(state);if !state.features.read().map(|f|f.meter.remember_scroll).unwrap_or(false){state.scroll=0;}}if state.history_index.is_none()&&!state.detail_hwnd.is_null()&&IsWindow(state.detail_hwnd)!=0&&state.detail_uid!=0{if let Some(row)=snapshot.rows.iter().find(|row|row.uid==state.detail_uid).cloned(){update_detail(state.detail_hwnd,row,snapshot.encounter_ms);}}state.dps=snapshot;let total=meter_rows(state).len();state.scroll=state.scroll.min(total.saturating_sub(1));});}"####,r####"pub unsafe fn update_dps(hwnd:HWND,snapshot:DpsSnapshot){
    let mut repaint_target=false;
    with_state(hwnd,|state|{
        let rolled=history::encounter_rolled(&state.dps,&snapshot);
        if rolled{archive_live_snapshot(state);if !state.features.read().map(|f|f.meter.remember_scroll).unwrap_or(false){state.scroll=0;}}
        if state.history_index.is_none()&&!state.detail_hwnd.is_null()&&IsWindow(state.detail_hwnd)!=0&&state.detail_uid!=0{
            if let Some(row)=snapshot.rows.iter().find(|row|row.uid==state.detail_uid).cloned(){update_detail(state.detail_hwnd,row,snapshot.encounter_ms);}
        }
        repaint_target=state.history_index.is_none()&&target_refresh_key(&state.dps)!=target_refresh_key(&snapshot);
        state.dps_updated_unix_ms=now_ms();
        state.dps=snapshot;
        let total=meter_rows(state).len();
        state.scroll=state.scroll.min(total.saturating_sub(1));
    });
    if repaint_target{InvalidateRect(hwnd,null(),0);}
}"####,"live target header invalidation");
    replace_once(&mut source,r####"fn target_title(snapshot:&DpsSnapshot)->String{match snapshot.target.as_ref(){Some(t)if!t.name.trim().is_empty()=>t.name.clone(),Some(t)=>format!("Target #{}",t.entity_uuid),None=>"No active target".into()}}
fn target_hp(snapshot:&DpsSnapshot)->String{snapshot.target.as_ref().map(|t|if t.max_hp>0{format!("HP {:.1}%",(t.hp.max(0)as f64*100.0/t.max_hp as f64).clamp(0.0,100.0))}else{"HP ?".into()}).unwrap_or_else(||"HP —".into())}
fn capture_short"####,r####"fn target_title(snapshot:&DpsSnapshot)->String{match snapshot.target.as_ref(){Some(t)if!t.name.trim().is_empty()=>t.name.clone(),Some(t)=>format!("Target #{}",t.entity_uuid),None=>"No active target".into()}}
fn target_hp(snapshot:&DpsSnapshot)->String{snapshot.target.as_ref().map(|t|if t.max_hp>0{format!("HP {:.1}%",(t.hp.max(0)as f64*100.0/t.max_hp as f64).clamp(0.0,100.0))}else{"HP ?".into()}).unwrap_or_else(||"HP —".into())}
fn target_refresh_key(snapshot:&DpsSnapshot)->Option<(i64,i64,i64,bool)>{snapshot.target.as_ref().map(|t|(t.entity_uuid,t.hp,t.max_hp,t.enrage_remaining_ms.is_some()))}
fn enrage_remaining_at(snapshot:&DpsSnapshot,received_unix_ms:i64,now:i64)->Option<i64>{
    let observed=snapshot.target.as_ref()?.enrage_remaining_ms?;
    if observed<=0{return None;}
    let elapsed=if received_unix_ms>0{now.saturating_sub(received_unix_ms).max(0)}else{0};
    let remaining=observed.saturating_sub(elapsed);
    (remaining>0).then_some(remaining)
}
fn live_enrage_remaining_ms(state:&State,snapshot:&DpsSnapshot,now:i64)->Option<i64>{
    if state.history_index.is_some(){return None;}
    enrage_remaining_at(snapshot,state.dps_updated_unix_ms,now)
}
fn format_enrage_countdown(ms:i64)->String{
    let seconds=ms.max(0).saturating_add(999)/1000;
    format!("{}:{:02}",seconds/60,seconds%60)
}
fn enrage_blink_visible(remaining_ms:i64,now:i64)->bool{
    remaining_ms>30_000||((now.max(0)/500)&1)==0
}
fn capture_short"####,"live Enrage target-header helpers");
    replace_once(&mut source,r####"unsafe fn paint_dps(hdc:HDC,rc:RECT,state:&State){let settings=state.features.read().map(|x|x.clone()).unwrap_or_default();let snapshot=view_snapshot(state);let accent=rgb(66,211,190);let target_top=TOOLBAR_H+4;let target_r=RECT{left:6,top:target_top,right:rc.right-6,bottom:target_top+27};fill(hdc,&target_r,rgb(22,29,35));fill(hdc,&RECT{left:target_r.left,top:target_r.top,right:target_r.left+3,bottom:target_r.bottom},accent);SetTextColor(hdc,rgb(235,242,245));draw(hdc,&if settings.meter.show_target{target_title(snapshot)}else{"Encounter".into()},RECT{left:target_r.left+10,top:target_r.top,right:(rc.right-285).max(target_r.left+120),bottom:target_r.bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);SetTextColor(hdc,rgb(179,193,202));let status=if settings.meter.show_target{format!("{}   TIME {}",target_hp(snapshot),format_time(snapshot.encounter_ms))}else{format!("TIME {}",format_time(snapshot.encounter_ms))};draw(hdc,&status,RECT{left:(rc.right-280).max(target_r.left+120),top:target_r.top,right:rc.right-10,bottom:target_r.bottom},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);if rc.right>=820{SetTextColor(hdc,rgb(112,205,190));draw(hdc,&capture_short(state),RECT{left:target_r.left+10,top:target_r.top,right:target_r.right-230,bottom:target_r.bottom},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}let tabs_top=target_r.bottom+4;"####,r####"unsafe fn paint_dps(hdc:HDC,rc:RECT,state:&State){
    let settings=state.features.read().map(|x|x.clone()).unwrap_or_default();
    let snapshot=view_snapshot(state);
    let accent=rgb(66,211,190);
    let target_top=TOOLBAR_H+4;
    let target_r=RECT{left:6,top:target_top,right:rc.right-6,bottom:target_top+27};
    fill(hdc,&target_r,rgb(22,29,35));
    fill(hdc,&RECT{left:target_r.left,top:target_r.top,right:target_r.left+3,bottom:target_r.bottom},accent);
    let now=now_ms();
    let enrage=if settings.meter.show_target{live_enrage_remaining_ms(state,snapshot,now)}else{None};
    let time_r=RECT{left:(rc.right-82).max(target_r.left+70),top:target_r.top,right:rc.right-10,bottom:target_r.bottom};
    let hp_r=RECT{left:(time_r.left-88).max(target_r.left+70),top:target_r.top,right:time_r.left-5,bottom:target_r.bottom};
    let enrage_r=RECT{left:(hp_r.left-150).max(target_r.left+120),top:target_r.top,right:hp_r.left-5,bottom:target_r.bottom};
    let title_right=if enrage.is_some(){enrage_r.left-6}else{hp_r.left-8};
    SetTextColor(hdc,rgb(235,242,245));
    draw(hdc,&if settings.meter.show_target{target_title(snapshot)}else{"Encounter".into()},RECT{left:target_r.left+10,top:target_r.top,right:title_right.max(target_r.left+120),bottom:target_r.bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);
    if settings.meter.show_target{
        SetTextColor(hdc,if snapshot.target.is_some(){rgb(181,30,46)}else{rgb(179,193,202)});
        draw(hdc,&target_hp(snapshot),hp_r,DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);
        if let Some(remaining)=enrage{
            if enrage_blink_visible(remaining,now){
                SetTextColor(hdc,rgb(255,35,45));
                draw(hdc,&format!("ENRAGE IN: {}",format_enrage_countdown(remaining)),enrage_r,DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);
            }
        }
    }
    SetTextColor(hdc,rgb(179,193,202));
    draw(hdc,&format!("TIME {}",format_time(snapshot.encounter_ms)),time_r,DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);
    if rc.right>=820&&enrage.is_none(){
        SetTextColor(hdc,rgb(112,205,190));
        draw(hdc,&capture_short(state),RECT{left:target_r.left+10,top:target_r.top,right:target_r.right-230,bottom:target_r.bottom},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);
    }
    let tabs_top=target_r.bottom+4;"####,"target header colors and Enrage countdown");
    replace_once(&mut source,r####"if y>=target_top&&y<target_top+27{return Some("Target context • name/ID, HP and encounter time".into());}"####,r####"if y>=target_top&&y<target_top+27{return Some("Target context • name/ID, live HP, Enrage / Power Seal timer and encounter time".into());}"####,"target header tooltip");
    source.push_str(r####"

#[cfg(test)]
mod v1176_target_header_tests {
    use super::*;

    fn target_snapshot(hp:i64,max_hp:i64,enrage:Option<i64>)->DpsSnapshot{
        let mut snapshot=DpsSnapshot::default();
        snapshot.target=Some(crate::model::TargetSnapshot{
            entity_uuid:123,
            name:"Power Seal target".into(),
            hp,
            max_hp,
            enrage_remaining_ms:enrage,
        });
        snapshot
    }

    #[test]
    fn enrage_countdown_uses_ceiling_seconds() {
        assert_eq!(format_enrage_countdown(348_000),"5:48");
        assert_eq!(format_enrage_countdown(30_000),"0:30");
        assert_eq!(format_enrage_countdown(29_001),"0:30");
        assert_eq!(format_enrage_countdown(1),"0:01");
    }

    #[test]
    fn observed_enrage_countdown_keeps_moving_between_packets() {
        let snapshot=target_snapshot(900,1_000,Some(60_000));
        assert_eq!(enrage_remaining_at(&snapshot,10_000,10_000),Some(60_000));
        assert_eq!(enrage_remaining_at(&snapshot,10_000,12_500),Some(57_500));
        assert_eq!(enrage_remaining_at(&snapshot,10_000,70_000),None);
    }

    #[test]
    fn enrage_blinks_only_during_final_thirty_seconds() {
        assert!(enrage_blink_visible(30_001,500));
        assert!(enrage_blink_visible(30_000,0));
        assert!(!enrage_blink_visible(30_000,500));
        assert!(enrage_blink_visible(5_000,1_000));
        assert!(!enrage_blink_visible(5_000,1_500));
    }

    #[test]
    fn target_refresh_key_reacts_to_hp_and_enrage_presence() {
        let a=target_snapshot(900,1_000,None);
        let b=target_snapshot(899,1_000,None);
        let c=target_snapshot(899,1_000,Some(60_000));
        assert_ne!(target_refresh_key(&a),target_refresh_key(&b));
        assert_ne!(target_refresh_key(&b),target_refresh_key(&c));
    }
}
"####);
    fs::write(path,source).expect("write v1.17.6 overlays");
}

fn main(){
    prior::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_telemetry(&out);
    patch_overlay(&out);
    println!("cargo:rerun-if-changed=build_v1176.rs");
}
