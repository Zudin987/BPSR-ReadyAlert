use std::{env, fs, path::{Path, PathBuf}};

mod prior {
    include!("build_v1164.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.16.5 patch {label} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn patch_telemetry(out: &Path) {
    let path = out.join("telemetry_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read generated v1.16.5 telemetry");

    replace_once(
        &mut source,
        "        let local_consumable_snapshot=entity_kind(host)==ENTITY_PLAYER&&host>>16==self.local_uid;\n        let mut snapshot_consumables=HashSet::new();",
        "        let player_consumable_snapshot=entity_kind(host)==ENTITY_PLAYER;\n        let local_consumable_snapshot=player_consumable_snapshot&&host>>16==self.local_uid;\n        let mut snapshot_consumables=HashSet::new();",
        "recognize authoritative consumable snapshots for every player",
    );
    replace_once(
        &mut source,
        "            if local_consumable_snapshot&&consumable_info(base_id).is_some(){snapshot_consumables.insert(buff_uuid);}",
        "            if player_consumable_snapshot&&consumable_info(base_id).is_some(){snapshot_consumables.insert(buff_uuid);}",
        "collect remote player consumables from snapshots",
    );
    replace_once(
        &mut source,
        r#"        if local_consumable_snapshot{
            let stale:Vec<(i64,i32)>=self.consumable_instances.keys().filter(|(entry_host,buff_uuid)|*entry_host==host&&!snapshot_consumables.contains(buff_uuid)).copied().collect();
            if !stale.is_empty(){
                for key in stale{self.consumable_instances.remove(&key);}
                self.refresh_local_consumables(host);
                self.emit_mechanics(true);
            }
        }"#,
        r#"        if player_consumable_snapshot{
            let stale:Vec<(i64,i32)>=self.consumable_instances.keys().filter(|(entry_host,buff_uuid)|*entry_host==host&&!snapshot_consumables.contains(buff_uuid)).copied().collect();
            if !stale.is_empty(){
                for key in stale{self.consumable_instances.remove(&key);}
                if local_consumable_snapshot{
                    self.refresh_local_consumables(host);
                    self.emit_mechanics(true);
                }
                self.emit_dps();
            }
        }"#,
        "clear stale consumables for every synchronized player",
    );
    replace_once(
        &mut source,
        r#"            if let Some((kind, _removed)) = self.consumable_instances.remove(&(host, buff_uuid)) {
                let now = now_ms();
                let replacement = self.consumable_instances.iter()
                    .filter(|((entry_host, _), (entry_kind, status))| {
                        *entry_host == host && *entry_kind == kind
                            && (status.expires_unix_ms <= 0 || status.expires_unix_ms > now)
                    })
                    .map(|(_, (_, status))| status)
                    .max_by_key(|status| status.expires_unix_ms)
                    .cloned();
                match kind {
                    ConsumableKind::Food => self.food = replacement,
                    ConsumableKind::Serum => self.serum = replacement,
                }
                self.emit_mechanics(true);
            }"#,
        r#"            if let Some((kind, _removed)) = self.consumable_instances.remove(&(host, buff_uuid)) {
                if entity_kind(host)==ENTITY_PLAYER&&host>>16==self.local_uid{
                    let now = now_ms();
                    let replacement = self.consumable_instances.iter()
                        .filter(|((entry_host, _), (entry_kind, status))| {
                            *entry_host == host && *entry_kind == kind
                                && (status.expires_unix_ms <= 0 || status.expires_unix_ms > now)
                        })
                        .map(|(_, (_, status))| status)
                        .max_by_key(|status| status.expires_unix_ms)
                        .cloned();
                    match kind {
                        ConsumableKind::Food => self.food = replacement,
                        ConsumableKind::Serum => self.serum = replacement,
                    }
                    self.emit_mechanics(true);
                }
                self.emit_dps();
            }"#,
        "remove remote consumable indicators immediately",
    );
    replace_once(
        &mut source,
        "    fn refresh_local_consumables(&mut self,host:i64){",
        r#"    fn player_consumable(&self,host:i64,kind:ConsumableKind)->Option<ConsumableStatus>{
        let now=now_ms();
        self.consumable_instances.iter()
            .filter(|((entry_host,_),(entry_kind,status))|*entry_host==host&&*entry_kind==kind&&(status.expires_unix_ms<=0||status.expires_unix_ms>now))
            .map(|(_,(_,status))|status)
            .max_by_key(|status|status.expires_unix_ms)
            .cloned()
    }

    fn refresh_local_consumables(&mut self,host:i64){"#,
        "add per-player consumable lookup",
    );
    replace_once(
        &mut source,
        r#"        if entity_kind(host) != ENTITY_PLAYER || host >> 16 != self.local_uid {
            return;
        }
        let Some((kind, name)) = consumable_info(base_id) else { return; };
        let status = ConsumableStatus {
            buff_id: base_id,
            name: name.into(),
            expires_unix_ms: observed_buff_expiry(info),
        };
        self.consumable_instances.insert((host, buff_uuid), (kind, status.clone()));
        match kind {
            ConsumableKind::Food => self.food = Some(status),
            ConsumableKind::Serum => self.serum = Some(status),
        }
        // Consumables are sparse events; never let the general 50 ms mechanics
        // throttle swallow the only add/update notification.
        self.emit_mechanics(true);"#,
        r#"        if entity_kind(host) != ENTITY_PLAYER {
            return;
        }
        let Some((kind, name)) = consumable_info(base_id) else { return; };
        let status = ConsumableStatus {
            buff_id: base_id,
            name: name.into(),
            expires_unix_ms: observed_buff_expiry(info),
            duration_ms: observed_buff_duration(info),
        };
        self.consumable_instances.insert((host, buff_uuid), (kind, status.clone()));
        if host>>16==self.local_uid{
            match kind {
                ConsumableKind::Food => self.food = Some(status.clone()),
                ConsumableKind::Serum => self.serum = Some(status.clone()),
            }
            self.emit_mechanics(true);
        }
        self.emit_dps();"#,
        "track consumables for all synchronized players",
    );
    replace_once(
        &mut source,
        "            imagines.sort_by_key(|badge| badge.skill_id);\n\n            rows.push(DpsRow {",
        r#"            imagines.sort_by_key(|badge| badge.skill_id);
            let consumable_host=if meta.actor_uuid!=0{meta.actor_uuid}else{canonical_player_uuid(uid)};
            let food=self.player_consumable(consumable_host,ConsumableKind::Food);
            let serum=self.player_consumable(consumable_host,ConsumableKind::Serum);

            rows.push(DpsRow {"#,
        "attach consumables to DPS player rows",
    );
    replace_once(
        &mut source,
        "                attributes,\n                imagines,",
        "                attributes,\n                food,\n                serum,\n                imagines,",
        "publish consumables on DPS rows",
    );
    replace_once(
        &mut source,
        "fn observed_buff_expiry(info: &[u8]) -> i64 {\n    let duration = proto::get_varint_field(info, 11).unwrap_or(0).min(24 * 60 * 60 * 1000) as i64;",
        "fn observed_buff_duration(info:&[u8])->i64{proto::get_varint_field(info,11).unwrap_or(0).min(24*60*60*1000)as i64}\n\nfn observed_buff_expiry(info: &[u8]) -> i64 {\n    let duration = observed_buff_duration(info);",
        "share authoritative buff duration with countdown rings",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1165_party_consumable_tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn remote_player_food_is_kept_and_published_on_dps_row(){
        let(tx,rx)=mpsc::channel();let mut runtime=TelemetryRuntime::new(tx);runtime.local_uid=42;
        let remote_uid=77;let host=canonical_player_uuid(remote_uid);runtime.team.insert(remote_uid);runtime.combat.entry(remote_uid).or_default();
        runtime.observe_consumable(host,123,2_010_003,&[]);
        let mut latest=None;while let Ok(event)=rx.try_recv(){if let AppEvent::Dps(snapshot)=event{latest=Some(snapshot);}}
        let snapshot=latest.expect("remote consumable should emit DPS snapshot");
        let row=snapshot.rows.iter().find(|row|row.uid==remote_uid).expect("remote roster row");
        assert_eq!(row.food.as_ref().map(|x|x.buff_id),Some(2_010_003));
    }

    #[test]
    fn remote_consumable_does_not_replace_local_mechanics_food(){
        let(tx,_rx)=mpsc::channel();let mut runtime=TelemetryRuntime::new(tx);runtime.local_uid=42;
        runtime.observe_consumable(canonical_player_uuid(42),1,2_010_003,&[]);
        let local=runtime.food.as_ref().map(|x|x.buff_id);
        runtime.observe_consumable(canonical_player_uuid(77),2,2_010_005,&[]);
        assert_eq!(runtime.food.as_ref().map(|x|x.buff_id),local);
    }

    #[test]
    fn authoritative_remote_snapshot_removes_stale_indicator(){
        let(tx,_rx)=mpsc::channel();let mut runtime=TelemetryRuntime::new(tx);runtime.local_uid=42;
        let host=canonical_player_uuid(77);runtime.observe_consumable(host,123,2_010_003,&[]);
        assert!(runtime.player_consumable(host,ConsumableKind::Food).is_some());
        runtime.handle_buff_snapshot(host,&[]);
        assert!(runtime.player_consumable(host,ConsumableKind::Food).is_none());
    }
}
"#);
    fs::write(path,source).expect("write v1.16.5 telemetry");
}

fn patch_overlay(out:&Path){
    let path=out.join("feature_overlays_v170_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read generated v1.16.5 overlay");

    replace_once(
        &mut source,
        "BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW, CreateSolidBrush, DeleteDC, DeleteObject, DrawTextW, EndPaint, FillRect, GetStockObject,",
        "Arc as GdiArc, BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW, CreatePen, CreateSolidBrush, DeleteDC, DeleteObject, DrawTextW, Ellipse, EndPaint, FillRect, GetStockObject,",
        "import circular consumable drawing APIs",
    );
    replace_once(
        &mut source,
        "InvalidateRect, SelectObject, SetBkMode, SetTextColor, StretchDIBits, BITMAPINFO, DEFAULT_GUI_FONT, DIB_RGB_COLORS, HDC, HFONT, PAINTSTRUCT, SRCCOPY,\nTRANSPARENT,",
        "InvalidateRect, SelectObject, SetArcDirection, SetBkMode, SetTextColor, StretchDIBits, AD_CLOCKWISE, BITMAPINFO, DEFAULT_GUI_FONT, DIB_RGB_COLORS, HDC, HFONT, NULL_BRUSH, PAINTSTRUCT, PS_SOLID, SRCCOPY,\nTRANSPARENT,",
        "import countdown ring constants",
    );
    replace_once(
        &mut source,
        "const DPS_ROW_H: i32 = 33;",
        "const DPS_ROW_H: i32 = 33;\nconst DPS_CONSUMABLE_ICON:i32=17;\nconst DPS_CONSUMABLE_GAP:i32=2;\nconst DPS_CONSUMABLE_GUTTER:i32=40;",
        "reserve an external indicator gutter without widening the meter",
    );
    replace_once(
        &mut source,
        "let hr=RECT{left:6,top:head_top,right:rc.right-8,bottom:head_top+19};",
        "let hr=RECT{left:6+DPS_CONSUMABLE_GUTTER,top:head_top,right:rc.right-8,bottom:head_top+19};",
        "align player header with inset row box",
    );
    replace_once(
        &mut source,
        "let r=RECT{left:6,top:y,right:rc.right-8,bottom:y+DPS_ROW_H-2};let bg=if row.is_dead",
        "let r=RECT{left:6+DPS_CONSUMABLE_GUTTER,top:y,right:rc.right-8,bottom:y+DPS_ROW_H-2};paint_player_consumables(hdc,row,y,now_ms());let bg=if row.is_dead",
        "paint F S indicators outside the colored row box",
    );
    replace_once(
        &mut source,
        "let r=RECT{left:6,top:top+screen_i as i32*DPS_ROW_H,right:rc.right-8,bottom:top+screen_i as i32*DPS_ROW_H+DPS_ROW_H-2};let layout=dps_row_layout(r,true,row);",
        "let r=RECT{left:6+DPS_CONSUMABLE_GUTTER,top:top+screen_i as i32*DPS_ROW_H,right:rc.right-8,bottom:top+screen_i as i32*DPS_ROW_H+DPS_ROW_H-2};let layout=dps_row_layout(r,true,row);",
        "keep Imagine hover aligned after row inset",
    );
    replace_once(
        &mut source,
        "let next=overlay_help(hwnd,state,x,y).or_else(||hover_badge_at(hwnd,state,x,y));",
        "let next=overlay_help(hwnd,state,x,y).or_else(||hover_consumable_at(hwnd,state,x,y)).or_else(||hover_badge_at(hwnd,state,x,y));",
        "add consumable hover help",
    );

    let helpers=r#"
fn active_consumable(status:Option<&ConsumableStatus>,now:i64)->Option<&ConsumableStatus>{status.filter(|s|s.expires_unix_ms<=0||s.expires_unix_ms>now)}
fn consumable_remaining_text(status:&ConsumableStatus,now:i64)->String{if status.expires_unix_ms<=0{"ACTIVE".into()}else{format!("{} left",format_countdown(status.expires_unix_ms.saturating_sub(now).max(0)as u64))}}
unsafe fn paint_consumable_ring(hdc:HDC,x:i32,y:i32,label:&str,status:&ConsumableStatus,now:i64){
    let d=DPS_CONSUMABLE_ICON;let cx=x+d/2;let cy=y+d/2;let bg=CreateSolidBrush(rgb(34,39,46));let outline_pen=CreatePen(PS_SOLID,1,rgb(110,48,48));let old_brush=SelectObject(hdc,bg);let old_pen=SelectObject(hdc,outline_pen);Ellipse(hdc,x,y,x+d,y+d);SelectObject(hdc,old_pen);SelectObject(hdc,old_brush);DeleteObject(outline_pen);DeleteObject(bg);
    let fraction=if status.expires_unix_ms<=0||status.duration_ms<=0{1.0}else{(status.expires_unix_ms.saturating_sub(now).max(0)as f64/status.duration_ms.max(1)as f64).clamp(0.0,1.0)};
    let pen=CreatePen(PS_SOLID,2,rgb(255,0,0));let old=SelectObject(hdc,pen);if fraction>=0.995{let brush=SelectObject(hdc,GetStockObject(NULL_BRUSH));Ellipse(hdc,x,y,x+d,y+d);SelectObject(hdc,brush);}else if fraction>0.002{let old_dir=SetArcDirection(hdc,AD_CLOCKWISE);let radius=(d/2)as f64;let angle=-std::f64::consts::FRAC_PI_2+fraction*std::f64::consts::TAU;let sx=cx;let sy=y;let ex=cx+(angle.cos()*radius).round()as i32;let ey=cy+(angle.sin()*radius).round()as i32;GdiArc(hdc,x,y,x+d,y+d,sx,sy,ex,ey);SetArcDirection(hdc,old_dir);}SelectObject(hdc,old);DeleteObject(pen);
    SelectObject(hdc,GetStockObject(DEFAULT_GUI_FONT));SetTextColor(hdc,rgb(245,245,245));draw(hdc,label,RECT{left:x,top:y,right:x+d,bottom:y+d},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);
}
unsafe fn paint_player_consumables(hdc:HDC,row:&DpsRow,row_top:i32,now:i64){let y=row_top+((DPS_ROW_H-2-DPS_CONSUMABLE_ICON)/2).max(0);let mut x=6;if let Some(food)=active_consumable(row.food.as_ref(),now){paint_consumable_ring(hdc,x,y,"F",food,now);}x+=DPS_CONSUMABLE_ICON+DPS_CONSUMABLE_GAP;if let Some(serum)=active_consumable(row.serum.as_ref(),now){paint_consumable_ring(hdc,x,y,"S",serum,now);}}
unsafe fn hover_consumable_at(hwnd:HWND,state:&State,x:i32,y:i32)->Option<String>{if state.kind!=Kind::Dps||state.collapsed{return None;}let mut rc:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rc);let top=dps_rows_top();if y<top||y>=rc.bottom{return None;}let screen_i=((y-top)/DPS_ROW_H)as usize;if screen_i>=visible_dps_rows(rc.bottom){return None;}let rows=meter_rows(state);let row=*rows.get(state.scroll+screen_i)?;let now=now_ms();let iy=top+screen_i as i32*DPS_ROW_H+((DPS_ROW_H-2-DPS_CONSUMABLE_ICON)/2).max(0);let food_x=6;let serum_x=food_x+DPS_CONSUMABLE_ICON+DPS_CONSUMABLE_GAP;if x>=food_x&&x<food_x+DPS_CONSUMABLE_ICON&&y>=iy&&y<iy+DPS_CONSUMABLE_ICON{let status=active_consumable(row.food.as_ref(),now)?;return Some(format!("Food: {} · {}",status.name,consumable_remaining_text(status,now)));}if x>=serum_x&&x<serum_x+DPS_CONSUMABLE_ICON&&y>=iy&&y<iy+DPS_CONSUMABLE_ICON{let status=active_consumable(row.serum.as_ref(),now)?;return Some(format!("Serum: {} · {}",status.name,consumable_remaining_text(status,now)));}None}
"#;
    replace_once(
        &mut source,
        "unsafe fn hover_badge_at(hwnd:HWND,state:&State,x:i32,y:i32)->Option<String>{",
        &format!("{}\nunsafe fn hover_badge_at(hwnd:HWND,state:&State,x:i32,y:i32)->Option<String>{{",helpers),
        "insert consumable indicator renderer and hover hit testing",
    );
    source.push_str(r#"

#[cfg(test)]
mod v1165_consumable_overlay_tests{
    use super::*;
    #[test]fn expired_consumable_is_not_active(){let status=ConsumableStatus{buff_id:1,name:"Food".into(),expires_unix_ms:100,duration_ms:1_000};assert!(active_consumable(Some(&status),101).is_none());}
    #[test]fn active_consumable_keeps_hover_timer(){let status=ConsumableStatus{buff_id:1,name:"Food".into(),expires_unix_ms:61_000,duration_ms:60_000};assert_eq!(consumable_remaining_text(&status,1_000),"1:00 left");}
}
"#);
    fs::write(path,source).expect("write v1.16.5 overlay");
}

fn main(){
    prior::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_telemetry(&out);
    patch_overlay(&out);
    println!("cargo:rerun-if-changed=build_v1165.rs");
}
