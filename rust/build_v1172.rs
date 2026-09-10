use std::{env, fs, path::{Path, PathBuf}};

mod prior {
    include!("build_v1171.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.17.2 patch {label} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn patch_telemetry(out: &Path) {
    let path = out.join("telemetry_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read v1.17.1 telemetry").replace("\r\n", "\n");

    replace_once(&mut source, "    last_mechanic_emit: Instant,", "    mechanics_dirty: bool,", "mechanics dirty state");
    replace_once(&mut source, "            last_mechanic_emit: now.checked_sub(Duration::from_secs(1)).unwrap_or(now),", "            mechanics_dirty: false,", "mechanics dirty init");
    replace_once(&mut source, "        self.prune_expired_mechanics();\n    }", "        self.prune_expired_mechanics();\n        self.flush_mechanics_dirty();\n    }", "packet-boundary mechanics flush");
    replace_once(&mut source,
        "            let key = format!(\"buff:{host}:{buff_uuid}:{base_id}\");",
        "            let key = format!(\"buff:{host}:{base_id}\");\n            self.buff_instances.retain(|(entry_host, entry_uuid), existing_key| !(*entry_host == host && *entry_uuid != buff_uuid && existing_key == &key));",
        "regular buff overwrite");
    replace_once(&mut source,
        "        let key = format!(\"trackedbuff:{host}:{buff_uuid}:{base_id}\");",
        "        let key = format!(\"trackedbuff:{host}:{base_id}\");\n        self.tracked_buff_instances.retain(|(entry_host, entry_uuid), existing_key| !(*entry_host == host && *entry_uuid != buff_uuid && existing_key == &key));",
        "tracked buff overwrite");
    replace_once(&mut source,
        "        if !force && self.last_mechanic_emit.elapsed() < Duration::from_millis(50) {\n            return;\n        }\n        self.last_mechanic_emit = Instant::now();",
        "        if !force { self.mechanics_dirty = true; return; }\n        self.mechanics_dirty = false;",
        "queue mechanics snapshot");
    replace_once(&mut source,
        "        }));\n    }\n}\n\n#[derive(Default)]\nstruct TeamCandidate",
        "        }));\n    }\n\n    fn flush_mechanics_dirty(&mut self) {\n        if self.mechanics_dirty { self.emit_mechanics(true); }\n    }\n}\n\n#[derive(Default)]\nstruct TeamCandidate",
        "flush mechanics helper");

    source.push_str(r#"

#[cfg(test)]
mod v1172_mechanics_tests {
    use super::*;

    #[test]
    fn tina_update_flushes_without_damage_packet() {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut runtime = TelemetryRuntime::new(tx);
        runtime.local_uid = 42;
        let host = canonical_player_uuid(42);
        runtime.observe_special_buff(host, 1001, 2_110_056, &[]);
        assert!(runtime.mechanics_dirty);
        assert!(rx.try_recv().is_err());
        runtime.flush_mechanics_dirty();
        let AppEvent::Mechanics(snapshot) = rx.recv_timeout(Duration::from_millis(50)).expect("mechanics snapshot") else { panic!("expected mechanics event"); };
        assert_eq!(snapshot.rows.iter().filter(|row| row.label == "TINA").count(), 1);
    }

    #[test]
    fn repeated_bl_base_id_overwrites_previous_uuid() {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut runtime = TelemetryRuntime::new(tx);
        runtime.local_uid = 42;
        let host = canonical_player_uuid(42);
        runtime.observe_special_buff(host, 2001, 2_110_065, &[]);
        runtime.observe_special_buff(host, 2002, 2_110_065, &[]);
        assert_eq!(runtime.mechanics.values().filter(|row| row.label == "BL").count(), 1);
        assert_eq!(runtime.tracked_buff_instances.len(), 1);
        assert!(!runtime.tracked_buff_instances.contains_key(&(host, 2001)));
        assert!(runtime.tracked_buff_instances.contains_key(&(host, 2002)));
        runtime.flush_mechanics_dirty();
        let AppEvent::Mechanics(snapshot) = rx.recv_timeout(Duration::from_millis(50)).expect("BL snapshot") else { panic!("expected mechanics event"); };
        assert_eq!(snapshot.rows.iter().filter(|row| row.label == "BL").count(), 1);
    }
}
"#);
    fs::write(path, source).expect("write v1.17.2 telemetry");
}

fn patch_overlay(out: &Path) {
    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read v1.17.1 overlays").replace("\r\n", "\n");

    replace_once(&mut source,
        "let limit=settings.meter.visible_rows;if limit>0&&rows.len()>limit{let local=if settings.meter.always_show_self{rows.iter().position(|row|row.is_local).and_then(|index|(index>=limit).then_some(rows[index]))}else{None};rows.truncate(limit);if let Some(local)=local{if !rows.is_empty(){rows.pop();}rows.push(local);}}rows}",
        "rows}\nfn meter_regular_page_size(state:&State,physical:usize)->usize{if physical==0{return 0;}let configured=state.features.read().map(|f|f.meter.visible_rows).unwrap_or(0);if configured==0{physical}else{configured.min(physical)}}\nfn sticky_rank_indices(total:usize,local_index:Option<usize>,scroll:usize,regular_page:usize,physical:usize)->Vec<usize>{if total==0||physical==0||regular_page==0{return Vec::new();}let page=regular_page.min(total).max(1);let start=scroll.min(total.saturating_sub(page));let end=(start+page).min(total);let mut indices:Vec<usize>=(start..end).collect();if indices.len()<physical{if let Some(local)=local_index.filter(|index|*index<total){if !indices.contains(&local){indices.push(local);}}}indices}\nfn meter_page_rows<'a>(state:&'a State,physical:usize)->Vec<(usize,&'a DpsRow)>{let rows=meter_rows(state);let regular=meter_regular_page_size(state,physical);let keep_self=state.features.read().map(|f|f.meter.always_show_self).unwrap_or(true);let local_index=keep_self.then(||rows.iter().position(|row|row.is_local)).flatten();sticky_rank_indices(rows.len(),local_index,state.scroll,regular,physical).into_iter().filter_map(|index|rows.get(index).map(|row|(index+1,*row))).collect()}",
        "sticky self paging");
    replace_once(&mut source,
        "if y<0{return None;}let screen_i=(y/DPS_ROW_H)as usize;let rows=meter_rows(state);let row=*rows.get(state.scroll+screen_i)?;",
        "if y<0{return None;}let screen_i=(y/DPS_ROW_H)as usize;let physical=(((state.expanded.bottom-state.expanded.top)-dps_rows_top())/DPS_ROW_H).max(0)as usize;let shown=meter_page_rows(state,physical);let row=shown.get(screen_i).map(|(_,row)|*row)?;",
        "consumable popup sticky hit row");
    replace_once(&mut source,
        "let now=now_ms();let rows=meter_rows(parent);let visible=(rc.bottom/DPS_ROW_H).max(0)as usize;\n        for screen_i in 0..visible{let Some(row)=rows.get(parent.scroll+screen_i).copied()else{break;};",
        "let now=now_ms();let visible=(rc.bottom/DPS_ROW_H).max(0)as usize;let shown=meter_page_rows(parent,visible);\n        for(screen_i,(_,row))in shown.iter().enumerate(){let row=*row;",
        "consumable popup sticky paint row");
    replace_once(&mut source,
        "if screen_i>=visible_dps_rows(rc.bottom){return None;}let rows=meter_rows(state);let row=*rows.get(state.scroll+screen_i)?;",
        "let physical=visible_dps_rows(rc.bottom);if screen_i>=physical{return None;}let shown=meter_page_rows(state,physical);let row=shown.get(screen_i).map(|(_,row)|*row)?;",
        "Imagine hover sticky row");
    replace_once(&mut source,
        "if screen_i>=visible_dps_rows(rc.bottom){return None;}meter_rows(state).get(state.scroll+screen_i).map(|row|(*row).clone())}",
        "let physical=visible_dps_rows(rc.bottom);if screen_i>=physical{return None;}meter_page_rows(state,physical).get(screen_i).map(|(_,row)|(*row).clone())}",
        "DPS click sticky row");
    replace_once(&mut source,
        "let leader=rows.first().map(|row|mode_metric(row,state.sort_mode)).unwrap_or(0).max(1);let bold_font=if state.font.is_null(){GetStockObject(DEFAULT_GUI_FONT)}else{state.font};for(screen_i,row)in rows.iter().skip(state.scroll).take(visible).enumerate(){let rank=state.scroll+screen_i+1;",
        "let leader=rows.first().map(|row|mode_metric(row,state.sort_mode)).unwrap_or(0).max(1);let shown=meter_page_rows(state,visible);let bold_font=if state.font.is_null(){GetStockObject(DEFAULT_GUI_FONT)}else{state.font};for(screen_i,(rank,row))in shown.iter().enumerate(){let rank=*rank;let row=*row;",
        "DPS sticky self paint/rank");
    replace_once(&mut source, "paint_scrollbar(hdc,rc,rows.len(),visible,state.scroll,top);", "paint_scrollbar(hdc,rc,rows.len(),meter_regular_page_size(state,visible),state.scroll,top);", "DPS sticky scrollbar");
    replace_once(&mut source,
        "unsafe fn clamp_scroll(hwnd:HWND,state:&mut State){let mut rc:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rc);let(total,visible)=if state.kind==Kind::Dps{(meter_rows(state).len(),visible_dps_rows(rc.bottom))}",
        "unsafe fn clamp_scroll(hwnd:HWND,state:&mut State){let mut rc:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rc);let(total,visible)=if state.kind==Kind::Dps{let physical=visible_dps_rows(rc.bottom);(meter_rows(state).len(),meter_regular_page_size(state,physical))}",
        "DPS sticky scroll bounds");
    replace_once(&mut source,
        "let(total,page)=if state.kind==Kind::Dps{(meter_rows(state).len(),visible_dps_rows(rc.bottom))}",
        "let(total,page)=if state.kind==Kind::Dps{let physical=visible_dps_rows(rc.bottom);(meter_rows(state).len(),meter_regular_page_size(state,physical))}",
        "DPS sticky keyboard page");

    replace_once(&mut source,
        "unsafe fn create_overlay_font(bold:bool)->HFONT{let face=wide(\"Segoe UI\");CreateFontW(if bold{-13}else{-14},0,0,0,if bold{700}else{400},0,0,0,1,0,0,5,0,face.as_ptr())}",
        "unsafe fn create_overlay_font(bold:bool)->HFONT{let face=wide(\"Segoe UI\");CreateFontW(if bold{-13}else{-14},0,0,0,if bold{700}else{400},0,0,0,1,0,0,5,0,face.as_ptr())}\nfn mechanic_attr_overlay_label<'a>(id:i32,fallback:&'a str)->&'a str{match id{feature_settings::ATTR_LUCKY_DAMAGE_MULTIPLIER=>\"Luck DMG\",feature_settings::ATTR_CRITICAL_DAMAGE=>\"Crit DMG\",feature_settings::ATTR_ATTACK_SPEED=>\"Atk Spd\",feature_settings::ATTR_CAST_SPEED=>\"Cast Spd\",feature_settings::ATTR_PANEL_STRENGTH=>\"STR\",feature_settings::ATTR_PANEL_INTELLIGENCE=>\"INT\",feature_settings::ATTR_PANEL_AGILITY=>\"AGI\",feature_settings::ATTR_PANEL_PHYSICAL_ATTACK=>\"Phy Atk\",feature_settings::ATTR_PANEL_MAGIC_ATTACK=>\"Mag Atk\",feature_settings::ATTR_PANEL_PHYSICAL_DEFENSE=>\"Phy Def\",feature_settings::ATTR_SHIELD_STRENGTH=>\"Shield\",feature_settings::ATTR_BLOCK_DAMAGE_REDUCTION=>\"Block DR\",feature_settings::ATTR_COOLDOWN_REDUCTION=>\"CD R\",feature_settings::ATTR_CD_ACCELERATE_PCT=>\"CD A\",feature_settings::ATTR_VERSATILITY=>\"Versa\",_=>fallback}}",
        "mechanics short labels");
    replace_once(&mut source,
        "format!(\"{} {}\",attr.label.trim_end_matches(\" %\"),format_attr_value(attr.attr_id,attr.value))",
        "format!(\"{} {}\",mechanic_attr_overlay_label(attr.attr_id,attr.label.trim_end_matches(\" %\")),format_attr_value(attr.attr_id,attr.value))",
        "mechanics use short labels");

    replace_once(&mut source,
        "draw(hdc,if state.kind==Kind::Dps{\"D\"}else{\"M\"},rc,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);",
        "draw(hdc,expand_glyph(state),rc,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);",
        "collapsed edge direction icon");
    replace_once(&mut source,
        "draw(hdc,\"◀\",RECT{left:rc.right-BUTTON_W*2,top:0,right:rc.right-BUTTON_W,bottom:TOOLBAR_H},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);",
        "draw(hdc,collapse_glyph(state),RECT{left:rc.right-BUTTON_W*2,top:0,right:rc.right-BUTTON_W,bottom:TOOLBAR_H},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);",
        "toolbar collapse direction icon");
    replace_once(&mut source,
        "fn layout_side(state:&State)->String{",
        "fn collapse_glyph_for_side(side:&str)->&'static str{match side.to_ascii_lowercase().as_str(){\"left\"=>\"◀\",\"top\"=>\"▲\",\"bottom\"=>\"▼\",_=>\"▶\"}}\nfn expand_glyph_for_side(side:&str)->&'static str{match side.to_ascii_lowercase().as_str(){\"left\"=>\"▶\",\"top\"=>\"▼\",\"bottom\"=>\"▲\",_=>\"◀\"}}\nfn collapse_glyph(state:&State)->&'static str{collapse_glyph_for_side(&layout_side(state))}\nfn expand_glyph(state:&State)->&'static str{expand_glyph_for_side(&layout_side(state))}\nfn layout_side(state:&State)->String{",
        "collapse direction helpers");

    source.push_str(r#"

#[cfg(test)]
mod v1172_overlay_tests {
    use super::*;

    #[test]
    fn sticky_self_keeps_top_n_and_real_rank() {
        let top = sticky_rank_indices(14, Some(13), 0, 5, 10);
        assert_eq!(top, vec![0, 1, 2, 3, 4, 13]);
        assert_eq!(top.iter().map(|index| index + 1).collect::<Vec<_>>(), vec![1, 2, 3, 4, 5, 14]);
        assert_eq!(sticky_rank_indices(10, Some(5), 0, 5, 10), vec![0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn sticky_self_disappears_when_scrolled_to_owner() {
        assert_eq!(sticky_rank_indices(14, Some(13), 9, 5, 10), vec![9, 10, 11, 12, 13]);
    }

    #[test]
    fn requested_mechanics_short_labels_are_overlay_only() {
        assert_eq!(mechanic_attr_overlay_label(feature_settings::ATTR_LUCKY_DAMAGE_MULTIPLIER,"x"),"Luck DMG");
        assert_eq!(mechanic_attr_overlay_label(feature_settings::ATTR_CRITICAL_DAMAGE,"x"),"Crit DMG");
        assert_eq!(mechanic_attr_overlay_label(feature_settings::ATTR_ATTACK_SPEED,"x"),"Atk Spd");
        assert_eq!(mechanic_attr_overlay_label(feature_settings::ATTR_CAST_SPEED,"x"),"Cast Spd");
        assert_eq!(mechanic_attr_overlay_label(feature_settings::ATTR_PANEL_STRENGTH,"x"),"STR");
        assert_eq!(mechanic_attr_overlay_label(feature_settings::ATTR_PANEL_INTELLIGENCE,"x"),"INT");
        assert_eq!(mechanic_attr_overlay_label(feature_settings::ATTR_PANEL_AGILITY,"x"),"AGI");
        assert_eq!(mechanic_attr_overlay_label(feature_settings::ATTR_PANEL_PHYSICAL_ATTACK,"x"),"Phy Atk");
        assert_eq!(mechanic_attr_overlay_label(feature_settings::ATTR_PANEL_MAGIC_ATTACK,"x"),"Mag Atk");
        assert_eq!(mechanic_attr_overlay_label(feature_settings::ATTR_PANEL_PHYSICAL_DEFENSE,"x"),"Phy Def");
        assert_eq!(mechanic_attr_overlay_label(feature_settings::ATTR_SHIELD_STRENGTH,"x"),"Shield");
        assert_eq!(mechanic_attr_overlay_label(feature_settings::ATTR_BLOCK_DAMAGE_REDUCTION,"x"),"Block DR");
        assert_eq!(mechanic_attr_overlay_label(feature_settings::ATTR_COOLDOWN_REDUCTION,"x"),"CD R");
        assert_eq!(mechanic_attr_overlay_label(feature_settings::ATTR_CD_ACCELERATE_PCT,"x"),"CD A");
        assert_eq!(mechanic_attr_overlay_label(feature_settings::ATTR_VERSATILITY,"x"),"Versa");
        assert_eq!(mechanic_attr_overlay_label(feature_settings::ATTR_HASTE,"Haste"),"Haste");
    }

    #[test]
    fn feature_overlay_arrows_match_chat_overlay() {
        assert_eq!((collapse_glyph_for_side("left"),expand_glyph_for_side("left")),("◀","▶"));
        assert_eq!((collapse_glyph_for_side("right"),expand_glyph_for_side("right")),("▶","◀"));
        assert_eq!((collapse_glyph_for_side("top"),expand_glyph_for_side("top")),("▲","▼"));
        assert_eq!((collapse_glyph_for_side("bottom"),expand_glyph_for_side("bottom")),("▼","▲"));
    }
}
"#);
    fs::write(path, source).expect("write v1.17.2 overlays");
}

fn main() {
    prior::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_telemetry(&out);
    patch_overlay(&out);
    println!("cargo:rerun-if-changed=build_v1172.rs");
}
