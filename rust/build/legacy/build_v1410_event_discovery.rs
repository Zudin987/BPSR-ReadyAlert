use std::{env, fs, path::{Path, PathBuf}, process::Command};

mod previous {
    include!("build_v1409_archive_attribute_before_save.rs");
    pub fn run() { main(); }
}

const ZDPS_COMMIT: &str = "cfeb58c0acc85bc17181b413b9e50b0b26c15c5d";
const DISCOVERY_LIMIT: usize = 192;

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count=source.matches(from).count();
    assert_eq!(count,1,"v1410 {label} expected one anchor, found {count}");
    *source=source.replacen(from,to,1);
}

fn replace_span(source:&mut String,start:&str,end:&str,replacement:&str,label:&str){
    let begin=source.find(start).unwrap_or_else(||panic!("v1410 {label}: missing start"));
    let rel=source[begin..].find(end).unwrap_or_else(||panic!("v1410 {label}: missing end"));
    source.replace_range(begin..begin+rel,replacement);
}

fn ps_quote(path:&Path)->String{path.to_string_lossy().replace('\'',"''")}

fn download_catalog(out:&Path,file:&str)->PathBuf{
    let path=out.join(file.replace('/',"_"));
    if !cfg!(windows){fs::write(&path,"{}").expect("write non-Windows discovery catalog");return path;}
    let url=format!("https://raw.githubusercontent.com/Blue-Protocol-Source/BPSR-ZDPS/{ZDPS_COMMIT}/BPSR-ZDPS/Data/{file}");
    let script=format!("$ErrorActionPreference='Stop'; Invoke-WebRequest -UseBasicParsing -Uri '{url}' -OutFile '{}'",ps_quote(&path));
    let status=Command::new("powershell").args(["-NoProfile","-ExecutionPolicy","Bypass","-Command",&script]).status()
        .unwrap_or_else(|err|panic!("launch PowerShell for {file}: {err}"));
    assert!(status.success(),"failed to download pinned ZDPS {file}");
    path
}

fn names(path:&Path,min_windows:usize)->Vec<(i32,String)>{
    let text=fs::read_to_string(path).expect("read discovery name catalog");
    let value:serde_json::Value=serde_json::from_str(&text).expect("parse discovery name catalog");
    let mut rows:Vec<_>=value.as_object().into_iter().flatten().filter_map(|(id,data)|{
        let id=id.parse::<i32>().ok()?;if id<=0{return None}
        let name=data.get("Name")?.as_str()?.trim();
        if name.is_empty(){return None}
        Some((id,name.chars().take(80).collect::<String>()))
    }).collect();
    rows.sort_by_key(|x|x.0);rows.dedup_by_key(|x|x.0);
    if cfg!(windows){assert!(rows.len()>=min_windows,"unexpectedly small ZDPS discovery catalog: {}",rows.len());}
    rows
}

fn name_fn(name:&str,rows:&[(i32,String)],fallback:&[(i32,&str)])->String{
    let mut merged=std::collections::BTreeMap::<i32,String>::new();
    for (id,label) in fallback{merged.insert(*id,(*label).into());}
    for (id,label) in rows{merged.insert(*id,label.clone());}
    let mut out=format!("fn {name}(id:i32)->Option<&'static str>{{match id{{\n");
    for (id,label) in merged {
        let q=serde_json::to_string(&label).expect("quote discovery label");
        out.push_str(&format!("        {id} => Some({q}),\n"));
    }
    out.push_str("        _ => None,\n    }\n}\n");
    out
}

fn patch_tracker(out:&Path){
    let skill_path=out.join("zdps_SkillOverrides.en.json");
    let skills=if skill_path.is_file(){names(&skill_path,400)}else{names(&download_catalog(out,"SkillOverrides.en.json"),400)};
    let buffs=names(&download_catalog(out,"BuffOverrides.en.json"),500);
    let skill_fn=name_fn("discovery_skill_name",&skills,&[]);
    let buff_fn=name_fn("discovery_buff_name",&buffs,&[
        (55_226,"DeterShot"),(2_110_049,"Mechanical Failure"),(2_110_050,"Element Stasis"),
        (2_110_055,"Exhausted: Flame Devour"),(2_110_056,"Time Stasis"),(2_110_065,"Fiery Battle Will"),
    ]);

    let path=out.join("event_tracker_v1321_generated.rs");
    let mut source=fs::read_to_string(&path).expect("read generated event tracker").replace("\r\n","\n");

    replace_once(&mut source,
        "#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]\n#[serde(rename_all = \"snake_case\")]\npub enum TrackerKind {",
        "#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]\n#[serde(rename_all = \"snake_case\")]\npub enum TrackerKind {",
        "hashable tracker kind");

    replace_once(&mut source,
        "#[derive(Clone, Debug, Default)]\nstruct RuleState {",
        &format!(r#"#[derive(Clone, Debug)]
pub struct DiscoveryRow {{
    pub kind: TrackerKind,
    pub event_id: i32,
    pub name: String,
    pub detail: String,
    pub count: u64,
    pub last_seen_unix_ms: i64,
    pub suggested_scope: TrackerScope,
}}

#[derive(Clone, Debug)]
struct DiscoveryState {{
    count: u64,
    last_seen_unix_ms: i64,
    detail: String,
    suggested_scope: TrackerScope,
}}
impl Default for DiscoveryState {{
    fn default()->Self{{Self{{count:0,last_seen_unix_ms:0,detail:String::new(),suggested_scope:TrackerScope::Any}}}}
}}

const MAX_DISCOVERY_ROWS: usize = {DISCOVERY_LIMIT};

{skill_fn}
{buff_fn}
fn discovery_event_name(kind:TrackerKind,id:i32)->Option<&'static str>{{
    match kind {{
        TrackerKind::Skill=>discovery_skill_name(id),
        TrackerKind::Buff=>discovery_buff_name(id),
        TrackerKind::Attribute=>None,
    }}
}}

#[derive(Clone, Debug, Default)]
struct RuleState {{"#),
        "discovery types and names");

    replace_once(&mut source,
        "    rule_states: HashMap<u32, RuleState>,",
        "    rule_states: HashMap<u32, RuleState>,\n    discovery_active: bool,\n    discovery: HashMap<(TrackerKind, i32), DiscoveryState>,",
        "discovery runtime fields");

    replace_once(&mut source,
        "impl RuntimeState {\n    fn skill(&mut self, settings: &TrackerSettings, skill_id: i32, source: i64, target: i64, now: i64) {",
        r#"impl RuntimeState {
    fn discover_event(&mut self, kind:TrackerKind, event_id:i32, source:i64, target:i64, now:i64) {
        if !self.discovery_active || event_id<=0 { return; }
        let is_self=self.local_uid>0&&(source==self.local_uid||target==self.local_uid);
        let is_party=is_self||(source>0&&self.party_uids.contains(&source))||(target>0&&self.party_uids.contains(&target));
        let suggested_scope=if is_self{TrackerScope::SelfOnly}else if is_party{TrackerScope::Party}else{TrackerScope::Any};
        let detail=match (source>0,target>0) {
            (true,true)=>format!("{} -> {}",scope_uid_label(source,self),scope_uid_label(target,self)),
            (true,false)=>format!("from {}",scope_uid_label(source,self)),
            (false,true)=>format!("on {}",scope_uid_label(target,self)),
            _=>"observed".into(),
        };
        if !self.discovery.contains_key(&(kind,event_id)) && self.discovery.len()>=MAX_DISCOVERY_ROWS {
            if let Some(oldest)=self.discovery.iter().min_by_key(|(_,v)|v.last_seen_unix_ms).map(|(k,_)|*k){self.discovery.remove(&oldest);}
        }
        let row=self.discovery.entry((kind,event_id)).or_default();
        row.count=row.count.saturating_add(1);row.last_seen_unix_ms=now;row.detail=detail;row.suggested_scope=suggested_scope;
    }

    fn discovery_rows(&self)->Vec<DiscoveryRow>{
        let mut rows:Vec<_>=self.discovery.iter().map(|(&(kind,event_id),state)|DiscoveryRow{
            kind,event_id,name:discovery_event_name(kind,event_id).unwrap_or("").to_string(),detail:state.detail.clone(),
            count:state.count,last_seen_unix_ms:state.last_seen_unix_ms,suggested_scope:state.suggested_scope,
        }).collect();
        rows.sort_by(|a,b|b.last_seen_unix_ms.cmp(&a.last_seen_unix_ms).then_with(||a.event_id.cmp(&b.event_id)));
        rows
    }

    fn skill(&mut self, settings: &TrackerSettings, skill_id: i32, source: i64, target: i64, now: i64) {"#,
        "discovery runtime methods");

    replace_once(&mut source,
        "        if !settings.enabled || skill_id <= 0 { return; }",
        "        if skill_id <= 0 { return; }\n        self.discover_event(TrackerKind::Skill,skill_id,source,target,now);\n        if !settings.enabled { return; }",
        "skill discovery");

    replace_once(&mut source,
        "        if host == 0 || instance_id == 0 { return; }\n        let key = (host, instance_id);",
        "        if host == 0 || instance_id == 0 { return; }\n        if !removed && base_id>0 { self.discover_event(TrackerKind::Buff,base_id,0,target_uid,now); }\n        let key = (host, instance_id);",
        "buff discovery");

    replace_once(&mut source,
        "pub fn current_settings() -> TrackerSettings {\n    RUNTIME.get().and_then(|r| r.settings.read().ok().map(|s| s.clone())).unwrap_or_default()\n}",
        r#"pub fn current_settings() -> TrackerSettings {
    RUNTIME.get().and_then(|r| r.settings.read().ok().map(|s| s.clone())).unwrap_or_default()
}
pub fn set_discovery_active(active:bool){
    if let Some(r)=RUNTIME.get(){if let Ok(mut s)=r.state.lock(){if active&&!s.discovery_active{s.discovery.clear();}s.discovery_active=active;}}
}
pub fn discovery_active()->bool{
    RUNTIME.get().and_then(|r|r.state.lock().ok().map(|s|s.discovery_active)).unwrap_or(false)
}
pub fn clear_discovery(){if let Some(r)=RUNTIME.get(){if let Ok(mut s)=r.state.lock(){s.discovery.clear();}}}
pub fn discovery_rows()->Vec<DiscoveryRow>{
    RUNTIME.get().and_then(|r|r.state.lock().ok().map(|s|s.discovery_rows())).unwrap_or_default()
}"#,
        "public discovery api");

    source.push_str(r#"

#[cfg(test)]
mod v1410_event_discovery_tests {
    use super::*;

    #[test]
    fn discovery_works_even_when_tracker_output_is_disabled() {
        let mut state=RuntimeState::default();state.local_uid=42;state.discovery_active=true;
        let settings=TrackerSettings{enabled:false,..TrackerSettings::default()};
        state.skill(&settings,1241,42,99,1_000);
        state.skill(&settings,1241,42,99,1_010);
        state.buff(&settings,123,42,7,2_110_056,false,61_000,1_020);
        let rows=state.discovery_rows();
        assert_eq!(rows.len(),2);
        let skill=rows.iter().find(|r|r.kind==TrackerKind::Skill).unwrap();
        assert_eq!(skill.event_id,1241);assert_eq!(skill.count,2);assert_eq!(skill.suggested_scope,TrackerScope::SelfOnly);
        let buff=rows.iter().find(|r|r.kind==TrackerKind::Buff).unwrap();
        assert_eq!(buff.event_id,2_110_056);assert_eq!(buff.suggested_scope,TrackerScope::SelfOnly);
    }

    #[test]
    fn discovery_is_bounded_and_newest_first() {
        let mut state=RuntimeState::default();state.discovery_active=true;
        let settings=TrackerSettings{enabled:false,..TrackerSettings::default()};
        for id in 1..=(MAX_DISCOVERY_ROWS as i32+10){state.skill(&settings,id,0,0,id as i64);}
        let rows=state.discovery_rows();
        assert_eq!(rows.len(),MAX_DISCOVERY_ROWS);
        assert!(rows.first().unwrap().last_seen_unix_ms>=rows.last().unwrap().last_seen_unix_ms);
    }
}
"#);
    fs::write(&path,source).expect("write discovery event tracker");
}

fn patch_ui(out:&Path){
    let path=out.join("event_tracker_ui_v1160_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read generated event tracker UI").replace("\r\n","\n");
    replace_once(&mut source,"const WIDTH: i32 = 760;","const WIDTH: i32 = 920;","tracker window width");
    replace_once(&mut source,"const HEIGHT: i32 = 600;","const HEIGHT: i32 = 780;","tracker window height");
    replace_once(&mut source,
        "const ID_CLOSE: i32 = 6199;",
        r#"const ID_CLOSE: i32 = 6199;
const ID_DISCOVERY_TOGGLE:i32=6130;
const ID_DISCOVERY_CLEAR:i32=6131;
const ID_DISCOVERY_LIST:i32=6132;
const ID_DISCOVERY_ADD:i32=6133;
const ID_DISCOVERY_STATUS:i32=6134;
const DISCOVERY_TIMER_ID:usize=14_110;
const LBN_DBLCLK_LOCAL:u16=2;"#,
        "discovery control ids");
    replace_once(&mut source,
        "extern \"system\" { fn EnableWindow(hwnd: HWND, enable: i32) -> i32; }",
        "extern \"system\" { fn EnableWindow(hwnd: HWND, enable: i32) -> i32; fn SetTimer(hwnd:HWND,id:usize,elapse:u32,timer:Option<unsafe extern \"system\" fn(HWND,u32,usize,u32)>)->usize; }",
        "discovery timer binding");
    replace_once(&mut source,
        "    selected: Option<usize>,",
        "    selected: Option<usize>,\n    discovered: Vec<event_tracker::DiscoveryRow>,",
        "UI discovery state");
    replace_once(&mut source,
        "        selected: None,",
        "        selected: None,\n        discovered: Vec::new(),",
        "UI discovery init");
    replace_once(&mut source,
        "        WM_COMMAND => {",
        "        0x0113 => { if !ptr.is_null() && wparam as usize==DISCOVERY_TIMER_ID { refresh_discovery(hwnd,&mut *ptr); } 0 }\n        WM_COMMAND => {",
        "UI discovery timer message");

    let build_controls=r#"unsafe fn build_controls(hwnd: HWND, state: &mut UiState) {
    create_static(hwnd,"Event Tracker",18,14,840,26);
    create_static(hwnd,"Easy setup: start listening, trigger the buff or combat skill in game, then track the event that appears.",18,42,850,24);
    create_checkbox(hwnd,ID_GLOBAL_ENABLED,"Enable tracked events in Dungeon Mechanics",18,76,310);
    create_static(hwnd,"Max visible rows",344,80,120,20);create_edit(hwnd,ID_MAX_VISIBLE,470,76,54,26);

    create_static(hwnd,"FIND AN EVENT AUTOMATICALLY",18,116,340,20);
    create_button(hwnd,ID_DISCOVERY_TOGGLE,"Start listening",18,142,130,30);
    create_button(hwnd,ID_DISCOVERY_CLEAR,"Clear results",156,142,112,30);
    create_control(hwnd,"STATIC","Not listening",ID_DISCOVERY_STATUS,282,146,570,22,WS_CHILD|WS_VISIBLE|0x80,0);
    create_listbox(hwnd,ID_DISCOVERY_LIST,18,180,834,176);SendMessageW(GetDlgItem(hwnd,ID_DISCOVERY_LIST),0x0194,1100,0);
    create_button(hwnd,ID_DISCOVERY_ADD,"Track selected",18,364,130,30);
    create_static(hwnd,"Newest events stay at the top. Known names come from the pinned ZDPS English catalog; unknown or future IDs stay visible as numbers.",160,368,692,34);

    create_static(hwnd,"TRACKED RULES",18,414,390,20);
    create_listbox(hwnd,ID_RULE_LIST,18,440,386,190);SendMessageW(GetDlgItem(hwnd,ID_RULE_LIST),0x0194,900,0);
    create_button(hwnd,ID_ADD,"Add manually",18,638,116,30);create_button(hwnd,ID_REMOVE,"Remove",142,638,96,30);

    let x=432;
    create_static(hwnd,"RULE DETAILS",x,414,390,20);
    create_checkbox(hwnd,ID_RULE_ENABLED,"Rule enabled",x,440,180);
    create_static(hwnd,"Type",x,474,110,20);create_combo(hwnd,ID_KIND,x+118,468,176,140);
    create_static(hwnd,"Numeric ID",x,510,110,20);create_edit(hwnd,ID_EVENT_ID,x+118,504,176,26);
    create_static(hwnd,"Label",x,546,110,20);create_edit(hwnd,ID_LABEL,x+118,540,278,26);
    create_static(hwnd,"Scope",x,582,110,20);create_combo(hwnd,ID_SCOPE,x+118,576,176,140);
    create_static(hwnd,"Display seconds",x,618,110,20);create_edit(hwnd,ID_HOLD,x+118,612,60,26);
    create_static(hwnd,"Skill / attribute only (1-30)",x+186,618,210,20);
    create_button(hwnd,ID_APPLY,"Apply",664,680,88,32);create_button(hwnd,ID_CLOSE,"Close",764,680,88,32);
    create_static(hwnd,"Buff timers use the game's timer. Skill/attribute display time is only how long the row stays visible; it is not a cooldown.",18,688,620,42);

    set_combo_items(hwnd,ID_KIND,&["Buff ID","Skill ID","Attribute ID"],0);
    set_combo_items(hwnd,ID_SCOPE,&["Any","Self","Party (includes self)"],0);
    set_check(hwnd,ID_GLOBAL_ENABLED,state.working.enabled);set_text(hwnd,ID_MAX_VISIBLE,&state.working.max_visible.to_string());
    for id in [ID_EVENT_ID,ID_MAX_VISIBLE,ID_HOLD]{SendMessageW(GetDlgItem(hwnd,id),0x00c5,10,0);}SendMessageW(GetDlgItem(hwnd,ID_LABEL),0x00c5,80,0);
    state.selected=if state.working.rules.is_empty(){None}else{Some(0)};refresh_list(hwnd,state);load_selected(hwnd,state);refresh_discovery(hwnd,state);
    SetTimer(hwnd,DISCOVERY_TIMER_ID,350,None);
}

"#;
    replace_span(&mut source,"unsafe fn build_controls(","unsafe fn handle_command(",build_controls,"build controls");

    let handle_and_helpers=r#"unsafe fn handle_command(hwnd: HWND, state: &mut UiState, id: i32, code: u16) {
    if id==ID_CLOSE || id==2 { close_form(hwnd,state); return; }
    if id==ID_DISCOVERY_LIST && (code==LBN_SELCHANGE||code==LBN_DBLCLK_LOCAL) {
        EnableWindow(GetDlgItem(hwnd,ID_DISCOVERY_ADD),(list_sel(hwnd,ID_DISCOVERY_LIST)>=0) as i32);
        if code==LBN_DBLCLK_LOCAL { add_discovered_rule(hwnd,state); }
        return;
    }
    if id==ID_RULE_LIST && code==LBN_SELCHANGE {
        let selected=list_sel(hwnd,ID_RULE_LIST);
        if !save_editor(hwnd,state) { if let Some(i)=state.selected {SendMessageW(GetDlgItem(hwnd,ID_RULE_LIST),LB_SETCURSEL,i,0);} return; }
        state.selected=(selected>=0).then_some(selected as usize).filter(|i|*i<state.working.rules.len());
        refresh_list(hwnd,state);load_selected(hwnd,state);return;
    }
    match id {
        ID_DISCOVERY_TOGGLE=>{
            let next=!event_tracker::discovery_active();
            event_tracker::set_discovery_active(next);
            refresh_discovery(hwnd,state);
        }
        ID_DISCOVERY_CLEAR=>{event_tracker::clear_discovery();refresh_discovery(hwnd,state);}
        ID_DISCOVERY_ADD=>add_discovered_rule(hwnd,state),
        ID_ADD=>{
            if state.working.rules.len()>=64||!save_editor(hwnd,state){return;}
            let rule_id=state.working.next_rule_id();
            state.working.rules.push(TrackerRule{rule_id,..TrackerRule::default()});state.selected=Some(state.working.rules.len()-1);
            refresh_list(hwnd,state);load_selected(hwnd,state);crate::ui::SetFocus(GetDlgItem(hwnd,ID_EVENT_ID));state.form.reveal_focus(hwnd);
        }
        ID_REMOVE=>{
            if let Some(index)=state.selected.filter(|i|*i<state.working.rules.len()){
                state.working.rules.remove(index);state.selected=if state.working.rules.is_empty(){None}else{Some(index.min(state.working.rules.len()-1))};
                refresh_list(hwnd,state);load_selected(hwnd,state);refresh_discovery(hwnd,state);
            }
        }
        ID_KIND=>{EnableWindow(GetDlgItem(hwnd,ID_HOLD),(combo_sel(hwnd,ID_KIND)!=0) as i32);}
        ID_APPLY|1=>{apply_settings(hwnd,state);}
        _=>{}
    }
}

unsafe fn refresh_discovery(hwnd:HWND,state:&mut UiState){
    let selected_key={let i=list_sel(hwnd,ID_DISCOVERY_LIST);if i>=0{state.discovered.get(i as usize).map(|r|(r.kind,r.event_id))}else{None}};
    state.discovered=event_tracker::discovery_rows();
    let list=GetDlgItem(hwnd,ID_DISCOVERY_LIST);SendMessageW(list,LB_RESETCONTENT,0,0);
    for row in &state.discovered{
        let kind=match row.kind{TrackerKind::Buff=>"BUFF",TrackerKind::Skill=>"SKILL",TrackerKind::Attribute=>"ATTR"};
        let name=if row.name.trim().is_empty(){format!("{} {}",row.kind.label(),row.event_id)}else{row.name.clone()};
        let tracked=state.working.rules.iter().any(|r|r.kind==row.kind&&r.event_id==row.event_id);
        let text=format!("[{kind}] {name}   |   ID {}   |   {}x   |   {}{}",row.event_id,row.count,row.detail,if tracked{"   |   TRACKED"}else{""});
        let w=wide(&text);SendMessageW(list,LB_ADDSTRING,0,w.as_ptr() as isize);
    }
    let selected=selected_key.and_then(|key|state.discovered.iter().position(|r|(r.kind,r.event_id)==key)).or_else(||(!state.discovered.is_empty()).then_some(0));
    if let Some(i)=selected{SendMessageW(list,LB_SETCURSEL,i,0);}
    let active=event_tracker::discovery_active();
    SetWindowTextW(GetDlgItem(hwnd,ID_DISCOVERY_TOGGLE),wide(if active{"Stop listening"}else{"Start listening"}).as_ptr());
    let status=if active{if state.discovered.is_empty(){"Listening - trigger the event in game now.".to_string()}else{format!("Listening - {} unique event{} found.",state.discovered.len(),if state.discovered.len()==1{""}else{"s"})}}else if state.discovered.is_empty(){"Not listening".into()}else{format!("Paused - {} result{} kept.",state.discovered.len(),if state.discovered.len()==1{""}else{"s"})};
    SetWindowTextW(GetDlgItem(hwnd,ID_DISCOVERY_STATUS),wide(&status).as_ptr());
    EnableWindow(GetDlgItem(hwnd,ID_DISCOVERY_ADD),selected.is_some() as i32);
    EnableWindow(GetDlgItem(hwnd,ID_DISCOVERY_CLEAR),(!state.discovered.is_empty()) as i32);
}

unsafe fn add_discovered_rule(hwnd:HWND,state:&mut UiState){
    let selected=list_sel(hwnd,ID_DISCOVERY_LIST);if selected<0{return;}
    let Some(found)=state.discovered.get(selected as usize).cloned()else{return;};
    if let Some(index)=state.working.rules.iter().position(|r|r.kind==found.kind&&r.event_id==found.event_id){
        state.selected=Some(index);refresh_list(hwnd,state);load_selected(hwnd,state);return;
    }
    if state.working.rules.len()>=64{message_error(hwnd,"Event Tracker already has the maximum of 64 rules.");return;}
    if !save_editor(hwnd,state){return;}
    let rule_id=state.working.next_rule_id();
    let label=if found.name.trim().is_empty(){String::new()}else{found.name};
    state.working.rules.push(TrackerRule{rule_id,enabled:true,kind:found.kind,event_id:found.event_id,label,scope:found.suggested_scope,hold_seconds:6});
    state.selected=Some(state.working.rules.len()-1);refresh_list(hwnd,state);load_selected(hwnd,state);refresh_discovery(hwnd,state);
}

"#;
    replace_span(&mut source,"unsafe fn handle_command(","unsafe fn apply_settings(",handle_and_helpers,"commands and discovery helpers");

    replace_once(&mut source,"    DestroyWindow(hwnd);","    event_tracker::set_discovery_active(false);\n    DestroyWindow(hwnd);","stop discovery on close");

    source.push_str(r#"
#[cfg(test)]
mod v1410_discovery_ui_contract_tests {
    use super::*;
    #[test]fn beginner_discovery_controls_are_present(){
        assert_eq!(DISCOVERY_TIMER_ID,14_110);assert_ne!(ID_DISCOVERY_ADD,ID_ADD);
    }
}
"#);
    fs::write(&path,source).expect("write beginner event tracker UI");
}

fn main(){
    previous::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_tracker(&out);patch_ui(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1410_event_discovery.rs");
}
