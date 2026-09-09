use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v183_buffs.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.8.4 patch `{label}` expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn replace_between(source: &mut String, start: &str, end: &str, replacement: &str, label: &str) {
    let count = source.matches(start).count();
    assert_eq!(count, 1, "v1.8.4 patch `{label}` start expected one match, found {count}");
    let begin = source.find(start).expect("start checked");
    let rel_end = source[begin..].find(end)
        .unwrap_or_else(|| panic!("v1.8.4 patch `{label}` end anchor missing"));
    source.replace_range(begin..begin + rel_end, replacement);
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let overlay_path = out.join("feature_overlays_v170_fixed.rs");
    let mut overlay = fs::read_to_string(&overlay_path).expect("read generated v1.8.3 overlay");

    // Keep DPS text bold but slightly smaller. Mechanics remains at the previous
    // size because its dense timers benefit from the extra pixel of height.
    replace_once(
        &mut overlay,
        "unsafe fn create_overlay_font(bold:bool)->HFONT{let face=wide(\"Segoe UI\");CreateFontW(-14,0,0,0,if bold{700}else{400},0,0,0,1,0,0,5,0,face.as_ptr())}",
        "unsafe fn create_overlay_font(bold:bool)->HFONT{let face=wide(\"Segoe UI\");CreateFontW(if bold{-13}else{-14},0,0,0,if bold{700}else{400},0,0,0,1,0,0,5,0,face.as_ptr())}",
        "slightly smaller bold DPS font",
    );

    // The game-facing tier field is one-based while the UI convention is T0+.
    // Keep the icon overlay compact and make the tier number red.
    replace_between(
        &mut overlay,
        "unsafe fn paint_badge(hdc:HDC,x:i32,y:i32,badge:&ImagineBadge){",
        "fn imagine_asset(skill_id:i32)->Option<&'static [u8]>{",
        r#"unsafe fn paint_badge(hdc:HDC,x:i32,y:i32,badge:&ImagineBadge){let r=RECT{left:x,top:y,right:x+BADGE_W,bottom:y+23};if !draw_imagine_asset(hdc,r,badge.skill_id){fill(hdc,&r,badge_color(&badge.icon_key));SetTextColor(hdc,rgb(248,250,252));let short=if badge.icon_key.trim().is_empty()||badge.icon_key.eq_ignore_ascii_case("BI"){"BI"}else{badge.icon_key.as_str()};draw(hdc,short,r,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}if badge.tier>0{let tier=RECT{left:r.right-11,top:r.bottom-11,right:r.right,bottom:r.bottom};fill(hdc,&tier,rgb(8,11,15));SetTextColor(hdc,rgb(255,70,70));draw(hdc,&badge.tier.saturating_sub(1).to_string(),tier,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}}
"#,
        "T0 Imagine tier overlay",
    );

    replace_between(
        &mut overlay,
        "unsafe fn hover_badge_at(hwnd:HWND,state:&State,x:i32,y:i32)->Option<String>{",
        "unsafe fn paint_hover",
        r#"unsafe fn hover_badge_at(hwnd:HWND,state:&State,x:i32,y:i32)->Option<String>{if state.kind!=Kind::Dps||state.collapsed||!state.features.read().map(|f|f.meter.show_imagines).unwrap_or(true){return None;}let mut rc:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rc);let top=dps_rows_top();if y<top||y>=rc.bottom{return None;}let screen_i=((y-top)/DPS_ROW_H)as usize;if screen_i>=visible_dps_rows(rc.bottom){return None;}let rows=sorted_rows(&state.dps,state.sort_mode);let row=*rows.get(state.scroll+screen_i)?;if row.is_dead{return None;}let r=RECT{left:6,top:top+screen_i as i32*DPS_ROW_H,right:rc.right-8,bottom:top+screen_i as i32*DPS_ROW_H+DPS_ROW_H-2};let layout=dps_row_layout(r,true,row);let mut bx=layout.badge_left;for badge in row.imagines.iter().take(2){if x>=bx&&x<bx+BADGE_W&&y>=r.top+4&&y<r.top+28{let tier=if badge.tier>0{badge.tier.saturating_sub(1).to_string()}else{"?".into()};return Some(format!("{} · Tier {}",badge.name,tier));}bx+=BADGE_W+BADGE_GAP;}None}
"#,
        "T0 Imagine hover and compact hitboxes",
    );

    // One packed, non-wrapping left group. Rank is intentionally small and has
    // no punctuation. Name/spec truncates before icons/score are allowed to
    // separate. The thin bottom bar is relative to the active tab leader.
    replace_between(
        &mut overlay,
        "#[derive(Clone,Copy)]struct DpsRowLayout",
        "unsafe fn draw_percent(",
        r#"#[derive(Clone,Copy)]struct DpsRowLayout{name_left:i32,name_right:i32,badge_left:i32,score_left:i32,left_end:i32,middle_left:i32,middle_right:i32,share_left:i32,share_right:i32,middle_col:i32}
fn approx_text_px(text:&str)->i32{(text.chars().count()as i32*6+2).max(10)}
fn dps_row_layout(r:RECT,show_imagines:bool,row:&DpsRow)->DpsRowLayout{let total=(r.right-r.left).max(1);let share_w=if total<650{56}else{62};let middle_w=if total<650{146}else{164};let gap=5;let share_right=r.right-2;let share_left=share_right-share_w;let middle_right=share_left-gap;let middle_left=middle_right-middle_w;let left_end=middle_left-gap;let name_left=r.left+21;let identity=dps_identity(row);let badge_count=if show_imagines&&!row.is_dead{row.imagines.len().min(2)as i32}else{0};let badge_span=badge_count*BADGE_W+badge_count.saturating_sub(1)*BADGE_GAP;let score=if row.is_dead{revive_status_text(row,now_ms()).0}else{dps_score_pair(row)};let score_w=if score.is_empty(){0}else{approx_text_px(&score).clamp(44,if row.is_dead{132}else{88})};let icon_gap=if badge_span>0{4}else{0};let score_gap=if score_w>0{4}else{0};let fixed=badge_span+icon_gap+score_w+score_gap;let name_max=(left_end-name_left-fixed).max(36);let name_w=approx_text_px(&identity).min(name_max);let name_right=(name_left+name_w).min(left_end);let badge_left=(name_right+icon_gap).min(left_end);let score_left=(badge_left+badge_span+score_gap).min(left_end);DpsRowLayout{name_left,name_right,badge_left,score_left,left_end,middle_left,middle_right,share_left,share_right,middle_col:(middle_w/2).max(1)}}
fn dps_identity(row:&DpsRow)->String{let spec=if !row.subprofession_name.trim().is_empty(){row.subprofession_name.as_str()}else{profession_name(row.profession_id)};if spec.trim().is_empty(){row.name.clone()}else{format!("{}-{}",row.name,spec)}}
fn dps_score_pair(row:&DpsRow)->String{match(row.ability_score>0,row.illusion_break>0){(true,true)=>format!("({}+{})",score(row.ability_score),score(row.illusion_break)),(true,false)=>format!("({})",score(row.ability_score)),(false,true)=>format!("(+{})",score(row.illusion_break)),_=>String::new()}}
fn revive_status_text(row:&DpsRow,now:i64)->(String,u32){if row.revive_blocked_until_ms<0{return("REVIVE IN : ?".into(),rgb(255,190,70));}if row.revive_blocked_until_ms>now{let left=row.revive_blocked_until_ms.saturating_sub(now)as u64;return(format!("REVIVE IN : {}",format_countdown(left)),rgb(255,190,70));}("CAN REVIVE".into(),rgb(72,226,116))}
fn rate(total:i64,encounter_ms:u64)->f64{if total<=0{0.0}else{let seconds=(encounter_ms.max(1)as f64/1000.0).max(0.001);total as f64/seconds}}
fn mode_values(row:&DpsRow,mode:SortMode,encounter_ms:u64)->(String,String){match mode{SortMode::Damage=>(compact(row.damage as f64),format!("{}/s",compact(row.dps))),SortMode::Heal=>(compact(row.healing as f64),format!("{}/s",compact(rate(row.healing,encounter_ms)))),SortMode::Tank=>(compact(row.damage_taken as f64),format!("{}/s",compact(rate(row.damage_taken,encounter_ms))))}}
fn mode_metric(row:&DpsRow,mode:SortMode)->i64{match mode{SortMode::Damage=>row.damage,SortMode::Heal=>row.healing,SortMode::Tank=>row.damage_taken}.max(0)}
fn active_share(row:&DpsRow,mode:SortMode,settings:&FeatureSettings)->Option<(f64,u32)>{match mode{SortMode::Damage if settings.meter.show_damage_share=>Some((row.damage_share,rgb(255,70,70))),SortMode::Heal if settings.meter.show_healing_share=>Some((row.healing_share,rgb(40,215,100))),SortMode::Tank if settings.meter.show_tank_share=>Some((row.tank_share,rgb(35,145,255))),_=>None}}
unsafe fn dps_name_row_at(hwnd:HWND,state:&State,x:i32,y:i32)->Option<DpsRow>{let mut rc:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rc);let top=dps_rows_top();if y<top||y>=rc.bottom{return None;}let screen_i=((y-top)/DPS_ROW_H)as usize;if screen_i>=visible_dps_rows(rc.bottom){return None;}let rows=sorted_rows(&state.dps,state.sort_mode);let row=*rows.get(state.scroll+screen_i)?;let r=RECT{left:6,top:top+screen_i as i32*DPS_ROW_H,right:rc.right-8,bottom:top+screen_i as i32*DPS_ROW_H+DPS_ROW_H-2};let show=state.features.read().map(|f|f.meter.show_imagines).unwrap_or(true);let layout=dps_row_layout(r,show,row);if x>=layout.name_left&&x<layout.name_right{Some(row.clone())}else{None}}
unsafe fn paint_dps(hdc:HDC,rc:RECT,state:&State){let settings=state.features.read().map(|x|x.clone()).unwrap_or_default();let control_top=TOOLBAR_H+4;paint_tab(hdc,8,control_top,70,"Damage",state.sort_mode==SortMode::Damage);paint_tab(hdc,83,control_top,65,"Heal",state.sort_mode==SortMode::Heal);paint_tab(hdc,153,control_top,65,"Tank",state.sort_mode==SortMode::Tank);paint_tab(hdc,225,control_top,73,"Reset",false);SetTextColor(hdc,rgb(145,160,180));draw(hdc,&format!("Total {}   Heal {}   Taken {}",compact(state.dps.total_damage as f64),compact(state.dps.total_healing as f64),compact(state.dps.total_damage_taken as f64)),RECT{left:304,top:control_top,right:rc.right-8,bottom:control_top+23},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);let rows=sorted_rows(&state.dps,state.sort_mode);let top=dps_rows_top();let visible=visible_dps_rows(rc.bottom);if rows.is_empty(){SetTextColor(hdc,rgb(132,145,162));draw(hdc,"Waiting for party / combat data...",RECT{left:10,top:top+18,right:rc.right-10,bottom:top+58},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);return;}let leader=rows.first().map(|row|mode_metric(row,state.sort_mode)).unwrap_or(0).max(1);let bold_font=if state.font.is_null(){GetStockObject(DEFAULT_GUI_FONT)}else{state.font};for(screen_i,row)in rows.iter().skip(state.scroll).take(visible).enumerate(){let rank=state.scroll+screen_i+1;let y=top+screen_i as i32*DPS_ROW_H;let r=RECT{left:6,top:y,right:rc.right-8,bottom:y+DPS_ROW_H-2};let bg=if row.is_dead{rgb(105,28,34)}else{spec_color(row)};fill(hdc,&r,bg);let base_text=text_on(bg);let layout=dps_row_layout(r,settings.meter.show_imagines,row);SelectObject(hdc,GetStockObject(DEFAULT_GUI_FONT));SetTextColor(hdc,base_text);draw(hdc,&rank.to_string(),RECT{left:r.left+3,top:r.top,right:r.left+19,bottom:r.bottom-2},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);SelectObject(hdc,bold_font);SetTextColor(hdc,if row.is_dead{rgb(255,120,120)}else{base_text});draw(hdc,&dps_identity(row),RECT{left:layout.name_left,top:r.top,right:layout.name_right,bottom:r.bottom-2},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);if settings.meter.show_imagines&&!row.is_dead{let mut bx=layout.badge_left;for badge in row.imagines.iter().take(2){paint_badge(hdc,bx,r.top+3,badge);bx+=BADGE_W+BADGE_GAP;}}if row.is_dead{let(status,color)=revive_status_text(row,now_ms());SetTextColor(hdc,color);draw(hdc,&status,RECT{left:layout.score_left,top:r.top,right:layout.left_end,bottom:r.bottom-2},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}else{let score_text=dps_score_pair(row);if !score_text.is_empty(){SetTextColor(hdc,base_text);draw(hdc,&score_text,RECT{left:layout.score_left,top:r.top,right:layout.left_end,bottom:r.bottom-2},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}}fill(hdc,&RECT{left:layout.left_end+1,top:r.top+5,right:layout.left_end+2,bottom:r.bottom-5},rgb(30,35,42));let(first,second)=mode_values(row,state.sort_mode,state.dps.encounter_ms);SetTextColor(hdc,base_text);draw(hdc,&first,RECT{left:layout.middle_left,top:r.top,right:layout.middle_left+layout.middle_col-3,bottom:r.bottom-2},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);draw(hdc,&second,RECT{left:layout.middle_left+layout.middle_col+2,top:r.top,right:layout.middle_right,bottom:r.bottom-2},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);fill(hdc,&RECT{left:layout.share_left-2,top:r.top+5,right:layout.share_left-1,bottom:r.bottom-5},rgb(30,35,42));if let Some((share,color))=active_share(row,state.sort_mode,&settings){SetTextColor(hdc,color);draw(hdc,&format!("{share:.1}%"),RECT{left:layout.share_left,top:r.top,right:layout.share_right,bottom:r.bottom-2},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}if settings.meter.show_deaths&&row.deaths>0{SetTextColor(hdc,rgb(255,70,70));draw(hdc,&format!("D{}",row.deaths),RECT{left:layout.share_left,top:r.top,right:layout.share_right,bottom:r.top+10},DT_RIGHT|DT_SINGLELINE|DT_NOPREFIX);}let metric=mode_metric(row,state.sort_mode);let bar=RECT{left:r.left,top:r.bottom-2,right:r.right,bottom:r.bottom};fill(hdc,&bar,rgb(20,24,29));if metric>0{let width=(r.right-r.left).max(1)as i64;let filled=(width.saturating_mul(metric).saturating_div(leader)).clamp(1,width)as i32;fill(hdc,&RECT{left:r.left,top:r.bottom-2,right:r.left+filled,bottom:r.bottom},if row.is_dead{rgb(255,110,110)}else{base_text});}if row.is_local{outline(hdc,r,rgb(212,175,55),2);}}paint_scrollbar(hdc,rc,rows.len(),visible,state.scroll,top);}
"#,
        "compact packed DPS rows with relative progress bar",
    );

    // Attribute tracker uses one row for up to three selections. If more than
    // three are selected, the first three stay on row one and every remainder
    // moves to a second row instead of squeezing the entire set horizontally.
    replace_between(
        &mut overlay,
        "unsafe fn paint_mechanics(hdc:HDC,rc:RECT,state:&State){",
        "fn mechanic_timer(",
        r#"unsafe fn paint_mechanics(hdc:HDC,rc:RECT,state:&State){
let features=state.features.read().map(|x|x.clone()).unwrap_or_default();
let tracked:Vec<_>=features.mechanic_attributes.tracked.iter().filter_map(|id|state.mechanics.tracked_attributes.iter().find(|attr|attr.attr_id==*id)).collect();
let mut top=TOOLBAR_H+5;
if !tracked.is_empty(){let rows=if tracked.len()>3{2}else{1};for row_i in 0..rows{let start=if row_i==0{0}else{3};let end=if row_i==0{tracked.len().min(3)}else{tracked.len()};let count=end.saturating_sub(start);if count==0{continue;}let strip=RECT{left:6,top,right:rc.right-6,bottom:top+MECH_ATTR_H-3};fill(hdc,&strip,rgb(27,32,39));let width=((strip.right-strip.left)/count as i32).max(60);for(i,attr)in tracked[start..end].iter().enumerate(){let left=strip.left+i as i32*width;SetTextColor(hdc,attr_color(attr.attr_id));draw(hdc,&format!("{} {}",attr.label.trim_end_matches(" %"),format_attr_value(attr.attr_id,attr.value)),RECT{left:left+2,top:strip.top,right:(left+width-2).min(strip.right),bottom:strip.bottom},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}top+=MECH_ATTR_H;}}
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
        "two-line attribute tracker layout",
    );

    fs::write(&overlay_path, overlay).expect("write v1.8.4 overlay");
    println!("cargo:rerun-if-changed=build_v184.rs");
}
