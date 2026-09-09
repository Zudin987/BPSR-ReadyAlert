use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v183_skillnames.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.8.3 buff/revive patch `{label}` expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn replace_count(source: &mut String, from: &str, to: &str, expected: usize, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, expected, "v1.8.3 buff/revive patch `{label}` expected {expected} matches, found {count}");
    *source = source.replace(from, to);
}

fn replace_between(source: &mut String, start: &str, end: &str, replacement: &str, label: &str) {
    let count = source.matches(start).count();
    assert_eq!(count, 1, "v1.8.3 buff/revive patch `{label}` start expected one match, found {count}");
    let begin = source.find(start).expect("start checked");
    let rel_end = source[begin..].find(end)
        .unwrap_or_else(|| panic!("v1.8.3 buff/revive patch `{label}` end anchor missing"));
    source.replace_range(begin..begin + rel_end, replacement);
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));

    // Track the six requested local buffs as timed mechanic rows and keep the
    // revive-block debuff on every observed player. All timers come from the
    // game's buff payload; ReadyAlert does not invent cooldown lengths.
    let telemetry_path = out.join("telemetry_v170_fixed.rs");
    let mut telemetry = fs::read_to_string(&telemetry_path).expect("read generated v1.8.3 telemetry");
    replace_once(
        &mut telemetry,
        "const FANTASY_MARKER_BUFF_ID: i32 = 2_199_999;",
        "const FANTASY_MARKER_BUFF_ID: i32 = 2_199_999;\nconst REVIVE_BLOCK_BUFF_ID: i32 = 2_110_057;",
        "revive block buff id",
    );
    replace_once(
        &mut telemetry,
        "    consumable_instances: HashMap<(i64, i32), (ConsumableKind, ConsumableStatus)>,",
        "    consumable_instances: HashMap<(i64, i32), (ConsumableKind, ConsumableStatus)>,\n    tracked_buff_instances: HashMap<(i64, i32), String>,\n    revive_block_instances: HashMap<(i64, i32), i64>,",
        "special buff state fields",
    );
    replace_once(
        &mut telemetry,
        "            consumable_instances: HashMap::new(),",
        "            consumable_instances: HashMap::new(),\n            tracked_buff_instances: HashMap::new(),\n            revive_block_instances: HashMap::new(),",
        "special buff state init",
    );
    replace_once(
        &mut telemetry,
        "        self.consumable_instances.clear();",
        "        self.consumable_instances.clear();\n        self.tracked_buff_instances.clear();\n        self.revive_block_instances.clear();",
        "special buff scene reset",
    );
    replace_count(
        &mut telemetry,
        "            self.observe_consumable(host, buff_uuid, base_id, info);",
        "            self.observe_consumable(host, buff_uuid, base_id, info);\n            self.observe_special_buff(host, buff_uuid, base_id, info);",
        2,
        "observe special buffs in snapshot and delta paths",
    );
    replace_once(
        &mut telemetry,
        "        if event_type == 2 {",
        r#"        if event_type == 2 {
            let mut special_changed = false;
            if let Some(key) = self.tracked_buff_instances.remove(&(host, buff_uuid)) {
                special_changed |= self.mechanics.remove(&key).is_some();
            }
            if self.revive_block_instances.remove(&(host, buff_uuid)).is_some() {
                self.emit_dps();
            }
            if special_changed {
                self.emit_mechanics(false);
            }"#,
        "remove special buffs immediately",
    );
    replace_once(
        &mut telemetry,
        "    fn observe_consumable(&mut self, host: i64, buff_uuid: i32, base_id: i32, info: &[u8]) {",
        r#"    fn observe_special_buff(&mut self, host: i64, buff_uuid: i32, base_id: i32, info: &[u8]) {
        if entity_kind(host) != ENTITY_PLAYER {
            return;
        }
        let expiry = observed_buff_expiry(info);
        if base_id == REVIVE_BLOCK_BUFF_ID {
            // A non-positive expiry means the packet did not expose a finite
            // duration. Keep -1 so the UI never incorrectly says CAN REVIVE.
            self.revive_block_instances.insert((host, buff_uuid), if expiry > 0 { expiry } else { -1 });
            self.emit_dps();
        }
        if (host >> 16) != self.local_uid {
            return;
        }
        let Some(alias) = tracked_buff_alias(base_id) else { return; };
        let key = format!("trackedbuff:{host}:{buff_uuid}:{base_id}");
        self.mechanics.insert(key.clone(), MechanicRow {
            key: key.clone(),
            label: alias.into(),
            target: None,
            created_unix_ms: now_ms(),
            expires_unix_ms: expiry,
            persistent: expiry <= 0,
            priority: 250,
        });
        self.tracked_buff_instances.insert((host, buff_uuid), key);
        self.emit_mechanics(false);
    }

    fn revive_block_until(&self, actor_uuid: i64) -> i64 {
        let now = now_ms();
        let mut unknown_active = false;
        let mut latest = 0i64;
        for ((host, _), expiry) in &self.revive_block_instances {
            if *host != actor_uuid { continue; }
            if *expiry < 0 {
                unknown_active = true;
            } else if *expiry > now {
                latest = latest.max(*expiry);
            }
        }
        if latest > 0 { latest } else if unknown_active { -1 } else { 0 }
    }

    fn observe_consumable(&mut self, host: i64, buff_uuid: i32, base_id: i32, info: &[u8]) {"#,
        "special buff observer",
    );
    replace_once(
        &mut telemetry,
        "                is_dead: stat.is_dead,\n                is_local: uid == self.local_uid && self.local_uid > 0,",
        "                is_dead: stat.is_dead,\n                revive_blocked_until_ms: self.revive_block_until(meta.actor_uuid),\n                is_local: uid == self.local_uid && self.local_uid > 0,",
        "revive timer in DPS row",
    );

    // Put the alias table immediately before the existing consumable catalog so
    // it is generated into the same compact native telemetry module.
    replace_once(
        &mut telemetry,
        "fn consumable_info(id: i32) -> Option<(ConsumableKind, &'static str)> {",
        r#"fn tracked_buff_alias(id: i32) -> Option<&'static str> {
    Some(match id {
        55_226 => "DeterShot",
        2_110_049 => "KARTGRIFF",
        2_110_050 => "BASIL",
        2_110_055 => "TATTA",
        2_110_056 => "TINA",
        2_110_065 => "BL",
        _ => return None,
    })
}

fn consumable_info(id: i32) -> Option<(ConsumableKind, &'static str)> {"#,
        "tracked buff aliases",
    );
    fs::write(&telemetry_path, telemetry).expect("write v1.8.3 buff/revive telemetry");

    let overlay_path = out.join("feature_overlays_v170_fixed.rs");
    let mut overlay = fs::read_to_string(&overlay_path).expect("read generated v1.8.3 overlay");

    // Dead rows retain the packed left-group geometry, but the Imagine/score
    // suffix is replaced by revive availability. The name truncates first.
    replace_once(
        &mut overlay,
        "let badge_count=if show_imagines{row.imagines.len().min(2)as i32}else{0};",
        "let badge_count=if show_imagines&&!row.is_dead{row.imagines.len().min(2)as i32}else{0};",
        "hide dead row Imagine width",
    );
    replace_once(
        &mut overlay,
        "let score=dps_score_pair(row);let score_w=if score.is_empty(){0}else{approx_text_px(&score).clamp(54,96)};",
        "let score=if row.is_dead{revive_status_text(row,now_ms()).0}else{dps_score_pair(row)};let score_w=if score.is_empty(){0}else{approx_text_px(&score).clamp(54,150)};",
        "reserve packed revive status width",
    );
    replace_once(
        &mut overlay,
        "fn rate(total:i64,encounter_ms:u64)->f64{",
        r#"fn revive_status_text(row:&DpsRow,now:i64)->(String,u32){
    if row.revive_blocked_until_ms < 0 {
        return ("REVIVE IN : ?".into(),rgb(255,190,70));
    }
    if row.revive_blocked_until_ms > now {
        let left=row.revive_blocked_until_ms.saturating_sub(now)as u64;
        return (format!("REVIVE IN : {}",format_countdown(left)),rgb(255,190,70));
    }
    ("CAN REVIVE".into(),rgb(72,226,116))
}
fn rate(total:i64,encounter_ms:u64)->f64{"#,
        "revive status formatter",
    );
    replace_once(
        &mut overlay,
        "let layout=dps_row_layout(r,true,row);let mut bx=layout.badge_left;",
        "if row.is_dead{return None;}let layout=dps_row_layout(r,true,row);let mut bx=layout.badge_left;",
        "disable dead Imagine hover",
    );
    replace_once(
        &mut overlay,
        "let bg=spec_color(row);fill(hdc,&r,bg);",
        "let bg=if row.is_dead{rgb(105,28,34)}else{spec_color(row)};fill(hdc,&r,bg);",
        "dead row red background",
    );
    replace_once(
        &mut overlay,
        "SetTextColor(hdc,if row.is_dead{rgb(255,45,45)}else{base_text});",
        "SetTextColor(hdc,if row.is_dead{rgb(255,120,120)}else{base_text});",
        "readable red dead name",
    );
    replace_once(
        &mut overlay,
        "if settings.meter.show_imagines{let mut bx=layout.badge_left;",
        "if settings.meter.show_imagines&&!row.is_dead{let mut bx=layout.badge_left;",
        "hide dead row Imagine icons",
    );
    replace_once(
        &mut overlay,
        "let score_text=dps_score_pair(row);if !score_text.is_empty(){SetTextColor(hdc,base_text);draw(hdc,&score_text,RECT{left:layout.score_left,top:r.top,right:layout.left_end,bottom:r.bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}",
        r#"if row.is_dead{let(status,color)=revive_status_text(row,now_ms());SetTextColor(hdc,color);draw(hdc,&status,RECT{left:layout.score_left,top:r.top,right:layout.left_end,bottom:r.bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}else{let score_text=dps_score_pair(row);if !score_text.is_empty(){SetTextColor(hdc,base_text);draw(hdc,&score_text,RECT{left:layout.score_left,top:r.top,right:layout.left_end,bottom:r.bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}}"#,
        "replace dead score with revive state",
    );

    // Red Food/Serum timers and a dedicated compact timed-buff section. Rows in
    // this section are filtered by wall-clock expiry on every 30 FPS repaint, so
    // they disappear immediately when their game-provided timer reaches zero.
    replace_between(
        &mut overlay,
        "unsafe fn paint_mechanics(hdc:HDC,rc:RECT,state:&State){",
        "fn mechanic_timer(",
        r#"unsafe fn paint_mechanics(hdc:HDC,rc:RECT,state:&State){
let features=state.features.read().map(|x|x.clone()).unwrap_or_default();
let tracked:Vec<_>=features.mechanic_attributes.tracked.iter().filter_map(|id|state.mechanics.tracked_attributes.iter().find(|attr|attr.attr_id==*id)).collect();
let mut top=TOOLBAR_H+5;
if !tracked.is_empty(){let strip=RECT{left:6,top,right:rc.right-6,bottom:top+MECH_ATTR_H-3};fill(hdc,&strip,rgb(27,32,39));let width=((strip.right-strip.left)/tracked.len()as i32).max(60);for(i,attr)in tracked.iter().enumerate(){let left=strip.left+i as i32*width;SetTextColor(hdc,attr_color(attr.attr_id));draw(hdc,&format!("{} {}",attr.label.trim_end_matches(" %"),format_attr_value(attr.attr_id,attr.value)),RECT{left:left+2,top:strip.top,right:(left+width-2).min(strip.right),bottom:strip.bottom},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}top+=MECH_ATTR_H;}
let consumables=RECT{left:6,top,right:rc.right-6,bottom:top+MECH_CONSUMABLE_H-4};fill(hdc,&consumables,rgb(23,28,34));paint_consumable_status(hdc,"Food",state.mechanics.food.as_ref(),RECT{left:consumables.left+8,top:consumables.top,right:consumables.right-8,bottom:consumables.top+20});paint_consumable_status(hdc,"Serum",state.mechanics.serum.as_ref(),RECT{left:consumables.left+8,top:consumables.top+20,right:consumables.right-8,bottom:consumables.bottom});top+=MECH_CONSUMABLE_H;
let now=now_ms();
let special:Vec<_>=state.mechanics.rows.iter().filter(|row|row.key.starts_with("trackedbuff:")&&(row.persistent||row.expires_unix_ms<=0||row.expires_unix_ms>now)).collect();
if !special.is_empty(){SetTextColor(hdc,rgb(155,168,185));draw(hdc,"Tracked Buffs",RECT{left:8,top,right:rc.right-8,bottom:top+18},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);top+=18;for row in special{let r=RECT{left:6,top,right:rc.right-8,bottom:top+27};fill(hdc,&r,rgb(25,30,36));fill(hdc,&RECT{left:r.left,top:r.top,right:r.left+3,bottom:r.bottom},rgb(205,75,75));SetTextColor(hdc,rgb(238,242,247));draw(hdc,&row.label,RECT{left:r.left+10,top:r.top,right:r.right-92,bottom:r.bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);SetTextColor(hdc,rgb(255,70,70));draw(hdc,&mechanic_timer(row.expires_unix_ms,row.persistent,now),RECT{left:r.right-88,top:r.top,right:r.right-8,bottom:r.bottom},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);top+=29;}}
let active:Vec<_>=state.mechanics.rows.iter().filter(|row|!row.key.starts_with("trackedbuff:")&&(row.persistent||row.expires_unix_ms<=0||row.expires_unix_ms>now)).collect();
let visible=((rc.bottom-top)/MECH_ROW_H).max(0)as usize;
if active.is_empty(){SetTextColor(hdc,rgb(132,145,162));draw(hdc,"No active mechanic",RECT{left:10,top:top+18,right:rc.right-10,bottom:top+58},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);return;}
for(screen_i,row)in active.iter().skip(state.scroll).take(visible).enumerate(){let y=top+screen_i as i32*MECH_ROW_H;let r=RECT{left:6,top:y,right:rc.right-8,bottom:y+MECH_ROW_H-3};fill(hdc,&r,if screen_i%2==0{rgb(28,33,40)}else{rgb(24,29,35)});let accent=if row.priority>=3{rgb(255,99,99)}else{rgb(99,199,255)};fill(hdc,&RECT{left:r.left,top:r.top,right:r.left+3,bottom:r.bottom},accent);let label=match&row.target{Some(target)if!target.trim().is_empty()=>format!("{}  •  {}",row.label,target),_=>row.label.clone()};SetTextColor(hdc,rgb(238,242,247));draw(hdc,&label,RECT{left:r.left+10,top:r.top,right:r.right-75,bottom:r.bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);SetTextColor(hdc,accent);draw(hdc,&mechanic_timer(row.expires_unix_ms,row.persistent,now),RECT{left:r.right-70,top:r.top,right:r.right-8,bottom:r.bottom},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}
paint_scrollbar(hdc,rc,active.len(),visible,state.scroll,top);
}
fn paint_consumable_status(hdc:HDC,label:&str,status:Option<&ConsumableStatus>,r:RECT){unsafe{SetTextColor(hdc,rgb(215,224,235));let name=status.map(|s|s.name.as_str()).unwrap_or("None");draw(hdc,&format!("{label}: {name}"),RECT{left:r.left,top:r.top,right:r.right-94,bottom:r.bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);if let Some(status)=status{if status.expires_unix_ms>0{let left=status.expires_unix_ms.saturating_sub(now_ms()).max(0)as u64;SetTextColor(hdc,rgb(255,70,70));draw(hdc,&format_countdown(left),RECT{left:r.right-90,top:r.top,right:r.right,bottom:r.bottom},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}}}}
"#,
        "red consumable timers and tracked buff section",
    );

    fs::write(&overlay_path, overlay).expect("write v1.8.3 buff/revive overlay");
    println!("cargo:rerun-if-changed=build_v183_buffs.rs");
}
