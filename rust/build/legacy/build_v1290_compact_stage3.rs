use std::{fs, path::Path};
fn replace_once(source:&mut String,from:&str,to:&str,label:&str){let count=source.matches(from).count();assert_eq!(count,1,"compact meter patch {label} expected one match, found {count}");*source=source.replacen(from,to,1);}
pub fn run(out:&Path){let path=out.join("feature_overlays_v170_fixed.rs");let mut source=fs::read_to_string(&path).expect("read generated feature overlay").replace("\r\n","\n");
replace_once(&mut source,r#"fn toolbar_action_rects_responsive(right:i32,scale:i32)->[(RECT,&'static str);8]{
    let tier=dps_layout_tier(right.max(1),scale);let button=dps_toolbar_button_w(right,scale);
    let(history,arrow,live,raid,copy,benchmark,reset,gap,label_raid,label_live,label_copy,label_benchmark,label_reset)=match tier{
        DpsLayoutTier::Comfortable=>(28,30,46,44,48,76,52,4,"Raid","Live","Copy","Benchmark","Reset"),
        DpsLayoutTier::Compact=>(24,24,30,28,28,28,28,3,"R","L","C","B","R"),
        DpsLayoutTier::Dense=>(22,20,28,26,26,26,26,2,"R","L","C","B","R"),
        DpsLayoutTier::Minimum=>(22,18,26,24,24,24,24,1,"R","L","C","B","R")
    };
    let mut x=right-button*3;
    let mut take=|w:i32,label:&'static str|{let r=RECT{left:x-w,top:5,right:x,bottom:29};x-=w+gap;(r,label)};
    let reset_r=take(reset,label_reset);let benchmark_r=take(benchmark,label_benchmark);let copy_r=take(copy,label_copy);
    let newer=take(arrow,">");let live_r=take(live,label_live);let older=take(arrow,"<");let raid_r=take(raid,label_raid);let history_r=take(history,"H");
    [history_r,raid_r,older,live_r,newer,copy_r,benchmark_r,reset_r]
}
"#,r#"fn toolbar_action_rects_responsive(right:i32,scale:i32)->[(RECT,&'static str);8]{
    let tier=dps_layout_tier(right.max(1),scale);let button=dps_toolbar_button_w(right,scale);
    let(history,arrow,live,raid,copy,benchmark,reset,gap,label_raid,label_live,label_copy,label_benchmark,label_reset)=match tier{
        DpsLayoutTier::Comfortable=>(28,30,46,44,48,76,52,4,"Raid","Live","Copy","Benchmark","Reset"),
        DpsLayoutTier::Compact=>(24,24,30,28,28,28,28,3,"R","L","C","B","R"),
        DpsLayoutTier::Dense=>(22,20,28,26,26,26,26,2,"R","L","C","B","R"),
        DpsLayoutTier::Minimum=>(22,18,26,24,24,24,24,1,"R","L","C","B","R")
    };
    let mut x=right-button*3;
    let mut take=|w:i32,label:&'static str|{let r=RECT{left:x-w,top:5,right:x,bottom:29};x-=w+gap;(r,label)};
    let reset_r=take(reset,label_reset);let benchmark_r=take(benchmark,label_benchmark);let copy_r=take(copy,label_copy);
    let newer=take(arrow,">");let live_r=take(live,label_live);let older=take(arrow,"<");let raid_r=take(raid,label_raid);let history_r=take(history,"H");
    [history_r,raid_r,older,live_r,newer,copy_r,benchmark_r,reset_r]
}

#[derive(Clone,Copy,Debug,Eq,PartialEq)]
enum ToolbarAction{History,Compact,Raid,Older,Live,Newer,Copy,Benchmark,Reset,More}
#[derive(Clone,Copy)]struct ToolbarItem{action:ToolbarAction,rect:RECT,label:&'static str}
fn toolbar_system_count(state:&State)->i32{if state.kind==Kind::Dps&&state.compact_mode{1}else{3}}
fn toolbar_items(state:&State,right:i32)->Vec<ToolbarItem>{
    if state.kind!=Kind::Dps{return Vec::new();}let scale=dps_layout_scale(state);let tier=dps_layout_tier(right.max(1),scale);let button=dps_toolbar_button_w(right,scale);let gap=match tier{DpsLayoutTier::Comfortable=>4,DpsLayoutTier::Compact=>3,DpsLayoutTier::Dense=>2,DpsLayoutTier::Minimum=>1};let mut x=right-button*toolbar_system_count(state);
    let mut out_rev:Vec<ToolbarItem>=Vec::new();let mut take=|action:ToolbarAction,w:i32,label:&'static str|{let r=RECT{left:x-w,top:5,right:x,bottom:29};x-=w+gap;out_rev.push(ToolbarItem{action,rect:r,label});};
    if state.compact_mode{take(ToolbarAction::More,24,"...");take(ToolbarAction::Compact,26,"C");take(ToolbarAction::Newer,20,">");take(ToolbarAction::Live,38,"Live");take(ToolbarAction::Older,20,"<");}
    else{let(history,compact,arrow,live,raid,copy,benchmark,reset,label_raid,label_live,label_copy,label_benchmark,label_reset)=match tier{DpsLayoutTier::Comfortable=>(28,26,30,46,44,48,76,52,"Raid","Live","Copy","Benchmark","Reset"),DpsLayoutTier::Compact=>(24,24,24,30,28,28,28,28,"R","L","Cp","B","R"),DpsLayoutTier::Dense=>(22,22,20,28,26,26,26,26,"R","L","Cp","B","R"),DpsLayoutTier::Minimum=>(22,22,18,26,24,24,24,24,"R","L","Cp","B","R")};take(ToolbarAction::Reset,reset,label_reset);take(ToolbarAction::Benchmark,benchmark,label_benchmark);take(ToolbarAction::Copy,copy,label_copy);take(ToolbarAction::Newer,arrow,">");take(ToolbarAction::Live,live,label_live);take(ToolbarAction::Older,arrow,"<");take(ToolbarAction::Raid,raid,label_raid);take(ToolbarAction::History,history,"H");take(ToolbarAction::Compact,compact,"C");}
    out_rev.reverse();out_rev
}
fn toolbar_action_active(state:&State,action:ToolbarAction)->bool{match action{ToolbarAction::Compact=>state.compact_mode,ToolbarAction::Raid=>raid_active(state),ToolbarAction::Live=>state.history_index.is_none(),_=>false}}
fn toolbar_action_help(action:ToolbarAction)->&'static str{match action{ToolbarAction::History=>"Open Encounter History",ToolbarAction::Compact=>"Compact Mode",ToolbarAction::Raid=>"Toggle 20-player Raid Mode",ToolbarAction::Older=>"Older encounter",ToolbarAction::Live=>"Return to live encounter",ToolbarAction::Newer=>"Newer encounter",ToolbarAction::Copy=>"Copy the current meter view as an image",ToolbarAction::Benchmark=>"Set up a timed benchmark",ToolbarAction::Reset=>"Reset the live encounter",ToolbarAction::More=>"More meter actions"}}
fn compact_mode_rect()->RECT{RECT{left:8,top:5,right:82,bottom:29}}
fn compact_mode_label(mode:SortMode)->&'static str{match mode{SortMode::Damage=>"Damage",SortMode::Heal=>"Heal",SortMode::Tank=>"Tank"}}
unsafe fn show_sort_mode_menu(hwnd:HWND,state:&mut State){let menu=CreatePopupMenu();if menu.is_null(){return;}for(id,label,mode)in[(1usize,"Damage",SortMode::Damage),(2,"Heal",SortMode::Heal),(3,"Tank",SortMode::Tank)]{AppendMenuW(menu,MF_STRING|if state.sort_mode==mode{0x8}else{0},id,wide(label).as_ptr());}let mut wr:RECT=std::mem::zeroed();GetWindowRect(hwnd,&mut wr);let cmd=TrackPopupMenu(menu,TPM_RETURNCMD|TPM_RIGHTBUTTON,wr.left+8,wr.top+TOOLBAR_H,0,hwnd,null());DestroyMenu(menu);state.sort_mode=match cmd{1=>SortMode::Damage,2=>SortMode::Heal,3=>SortMode::Tank,_=>state.sort_mode};state.scroll=0;}
unsafe fn show_meter_more_menu(hwnd:HWND,state:&mut State){let menu=CreatePopupMenu();if menu.is_null(){return;}AppendMenuW(menu,MF_STRING,10,wide("Encounter History").as_ptr());AppendMenuW(menu,MF_STRING,11,wide(if raid_active(state){"Exit Raid Mode"}else{"Raid Mode"}).as_ptr());AppendMenuW(menu,MF_SEPARATOR,0,null());AppendMenuW(menu,MF_STRING,12,wide("Copy as Image").as_ptr());AppendMenuW(menu,MF_STRING,13,wide("Benchmark").as_ptr());AppendMenuW(menu,MF_STRING,14,wide("Reset Encounter").as_ptr());AppendMenuW(menu,MF_SEPARATOR,0,null());AppendMenuW(menu,MF_STRING,15,wide("Settings").as_ptr());AppendMenuW(menu,MF_STRING,16,wide("Collapse").as_ptr());let mut wr:RECT=std::mem::zeroed();GetWindowRect(hwnd,&mut wr);let cmd=TrackPopupMenu(menu,TPM_RETURNCMD|TPM_RIGHTBUTTON,wr.right-170,wr.top+TOOLBAR_H,0,hwnd,null());DestroyMenu(menu);match cmd{10=>crate::encounter_archive::open_local(hwnd),11=>toggle_raid_mode(hwnd,state),12=>copy_view_image(hwnd,state),13=>crate::telemetry::benchmark_ui::show(hwnd),14=>{if state.history_index.is_none(){archive_live_snapshot(state);crate::telemetry::request_manual_reset();state.dps=DpsSnapshot::default();state.scroll=0;}else{state.history_index=None;state.scroll=0;}},15=>open_feature_settings(hwnd,state),16=>collapse(hwnd,state),_=>{}}}
unsafe fn dispatch_toolbar_action(hwnd:HWND,state:&mut State,action:ToolbarAction){match action{ToolbarAction::History=>crate::encounter_archive::open_local(hwnd),ToolbarAction::Compact=>toggle_compact_mode(hwnd,state),ToolbarAction::Raid=>toggle_raid_mode(hwnd,state),ToolbarAction::Older=>history_older(state),ToolbarAction::Live=>{state.history_index=None;state.scroll=0;},ToolbarAction::Newer=>history_newer(state),ToolbarAction::Copy=>copy_view_image(hwnd,state),ToolbarAction::Benchmark=>crate::telemetry::benchmark_ui::show(hwnd),ToolbarAction::Reset=>{if state.history_index.is_none(){archive_live_snapshot(state);crate::telemetry::request_manual_reset();state.dps=DpsSnapshot::default();state.scroll=0;}else{state.history_index=None;state.scroll=0;}},ToolbarAction::More=>show_meter_more_menu(hwnd,state)}}
"#,"semantic toolbar");
fs::write(path,source).expect("write compact meter generated overlay");}
