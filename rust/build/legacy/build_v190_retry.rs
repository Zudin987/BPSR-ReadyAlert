use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v185.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.9.0 retry patch `{label}` expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn replace_between(source: &mut String, start: &str, end: &str, replacement: &str, label: &str) {
    let count = source.matches(start).count();
    assert_eq!(count, 1, "v1.9.0 retry patch `{label}` start expected one match, found {count}");
    let begin = source.find(start).expect("start checked");
    let rel_end = source[begin..].find(end)
        .unwrap_or_else(|| panic!("v1.9.0 retry patch `{label}` end anchor missing"));
    source.replace_range(begin..begin + rel_end, replacement);
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let overlay_path = out.join("feature_overlays_v170_fixed.rs");
    let mut overlay = fs::read_to_string(&overlay_path).expect("read generated v1.8.5 overlay");

    replace_once(
        &mut overlay,
        "paths::AppPaths,\n};",
        "paths::AppPaths,\nhistory::{self, HistoryEncounter},\n};",
        "history imports",
    );
    replace_once(
        &mut overlay,
        "capture_status: String,\nhover_text: Option<String>,",
        "capture_status: String,\nhistory: Vec<HistoryEncounter>,\nhistory_index: Option<usize>,\nhover_text: Option<String>,",
        "history state fields",
    );
    replace_once(
        &mut overlay,
        "let state=Box::new(State{kind",
        "let history_records=if kind==Kind::Dps{history::load_recent(&paths.root,snapshot.meter.history_limit)}else{Vec::new()};let state=Box::new(State{kind",
        "load history at overlay creation",
    );
    replace_once(
        &mut overlay,
        "capture_status:\"Starting capture...\".into(),hover_text:None,hover_x:0,hover_y:0}",
        "capture_status:\"Starting capture...\".into(),history:history_records,history_index:None,hover_text:None,hover_x:0,hover_y:0}",
        "history state initialization",
    );

    replace_between(
        &mut overlay,
        "pub unsafe fn update_dps(hwnd:HWND,snapshot:DpsSnapshot){",
        "pub unsafe fn update_mechanics",
        r#"pub unsafe fn update_dps(hwnd:HWND,snapshot:DpsSnapshot){with_state(hwnd,|state|{let rolled=history::encounter_rolled(&state.dps,&snapshot);if rolled{archive_live_snapshot(state);if !state.features.read().map(|f|f.meter.remember_scroll).unwrap_or(false){state.scroll=0;}}if state.history_index.is_none()&&!state.detail_hwnd.is_null()&&IsWindow(state.detail_hwnd)!=0&&state.detail_uid!=0{if let Some(row)=snapshot.rows.iter().find(|row|row.uid==state.detail_uid).cloned(){update_detail(state.detail_hwnd,row,snapshot.encounter_ms);}}state.dps=snapshot;let total=meter_rows(state).len();state.scroll=state.scroll.min(total.saturating_sub(1));});}
fn view_snapshot(state:&State)->&DpsSnapshot{if let Some(index)=state.history_index{if let Some(record)=state.history.get(index){return &record.snapshot;}}&state.dps}
fn archive_live_snapshot(state:&mut State){let limit=state.features.read().map(|f|f.meter.history_limit).unwrap_or(50).clamp(10,200);match history::archive(&state.paths.root,&state.dps,limit){Ok(Some(record))=>{if let Some(index)=state.history_index{state.history_index=Some(index.saturating_add(1));}state.history.insert(0,record);state.history.truncate(limit);if let Some(index)=state.history_index{if index>=state.history.len(){state.history_index=state.history.len().checked_sub(1);}}},Ok(None)=>{},Err(err)=>crate::logging::write(format!("history: archive failed: {err}")),}}
fn history_older(state:&mut State){if state.history.is_empty(){return;}state.history_index=Some(match state.history_index{None=>0,Some(index)=>(index+1).min(state.history.len()-1)});state.scroll=0;}
fn history_newer(state:&mut State){state.history_index=match state.history_index{None=>None,Some(0)=>None,Some(index)=>Some(index-1)};state.scroll=0;}
"#,
        "history update and navigation state",
    );

    replace_between(
        &mut overlay,
        "unsafe fn on_click(hwnd:HWND,state:&mut State,lparam:LPARAM){",
        "unsafe fn on_wheel",
        r#"unsafe fn on_click(hwnd:HWND,state:&mut State,lparam:LPARAM){if state.collapsed{expand_state(hwnd,state);return;}let x=lo_signed(lparam);let y=hi_signed(lparam);let mut rc:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rc);if y<TOOLBAR_H{if x>=rc.right-BUTTON_W{PostMessageW(state.main_hwnd,0x0111,if state.kind==Kind::Dps{CMD_HIDE_DPS as usize}else{CMD_HIDE_MECHANICS as usize},0);}else if x>=rc.right-BUTTON_W*2{collapse(hwnd,state);}else if x>=rc.right-BUTTON_W*3{open_feature_settings(hwnd,state);}else{drag_window(hwnd);}return;}if state.kind==Kind::Dps&&y<dps_rows_top(){let control_y=TOOLBAR_H+4;if y>=control_y&&y<control_y+23{match x{8..=78=>state.sort_mode=SortMode::Damage,83..=148=>state.sort_mode=SortMode::Heal,153..=218=>state.sort_mode=SortMode::Tank,225..=298=>{if state.history_index.is_none(){archive_live_snapshot(state);crate::telemetry::request_manual_reset();state.dps=DpsSnapshot::default();state.scroll=0;}},304..=336=>history_older(state),341..=395=>{state.history_index=None;state.scroll=0;},400..=432=>history_newer(state),_=>{}}clamp_scroll(hwnd,state);InvalidateRect(hwnd,null(),0);}return;}if state.kind==Kind::Dps{if let Some(row)=dps_name_row_at(hwnd,state,x,y){open_detail(hwnd,state,row);}}}
"#,
        "history control clicks",
    );

    const OLD_TOOLBAR: &str = "fn toolbar_title(state:&State)->String{if state.kind==Kind::Mechanics{return format!(\"Dungeon Mechanics  |  {}\",state.capture_status);}let show_target=state.features.read().map(|f|f.meter.show_target).unwrap_or(true);if !show_target{return format!(\"DPS Meter  {}\",format_time(state.dps.encounter_ms));}let Some(target)=state.dps.target.as_ref()else{return format!(\"DPS Meter  {}\",format_time(state.dps.encounter_ms));};let hp=if target.max_hp>0{format!(\"{:.1}%\",(target.hp.max(0)as f64*100.0/target.max_hp as f64).clamp(0.0,100.0))}else{\"?%\".into()};format!(\"DPS Meter  {}  |  {} {}\",format_time(state.dps.encounter_ms),target.name,hp)}";
    const NEW_TOOLBAR: &str = "fn toolbar_title(state:&State)->String{if state.kind==Kind::Mechanics{return format!(\"Dungeon Mechanics  |  {}\",state.capture_status);}let snapshot=view_snapshot(state);let label=match state.history_index{Some(index)=>format!(\"History {}/{}\",index+1,state.history.len()),None=>\"DPS Meter\".into()};let show_target=state.features.read().map(|f|f.meter.show_target).unwrap_or(true);if !show_target{return format!(\"{}  {}\",label,format_time(snapshot.encounter_ms));}let Some(target)=snapshot.target.as_ref()else{return format!(\"{}  {}\",label,format_time(snapshot.encounter_ms));};let hp=if target.max_hp>0{format!(\"{:.1}%\",(target.hp.max(0)as f64*100.0/target.max_hp as f64).clamp(0.0,100.0))}else{\"?%\".into()};format!(\"{}  {}  |  {} {}\",label,format_time(snapshot.encounter_ms),target.name,hp)}";
    replace_once(&mut overlay, OLD_TOOLBAR, NEW_TOOLBAR, "history toolbar title only");

    replace_between(
        &mut overlay,
        "unsafe fn hover_badge_at(hwnd:HWND,state:&State,x:i32,y:i32)->Option<String>{",
        "unsafe fn paint_hover",
        r#"unsafe fn hover_badge_at(hwnd:HWND,state:&State,x:i32,y:i32)->Option<String>{if state.kind!=Kind::Dps||state.collapsed||!state.features.read().map(|f|f.meter.show_imagines).unwrap_or(true){return None;}let mut rc:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rc);let top=dps_rows_top();if y<top||y>=rc.bottom{return None;}let screen_i=((y-top)/DPS_ROW_H)as usize;if screen_i>=visible_dps_rows(rc.bottom){return None;}let rows=meter_rows(state);let row=*rows.get(state.scroll+screen_i)?;if row.is_dead{return None;}let r=RECT{left:6,top:top+screen_i as i32*DPS_ROW_H,right:rc.right-8,bottom:top+screen_i as i32*DPS_ROW_H+DPS_ROW_H-2};let layout=dps_row_layout(r,true,row);let mut bx=layout.badge_left;for badge in row.imagines.iter().take(2){if x>=bx&&x<bx+BADGE_W&&y>=r.top+3&&y<r.top+28{let tier=if badge.tier>0{badge.tier.saturating_sub(1).to_string()}else{"?".into()};return Some(format!("{} · Tier {}",badge.name,tier));}bx+=BADGE_W+BADGE_GAP;}None}
"#,
        "history-aware Imagine hover",
    );

    replace_between(
        &mut overlay,
        "#[derive(Clone,Copy)]struct DpsRowLayout",
        "unsafe fn draw_percent(",
        include_str!("overlay_v190_meter_patch.txt"),
        "history meter body",
    );

    replace_between(
        &mut overlay,
        "unsafe fn clamp_scroll(hwnd:HWND,state:&mut State){",
        "unsafe fn open_detail",
        r#"unsafe fn clamp_scroll(hwnd:HWND,state:&mut State){let mut rc:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rc);let(total,visible)=if state.kind==Kind::Dps{(meter_rows(state).len(),visible_dps_rows(rc.bottom))}else{let tracked=state.features.read().map(|f|f.mechanic_attributes.tracked.len()).unwrap_or(0);let top=TOOLBAR_H+5+if tracked>0{MECH_ATTR_H}else{0}+MECH_CONSUMABLE_H;let visible=((rc.bottom-top)/MECH_ROW_H).max(0)as usize;let now=now_ms();let total=state.mechanics.rows.iter().filter(|row|row.persistent||row.expires_unix_ms<=0||row.expires_unix_ms>now).count();(total,visible)};state.scroll=state.scroll.min(total.saturating_sub(visible.max(1)));}
"#,
        "filtered scroll bounds",
    );

    replace_once(
        &mut overlay,
        "encounter_ms:state.dps.encounter_ms",
        "encounter_ms:view_snapshot(state).encounter_ms",
        "historical inspector duration",
    );
    replace_once(
        &mut overlay,
        "let(width,height)=if state.kind==Kind::Dps{(430,390)}else{(720,580)};",
        "let(width,height)=if state.kind==Kind::Dps{(500,610)}else{(720,580)};",
        "larger DPS settings window",
    );
    replace_between(
        &mut overlay,
        "unsafe fn paint_feature_settings(hwnd:HWND,state:&SettingsState){",
        "fn mutate_features",
        include_str!("overlay_v190_settings_patch.txt"),
        "v1.9 settings body",
    );

    fs::write(&overlay_path, overlay).expect("write v1.9 generated overlay");
    println!("cargo:rerun-if-changed=build_v190_retry.rs");
    println!("cargo:rerun-if-changed=overlay_v190_meter_patch.txt");
    println!("cargo:rerun-if-changed=overlay_v190_settings_patch.txt");
    println!("cargo:rerun-if-changed=src/history.rs");
    println!("cargo:rerun-if-changed=src/telemetry_v190.rs");
}
