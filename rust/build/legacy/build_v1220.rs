use std::{env,fs,path::{Path,PathBuf}};

mod prior {
    include!("build_v1214_fix.rs");
    pub fn run(){main();}
}

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){
    let count=source.matches(from).count();
    assert_eq!(count,1,"v1.22.0 patch {label} expected one match, found {count}");
    *source=source.replacen(from,to,1);
}

fn replace_between(source:&mut String,start:&str,end:&str,replacement:&str,label:&str){
    let count=source.matches(start).count();
    assert_eq!(count,1,"v1.22.0 patch {label} start expected one match, found {count}");
    let begin=source.find(start).expect("v1.22.0 start anchor");
    let rel_end=source[begin..].find(end).unwrap_or_else(||panic!("v1.22.0 patch {label} end anchor missing"));
    source.replace_range(begin..begin+rel_end,replacement);
}

fn insert_fn_guard(source:&mut String,signature:&str,guard:&str,label:&str){
    let count=source.matches(signature).count();
    assert_eq!(count,1,"v1.22.0 patch {label} expected one function, found {count}");
    let start=source.find(signature).expect("function signature checked");
    let open_rel=source[start..].find('{').unwrap_or_else(||panic!("v1.22.0 patch {label} function body missing"));
    let at=start+open_rel+1;
    source.insert_str(at,guard);
}

fn patch_feature_overlay(out:&Path){
    let path=out.join("feature_overlays_v170_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read v1.21.4 generated overlays").replace("\r\n","\n");

    // RAID sits immediately to the left of the existing < LIVE > history cluster.
    replace_between(
        &mut source,
        "fn toolbar_action_rects_responsive(",
        "fn dps_identity(",
        r#"fn toolbar_action_rects_responsive(right:i32,scale:i32)->[(RECT,&'static str);6]{
    let tier=dps_layout_tier(right.max(1),scale);let button=dps_toolbar_button_w(right,scale);
    let(arrow,live,raid,image,reset,gap,label_image,label_reset)=match tier{
        DpsLayoutTier::Comfortable=>(30,46,44,116,52,12,"Copy as Image","Reset"),
        DpsLayoutTier::Compact=>(24,40,38,82,44,8,"Copy Image","Reset"),
        DpsLayoutTier::Dense=>(20,34,32,54,36,4,"Copy","Reset"),
        DpsLayoutTier::Minimum=>(18,30,28,38,28,2,"Img","R")
    };
    let mut x=right-button*3;
    let mut take=|w:i32,label:&'static str|{let r=RECT{left:x-w,top:5,right:x,bottom:29};x-=w;(r,label)};
    let reset_r=take(reset,label_reset);let image_r=take(image,label_image);let _=take(gap,"");
    let newer=take(arrow,">");let live_r=take(live,"LIVE");let older=take(arrow,"<");let raid_r=take(raid,"RAID");
    [raid_r,older,live_r,newer,image_r,reset_r]
}
"#,
        "raid toolbar action layout",
    );

    replace_once(
        &mut source,
        "match index{0=>history_older(state),1=>{state.history_index=None;state.scroll=0;},2=>history_newer(state),3=>copy_view_image(hwnd,state),4=>{if state.history_index.is_none(){archive_live_snapshot(state);crate::telemetry::request_manual_reset();state.dps=DpsSnapshot::default();state.scroll=0;}else{state.history_index=None;state.scroll=0;}},_=>{}}",
        "match index{0=>toggle_raid_mode(hwnd,state),1=>history_older(state),2=>{state.history_index=None;state.scroll=0;},3=>history_newer(state),4=>copy_view_image(hwnd,state),5=>{if state.history_index.is_none(){archive_live_snapshot(state);crate::telemetry::request_manual_reset();state.dps=DpsSnapshot::default();state.scroll=0;}else{state.history_index=None;state.scroll=0;}},_=>{}}",
        "raid toolbar click action",
    );

    replace_once(
        &mut source,
        "let active=index==1&&state.history_index.is_none();",
        "let active=(index==0&&raid_active(state))||(index==2&&state.history_index.is_none());",
        "raid and LIVE active states",
    );

    replace_once(
        &mut source,
        "match index{0=>\"Older encounter\",1=>\"Return to live encounter\",2=>\"Newer encounter\",3=>\"Copy the full DPS Meter as an image\",4=>\"Reset the live encounter\",_=>\"Combat action\"}",
        "match index{0=>\"Toggle 20-player Raid Mode\",1=>\"Older encounter\",2=>\"Return to live encounter\",3=>\"Newer encounter\",4=>\"Copy the full DPS Meter as an image\",5=>\"Reset the live encounter\",_=>\"Combat action\"}",
        "raid toolbar help",
    );

    // Keep the normal painter intact, then replace only its roster region in Raid Mode.
    replace_once(
        &mut source,
        "Kind::Dps=>paint_dps(hdc,rc,state)",
        "Kind::Dps=>paint_dps_with_raid(hdc,rc,state)",
        "raid DPS paint dispatcher",
    );

    // Whole-row inspection remains available on either raid column.
    replace_once(
        &mut source,
        "if state.kind==Kind::Dps{if let Some(row)=dps_row_at(hwnd,state,y){open_detail(hwnd,state,row);}}",
        "if state.kind==Kind::Dps{if let Some(row)=if raid_active(state){raid_row_at(hwnd,state,x,y)}else{dps_row_at(hwnd,state,y)}{open_detail(hwnd,state,row);}}",
        "raid row click mapping",
    );

    // The detached normal F/S bars are intentionally hidden while Raid Mode draws compact letters inline.
    insert_fn_guard(
        &mut source,
        "unsafe fn sync_consumable_popup(",
        "if state.kind==Kind::Dps&&raid_active(state){if !state.consumable_hwnd.is_null(){ShowWindow(state.consumable_hwnd,SW_HIDE);}return;}",
        "raid consumable popup suppression",
    );

    // Any normal persistence route (drag, resize, scale) must not overwrite the single-column bounds while raiding.
    insert_fn_guard(
        &mut source,
        "unsafe fn persist_overlay_rect(",
        "if state.kind==Kind::Dps&&raid_active(state){raid_store_rect(state,rect);return;}",
        "separate raid geometry persistence",
    );

    // Badge hit testing belongs to normal mode only; raid rows deliberately omit badges for density.
    replace_once(
        &mut source,
        "if state.kind!=Kind::Dps||state.collapsed||!state.features.read().map(|f|f.meter.show_imagines).unwrap_or(true){return None;}",
        "if state.kind!=Kind::Dps||state.collapsed||raid_active(state)||!state.features.read().map(|f|f.meter.show_imagines).unwrap_or(true){return None;}",
        "disable badge hover in raid mode",
    );

    source.push_str(r#"

// ---- v1.22.0 compact Raid Mode -------------------------------------------------
const RAID_ROWS_PER_COLUMN:usize=10;
const RAID_MAX_ROWS:usize=20;
const RAID_BASE_WIDTH:i32=1080;
const RAID_CENTER_GAP:i32=76;
const RAID_FS_YELLOW_MS:i64=120_000;
const RAID_FS_RED_MS:i64=30_000;

#[derive(Clone,Copy)]
struct RaidWindowState{active:bool,normal:RECT,raid:RECT,has_raid:bool}

std::thread_local!{
    static RAID_WINDOWS:std::cell::RefCell<std::collections::HashMap<isize,RaidWindowState>>=
        std::cell::RefCell::new(std::collections::HashMap::new());
}

fn raid_key(state:&State)->isize{state as *const State as isize}
fn raid_active(state:&State)->bool{RAID_WINDOWS.with(|items|items.borrow().get(&raid_key(state)).map(|v|v.active).unwrap_or(false))}
fn raid_layout_path(state:&State)->std::path::PathBuf{state.paths.root.join("dps-raid-layout.txt")}
fn raid_rect_valid(r:RECT)->bool{r.right-r.left>=300&&r.bottom-r.top>=180}
fn raid_load_rect(state:&State)->Option<RECT>{
    let text=std::fs::read_to_string(raid_layout_path(state)).ok()?;let values:Vec<i32>=text.split_whitespace().filter_map(|v|v.parse().ok()).collect();
    if values.len()!=4{return None;}let r=RECT{left:values[0],top:values[1],right:values[0].saturating_add(values[2]),bottom:values[1].saturating_add(values[3])};raid_rect_valid(r).then_some(r)
}
fn raid_save_rect_file(state:&State,r:RECT){
    if !raid_rect_valid(r){return;}let text=format!("{} {} {} {}\n",r.left,r.top,(r.right-r.left).max(1),(r.bottom-r.top).max(1));
    if let Err(err)=std::fs::write(raid_layout_path(state),text){crate::logging::write(format!("raid layout: save failed: {err}"));}
}
fn raid_store_rect(state:&State,r:RECT){
    if !raid_rect_valid(r){return;}RAID_WINDOWS.with(|items|{let mut items=items.borrow_mut();let key=raid_key(state);let entry=items.entry(key).or_insert(RaidWindowState{active:true,normal:r,raid:r,has_raid:true});entry.raid=r;entry.has_raid=true;});raid_save_rect_file(state,r);
}
fn raid_auto_rect(hwnd:HWND,state:&State,normal:RECT)->RECT{
    let scale=dps_layout_scale(state);let logical_h=dps_rows_top()+RAID_ROWS_PER_COLUMN as i32*dps_row_h(scale)+8;
    let target_w=scale_px(RAID_BASE_WIDTH,state.scale_percent).max(overlay_min_width(Kind::Dps,state.scale_percent));
    let target_h=scale_px(logical_h,state.scale_percent).max(overlay_min_height(Kind::Dps,state.scale_percent));
    let work=crate::ui::work_area(hwnd);let ax=preferred_axis_anchor(normal.left,normal.right,work.left,work.right);let ay=preferred_axis_anchor(normal.top,normal.bottom,work.top,work.bottom);
    anchored_scaled_rect(normal,work,target_w,target_h,ax,ay)
}
unsafe fn toggle_raid_mode(hwnd:HWND,state:&mut State){
    if state.kind!=Kind::Dps||state.collapsed{return;}let mut current:RECT=std::mem::zeroed();if GetWindowRect(hwnd,&mut current)==0{return;}
    let key=raid_key(state);let active=raid_active(state);
    if !active{
        // Persist the exact single-column layout before Raid Mode owns the window.
        persist_overlay_rect(state,current);
        let saved=RAID_WINDOWS.with(|items|items.borrow().get(&key).copied()).and_then(|v|v.has_raid.then_some(v.raid)).or_else(||raid_load_rect(state));
        let target=saved.filter(|r|raid_rect_valid(*r)).unwrap_or_else(||raid_auto_rect(hwnd,state,current));
        RAID_WINDOWS.with(|items|{items.borrow_mut().insert(key,RaidWindowState{active:true,normal:current,raid:target,has_raid:true});});
        state.expanded=target;state.scroll=0;SetWindowPos(hwnd,HWND_TOPMOST,target.left,target.top,(target.right-target.left).max(1),(target.bottom-target.top).max(1),SWP_NOACTIVATE);
    }else{
        raid_store_rect(state,current);
        let normal=RAID_WINDOWS.with(|items|items.borrow().get(&key).map(|v|v.normal)).unwrap_or(current);
        RAID_WINDOWS.with(|items|{if let Some(v)=items.borrow_mut().get_mut(&key){v.active=false;v.raid=current;v.has_raid=true;}});
        state.expanded=normal;state.scroll=0;SetWindowPos(hwnd,HWND_TOPMOST,normal.left,normal.top,(normal.right-normal.left).max(1),(normal.bottom-normal.top).max(1),SWP_NOACTIVATE);
    }
    sync_consumable_popup(hwnd,state);InvalidateRect(hwnd,null(),0);if !state.consumable_hwnd.is_null(){InvalidateRect(state.consumable_hwnd,null(),0);}
}

fn raid_consumable_color(status:Option<&ConsumableStatus>,now:i64)->u32{
    let Some(status)=active_consumable(status,now)else{return crate::ui_theme::MUTED;};
    if status.expires_unix_ms<=0{return rgb(72,226,116);}let left=status.expires_unix_ms.saturating_sub(now);
    if left<=0{return crate::ui_theme::MUTED;}if left<=RAID_FS_RED_MS{return crate::ui_theme::CRITICAL;}if left<=RAID_FS_YELLOW_MS{return rgb(245,190,55);}rgb(72,226,116)
}

unsafe fn paint_raid_fs(hdc:HDC,state:&State,row:Option<&&DpsRow>,x:i32,y:i32,w:i32,h:i32){
    let old=SelectObject(hdc,dps_primary_font(state));let now=now_ms();let(food,serum)=row.map(|r|(raid_consumable_color(r.food.as_ref(),now),raid_consumable_color(r.serum.as_ref(),now))).unwrap_or((crate::ui_theme::MUTED,crate::ui_theme::MUTED));
    let half=(w/2).max(1);SetTextColor(hdc,food);draw(hdc,"F",RECT{left:x,top:y,right:x+half,bottom:y+h},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);SetTextColor(hdc,serum);draw(hdc,"S",RECT{left:x+half,top:y,right:x+w,bottom:y+h},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);SelectObject(hdc,old);
}

unsafe fn paint_raid_player(hdc:HDC,state:&State,row:&DpsRow,rank:usize,r:RECT,leader:i64,snapshot:&DpsSnapshot,settings:&FeatureSettings){
    let bg=if row.is_dead{rgb(105,28,34)}else{spec_color(row)};fill(hdc,&r,bg);if row.is_local{outline(hdc,r,rgb(212,175,55),2);}let base=text_on(bg);let scale=dps_layout_scale(state);
    let rank_w=dps_adaptive_logical_px(24,scale);let total_w=dps_adaptive_logical_px(66,scale);let rate_w=dps_adaptive_logical_px(76,scale);let share_w=if dps_mode_share_enabled(settings,state.sort_mode){dps_adaptive_logical_px(48,scale)}else{0};let death_w=if settings.meter.show_deaths{dps_adaptive_logical_px(24,scale)}else{0};
    let mut right=r.right-4;let death_left=right-death_w;if death_w>0{right=death_left-2;}let share_left=right-share_w;if share_w>0{right=share_left-2;}let rate_left=right-rate_w;right=rate_left-2;let total_left=right-total_w;let name_right=(total_left-5).max(r.left+rank_w+30);
    SelectObject(hdc,dps_secondary_font(state));SetTextColor(hdc,base);draw(hdc,&rank.to_string(),RECT{left:r.left+3,top:r.top,right:r.left+rank_w,bottom:r.bottom-2},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);
    SelectObject(hdc,dps_primary_font(state));SetTextColor(hdc,if row.is_dead{rgb(255,120,120)}else{base});draw(hdc,&row.name,RECT{left:r.left+rank_w,top:r.top,right:name_right,bottom:r.bottom-2},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);
    let(total,rate)=mode_values(row,state.sort_mode,snapshot.encounter_ms,settings);SetTextColor(hdc,base);draw(hdc,&total,RECT{left:total_left,top:r.top,right:total_left+total_w,bottom:r.bottom-2},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);draw(hdc,&rate,RECT{left:rate_left,top:r.top,right:rate_left+rate_w,bottom:r.bottom-2},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);
    if share_w>0{if let Some((share,color))=active_share(row,state.sort_mode,settings){SetTextColor(hdc,color);draw(hdc,&format!("{share:.1}%"),RECT{left:share_left,top:r.top,right:share_left+share_w,bottom:r.bottom-2},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}}
    if death_w>0&&row.deaths>0{SetTextColor(hdc,crate::ui_theme::CRITICAL);draw(hdc,&format!("D{}",row.deaths),RECT{left:death_left,top:r.top,right:r.right-3,bottom:r.bottom-2},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}
    let metric=mode_metric(row,state.sort_mode);let bar=RECT{left:r.left,top:r.bottom-2,right:r.right,bottom:r.bottom};fill(hdc,&bar,rgb(20,24,29));if metric>0{let width=(r.right-r.left).max(0)as i64;let filled=(width.saturating_mul(metric.min(leader))/leader.max(1)).clamp(0,width)as i32;let color=match state.sort_mode{SortMode::Damage=>rgb(235,74,74),SortMode::Heal=>rgb(55,205,105),SortMode::Tank=>rgb(65,145,235)};fill(hdc,&RECT{left:r.left,top:r.bottom-2,right:r.left+filled,bottom:r.bottom},color);}
}

unsafe fn paint_raid_rows(hdc:HDC,rc:RECT,state:&State){
    let top=dps_rows_top();if rc.bottom<=top{return;}fill(hdc,&RECT{left:0,top,right:rc.right,bottom:rc.bottom},crate::ui_theme::BG);
    let rows=meter_rows(state);if rows.is_empty(){SetTextColor(hdc,crate::ui_theme::MUTED);draw(hdc,"Waiting for raid / combat data...",RECT{left:10,top:top+18,right:rc.right-10,bottom:top+58},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);return;}
    let settings=state.features.read().map(|v|v.clone()).unwrap_or_default();let snapshot=view_snapshot(state);let scale=dps_layout_scale(state);let row_h=dps_row_h(scale);let gap=dps_adaptive_logical_px(RAID_CENTER_GAP,scale).max(56);let mid=rc.right/2;let left=RECT{left:6,top,right:(mid-gap/2-4).max(160),bottom:rc.bottom};let right=RECT{left:(mid+gap/2+4).min(rc.right-160),top,right:rc.right-8,bottom:rc.bottom};let leader=rows.iter().take(RAID_MAX_ROWS).map(|r|mode_metric(r,state.sort_mode)).max().unwrap_or(1).max(1);
    fill(hdc,&RECT{left:mid,top:top+2,right:mid+1,bottom:(top+RAID_ROWS_PER_COLUMN as i32*row_h).min(rc.bottom)},crate::ui_theme::RAISED);
    for slot in 0..RAID_ROWS_PER_COLUMN{
        let y=top+slot as i32*row_h;if y>=rc.bottom{break;}let bottom=(y+row_h-2).min(rc.bottom);let li=slot;let ri=slot+RAID_ROWS_PER_COLUMN;
        if let Some(row)=rows.get(li){paint_raid_player(hdc,state,row,li+1,RECT{left:left.left,top:y,right:left.right,bottom},leader,snapshot,&settings);}
        if let Some(row)=rows.get(ri){paint_raid_player(hdc,state,row,ri+1,RECT{left:right.left,top:y,right:right.right,bottom},leader,snapshot,&settings);}
        let pair_w=((gap/2)-8).max(24);paint_raid_fs(hdc,state,rows.get(li),mid-gap/2+2,y,pair_w,bottom-y);paint_raid_fs(hdc,state,rows.get(ri),mid+6,y,pair_w,bottom-y);
    }
}

unsafe fn paint_dps_with_raid(hdc:HDC,rc:RECT,state:&State){paint_dps(hdc,rc,state);if raid_active(state){paint_raid_rows(hdc,rc,state);}}

unsafe fn raid_row_at(hwnd:HWND,state:&State,x:i32,y:i32)->Option<DpsRow>{
    let rc=logical_client_rect(hwnd,state.scale_percent);let top=dps_rows_top();if y<top||y>=rc.bottom{return None;}let scale=dps_layout_scale(state);let row_h=dps_row_h(scale).max(1);let slot=((y-top)/row_h)as usize;if slot>=RAID_ROWS_PER_COLUMN{return None;}let gap=dps_adaptive_logical_px(RAID_CENTER_GAP,scale).max(56);let mid=rc.right/2;let index=if x<mid-gap/2{slot}else if x>mid+gap/2{slot+RAID_ROWS_PER_COLUMN}else{return None;};let rows=meter_rows(state);rows.get(index).map(|row|(*row).clone())
}

#[cfg(test)]
mod v1220_raid_mode_tests{
    use super::*;
    fn status(left:i64,now:i64)->ConsumableStatus{ConsumableStatus{buff_id:1,name:"Buff".into(),expires_unix_ms:now+left,duration_ms:300_000}}
    #[test]fn raid_columns_are_ten_plus_ten(){assert_eq!(RAID_ROWS_PER_COLUMN,10);assert_eq!(RAID_MAX_ROWS,20);}
    #[test]fn raid_food_serum_turn_red_at_thirty_seconds(){let now=100_000;assert_eq!(raid_consumable_color(Some(&status(30_000,now)),now),crate::ui_theme::CRITICAL);}
    #[test]fn raid_food_serum_are_yellow_before_red_window(){let now=100_000;assert_eq!(raid_consumable_color(Some(&status(90_000,now)),now),rgb(245,190,55));}
    #[test]fn raid_food_serum_are_green_when_healthy(){let now=100_000;assert_eq!(raid_consumable_color(Some(&status(180_000,now)),now),rgb(72,226,116));}
}
"#);

    fs::write(path,source).expect("write v1.22.0 raid-mode overlay patch");
}

fn main(){
    prior::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_feature_overlay(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1220.rs");
}
