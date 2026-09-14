use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1332_meter_alignment.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.33.3 {label} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn main() {
    previous::run();

    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .expect("read generated feature overlay for v1.33.3 consumable controls")
        .replace("\r\n", "\n");

    replace_once(
        &mut source,
        r#"        let display=["Damage share %","Healing share %","Tank share %","Death count","Battle Imagine badges","Target / HP summary","Active rate"];
        for(i,label)in display.iter().enumerate(){let col=(i/4)as i32;let row=(i%4)as i32;feature_check(hwnd,7100+i as i32,label,18+col*286,120+row*27,270);}"#,
        r#"        let display=["Damage share %","Healing share %","Tank share %","Death count","Battle Imagine badges","Target / HP summary","Active rate"];
        for(i,label)in display.iter().enumerate(){let col=(i/4)as i32;let row=(i%4)as i32;feature_check(hwnd,7100+i as i32,label,18+col*286,120+row*27,270);}
        feature_check(hwnd,7111,"Food / Serum indicators",304,201,270);"#,
        "Food/Serum settings checkbox",
    );

    replace_once(
        &mut source,
        r#"        for(i,on)in [f.meter.show_damage_share,f.meter.show_healing_share,f.meter.show_tank_share,f.meter.show_deaths,f.meter.show_imagines,f.meter.show_target,f.meter.show_active_rates,f.meter.only_contributors,f.meter.party_only,f.meter.always_show_self,f.meter.remember_scroll].into_iter().enumerate(){SendMessageW(fc(hwnd,7100+i as i32),0x00f1,on as usize,0);}"#,
        r#"        for(i,on)in [f.meter.show_damage_share,f.meter.show_healing_share,f.meter.show_tank_share,f.meter.show_deaths,f.meter.show_imagines,f.meter.show_target,f.meter.show_active_rates,f.meter.only_contributors,f.meter.party_only,f.meter.always_show_self,f.meter.remember_scroll].into_iter().enumerate(){SendMessageW(fc(hwnd,7100+i as i32),0x00f1,on as usize,0);}SendMessageW(fc(hwnd,7111),0x00f1,f.meter.show_consumables as usize,0);"#,
        "Food/Serum settings refresh",
    );

    replace_once(
        &mut source,
        r#"                    7108=>f.meter.party_only=checked,7109=>f.meter.always_show_self=checked,
                    7110=>f.meter.remember_scroll=checked,"#,
        r#"                    7108=>f.meter.party_only=checked,7109=>f.meter.always_show_self=checked,
                    7110=>f.meter.remember_scroll=checked,7111=>f.meter.show_consumables=checked,"#,
        "Food/Serum settings mutation",
    );

    replace_once(
        &mut source,
        r#"const RAID_FS_YELLOW_MS:i64=120_000;
const RAID_FS_RED_MS:i64=30_000;"#,
        "",
        "obsolete consumable threshold constants",
    );

    replace_once(
        &mut source,
        r#"fn raid_consumable_color(status:Option<&ConsumableStatus>,now:i64)->u32{
    let Some(status)=active_consumable(status,now)else{return crate::ui_modern::BPSR_MUTED;};
    if status.expires_unix_ms<=0{return rgb(72,226,116);}let left=status.expires_unix_ms.saturating_sub(now);
    if left<=0{return crate::ui_modern::BPSR_MUTED;}if left<=RAID_FS_RED_MS{return crate::ui_modern::BPSR_DANGER;}if left<=RAID_FS_YELLOW_MS{return rgb(245,190,55);}rgb(72,226,116)
}

unsafe fn paint_raid_fs(hdc:HDC,state:&State,row:Option<&&DpsRow>,x:i32,y:i32,w:i32,h:i32){
    let old=SelectObject(hdc,dps_primary_font(state));let now=now_ms();let(food,serum)=row.map(|r|(raid_consumable_color(r.food.as_ref(),now),raid_consumable_color(r.serum.as_ref(),now))).unwrap_or((crate::ui_modern::BPSR_MUTED,crate::ui_modern::BPSR_MUTED));
    let half=(w/2).max(1);if food!=crate::ui_modern::BPSR_MUTED{SetTextColor(hdc,food);draw(hdc,"F",RECT{left:x,top:y,right:x+half,bottom:y+h},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}if serum!=crate::ui_modern::BPSR_MUTED{SetTextColor(hdc,serum);draw(hdc,"S",RECT{left:x+half,top:y,right:x+w,bottom:y+h},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}SelectObject(hdc,old);
}"#,
        r#"fn raid_consumable_color(status:Option<&ConsumableStatus>,now:i64)->u32{
    let Some(status)=active_consumable(status,now)else{return crate::ui_modern::BPSR_MUTED;};
    if status.expires_unix_ms<=0{return rgb(72,226,116);}let left=status.expires_unix_ms.saturating_sub(now);
    if left<=0{return crate::ui_modern::BPSR_MUTED;}
    if left<=30_000{return if consumable_blink_on(now){crate::ui_modern::BPSR_DANGER}else{rgb(108,34,34)};}
    if left<=60_000{return crate::ui_modern::BPSR_DANGER;}
    if left<=180_000{return rgb(245,190,55);}
    rgb(72,226,116)
}

unsafe fn paint_raid_fs(hdc:HDC,state:&State,row:Option<&&DpsRow>,x:i32,y:i32,w:i32,h:i32){
    let old=SelectObject(hdc,dps_primary_font(state));let Some(row)=row else{SelectObject(hdc,old);return;};let now=now_ms();let food=raid_consumable_color(row.food.as_ref(),now);let serum=raid_consumable_color(row.serum.as_ref(),now);
    let half=(w/2).max(1);SetTextColor(hdc,food);draw(hdc,"F",RECT{left:x,top:y,right:x+half,bottom:y+h},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);SetTextColor(hdc,serum);draw(hdc,"S",RECT{left:x+half,top:y,right:x+w,bottom:y+h},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);SelectObject(hdc,old);
}"#,
        "consumable colors and gray inactive indicators",
    );

    replace_once(
        &mut source,
        r#"let header_layout_rect=if raid_active(state){hr}else{normal_meter_content_rect(hr,scale)};let layout=dps_row_layout_responsive"#,
        r#"let header_layout_rect=if raid_active(state)||!settings.meter.show_consumables{hr}else{normal_meter_content_rect(hr,scale)};let layout=dps_row_layout_responsive"#,
        "normal header F/S reservation toggle",
    );

    replace_once(
        &mut source,
        r#"let row_layout_rect=if raid_active(state){r}else{normal_meter_content_rect(r,scale)};let layout=dps_row_layout_responsive(hdc,row_layout_rect,scale,settings.meter.show_imagines,settings.meter.show_active_rates,show_share,settings.meter.show_deaths,row,primary_font,secondary_font);if !raid_active(state){paint_normal_meter_fs(hdc,state,row,r);}"#,
        r#"let row_layout_rect=if raid_active(state)||!settings.meter.show_consumables{r}else{normal_meter_content_rect(r,scale)};let layout=dps_row_layout_responsive(hdc,row_layout_rect,scale,settings.meter.show_imagines,settings.meter.show_active_rates,show_share,settings.meter.show_deaths,row,primary_font,secondary_font);if !raid_active(state)&&settings.meter.show_consumables{paint_normal_meter_fs(hdc,state,row,r);}"#,
        "normal row F/S reservation toggle",
    );

    replace_once(
        &mut source,
        r#"    let settings=state.features.read().map(|v|v.clone()).unwrap_or_default();let snapshot=view_snapshot(state);let scale=dps_layout_scale(state);let row_h=dps_row_h(scale);let gap=dps_adaptive_logical_px(RAID_CENTER_GAP,scale).max(56);let mid=rc.right/2;let left=RECT{left:6,top,right:(mid-gap/2-4).max(160),bottom:rc.bottom};let right=RECT{left:(mid+gap/2+4).min(rc.right-160),top,right:rc.right-8,bottom:rc.bottom};let leader=rows.iter().take(RAID_MAX_ROWS).map(|r|mode_metric(r,state.sort_mode)).max().unwrap_or(1).max(1);
    let gutter_bottom=(top+RAID_ROWS_PER_COLUMN as i32*row_h).min(rc.bottom);crate::ui_modern::fill_round_rect(hdc,RECT{left:mid-gap/2,top:top+2,right:mid+gap/2,bottom:gutter_bottom},crate::ui_modern::DARK_SURFACE,crate::ui_modern::RADIUS_SMALL);fill(hdc,&RECT{left:mid,top:top+4,right:mid+1,bottom:gutter_bottom-2},crate::ui_modern::DARK_BORDER);"#,
        r#"    let settings=state.features.read().map(|v|v.clone()).unwrap_or_default();let snapshot=view_snapshot(state);let scale=dps_layout_scale(state);let row_h=dps_row_h(scale);let gap=if settings.meter.show_consumables{dps_adaptive_logical_px(RAID_CENTER_GAP,scale).max(56)}else{0};let mid=rc.right/2;let left=RECT{left:6,top,right:(mid-gap/2-4).max(160),bottom:rc.bottom};let right=RECT{left:(mid+gap/2+4).min(rc.right-160),top,right:rc.right-8,bottom:rc.bottom};let leader=rows.iter().take(RAID_MAX_ROWS).map(|r|mode_metric(r,state.sort_mode)).max().unwrap_or(1).max(1);
    let gutter_bottom=(top+RAID_ROWS_PER_COLUMN as i32*row_h).min(rc.bottom);if settings.meter.show_consumables{crate::ui_modern::fill_round_rect(hdc,RECT{left:mid-gap/2,top:top+2,right:mid+gap/2,bottom:gutter_bottom},crate::ui_modern::DARK_SURFACE,crate::ui_modern::RADIUS_SMALL);}fill(hdc,&RECT{left:mid,top:top+4,right:mid+1,bottom:gutter_bottom-2},crate::ui_modern::DARK_BORDER);"#,
        "raid F/S gutter toggle",
    );

    replace_once(
        &mut source,
        r#"        let pair_w=((gap/2)-8).max(24);paint_raid_fs(hdc,state,rows.get(li),mid-gap/2+2,y,pair_w,bottom-y);paint_raid_fs(hdc,state,rows.get(ri),mid+6,y,pair_w,bottom-y);"#,
        r#"        if settings.meter.show_consumables{let pair_w=((gap/2)-8).max(24);paint_raid_fs(hdc,state,rows.get(li),mid-gap/2+2,y,pair_w,bottom-y);paint_raid_fs(hdc,state,rows.get(ri),mid+6,y,pair_w,bottom-y);}"#,
        "raid F/S paint toggle",
    );

    replace_once(
        &mut source,
        r#"    let rc=logical_client_rect(hwnd,state.scale_percent);let top=dps_rows_top_for(state);if y<top||y>=rc.bottom{return None;}let scale=dps_layout_scale(state);let row_h=dps_row_h_for(state,scale).max(1);let slot=((y-top)/row_h)as usize;if slot>=RAID_ROWS_PER_COLUMN{return None;}let gap=if state.compact_mode{dps_adaptive_logical_px(18,scale).max(14)}else{dps_adaptive_logical_px(RAID_CENTER_GAP,scale).max(56)};let mid=rc.right/2;let index=if x<mid-gap/2{slot}else if x>mid+gap/2{slot+RAID_ROWS_PER_COLUMN}else{return None;};let rows=meter_rows(state);rows.get(index).map(|row|(*row).clone())"#,
        r#"    let rc=logical_client_rect(hwnd,state.scale_percent);let top=dps_rows_top_for(state);if y<top||y>=rc.bottom{return None;}let settings=state.features.read().map(|v|v.clone()).unwrap_or_default();let scale=dps_layout_scale(state);let row_h=dps_row_h_for(state,scale).max(1);let slot=((y-top)/row_h)as usize;if slot>=RAID_ROWS_PER_COLUMN{return None;}let gap=if state.compact_mode{dps_adaptive_logical_px(18,scale).max(14)}else if settings.meter.show_consumables{dps_adaptive_logical_px(RAID_CENTER_GAP,scale).max(56)}else{0};let mid=rc.right/2;let index=if x<mid-gap/2{slot}else if x>mid+gap/2{slot+RAID_ROWS_PER_COLUMN}else{return None;};let rows=meter_rows(state);rows.get(index).map(|row|(*row).clone())"#,
        "raid row hit testing without hidden gutter",
    );

    replace_once(
        &mut source,
        r#"unsafe fn overlay_help(hwnd:HWND,state:&State,x:i32,y:i32)->Option<String>{"#,
        r#"fn consumable_hover_text(kind:&str,status:Option<&ConsumableStatus>,now:i64)->String{match active_consumable(status,now){Some(status)=>{let name=if status.name.trim().is_empty(){"Active"}else{status.name.trim()};format!("{kind}: {name} • {}",consumable_remaining_text(status,now))},None=>format!("{kind}: not detected")}}
unsafe fn consumable_hover_at(hwnd:HWND,state:&State,x:i32,y:i32)->Option<String>{
    if state.kind!=Kind::Dps||state.collapsed||state.compact_mode{return None;}let settings=state.features.read().map(|v|v.clone()).unwrap_or_default();if !settings.meter.show_consumables{return None;}let rc=logical_client_rect(hwnd,state.scale_percent);let scale=dps_layout_scale(state);let now=now_ms();
    if raid_active(state){let top=dps_rows_top_for(state);let row_h=dps_row_h_for(state,scale).max(1);if y<top||y>=rc.bottom{return None;}let slot=((y-top)/row_h)as usize;if slot>=RAID_ROWS_PER_COLUMN{return None;}let rows=meter_rows(state);let gap=dps_adaptive_logical_px(RAID_CENTER_GAP,scale).max(56);let mid=rc.right/2;let pair_w=((gap/2)-8).max(24);let(left_x,index)=if x>=mid-gap/2+2&&x<mid-gap/2+2+pair_w{(mid-gap/2+2,slot)}else if x>=mid+6&&x<mid+6+pair_w{(mid+6,slot+RAID_ROWS_PER_COLUMN)}else{return None;};let row=*rows.get(index)?;let half=(pair_w/2).max(1);return Some(if x<left_x+half{consumable_hover_text("Food",row.food.as_ref(),now)}else{consumable_hover_text("Serum",row.serum.as_ref(),now)});}
    let top=dps_rows_top();let row_h=dps_row_h(scale);if y<top||y>=rc.bottom{return None;}let screen_i=((y-top)/row_h)as usize;let physical=visible_dps_rows_scaled(rc.bottom,scale);if screen_i>=physical{return None;}let shown=meter_page_rows(state,physical);let row=*shown.get(screen_i).map(|(_,row)|row)?;let r=RECT{left:6,top:top+screen_i as i32*row_h,right:rc.right-8,bottom:top+screen_i as i32*row_h+row_h-2};let fs=normal_meter_fs_rect(r,scale);if x<fs.left||x>=fs.right||y<fs.top||y>=fs.bottom{return None;}let half=((fs.right-fs.left)/2).max(1);Some(if x<fs.left+half{consumable_hover_text("Food",row.food.as_ref(),now)}else{consumable_hover_text("Serum",row.serum.as_ref(),now)})
}
unsafe fn overlay_help(hwnd:HWND,state:&State,x:i32,y:i32)->Option<String>{"#,
        "integrated F/S hover details",
    );

    replace_once(
        &mut source,
        r#"    if state.kind==Kind::Dps{if raid_active(state)&&!state.compact_mode{let scale=dps_layout_scale(state);let gap=dps_adaptive_logical_px(RAID_CENTER_GAP,scale).max(56);let mid=rc.right/2;if y>=dps_rows_top_for(state)&&x>=mid-gap/2&&x<=mid+gap/2{return Some("Raid consumables • F = Food • S = Serum".into());}}if !state.compact_mode&&hover_badge_at(hwnd,state,x,y).is_some(){return None;}"#,
        r#"    if state.kind==Kind::Dps{if let Some(text)=consumable_hover_at(hwnd,state,x,y){return Some(text);}if !state.compact_mode&&hover_badge_at(hwnd,state,x,y).is_some(){return None;}"#,
        "F/S hover priority",
    );

    replace_once(
        &mut source,
        r#"    #[test]fn raid_food_serum_are_green_when_healthy(){let now=100_000;assert_eq!(raid_consumable_color(Some(&status(180_000,now)),now),rgb(72,226,116));}"#,
        r#"    #[test]fn raid_food_serum_are_green_when_healthy(){let now=100_000;assert_eq!(raid_consumable_color(Some(&status(180_001,now)),now),rgb(72,226,116));}"#,
        "legacy green threshold test",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1333_consumable_control_tests {
    use super::*;

    fn status(left:i64,now:i64)->ConsumableStatus{ConsumableStatus{buff_id:42,name:"Test Consumable".into(),expires_unix_ms:now+left,duration_ms:300_000}}

    #[test]
    fn fresh_user_meter_defaults_match_minimal_requested_view() {
        let meter=feature_settings::MeterSettings::default();
        assert!(!meter.show_damage_share);
        assert!(!meter.show_healing_share);
        assert!(!meter.show_tank_share);
        assert!(!meter.show_deaths);
        assert!(meter.show_imagines);
        assert!(meter.show_consumables);
        assert!(meter.show_target);
        assert!(meter.show_active_rates);
        assert!(meter.always_show_self);
    }

    #[test]
    fn old_meter_json_without_consumable_key_preserves_visible_fs() {
        let meter:feature_settings::MeterSettings=serde_json::from_str(r#"{"showImagines":false}"#).unwrap();
        assert!(!meter.show_imagines);
        assert!(meter.show_consumables);
    }

    #[test]
    fn consumable_indicator_thresholds_are_green_amber_red_and_blinking_red() {
        let now=400_000;
        assert_eq!(raid_consumable_color(None,now),crate::ui_modern::BPSR_MUTED);
        assert_eq!(raid_consumable_color(Some(&status(180_001,now)),now),rgb(72,226,116));
        assert_eq!(raid_consumable_color(Some(&status(180_000,now)),now),rgb(245,190,55));
        assert_eq!(raid_consumable_color(Some(&status(60_000,now)),now),crate::ui_modern::BPSR_DANGER);
        assert_eq!(raid_consumable_color(Some(&status(30_000,now)),now),crate::ui_modern::BPSR_DANGER);
        assert_eq!(raid_consumable_color(Some(&status(30_000,now+400)),now+400),rgb(108,34,34));
    }

    #[test]
    fn hidden_normal_consumables_release_the_reserved_right_slot() {
        let row=RECT{left:6,top:117,right:620,bottom:148};
        let reserved=normal_meter_content_rect(row,100);
        assert!(reserved.right<row.right);
        let meter=feature_settings::MeterSettings{show_consumables:false,..Default::default()};
        let layout_rect=if meter.show_consumables{reserved}else{row};
        assert_eq!(layout_rect.right,row.right);
    }

    #[test]
    fn hover_text_reports_name_timer_and_missing_state() {
        let now=1_000;
        let active=ConsumableStatus{buff_id:7,name:"Feast".into(),expires_unix_ms:61_000,duration_ms:60_000};
        assert_eq!(consumable_hover_text("Food",Some(&active),now),"Food: Feast • 1:00 left");
        assert_eq!(consumable_hover_text("Serum",None,now),"Serum: not detected");
    }
}
"#);

    fs::write(path, source).expect("write v1.33.3 consumable controls overlay");
    println!("cargo:rerun-if-changed=build/legacy/build_v1333_consumable_controls.rs");
}
