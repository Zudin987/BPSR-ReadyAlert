use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v185.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.9.0 patch `{label}` expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn replace_between(source: &mut String, start: &str, end: &str, replacement: &str, label: &str) {
    let count = source.matches(start).count();
    assert_eq!(count, 1, "v1.9.0 patch `{label}` start expected one match, found {count}");
    let begin = source.find(start).expect("start checked");
    let rel_end = source[begin..].find(end)
        .unwrap_or_else(|| panic!("v1.9.0 patch `{label}` end anchor missing"));
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
        "history overlay state",
    );
    replace_once(
        &mut overlay,
        "let state=Box::new(State{kind",
        "let history_records=if kind==Kind::Dps{history::load_recent(&paths.root,snapshot.meter.history_limit)}else{Vec::new()};let state=Box::new(State{kind",
        "load recent history",
    );
    replace_once(
        &mut overlay,
        "capture_status:\"Starting capture...\".into(),hover_text:None,hover_x:0,hover_y:0}",
        "capture_status:\"Starting capture...\".into(),history:history_records,history_index:None,hover_text:None,hover_x:0,hover_y:0}",
        "initialize history state",
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
        "history rollover and navigation state",
    );

    replace_between(
        &mut overlay,
        "unsafe fn on_click(hwnd:HWND,state:&mut State,lparam:LPARAM){",
        "unsafe fn on_wheel",
        r#"unsafe fn on_click(hwnd:HWND,state:&mut State,lparam:LPARAM){if state.collapsed{expand_state(hwnd,state);return;}let x=lo_signed(lparam);let y=hi_signed(lparam);let mut rc:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rc);if y<TOOLBAR_H{if x>=rc.right-BUTTON_W{PostMessageW(state.main_hwnd,0x0111,if state.kind==Kind::Dps{CMD_HIDE_DPS as usize}else{CMD_HIDE_MECHANICS as usize},0);}else if x>=rc.right-BUTTON_W*2{collapse(hwnd,state);}else if x>=rc.right-BUTTON_W*3{open_feature_settings(hwnd,state);}else{drag_window(hwnd);}return;}if state.kind==Kind::Dps&&y<dps_rows_top(){let control_y=TOOLBAR_H+4;if y>=control_y&&y<control_y+23{match x{8..=78=>state.sort_mode=SortMode::Damage,83..=148=>state.sort_mode=SortMode::Heal,153..=218=>state.sort_mode=SortMode::Tank,225..=298=>{if state.history_index.is_none(){archive_live_snapshot(state);crate::telemetry::request_manual_reset();state.dps=DpsSnapshot::default();state.scroll=0;}},304..=336=>history_older(state),341..=395=>{state.history_index=None;state.scroll=0;},400..=432=>history_newer(state),_=>{}}clamp_scroll(hwnd,state);InvalidateRect(hwnd,null(),0);}return;}if state.kind==Kind::Dps{if let Some(row)=dps_name_row_at(hwnd,state,x,y){open_detail(hwnd,state,row);}}}
"#,
        "DPS history controls",
    );

    replace_between(
        &mut overlay,
        "fn toolbar_title(state:&State)->String{",
        "unsafe fn paint_dps",
        r#"fn toolbar_title(state:&State)->String{if state.kind==Kind::Mechanics{return format!("Dungeon Mechanics  |  {}",state.capture_status);}let snapshot=view_snapshot(state);let label=match state.history_index{Some(index)=>format!("History {}/{}",index+1,state.history.len()),None=>"DPS Meter".into()};let show_target=state.features.read().map(|f|f.meter.show_target).unwrap_or(true);if !show_target{return format!("{}  {}",label,format_time(snapshot.encounter_ms));}let Some(target)=snapshot.target.as_ref()else{return format!("{}  {}",label,format_time(snapshot.encounter_ms));};let hp=if target.max_hp>0{format!("{:.1}%",(target.hp.max(0)as f64*100.0/target.max_hp as f64).clamp(0.0,100.0))}else{"?%".into()};format!("{}  {}  |  {} {}",label,format_time(snapshot.encounter_ms),target.name,hp)}
"#,
        "history-aware toolbar title",
    );

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
        r#"#[derive(Clone,Copy)]struct DpsRowLayout{name_left:i32,name_right:i32,badge_left:i32,score_left:i32,left_end:i32,middle_left:i32,middle_right:i32,share_left:i32,share_right:i32,middle_col:i32}
fn approx_text_px(text:&str)->i32{(text.chars().count()as i32*6+2).max(10)}
fn dps_row_layout(r:RECT,show_imagines:bool,row:&DpsRow)->DpsRowLayout{let total=(r.right-r.left).max(1);let share_w=if total<650{56}else{62};let middle_w=if total<650{156}else{176};let gap=5;let share_right=r.right-2;let share_left=share_right-share_w;let middle_right=share_left-gap;let middle_left=middle_right-middle_w;let left_end=middle_left-gap;let name_left=r.left+21;let identity=dps_identity(row);let badge_count=if show_imagines&&!row.is_dead{row.imagines.len().min(2)as i32}else{0};let badge_span=badge_count*BADGE_W+badge_count.saturating_sub(1)*BADGE_GAP;let score=if row.is_dead{revive_status_text(row,now_ms()).0}else{dps_score_pair(row)};let score_w=if score.is_empty(){0}else{approx_text_px(&score).clamp(44,if row.is_dead{132}else{88})};let icon_gap=if badge_span>0{4}else{0};let score_gap=if score_w>0{4}else{0};let fixed=badge_span+icon_gap+score_w+score_gap;let name_max=(left_end-name_left-fixed).max(36);let name_w=approx_text_px(&identity).min(name_max);let name_right=(name_left+name_w).min(left_end);let badge_left=(name_right+icon_gap).min(left_end);let score_left=(badge_left+badge_span+score_gap).min(left_end);DpsRowLayout{name_left,name_right,badge_left,score_left,left_end,middle_left,middle_right,share_left,share_right,middle_col:(middle_w*43/100).max(1)}}
fn dps_identity(row:&DpsRow)->String{let spec=if !row.subprofession_name.trim().is_empty(){row.subprofession_name.as_str()}else{profession_name(row.profession_id)};if spec.trim().is_empty(){row.name.clone()}else{format!("{}-{}",row.name,spec)}}
fn dps_score_pair(row:&DpsRow)->String{match(row.ability_score>0,row.illusion_break>0){(true,true)=>format!("({}+{})",score(row.ability_score),score(row.illusion_break)),(true,false)=>format!("({})",score(row.ability_score)),(false,true)=>format!("(+{})",score(row.illusion_break)),_=>String::new()}}
fn revive_status_text(row:&DpsRow,now:i64)->(String,u32){if row.revive_blocked_until_ms<0{return("REVIVE IN : ?".into(),rgb(255,190,70));}if row.revive_blocked_until_ms>now{let left=row.revive_blocked_until_ms.saturating_sub(now)as u64;return(format!("REVIVE IN : {}",format_countdown(left)),rgb(255,190,70));}("CAN REVIVE".into(),rgb(72,226,116))}
fn rate(total:i64,encounter_ms:u64)->f64{if total<=0{0.0}else{let seconds=(encounter_ms.max(1)as f64/1000.0).max(0.001);total as f64/seconds}}
fn encounter_rate(row:&DpsRow,mode:SortMode,encounter_ms:u64)->f64{match mode{SortMode::Damage=>row.dps,SortMode::Heal=>rate(row.healing,encounter_ms),SortMode::Tank=>rate(row.damage_taken,encounter_ms)}}
fn active_rate(row:&DpsRow,mode:SortMode)->f64{match mode{SortMode::Damage=>row.active_dps,SortMode::Heal=>row.active_hps,SortMode::Tank=>row.active_dtps}}
fn mode_values(row:&DpsRow,mode:SortMode,encounter_ms:u64,settings:&FeatureSettings)->(String,String){let total=match mode{SortMode::Damage=>row.damage,SortMode::Heal=>row.healing,SortMode::Tank=>row.damage_taken};let encounter=encounter_rate(row,mode,encounter_ms);let rates=if settings.meter.show_active_rates{format!("A{}/E{}",compact(active_rate(row,mode)),compact(encounter))}else{format!("{}/s",compact(encounter))};(compact(total as f64),rates)}
fn mode_metric(row:&DpsRow,mode:SortMode)->i64{match mode{SortMode::Damage=>row.damage,SortMode::Heal=>row.healing,SortMode::Tank=>row.damage_taken}.max(0)}
fn active_share(row:&DpsRow,mode:SortMode,settings:&FeatureSettings)->Option<(f64,u32)>{match mode{SortMode::Damage if settings.meter.show_damage_share=>Some((row.damage_share,rgb(255,70,70))),SortMode::Heal if settings.meter.show_healing_share=>Some((row.healing_share,rgb(40,215,100))),SortMode::Tank if settings.meter.show_tank_share=>Some((row.tank_share,rgb(35,145,255))),_=>None}}
fn meter_rows(state:&State)->Vec<&DpsRow>{let snapshot=view_snapshot(state);let settings=state.features.read().map(|f|f.clone()).unwrap_or_default();let mut rows=sorted_rows(snapshot,state.sort_mode);rows.retain(|row|{if settings.meter.always_show_self&&row.is_local{return true;}(!settings.meter.party_only||row.is_party)&&(!settings.meter.only_contributors||mode_metric(row,state.sort_mode)>0)});let limit=settings.meter.visible_rows;if limit>0&&rows.len()>limit{let self_out=if settings.meter.always_show_self{rows.iter().position(|row|row.is_local).and_then(|index|(index>=limit).then_some(rows[index]))}else{None};rows.truncate(limit);if let Some(local)=self_out{if !rows.is_empty(){rows.pop();}rows.push(local);}}rows}
unsafe fn dps_name_row_at(hwnd:HWND,state:&State,x:i32,y:i32)->Option<DpsRow>{let mut rc:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rc);let top=dps_rows_top();if y<top||y>=rc.bottom{return None;}let screen_i=((y-top)/DPS_ROW_H)as usize;if screen_i>=visible_dps_rows(rc.bottom){return None;}let rows=meter_rows(state);let row=*rows.get(state.scroll+screen_i)?;let r=RECT{left:6,top:top+screen_i as i32*DPS_ROW_H,right:rc.right-8,bottom:top+screen_i as i32*DPS_ROW_H+DPS_ROW_H-2};let show=state.features.read().map(|f|f.meter.show_imagines).unwrap_or(true);let layout=dps_row_layout(r,show,row);if x>=layout.name_left&&x<layout.name_right{Some(row.clone())}else{None}}
unsafe fn paint_dps(hdc:HDC,rc:RECT,state:&State){let settings=state.features.read().map(|x|x.clone()).unwrap_or_default();let snapshot=view_snapshot(state);let control_top=TOOLBAR_H+4;paint_tab(hdc,8,control_top,70,"Damage",state.sort_mode==SortMode::Damage);paint_tab(hdc,83,control_top,65,"Heal",state.sort_mode==SortMode::Heal);paint_tab(hdc,153,control_top,65,"Tank",state.sort_mode==SortMode::Tank);paint_tab(hdc,225,control_top,73,"Reset",false);paint_tab(hdc,304,control_top,32,"<",false);paint_tab(hdc,341,control_top,54,"LIVE",state.history_index.is_none());paint_tab(hdc,400,control_top,32,">",false);SetTextColor(hdc,rgb(145,160,180));draw(hdc,&format!("D {}  H {}  T {}",compact(snapshot.total_damage as f64),compact(snapshot.total_healing as f64),compact(snapshot.total_damage_taken as f64)),RECT{left:438,top:control_top,right:rc.right-8,bottom:control_top+23},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);let rows=meter_rows(state);let top=dps_rows_top();let visible=visible_dps_rows(rc.bottom);if rows.is_empty(){SetTextColor(hdc,rgb(132,145,162));draw(hdc,if state.history_index.is_some(){"No rows match the current meter filters."}else{"Waiting for party / combat data..."},RECT{left:10,top:top+18,right:rc.right-10,bottom:top+58},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);return;}let leader=rows.first().map(|row|mode_metric(row,state.sort_mode)).unwrap_or(0).max(1);let bold_font=if state.font.is_null(){GetStockObject(DEFAULT_GUI_FONT)}else{state.font};for(screen_i,row)in rows.iter().skip(state.scroll).take(visible).enumerate(){let rank=state.scroll+screen_i+1;let y=top+screen_i as i32*DPS_ROW_H;let r=RECT{left:6,top:y,right:rc.right-8,bottom:y+DPS_ROW_H-2};let bg=if row.is_dead{rgb(105,28,34)}else{spec_color(row)};fill(hdc,&r,bg);if row.is_local{outline(hdc,r,rgb(212,175,55),2);}let base_text=text_on(bg);let layout=dps_row_layout(r,settings.meter.show_imagines,row);SelectObject(hdc,GetStockObject(DEFAULT_GUI_FONT));SetTextColor(hdc,base_text);draw(hdc,&rank.to_string(),RECT{left:r.left+3,top:r.top,right:r.left+19,bottom:r.bottom-2},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);SelectObject(hdc,bold_font);SetTextColor(hdc,if row.is_dead{rgb(255,120,120)}else{base_text});draw(hdc,&dps_identity(row),RECT{left:layout.name_left,top:r.top,right:layout.name_right,bottom:r.bottom-2},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);if settings.meter.show_imagines&&!row.is_dead{let mut bx=layout.badge_left;for badge in row.imagines.iter().take(2){paint_badge(hdc,bx,r.top+3,badge);bx+=BADGE_W+BADGE_GAP;}}if row.is_dead{let(status,color)=revive_status_text(row,now_ms());SetTextColor(hdc,color);draw(hdc,&status,RECT{left:layout.score_left,top:r.top,right:layout.left_end,bottom:r.bottom-2},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}else{let score_text=dps_score_pair(row);if !score_text.is_empty(){SetTextColor(hdc,base_text);draw(hdc,&score_text,RECT{left:layout.score_left,top:r.top,right:layout.left_end,bottom:r.bottom-2},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}}fill(hdc,&RECT{left:layout.left_end+1,top:r.top+5,right:layout.left_end+2,bottom:r.bottom-5},rgb(30,35,42));let(first,second)=mode_values(row,state.sort_mode,snapshot.encounter_ms,&settings);SetTextColor(hdc,base_text);draw(hdc,&first,RECT{left:layout.middle_left,top:r.top,right:layout.middle_left+layout.middle_col-3,bottom:r.bottom-2},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);draw(hdc,&second,RECT{left:layout.middle_left+layout.middle_col+2,top:r.top,right:layout.middle_right,bottom:r.bottom-2},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);fill(hdc,&RECT{left:layout.share_left-2,top:r.top+5,right:layout.share_left-1,bottom:r.bottom-5},rgb(30,35,42));if let Some((share,color))=active_share(row,state.sort_mode,&settings){SetTextColor(hdc,color);draw(hdc,&format!("{share:.1}%"),RECT{left:layout.share_left,top:r.top,right:layout.share_right,bottom:r.bottom-2},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}if settings.meter.show_deaths&&row.deaths>0{SetTextColor(hdc,rgb(255,70,70));draw(hdc,&format!("D{}",row.deaths),RECT{left:layout.share_left,top:r.top,right:layout.share_right,bottom:r.top+10},DT_RIGHT|DT_SINGLELINE|DT_NOPREFIX);}let metric=mode_metric(row,state.sort_mode);let bar=RECT{left:r.left,top:r.bottom-2,right:r.right,bottom:r.bottom};fill(hdc,&bar,rgb(20,24,29));if metric>0{let width=(r.right-r.left).max(0)as i64;let filled=(width.saturating_mul(metric.min(leader))/leader).clamp(0,width)as i32;let color=match state.sort_mode{SortMode::Damage=>rgb(235,74,74),SortMode::Heal=>rgb(55,205,105),SortMode::Tank=>rgb(65,145,235)};fill(hdc,&RECT{left:r.left,top:r.bottom-2,right:r.left+filled,bottom:r.bottom},color);}}paint_scrollbar(hdc,rc,rows.len(),visible,state.scroll,top);}
"#,
        "history filters and active encounter rates",
    );

    replace_between(
        &mut overlay,
        "unsafe fn clamp_scroll(hwnd:HWND,state:&mut State){",
        "unsafe fn open_detail",
        r#"unsafe fn clamp_scroll(hwnd:HWND,state:&mut State){let mut rc:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rc);let(total,visible)=if state.kind==Kind::Dps{(meter_rows(state).len(),visible_dps_rows(rc.bottom))}else{let tracked=state.features.read().map(|f|f.mechanic_attributes.tracked.len()).unwrap_or(0);let top=TOOLBAR_H+5+if tracked>0{MECH_ATTR_H}else{0}+MECH_CONSUMABLE_H;let visible=((rc.bottom-top)/MECH_ROW_H).max(0)as usize;let now=now_ms();let total=state.mechanics.rows.iter().filter(|row|row.persistent||row.expires_unix_ms<=0||row.expires_unix_ms>now).count();(total,visible)};state.scroll=state.scroll.min(total.saturating_sub(visible.max(1)));}
"#,
        "filtered meter scroll bounds",
    );

    replace_once(
        &mut overlay,
        "encounter_ms:state.dps.encounter_ms",
        "encounter_ms:view_snapshot(state).encounter_ms",
        "historical inspector encounter duration",
    );
    replace_once(
        &mut overlay,
        "let(width,height)=if state.kind==Kind::Dps{(430,390)}else{(720,580)};",
        "let(width,height)=if state.kind==Kind::Dps{(500,610)}else{(720,580)};",
        "expanded DPS settings window",
    );

    replace_between(
        &mut overlay,
        "unsafe fn paint_feature_settings(hwnd:HWND,state:&SettingsState){",
        "fn mutate_features",
        r#"fn visible_rows_label(value:usize)->String{if value==0{"Auto".into()}else{value.to_string()}}
unsafe fn paint_feature_settings(hwnd:HWND,state:&SettingsState){let mut ps:PAINTSTRUCT=std::mem::zeroed();let hdc=BeginPaint(hwnd,&mut ps);if hdc.is_null(){return;}let mut rc:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rc);fill(hdc,&rc,rgb(18,22,27));SelectObject(hdc,GetStockObject(DEFAULT_GUI_FONT));SetBkMode(hdc,TRANSPARENT as i32);paint_popup_toolbar(hdc,rc,if state.kind==Kind::Dps{"DPS Meter Settings"}else{"Dungeon Mechanics Settings"});let features=state.features.read().map(|x|x.clone()).unwrap_or_default();let layout=if state.kind==Kind::Dps{&features.dps}else{&features.mechanics};SetTextColor(hdc,rgb(205,215,227));draw(hdc,&format!("Opacity: {}%",layout.opacity),RECT{left:16,top:TOOLBAR_H+10,right:225,bottom:TOOLBAR_H+38},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);paint_tab(hdc,245,TOOLBAR_H+12,34,"-",false);paint_tab(hdc,286,TOOLBAR_H+12,34,"+",false);draw(hdc,&format!("Collapse side: {} (click to cycle)",layout.collapse_side),RECT{left:16,top:TOOLBAR_H+44,right:rc.right-16,bottom:TOOLBAR_H+72},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);if state.kind==Kind::Dps{SetTextColor(hdc,rgb(238,242,247));draw(hdc,"Meter fields",RECT{left:16,top:TOOLBAR_H+82,right:rc.right-16,bottom:TOOLBAR_H+108},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);let values=[("Damage %",features.meter.show_damage_share),("Healing %",features.meter.show_healing_share),("Tank %",features.meter.show_tank_share),("Death count",features.meter.show_deaths),("Battle Imagine badges",features.meter.show_imagines),("Target / HP header",features.meter.show_target)];for(i,(label,checked))in values.iter().enumerate(){paint_check(hdc,16,TOOLBAR_H+114+i as i32*28,label,*checked,rc.right);}let section=TOOLBAR_H+292;SetTextColor(hdc,rgb(238,242,247));draw(hdc,"History & row behavior",RECT{left:16,top:section,right:rc.right-16,bottom:section+26},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);let advanced=[("Active + encounter rates",features.meter.show_active_rates),("Only contributors",features.meter.only_contributors),("Party only",features.meter.party_only),("Always show self",features.meter.always_show_self)];for(i,(label,checked))in advanced.iter().enumerate(){paint_check(hdc,16,section+30+i as i32*28,label,*checked,rc.right);}let rows_y=section+148;SetTextColor(hdc,rgb(205,215,227));draw(hdc,"Visible rows",RECT{left:16,top:rows_y,right:220,bottom:rows_y+28},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);paint_tab(hdc,250,rows_y+2,92,&visible_rows_label(features.meter.visible_rows),false);let history_y=rows_y+38;draw(hdc,&format!("Saved encounters: {}",features.meter.history_limit),RECT{left:16,top:history_y,right:245,bottom:history_y+28},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);paint_tab(hdc,260,history_y+2,34,"-",false);paint_tab(hdc,301,history_y+2,34,"+",false);SetTextColor(hdc,rgb(132,145,162));draw(hdc,"History is stored locally as compressed files. < / LIVE / > navigates fights.",RECT{left:16,top:history_y+39,right:rc.right-16,bottom:history_y+68},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}else{let selected=features.mechanic_attributes.tracked.len();SetTextColor(hdc,rgb(238,242,247));draw(hdc,&format!("Track attributes ({selected}/6)"),RECT{left:16,top:TOOLBAR_H+82,right:rc.right-16,bottom:TOOLBAR_H+108},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);let per_col=(ATTRIBUTE_CATALOG.len()+1)/2;let col_w=(rc.right-32)/2;for(i,(id,label))in ATTRIBUTE_CATALOG.iter().enumerate(){let col=i/per_col;let row=i%per_col;let x=16+col as i32*col_w;let y=TOOLBAR_H+112+row as i32*25;paint_check(hdc,x,y,label,features.mechanic_attributes.tracked.contains(id),(x+col_w).min(rc.right));}}EndPaint(hwnd,&ps);}
unsafe fn settings_click(hwnd:HWND,state:&SettingsState,x:i32,y:i32){let mut rc:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rc);if y<TOOLBAR_H{if x>=rc.right-BUTTON_W{DestroyWindow(hwnd);}else{drag_window(hwnd);}return;}if(TOOLBAR_H+8..=TOOLBAR_H+42).contains(&y)&&(235..=330).contains(&x){let delta=if x<283{-5}else{5};mutate_features(state,|features|{let layout=if state.kind==Kind::Dps{&mut features.dps}else{&mut features.mechanics};layout.opacity=(layout.opacity+delta).clamp(25,100);});}else if(TOOLBAR_H+42..=TOOLBAR_H+76).contains(&y){mutate_features(state,|features|{let layout=if state.kind==Kind::Dps{&mut features.dps}else{&mut features.mechanics};layout.collapse_side=match layout.collapse_side.to_ascii_lowercase().as_str(){"right"=>"Bottom".into(),"bottom"=>"Left".into(),"left"=>"Top".into(),_=>"Right".into()};});}else if state.kind==Kind::Dps{let basic=TOOLBAR_H+114;if y>=basic&&y<basic+6*28{let idx=((y-basic)/28)as usize;mutate_features(state,|features|match idx{0=>features.meter.show_damage_share=!features.meter.show_damage_share,1=>features.meter.show_healing_share=!features.meter.show_healing_share,2=>features.meter.show_tank_share=!features.meter.show_tank_share,3=>features.meter.show_deaths=!features.meter.show_deaths,4=>features.meter.show_imagines=!features.meter.show_imagines,5=>features.meter.show_target=!features.meter.show_target,_=>{}});}else{let section=TOOLBAR_H+292;let advanced=section+30;if y>=advanced&&y<advanced+4*28{let idx=((y-advanced)/28)as usize;mutate_features(state,|features|match idx{0=>features.meter.show_active_rates=!features.meter.show_active_rates,1=>features.meter.only_contributors=!features.meter.only_contributors,2=>features.meter.party_only=!features.meter.party_only,3=>features.meter.always_show_self=!features.meter.always_show_self,_=>{}});}else{let rows_y=section+148;if y>=rows_y&&y<rows_y+32{mutate_features(state,|features|{features.meter.visible_rows=match features.meter.visible_rows{0=>5,5=>10,10=>20,20=>30,30=>50,_=>0};});}else{let history_y=rows_y+38;if y>=history_y&&y<history_y+32&&(250..=345).contains(&x){let delta=if x<300{-10}else{10};mutate_features(state,|features|{features.meter.history_limit=((features.meter.history_limit as i32)+delta).clamp(10,200)as usize;});}}}}}else{let per_col=(ATTRIBUTE_CATALOG.len()+1)/2;let col_w=(rc.right-32)/2;let start=TOOLBAR_H+112;if y>=start{let col=if x>=16+col_w{1usize}else{0usize};let row=((y-start)/25)as usize;let idx=col*per_col+row;if let Some((id,_))=ATTRIBUTE_CATALOG.get(idx).copied(){mutate_features(state,|features|{if let Some(pos)=features.mechanic_attributes.tracked.iter().position(|value|*value==id){features.mechanic_attributes.tracked.remove(pos);}else if features.mechanic_attributes.tracked.len()<6{features.mechanic_attributes.tracked.push(id);}});}}}InvalidateRect(hwnd,null(),0);InvalidateRect(state.parent,null(),0);}
"#,
        "v1.9 DPS settings",
    );

    fs::write(&overlay_path, overlay).expect("write v1.9 overlay");
    println!("cargo:rerun-if-changed=build_v190.rs");
    println!("cargo:rerun-if-changed=src/history.rs");
    println!("cargo:rerun-if-changed=src/telemetry_v190.rs");
}
