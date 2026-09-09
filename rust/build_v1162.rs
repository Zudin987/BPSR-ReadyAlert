use std::{env, fs, path::{Path, PathBuf}};

mod prior {
    include!("build_v1161.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.16.2 patch {label} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn replace_between(source: &mut String, start: &str, end: &str, replacement: &str, label: &str) {
    let start_count = source.matches(start).count();
    let end_count = source.matches(end).count();
    assert_eq!(start_count, 1, "v1.16.2 patch {label} start expected one match, found {start_count}");
    assert_eq!(end_count, 1, "v1.16.2 patch {label} end expected one match, found {end_count}");
    let a = source.find(start).expect("v1.16.2 start anchor");
    let b = source[a..].find(end).map(|i| a + i).expect("v1.16.2 end anchor");
    source.replace_range(a..b, replacement);
}

fn patch_overlay(out: &Path) {
    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read generated v1.16.2 overlay");

    replace_once(&mut source, "const DPS_ROW_H: i32 = 42;", "const DPS_ROW_H: i32 = 36;", "compact DPS row height");

    let identity_layout = r#"#[derive(Clone,Copy)]struct DpsRowLayout{name_left:i32,name_right:i32,spec_left:i32,spec_right:i32,badge_left:i32,badge_count:usize,total_left:i32,total_right:i32,active_left:i32,active_right:i32,share_left:i32,share_right:i32,death_left:i32,death_right:i32}
fn identity_text_px(text:&str,bold:bool)->i32{text.chars().map(|ch|if ch.is_ascii(){if bold{8}else{7}}else{14}).sum::<i32>().max(0)}
fn dps_secondary(row:&DpsRow)->String{let spec=dps_spec(row);let score=dps_score_pair(row);if score.is_empty(){spec}else if spec.is_empty(){score}else{format!("{} · {}",spec,score)}}
fn dps_row_layout(r:RECT,show_imagines:bool,row:&DpsRow)->DpsRowLayout{let width=(r.right-r.left).max(1);let death_w=if width>=760{24}else{20};let share_w=50;let active_w=74;let total_w=80;let gap=3;let death_right=r.right-4;let death_left=death_right-death_w;let share_right=death_left-gap;let share_left=share_right-share_w;let active_right=share_left-gap;let active_left=active_right-active_w;let total_right=active_left-gap;let total_left=total_right-total_w;let name_left=r.left+25;let badge_count=if show_imagines&&!row.is_dead{row.imagines.len().min(2)}else{0};let badge_span=badge_count as i32*BADGE_W+(badge_count.saturating_sub(1)as i32)*BADGE_GAP;let identity_right=if badge_count>0{(total_left-badge_span-4).max(name_left+80)}else{(total_left-4).max(name_left+80)};let name_w=identity_text_px(&row.name,true);let name_right=(name_left+name_w).min(identity_right);let spec_left=(name_right+5).min(identity_right);let secondary=dps_secondary(row);let spec_w=identity_text_px(&secondary,false);let spec_right=(spec_left+spec_w).min(identity_right);let badge_left=if badge_count>0{(spec_right+4).min(total_left-badge_span-2)}else{total_left};DpsRowLayout{name_left,name_right,spec_left,spec_right,badge_left,badge_count,total_left,total_right,active_left,active_right,share_left,share_right,death_left,death_right}}
"#;
    replace_between(&mut source, "#[derive(Clone,Copy)]struct DpsRowLayout", "fn dps_identity(row:&DpsRow)->String{", identity_layout, "left-packed meter identity and metrics");

    let hover = r#"unsafe fn hover_badge_at(hwnd:HWND,state:&State,x:i32,y:i32)->Option<String>{if state.kind!=Kind::Dps||state.collapsed||!state.features.read().map(|f|f.meter.show_imagines).unwrap_or(true){return None;}let mut rc:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rc);let top=dps_rows_top();if y<top||y>=rc.bottom{return None;}let screen_i=((y-top)/DPS_ROW_H)as usize;if screen_i>=visible_dps_rows(rc.bottom){return None;}let rows=meter_rows(state);let row=*rows.get(state.scroll+screen_i)?;if row.is_dead{return None;}let r=RECT{left:6,top:top+screen_i as i32*DPS_ROW_H,right:rc.right-8,bottom:top+screen_i as i32*DPS_ROW_H+DPS_ROW_H-2};let layout=dps_row_layout(r,true,row);if layout.badge_count==0{return None;}let mut bx=layout.badge_left;for badge in row.imagines.iter().take(layout.badge_count){if x>=bx&&x<bx+BADGE_W&&y>=r.top+4&&y<r.top+27{return Some(format!("{} · {}",badge.name,crate::model::imagine_tier_label(badge.tier)));}bx+=BADGE_W+BADGE_GAP;}None}
"#;
    replace_between(&mut source, "unsafe fn hover_badge_at(hwnd:HWND,state:&State,x:i32,y:i32)->Option<String>{", "unsafe fn paint_hover", hover, "Imagine hover without width cutoff");

    replace_once(&mut source,
        "draw(hdc,&row.name,RECT{left:layout.name_left,top:r.top,right:layout.name_right,bottom:content_bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);",
        "draw(hdc,&row.name,RECT{left:layout.name_left,top:r.top,right:layout.name_right,bottom:content_bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);",
        "full player name without ellipsis");
    replace_once(&mut source,
        "draw(hdc,&secondary,RECT{left:layout.spec_left,top:r.top,right:layout.spec_right,bottom:content_bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);",
        "draw(hdc,&secondary,RECT{left:layout.spec_left,top:r.top,right:layout.spec_right,bottom:content_bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);",
        "identity details without ellipsis");
    replace_once(&mut source,
        "if settings.meter.show_imagines&&!row.is_dead&&(r.right-r.left)>=690{let mut bx=layout.badge_left;for badge in row.imagines.iter().take(layout.badge_count){paint_badge(hdc,bx,r.top+6,badge);bx+=BADGE_W+BADGE_GAP;}}",
        "if settings.meter.show_imagines&&!row.is_dead&&layout.badge_count>0{let mut bx=layout.badge_left;for badge in row.imagines.iter().take(layout.badge_count){paint_badge(hdc,bx,r.top+4,badge);bx+=BADGE_W+BADGE_GAP;}}",
        "keep Imagine badges visible when narrow");

    fs::write(path, source).expect("write v1.16.2 overlay");
}

fn patch_telemetry(out: &Path) {
    let path = out.join("telemetry_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read generated v1.16.2 telemetry");

    replace_once(&mut source,
        "const ATTR_SKILL_ID: u64 = 0x64;\nconst ATTR_SKILL_REMODEL_LEVEL: u64 = 0x79;",
        "const ATTR_SKILL_ID: u64 = 0x64;\nconst ATTR_SKILL_LEVEL_ID_LIST: u64 = 0x74;\nconst ATTR_SKILL_REMODEL_LEVEL: u64 = 0x79;",
        "SkillLevelIdList attribute id");

    let decoder = r#"fn decode_imagine_skill_tiers(raw:&[u8])->Vec<(i32,i32)>{
    proto::len_fields(raw,1).into_iter().filter_map(|entry|{
        let skill=proto::get_varint_field(entry,1)?;
        let skill=i32::try_from(skill).ok()?;
        if !is_fantasy_skill(skill){return None;}
        let tier=proto::get_varint_field(entry,3).unwrap_or(0).min(i32::MAX as u64) as i32;
        Some((skill,tier))
    }).collect()
}

"#;
    replace_once(&mut source,
        "    fn update_entity_from_attrs(&mut self, uuid: i64, attrs: Option<&[u8]>) {",
        &format!("{decoder}    fn update_entity_from_attrs(&mut self, uuid: i64, attrs: Option<&[u8]>) {{"),
        "per-skill Imagine tier decoder");

    replace_once(&mut source,
        "            let old_state = self.players.get(&uid).map(|p| p.actor_state).unwrap_or_default();\n            let mut actor_state_seen = false;",
        "            let old_state = self.players.get(&uid).map(|p| p.actor_state).unwrap_or_default();\n            let mut actor_state_seen = false;\n            let mut player_imagine_tiers: Vec<(i32,i32)> = Vec::new();",
        "player Imagine tier staging");

    replace_once(&mut source,
        "                        ATTR_PROFESSION_ID => {",
        "                        ATTR_SKILL_LEVEL_ID_LIST => {\n                            player_imagine_tiers.extend(Self::decode_imagine_skill_tiers(raw));\n                        }\n                        ATTR_PROFESSION_ID => {",
        "read player SkillLevelIdList");

    replace_once(&mut source,
        "            }\n            if actor_state_seen {\n                let new_state = self.players.get(&uid).map(|p| p.actor_state).unwrap_or_default();",
        "            }\n            for (skill,tier) in player_imagine_tiers {\n                let tier=tier.max(0);\n                self.trusted_imagine_tiers.insert((uid,skill),tier);\n                if let Some(combat)=self.combat.get_mut(&uid){combat.imagines.insert(skill,tier);}\n            }\n            if actor_state_seen {\n                let new_state = self.players.get(&uid).map(|p| p.actor_state).unwrap_or_default();",
        "publish authoritative player Imagine tiers");

    replace_once(&mut source,
        "            if self.team.contains(&uid) || uid == self.local_uid {\n                self.combat.entry(uid).or_default();\n            }",
        "            if self.team.contains(&uid) || uid == self.local_uid {\n                let known:Vec<(i32,i32)>=self.trusted_imagine_tiers.iter().filter_map(|(&(owner,skill),&tier)|(owner==uid).then_some((skill,tier))).collect();\n                let combat=self.combat.entry(uid).or_default();\n                for(skill,tier)in known{combat.imagines.insert(skill,tier);}\n            }",
        "hydrate combat row from authoritative Imagine cache");

    let fallback = r#"    fn record_imagine_fallback(&mut self, uid: i64, skill: i32, _tier: i32) {
        // damage.owner_level is part of the hit identity, not a Battle Imagine
        // remodel tier. Never invent T1/Tn from it; 0x74/0x79 are authoritative.
        if let Some(trusted)=self.trusted_imagine_tiers.get(&(uid,skill)).copied(){
            self.combat.entry(uid).or_default().imagines.insert(skill,trusted);
        }
    }

"#;
    replace_between(&mut source,
        "    fn record_imagine_fallback(&mut self, uid: i64, skill: i32, tier: i32) {",
        "    fn reset_encounter_keep_roster(&mut self) {",
        fallback,
        "stop treating damage owner_level as Imagine tier");

    source.push_str(r#"

#[cfg(test)]
mod v1162_imagine_tier_tests {
    use super::*;
    use std::sync::mpsc;

    fn push_varint(mut value:u64,out:&mut Vec<u8>){while value>=0x80{out.push((value as u8&0x7f)|0x80);value>>=7;}out.push(value as u8);}
    fn skill_level(skill:i32,tier:i32)->Vec<u8>{let mut inner=vec![0x08];push_varint(skill as u64,&mut inner);inner.push(0x18);push_varint(tier as u64,&mut inner);let mut outer=vec![0x0a];push_varint(inner.len()as u64,&mut outer);outer.extend(inner);outer}

    #[test]
    fn imagine_tier_skill_level_list_preserves_t0_and_t5() {
        let mut raw=skill_level(3921,0);
        raw.extend(skill_level(3956,5));
        assert_eq!(TelemetryRuntime::decode_imagine_skill_tiers(&raw),vec![(3921,0),(3956,5)]);
    }

    #[test]
    fn imagine_tier_damage_owner_level_is_not_used_as_tier() {
        let (tx,_rx)=mpsc::channel();
        let mut runtime=TelemetryRuntime::new(tx);
        runtime.record_imagine_fallback(42,3921,1);
        assert!(runtime.combat.get(&42).and_then(|a|a.imagines.get(&3921)).is_none());
        runtime.record_imagine_observed(42,3921,0);
        runtime.record_imagine_fallback(42,3921,1);
        assert_eq!(runtime.combat.get(&42).and_then(|a|a.imagines.get(&3921)).copied(),Some(0));
    }
}
"#);

    fs::write(path, source).expect("write v1.16.2 telemetry");
}

fn patch_settings(out: &Path) {
    let path = out.join("settings_ui_v1160_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read generated main settings");
    replace_once(&mut source,
        "WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_THICKFRAME | WS_VSCROLL | WS_HSCROLL,",
        "WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_THICKFRAME,",
        "main settings outer scrollbars");
    replace_once(&mut source,"CW_USEDEFAULT, CW_USEDEFAULT, 900, 710,","CW_USEDEFAULT, CW_USEDEFAULT, 900, 680,","main settings fit height");
    replace_once(&mut source,"create_button(hwnd, ID_APPLY, \"&Apply\", 690, 630, 86, 34);","create_button(hwnd, ID_APPLY, \"&Apply\", 690, 600, 86, 34);","main Apply position");
    replace_once(&mut source,"create_button(hwnd, ID_CLOSE, \"&Close\", 782, 630, 86, 34);","create_button(hwnd, ID_CLOSE, \"&Close\", 782, 600, 86, 34);","main Close position");
    replace_once(&mut source,"(*state_ptr).form.capture(hwnd, 878, 680);","(*state_ptr).form.capture(hwnd, 878, 640);","main settings virtual fit");
    replace_once(&mut source,
        "        WM_MOUSEWHEEL | WM_VSCROLL | WM_HSCROLL => { if !state_ptr.is_null() { (*state_ptr).form.scroll(hwnd,msg,wparam); } 0 }\n",
        "",
        "disable main settings outer scrolling");
    fs::write(path,source).expect("write main settings");

    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read generated feature settings");
    replace_once(&mut source,
        "WS_POPUP|WS_THICKFRAME|0x00c00000|0x00080000|0x00200000|0x00100000|0x02000000",
        "WS_POPUP|WS_THICKFRAME|0x00c00000|0x00080000|0x02000000",
        "feature settings outer scrollbars");
    replace_once(&mut source,
        "        WM_MOUSEWHEEL|0x0114|0x0115=>{state.form.scroll(hwnd,msg,wparam);0}\n",
        "",
        "disable feature settings outer scrolling");
    fs::write(path,source).expect("write feature settings");

    let path = out.join("event_tracker_ui_v1160_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read generated Event Tracker settings");
    replace_once(&mut source,
        "WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | 0x00040000 | WS_VSCROLL | 0x00100000,",
        "WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | 0x00040000,",
        "Event Tracker outer scrollbars");
    replace_once(&mut source,
        "        0x020a | 0x0115 | 0x0114 => { if !ptr.is_null() { (*ptr).form.scroll(hwnd,msg,wparam); } 0 }\n",
        "",
        "disable Event Tracker outer scrolling");
    fs::write(path,source).expect("write Event Tracker settings");
}

fn main() {
    prior::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_overlay(&out);
    patch_telemetry(&out);
    patch_settings(&out);
    println!("cargo:rerun-if-changed=build_v1162.rs");
}
