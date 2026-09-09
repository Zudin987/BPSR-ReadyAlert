use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v183.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.8.3 follow-up patch `{label}` expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn replace_between(source: &mut String, start: &str, end: &str, replacement: &str, label: &str) {
    let count = source.matches(start).count();
    assert_eq!(count, 1, "v1.8.3 follow-up patch `{label}` start expected one match, found {count}");
    let begin = source.find(start).expect("start checked");
    let rel_end = source[begin..].find(end)
        .unwrap_or_else(|| panic!("v1.8.3 follow-up patch `{label}` end anchor missing"));
    source.replace_range(begin..begin + rel_end, replacement);
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));

    // Capture incoming damage by skill as well as outgoing damage/healing so the
    // entity inspector can provide a real Taken tab rather than invented data.
    let telemetry_path = out.join("telemetry_v170_fixed.rs");
    let mut telemetry = fs::read_to_string(&telemetry_path).expect("read generated telemetry");
    replace_once(
        &mut telemetry,
        "    skills: HashMap<i32, SkillStat>,\n    imagines: HashMap<i32, i32>,",
        "    skills: HashMap<i32, SkillStat>,\n    taken_skills: HashMap<i32, SkillStat>,\n    imagines: HashMap<i32, i32>,",
        "incoming skill accumulator",
    );
    replace_once(
        &mut telemetry,
        r#"                self.combat.entry(uid).or_default().damage_taken = self
                    .combat
                    .get(&uid)
                    .map(|actor| actor.damage_taken)
                    .unwrap_or(0)
                    .saturating_add(value);"#,
        r#"                let actor = self.combat.entry(uid).or_default();
                actor.damage_taken = actor.damage_taken.saturating_add(value);
                if skill != 0 {
                    let incoming = actor.taken_skills.entry(skill).or_default();
                    incoming.damage = incoming.damage.saturating_add(value);
                    incoming.hits = incoming.hits.saturating_add(1);
                    if crit { incoming.crits = incoming.crits.saturating_add(1); }
                    if lucky { incoming.luckies = incoming.luckies.saturating_add(1); }
                    incoming.max_value = incoming.max_value.max(value);
                }"#,
        "incoming damage skill stats",
    );
    replace_once(
        &mut telemetry,
        r#"            skills.sort_by(|a, b| (b.damage + b.healing).cmp(&(a.damage + a.healing)));

            let mut imagines: Vec<ImagineBadge> = stat"#,
        r#"            skills.sort_by(|a, b| (b.damage + b.healing).cmp(&(a.damage + a.healing)));
            let mut taken_skills: Vec<SkillBreakdown> = stat
                .taken_skills
                .iter()
                .map(|(id, value)| SkillBreakdown {
                    skill_id: *id,
                    name: skill_name(*id),
                    damage: value.damage,
                    healing: 0,
                    hits: value.hits,
                    crits: value.crits,
                    lucky_hits: value.luckies,
                    max_value: value.max_value,
                })
                .collect();
            taken_skills.sort_by(|a, b| b.damage.cmp(&a.damage));
            let attributes: Vec<TrackedAttribute> = tracked_attr_ids()
                .map(|id| TrackedAttribute {
                    attr_id: id,
                    label: attr_label(id).into(),
                    value: *meta.attrs.get(&id).unwrap_or(&0),
                })
                .collect();

            let mut imagines: Vec<ImagineBadge> = stat"#,
        "inspector row metadata",
    );
    replace_once(
        &mut telemetry,
        "                illusion_break: meta.illusion_break,\n                damage: stat.damage,",
        "                illusion_break: meta.illusion_break,\n                hp: meta.hp,\n                max_hp: meta.max_hp,\n                damage: stat.damage,",
        "DPS row HP",
    );
    replace_once(
        &mut telemetry,
        r#"                is_local: uid == self.local_uid && self.local_uid > 0,
                imagines,
                skills,"#,
        r#"                is_local: uid == self.local_uid && self.local_uid > 0,
                attributes,
                imagines,
                skills,
                taken_skills,"#,
        "DPS row inspector payload",
    );
    fs::write(&telemetry_path, telemetry).expect("write follow-up telemetry");

    let overlay_path = out.join("feature_overlays_v170_fixed.rs");
    let mut overlay = fs::read_to_string(&overlay_path).expect("read generated overlay");
    replace_once(
        &mut overlay,
        "feature_settings::{self, format_attr_value, FeatureSettings, ATTRIBUTE_CATALOG},",
        "feature_settings::{self, format_attr_value, FeatureSettings, ATTRIBUTE_CATALOG, ATTR_CRIT, ATTR_DEFENSE_POWER, ATTR_ENDURANCE, ATTR_HASTE, ATTR_LUCKY, ATTR_MAGIC_ATTACK, ATTR_MASTERY},",
        "entity inspector attribute imports",
    );
    replace_once(
        &mut overlay,
        "model::{ConsumableStatus, DpsRow, DpsSnapshot, ImagineBadge, MechanicSnapshot},",
        "model::{ConsumableStatus, DpsRow, DpsSnapshot, ImagineBadge, MechanicSnapshot, SkillBreakdown},",
        "entity inspector skill import",
    );
    replace_once(
        &mut overlay,
        "struct DetailState {\nrow: DpsRow,\nscroll: usize,\n}",
        "struct DetailState {\nrow: DpsRow,\nscroll: usize,\nmode: SortMode,\nencounter_ms: u64,\n}",
        "detail tab state",
    );
    replace_once(
        &mut overlay,
        "update_detail(state.detail_hwnd,row);",
        "update_detail(state.detail_hwnd,row,snapshot.encounter_ms);",
        "live inspector encounter time",
    );
    replace_once(
        &mut overlay,
        "if state.kind==Kind::Dps{if let Some(row)=dps_row_at(hwnd,state,y){open_detail(hwnd,state,row);}}}",
        "if state.kind==Kind::Dps{if let Some(row)=dps_name_row_at(hwnd,state,x,y){open_detail(hwnd,state,row);}}}",
        "open inspector only from player name",
    );

    replace_between(
        &mut overlay,
        "unsafe fn hover_badge_at(hwnd:HWND,state:&State,x:i32,y:i32)->Option<String>{",
        "unsafe fn paint_hover",
        r#"unsafe fn hover_badge_at(hwnd:HWND,state:&State,x:i32,y:i32)->Option<String>{if state.kind!=Kind::Dps||state.collapsed||!state.features.read().map(|f|f.meter.show_imagines).unwrap_or(true){return None;}let mut rc:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rc);let top=dps_rows_top();if y<top||y>=rc.bottom{return None;}let screen_i=((y-top)/DPS_ROW_H)as usize;if screen_i>=visible_dps_rows(rc.bottom){return None;}let rows=sorted_rows(&state.dps,state.sort_mode);let row=*rows.get(state.scroll+screen_i)?;let r=RECT{left:6,top:top+screen_i as i32*DPS_ROW_H,right:rc.right-8,bottom:top+screen_i as i32*DPS_ROW_H+DPS_ROW_H-2};let layout=dps_row_layout(r,true,row);let mut bx=layout.badge_left;for badge in row.imagines.iter().take(2){if x>=bx&&x<bx+BADGE_W&&y>=r.top+4&&y<r.top+28{let tier=if badge.tier>0{badge.tier.to_string()}else{"?".into()};return Some(format!("{} · Tier {}",badge.name,tier));}bx+=BADGE_W+BADGE_GAP;}None}
"#,
        "hover follows packed left group",
    );

    replace_between(
        &mut overlay,
        "#[derive(Clone,Copy)]struct DpsRowLayout",
        "unsafe fn draw_percent(",
        r#"#[derive(Clone,Copy)]struct DpsRowLayout{name_left:i32,name_right:i32,badge_left:i32,score_left:i32,left_end:i32,middle_left:i32,middle_right:i32,share_left:i32,share_right:i32,middle_col:i32}
fn approx_text_px(text:&str)->i32{(text.chars().count()as i32*7+3).max(12)}
fn dps_row_layout(r:RECT,show_imagines:bool,row:&DpsRow)->DpsRowLayout{let total=(r.right-r.left).max(1);let share_w=if total<620{56}else{62};let middle_w=if total<620{142}else{164};let gap=5;let share_right=r.right-2;let share_left=share_right-share_w;let middle_right=share_left-gap;let middle_left=middle_right-middle_w;let left_end=middle_left-gap;let name_left=r.left+30;let identity=dps_identity(row);let badge_count=if show_imagines{row.imagines.len().min(2)as i32}else{0};let badge_span=badge_count*BADGE_W+badge_count.saturating_sub(1)*BADGE_GAP;let score=dps_score_pair(row);let score_w=if score.is_empty(){0}else{approx_text_px(&score).clamp(54,96)};let fixed=badge_span+score_w+if badge_span>0{6}else{0}+if score_w>0{6}else{0};let name_max=(left_end-name_left-fixed).max(46);let name_w=approx_text_px(&identity).min(name_max);let name_right=(name_left+name_w).min(left_end);let badge_left=(name_right+5).min(left_end);let score_left=(badge_left+badge_span+if badge_span>0{6}else{0}).min(left_end);DpsRowLayout{name_left,name_right,badge_left,score_left,left_end,middle_left,middle_right,share_left,share_right,middle_col:(middle_w/2).max(1)}}
fn dps_identity(row:&DpsRow)->String{let spec=if !row.subprofession_name.trim().is_empty(){row.subprofession_name.as_str()}else{profession_name(row.profession_id)};if spec.trim().is_empty(){row.name.clone()}else{format!("{}-{}",row.name,spec)}}
fn dps_score_pair(row:&DpsRow)->String{match(row.ability_score>0,row.illusion_break>0){(true,true)=>format!("({} + {})",score(row.ability_score),score(row.illusion_break)),(true,false)=>format!("({})",score(row.ability_score)),(false,true)=>format!("(+ {})",score(row.illusion_break)),_=>String::new()}}
fn rate(total:i64,encounter_ms:u64)->f64{if total<=0{0.0}else{let seconds=(encounter_ms.max(1)as f64/1000.0).max(0.001);total as f64/seconds}}
fn mode_values(row:&DpsRow,mode:SortMode,encounter_ms:u64)->(String,String){match mode{SortMode::Damage=>(compact(row.damage as f64),format!("{}/s",compact(row.dps))),SortMode::Heal=>(compact(row.healing as f64),format!("{}/s",compact(rate(row.healing,encounter_ms)))),SortMode::Tank=>(compact(row.damage_taken as f64),format!("{}/s",compact(rate(row.damage_taken,encounter_ms))))}}
fn active_share(row:&DpsRow,mode:SortMode,settings:&FeatureSettings)->Option<(f64,u32)>{match mode{SortMode::Damage if settings.meter.show_damage_share=>Some((row.damage_share,rgb(255,70,70))),SortMode::Heal if settings.meter.show_healing_share=>Some((row.healing_share,rgb(40,215,100))),SortMode::Tank if settings.meter.show_tank_share=>Some((row.tank_share,rgb(35,145,255))),_=>None}}
unsafe fn dps_name_row_at(hwnd:HWND,state:&State,x:i32,y:i32)->Option<DpsRow>{let mut rc:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rc);let top=dps_rows_top();if y<top||y>=rc.bottom{return None;}let screen_i=((y-top)/DPS_ROW_H)as usize;if screen_i>=visible_dps_rows(rc.bottom){return None;}let rows=sorted_rows(&state.dps,state.sort_mode);let row=*rows.get(state.scroll+screen_i)?;let r=RECT{left:6,top:top+screen_i as i32*DPS_ROW_H,right:rc.right-8,bottom:top+screen_i as i32*DPS_ROW_H+DPS_ROW_H-2};let show=state.features.read().map(|f|f.meter.show_imagines).unwrap_or(true);let layout=dps_row_layout(r,show,row);if x>=layout.name_left&&x<layout.name_right{Some(row.clone())}else{None}}
unsafe fn paint_dps(hdc:HDC,rc:RECT,state:&State){let settings=state.features.read().map(|x|x.clone()).unwrap_or_default();let control_top=TOOLBAR_H+4;paint_tab(hdc,8,control_top,70,"Damage",state.sort_mode==SortMode::Damage);paint_tab(hdc,83,control_top,65,"Heal",state.sort_mode==SortMode::Heal);paint_tab(hdc,153,control_top,65,"Tank",state.sort_mode==SortMode::Tank);paint_tab(hdc,225,control_top,73,"Reset",false);SetTextColor(hdc,rgb(145,160,180));draw(hdc,&format!("Total {}   Heal {}   Taken {}",compact(state.dps.total_damage as f64),compact(state.dps.total_healing as f64),compact(state.dps.total_damage_taken as f64)),RECT{left:304,top:control_top,right:rc.right-8,bottom:control_top+23},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);let rows=sorted_rows(&state.dps,state.sort_mode);let top=dps_rows_top();let visible=visible_dps_rows(rc.bottom);if rows.is_empty(){SetTextColor(hdc,rgb(132,145,162));draw(hdc,"Waiting for party / combat data...",RECT{left:10,top:top+18,right:rc.right-10,bottom:top+58},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);return;}for(screen_i,row)in rows.iter().skip(state.scroll).take(visible).enumerate(){let rank=state.scroll+screen_i+1;let y=top+screen_i as i32*DPS_ROW_H;let r=RECT{left:6,top:y,right:rc.right-8,bottom:y+DPS_ROW_H-2};let bg=spec_color(row);fill(hdc,&r,bg);if row.is_local{outline(hdc,r,rgb(212,175,55),2);}let base_text=text_on(bg);let layout=dps_row_layout(r,settings.meter.show_imagines,row);SetTextColor(hdc,base_text);draw(hdc,&format!("{rank}."),RECT{left:r.left+4,top:r.top,right:r.left+28,bottom:r.bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);SetTextColor(hdc,if row.is_dead{rgb(255,45,45)}else{base_text});draw(hdc,&dps_identity(row),RECT{left:layout.name_left,top:r.top,right:layout.name_right,bottom:r.bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);if settings.meter.show_imagines{let mut bx=layout.badge_left;for badge in row.imagines.iter().take(2){paint_badge(hdc,bx,r.top+4,badge);bx+=BADGE_W+BADGE_GAP;}}let score_text=dps_score_pair(row);if !score_text.is_empty(){SetTextColor(hdc,base_text);draw(hdc,&score_text,RECT{left:layout.score_left,top:r.top,right:layout.left_end,bottom:r.bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}fill(hdc,&RECT{left:layout.left_end+1,top:r.top+5,right:layout.left_end+2,bottom:r.bottom-5},rgb(30,35,42));let(first,second)=mode_values(row,state.sort_mode,state.dps.encounter_ms);SetTextColor(hdc,base_text);draw(hdc,&first,RECT{left:layout.middle_left,top:r.top,right:layout.middle_left+layout.middle_col-3,bottom:r.bottom},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);draw(hdc,&second,RECT{left:layout.middle_left+layout.middle_col+2,top:r.top,right:layout.middle_right,bottom:r.bottom},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);fill(hdc,&RECT{left:layout.share_left-2,top:r.top+5,right:layout.share_left-1,bottom:r.bottom-5},rgb(30,35,42));if let Some((share,color))=active_share(row,state.sort_mode,&settings){SetTextColor(hdc,color);draw(hdc,&format!("{share:.1}%"),RECT{left:layout.share_left,top:r.top,right:layout.share_right,bottom:r.bottom},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}if settings.meter.show_deaths&&row.deaths>0{SetTextColor(hdc,rgb(255,45,45));draw(hdc,&format!("D{}",row.deaths),RECT{left:layout.share_left,top:r.top,right:layout.share_right,bottom:r.top+11},DT_RIGHT|DT_SINGLELINE|DT_NOPREFIX);}}paint_scrollbar(hdc,rc,rows.len(),visible,state.scroll,top);}
"#,
        "packed responsive DPS rows and active share only",
    );

    replace_between(
        &mut overlay,
        "unsafe fn open_detail(parent:HWND,state:&mut State,row:DpsRow){",
        "unsafe fn open_feature_settings",
        r#"unsafe fn open_detail(parent:HWND,state:&mut State,row:DpsRow){if !state.detail_hwnd.is_null()&&IsWindow(state.detail_hwnd)!=0{DestroyWindow(state.detail_hwnd);}let instance=GetModuleHandleW(null());let class=wide(DETAIL_CLASS);let wc=WNDCLASSW{lpfnWndProc:Some(detail_wnd_proc),hInstance:instance,hCursor:LoadCursorW(null_mut(),IDC_ARROW),lpszClassName:class.as_ptr(),..std::mem::zeroed()};if RegisterClassW(&wc)==0&&GetLastError()!=1410{return;}let mut pr:RECT=std::mem::zeroed();GetWindowRect(parent,&mut pr);let ptr=Box::into_raw(Box::new(DetailState{row:row.clone(),scroll:0,mode:state.sort_mode,encounter_ms:state.dps.encounter_ms}));let title=wide("Entity Inspector");let hwnd=CreateWindowExW(WS_EX_TOOLWINDOW|WS_EX_TOPMOST,class.as_ptr(),title.as_ptr(),WS_POPUP|WS_THICKFRAME,pr.right+8,pr.top,900,600,parent,null_mut(),instance,ptr.cast::<c_void>());if hwnd.is_null(){drop(Box::from_raw(ptr));return;}state.detail_hwnd=hwnd;state.detail_uid=row.uid;ShowWindow(hwnd,SW_SHOW);SetForegroundWindow(hwnd);}
unsafe fn update_detail(hwnd:HWND,row:DpsRow,encounter_ms:u64){let ptr=GetWindowLongPtrW(hwnd,GWLP_USERDATA)as *mut DetailState;if !ptr.is_null(){(*ptr).row=row;(*ptr).encounter_ms=encounter_ms;let count=detail_skills(&*ptr).len();(*ptr).scroll=(*ptr).scroll.min(count.saturating_sub(1));InvalidateRect(hwnd,null(),0);}}
unsafe extern "system" fn detail_wnd_proc(hwnd:HWND,msg:u32,wparam:WPARAM,lparam:LPARAM)->LRESULT{if msg==WM_NCCREATE{let create=lparam as *const CREATESTRUCTW;if !create.is_null(){SetWindowLongPtrW(hwnd,GWLP_USERDATA,(*create).lpCreateParams as isize);}}let ptr=GetWindowLongPtrW(hwnd,GWLP_USERDATA)as *mut DetailState;match msg{WM_ERASEBKGND=>1,WM_PAINT=>{if !ptr.is_null(){paint_detail(hwnd,&*ptr);}0},WM_LBUTTONDOWN=>{let x=lo_signed(lparam);let y=hi_signed(lparam);let mut rc:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rc);if y<TOOLBAR_H{if x>=rc.right-BUTTON_W{DestroyWindow(hwnd);}else{drag_window(hwnd);}}else if !ptr.is_null()&&y>=250&&y<278{let state=&mut*ptr;state.mode=if x<76{SortMode::Damage}else if x<151{SortMode::Heal}else if x<226{SortMode::Tank}else{state.mode};state.scroll=0;InvalidateRect(hwnd,null(),0);}0},WM_MOUSEWHEEL=>{if !ptr.is_null(){let delta=(((wparam>>16)&0xffff)as u16 as i16)as i32;let state=&mut*ptr;let count=detail_skills(state).len();if delta>0{state.scroll=state.scroll.saturating_sub(3);}else if delta<0{state.scroll=state.scroll.saturating_add(3).min(count.saturating_sub(1));}InvalidateRect(hwnd,null(),0);}0},WM_NCDESTROY=>{SetWindowLongPtrW(hwnd,GWLP_USERDATA,0);if !ptr.is_null(){drop(Box::from_raw(ptr));}DefWindowProcW(hwnd,msg,wparam,lparam)},_=>DefWindowProcW(hwnd,msg,wparam,lparam)}}
fn detail_attr(row:&DpsRow,id:i32)->i64{row.attributes.iter().find(|a|a.attr_id==id).map(|a|a.value).unwrap_or(0)}
fn detail_skills(state:&DetailState)->Vec<&SkillBreakdown>{let mut out:Vec<&SkillBreakdown>=match state.mode{SortMode::Damage=>state.row.skills.iter().filter(|s|s.damage>0).collect(),SortMode::Heal=>state.row.skills.iter().filter(|s|s.healing>0).collect(),SortMode::Tank=>state.row.taken_skills.iter().filter(|s|s.damage>0).collect()};out.sort_by(|a,b|detail_skill_amount(b,state.mode).cmp(&detail_skill_amount(a,state.mode)));out}
fn detail_skill_amount(skill:&SkillBreakdown,mode:SortMode)->i64{match mode{SortMode::Damage=>skill.damage,SortMode::Heal=>skill.healing,SortMode::Tank=>skill.damage}}
fn detail_total(row:&DpsRow,mode:SortMode)->i64{match mode{SortMode::Damage=>row.damage,SortMode::Heal=>row.healing,SortMode::Tank=>row.damage_taken}}
fn detail_mode_label(mode:SortMode)->&'static str{match mode{SortMode::Damage=>"Damage",SortMode::Heal=>"Healing",SortMode::Tank=>"Taken"}}
unsafe fn paint_detail(hwnd:HWND,state:&DetailState){let mut ps:PAINTSTRUCT=std::mem::zeroed();let hdc=BeginPaint(hwnd,&mut ps);if hdc.is_null(){return;}let mut rc:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rc);fill(hdc,&rc,rgb(24,24,24));SelectObject(hdc,GetStockObject(DEFAULT_GUI_FONT));SetBkMode(hdc,TRANSPARENT as i32);let row=&state.row;paint_popup_toolbar(hdc,rc,&format!("Entity Inspector - {} [{}]",row.name,row.uid));let profile_top=TOOLBAR_H+3;let col=(rc.right/3).max(190);SetTextColor(hdc,rgb(235,235,235));let hp_pct=if row.max_hp>0{format!("{:.0}%",(row.hp.max(0)as f64*100.0/row.max_hp as f64).clamp(0.0,100.0))}else{"?%".into()};let left=[format!("Name: {}",row.name),format!("Ability Score: {} (+{})",row.ability_score,row.illusion_break),format!("Profession: {}",profession_name(row.profession_id)),format!("ProfessionSpec: {}",if row.subprofession_name.is_empty(){"Unknown"}else{&row.subprofession_name}),format!("Status: {}",if row.is_dead{"Dead"}else{"Alive"})];let mid=[format!("HP: {} ({})",compact(row.hp as f64),hp_pct),format!("Max HP: {}",compact(row.max_hp as f64)),format!("Magic ATK: {}",detail_attr(row,ATTR_MAGIC_ATTACK)),format!("Defense: {}",detail_attr(row,ATTR_DEFENSE_POWER)),format!("Endurance: {}",detail_attr(row,ATTR_ENDURANCE))];let right=[format!("Crit: {}",format_attr_value(ATTR_CRIT,detail_attr(row,ATTR_CRIT))),format!("Haste: {}",format_attr_value(ATTR_HASTE,detail_attr(row,ATTR_HASTE))),format!("Luck: {}",format_attr_value(ATTR_LUCKY,detail_attr(row,ATTR_LUCKY))),format!("Mastery: {}",format_attr_value(ATTR_MASTERY,detail_attr(row,ATTR_MASTERY))),format!("Deaths: {}",row.deaths)];for(i,text)in left.iter().enumerate(){draw(hdc,text,RECT{left:5,top:profile_top+i as i32*20,right:col-5,bottom:profile_top+i as i32*20+20},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}for(i,text)in mid.iter().enumerate(){draw(hdc,text,RECT{left:col+5,top:profile_top+i as i32*20,right:col*2-5,bottom:profile_top+i as i32*20+20},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}for(i,text)in right.iter().enumerate(){draw(hdc,text,RECT{left:col*2+5,top:profile_top+i as i32*20,right:rc.right-5,bottom:profile_top+i as i32*20+20},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}let summary_top=profile_top+105;outline(hdc,RECT{left:3,top:summary_top,right:rc.right-3,bottom:summary_top+76},rgb(110,110,110),1);let skills=detail_skills(state);let total=detail_total(row,state.mode);let hits: u64=skills.iter().map(|s|s.hits).sum();let crits:u64=skills.iter().map(|s|s.crits).sum();let luckies:u64=skills.iter().map(|s|s.lucky_hits).sum();let crit_rate=if hits>0{crits as f64*100.0/hits as f64}else{0.0};let lucky_rate=if hits>0{luckies as f64*100.0/hits as f64}else{0.0};let avg=if hits>0{total as f64/hits as f64}else{0.0};let lines=[format!("Total {}: {}",detail_mode_label(state.mode),compact(total as f64)),format!("{}PS: {}",match state.mode{SortMode::Damage=>"D",SortMode::Heal=>"H",SortMode::Tank=>"DT"},compact(rate(total,state.encounter_ms))),format!("Total Hits: {}",hits),format!("Total Crit Rate: {:.1}%",crit_rate),format!("Total Lucky Rate: {:.1}%",lucky_rate),format!("Crits: {}   Lucky: {}",crits,luckies),format!("Average / Hit: {}",compact(avg)),format!("Encounter: {}",format_time(state.encounter_ms))];for(i,text)in lines.iter().enumerate(){let c=(i%4)as i32;let r=(i/4)as i32;let w=(rc.right-10)/4;draw(hdc,text,RECT{left:6+c*w,top:summary_top+4+r*31,right:6+(c+1)*w-4,bottom:summary_top+32+r*31},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}let tab_top=250;paint_tab(hdc,4,tab_top,67,"Damage",state.mode==SortMode::Damage);paint_tab(hdc,76,tab_top,70,"Healing",state.mode==SortMode::Heal);paint_tab(hdc,151,tab_top,70,"Taken",state.mode==SortMode::Tank);let head=281;fill(hdc,&RECT{left:3,top:head,right:rc.right-3,bottom:head+24},rgb(49,49,49));SetTextColor(hdc,rgb(225,225,225));let id_r=58;let right=rc.right-6;let share_l=right-62;let avg_l=share_l-92;let crit_l=avg_l-72;let hits_l=crit_l-60;let rate_l=hits_l-82;let amount_l=rate_l-98;draw(hdc,"ID",RECT{left:4,top:head,right:id_r,bottom:head+24},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);draw(hdc,"Skill Name",RECT{left:id_r+2,top:head,right:amount_l-5,bottom:head+24},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);draw(hdc,detail_mode_label(state.mode),RECT{left:amount_l,top:head,right:rate_l-4,bottom:head+24},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);draw(hdc,"Rate",RECT{left:rate_l,top:head,right:hits_l-4,bottom:head+24},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);draw(hdc,"Hits",RECT{left:hits_l,top:head,right:crit_l-4,bottom:head+24},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);draw(hdc,"Crit %",RECT{left:crit_l,top:head,right:avg_l-4,bottom:head+24},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);draw(hdc,"Avg / Hit",RECT{left:avg_l,top:head,right:share_l-4,bottom:head+24},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);draw(hdc,"Share",RECT{left:share_l,top:head,right,bottom:head+24},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);let top=head+27;let visible=((rc.bottom-top)/DETAIL_ROW_H).max(0)as usize;if skills.is_empty(){SetTextColor(hdc,rgb(150,150,150));draw(hdc,"No events recorded for this tab yet.",RECT{left:10,top:top+20,right:rc.right-10,bottom:top+55},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}else{for(i,skill)in skills.iter().skip(state.scroll).take(visible).enumerate(){let y=top+i as i32*DETAIL_ROW_H;let rr=RECT{left:3,top:y,right:rc.right-3,bottom:y+DETAIL_ROW_H-2};fill(hdc,&rr,if i%2==0{rgb(38,38,38)}else{rgb(34,34,34)});let amount=detail_skill_amount(skill,state.mode);let sr=if total>0{amount as f64*100.0/total as f64}else{0.0};let cr=if skill.hits>0{skill.crits as f64*100.0/skill.hits as f64}else{0.0};let av=if skill.hits>0{amount as f64/skill.hits as f64}else{0.0};SetTextColor(hdc,rgb(235,235,235));draw(hdc,&skill.skill_id.to_string(),RECT{left:4,top:y,right:id_r,bottom:y+DETAIL_ROW_H},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);draw(hdc,&skill.name,RECT{left:id_r+2,top:y,right:amount_l-5,bottom:y+DETAIL_ROW_H},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);draw(hdc,&compact(amount as f64),RECT{left:amount_l,top:y,right:rate_l-4,bottom:y+DETAIL_ROW_H},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);draw(hdc,&compact(rate(amount,state.encounter_ms)),RECT{left:rate_l,top:y,right:hits_l-4,bottom:y+DETAIL_ROW_H},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);draw(hdc,&skill.hits.to_string(),RECT{left:hits_l,top:y,right:crit_l-4,bottom:y+DETAIL_ROW_H},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);draw(hdc,&format!("{cr:.1}%"),RECT{left:crit_l,top:y,right:avg_l-4,bottom:y+DETAIL_ROW_H},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);draw(hdc,&compact(av),RECT{left:avg_l,top:y,right:share_l-4,bottom:y+DETAIL_ROW_H},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);draw(hdc,&format!("{sr:.1}%"),RECT{left:share_l,top:y,right,bottom:y+DETAIL_ROW_H},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}}EndPaint(hwnd,&ps);}
"#,
        "entity inspector tabs and tables",
    );

    fs::write(&overlay_path, overlay).expect("write follow-up overlay");
    println!("cargo:rerun-if-changed=build_v183_followup.rs");
}
