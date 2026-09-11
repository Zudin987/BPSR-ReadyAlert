use std::{env,fs,path::PathBuf};

mod base {
    include!("build_v1214_fix.rs");
    pub fn run(){main();}
}

mod raid {
    include!("build_v1220.rs");
    pub fn apply(out:&std::path::Path){patch_feature_overlay(out);}
}

fn insert_fn_guard(source:&mut String,signature:&str,guard:&str,label:&str){
    let count=source.matches(signature).count();
    assert_eq!(count,1,"v1.22.0 fix {label} expected one function, found {count}");
    let start=source.find(signature).expect("function signature checked");
    let open_rel=source[start..].find('{').unwrap_or_else(||panic!("v1.22.0 fix {label} function body missing"));
    source.insert_str(start+open_rel+1,guard);
}

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){
    let count=source.matches(from).count();
    assert_eq!(count,1,"v1.22.0 fix {label} expected one match, found {count}");
    *source=source.replacen(from,to,1);
}

fn replace_between(source:&mut String,start:&str,end:&str,replacement:&str,label:&str){
    let count=source.matches(start).count();
    assert_eq!(count,1,"v1.22.1 fix {label} start expected one match, found {count}");
    let begin=source.find(start).expect("v1.22.1 start anchor");
    let rel_end=source[begin..].find(end).unwrap_or_else(||panic!("v1.22.1 fix {label} end anchor missing"));
    source.replace_range(begin..begin+rel_end,replacement);
}

fn main(){
    base::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let overlay=out.join("feature_overlays_v170_fixed.rs");

    // v1.21.x evolved hover_badge_at, so the exact historical condition used by
    // build_v1220 may no longer occur. Give that one legacy replacement a harmless
    // comment anchor, then apply the real behavior below by function signature.
    let legacy_hover="if state.kind!=Kind::Dps||state.collapsed||!state.features.read().map(|f|f.meter.show_imagines).unwrap_or(true){return None;}";
    let mut source=fs::read_to_string(&overlay).expect("read v1.21.4 generated overlays").replace("\r\n","\n");
    let count=source.matches(legacy_hover).count();
    assert!(count<=1,"v1.22.0 fix hover compatibility anchor unexpectedly matched {count} times");
    if count==0{
        source.push_str("\n/* v1.22.0 legacy hover anchor: ");
        source.push_str(legacy_hover);
        source.push_str(" */\n");
        fs::write(&overlay,&source).expect("write v1.22.0 hover compatibility anchor");
    }

    raid::apply(&out);

    let mut source=fs::read_to_string(&overlay).expect("read v1.22.0 raid generated overlays").replace("\r\n","\n");
    insert_fn_guard(
        &mut source,
        "unsafe fn hover_badge_at(",
        "if raid_active(state){return None;}",
        "raid badge hover guard",
    );

    // The current generated overlay does not import SW_HIDE. ShowWindow uses 0 for
    // SW_HIDE, so keep the generated patch independent of another import rewrite.
    replace_once(
        &mut source,
        "ShowWindow(state.consumable_hwnd,SW_HIDE);",
        "ShowWindow(state.consumable_hwnd,0);",
        "raid consumable hide constant",
    );

    // ui::work_area is unsafe in the current UI helper API. Anchor this rewrite to
    // raid_auto_rect's target-height calculation because older overlay helpers also
    // call work_area and must remain untouched.
    replace_once(
        &mut source,
        "let target_h=scale_px(logical_h,state.scale_percent).max(overlay_min_height(Kind::Dps,state.scale_percent));\n    let work=crate::ui::work_area(hwnd);",
        "let target_h=scale_px(logical_h,state.scale_percent).max(overlay_min_height(Kind::Dps,state.scale_percent));\n    let work=unsafe{crate::ui::work_area(hwnd)};",
        "raid work area unsafe call",
    );

    // mode_values was removed by the newer DPS presentation patches. Recreate the
    // two compact values from the same row metrics so Raid Mode stays compatible
    // with the current generated source instead of calling the stale helper.
    replace_once(
        &mut source,
        "let(total,rate)=mode_values(row,state.sort_mode,snapshot.encounter_ms,settings);",
        "let total_value=match state.sort_mode{SortMode::Damage=>row.damage,SortMode::Heal=>row.healing,SortMode::Tank=>row.damage_taken};let total=compact(total_value as f64);let rate=compact(if snapshot.encounter_ms>0{total_value as f64*1000.0/snapshot.encounter_ms as f64}else{0.0});",
        "raid metric value formatting",
    );

    // v1.22.1: tighten the Total/Active-rate pair and make revive state the
    // highest-priority content in a cramped raid row. Dead rows first reserve the
    // revive label, then Total, then Active/s. Share/death badges yield entirely
    // while dead so revive information cannot be pushed out by lower-priority data.
    replace_between(
        &mut source,
        "unsafe fn paint_raid_player(",
        "unsafe fn paint_raid_rows(",
        r#"unsafe fn paint_raid_player(hdc:HDC,state:&State,row:&DpsRow,rank:usize,r:RECT,leader:i64,snapshot:&DpsSnapshot,settings:&FeatureSettings){
    let bg=if row.is_dead{rgb(105,28,34)}else{spec_color(row)};fill(hdc,&r,bg);if row.is_local{outline(hdc,r,rgb(212,175,55),2);}let base=text_on(bg);let scale=dps_layout_scale(state);
    let rank_w=dps_adaptive_logical_px(24,scale);let total_w=dps_adaptive_logical_px(66,scale);let rate_w=dps_adaptive_logical_px(58,scale);let metric_gap=dps_adaptive_logical_px(1,scale).max(1);
    let share_w=if !row.is_dead&&dps_mode_share_enabled(settings,state.sort_mode){dps_adaptive_logical_px(48,scale)}else{0};let death_w=if !row.is_dead&&settings.meter.show_deaths{dps_adaptive_logical_px(24,scale)}else{0};
    let mut right=r.right-4;let death_left=right-death_w;if death_w>0{right=death_left-2;}let share_left=right-share_w;if share_w>0{right=share_left-2;}
    let min_name_w=dps_adaptive_logical_px(34,scale);let name_floor=r.left+rank_w+min_name_w;let mut total_left=right;let mut rate_left=right;let mut show_total=!row.is_dead;let mut show_rate=!row.is_dead;let mut revive_slot:Option<(RECT,String,u32)>=None;
    if row.is_dead{
        let(status,color)=revive_status_text(row,now_ms());let revive_w=dps_adaptive_logical_px(116,scale);let room=(right-name_floor).max(0);show_total=room>=revive_w+metric_gap+total_w;show_rate=show_total&&room>=revive_w+metric_gap+total_w+metric_gap+rate_w;
        let mut cursor=right;if show_rate{rate_left=cursor-rate_w;cursor=rate_left-metric_gap;}if show_total{total_left=cursor-total_w;cursor=total_left-metric_gap;}let revive_left=(cursor-revive_w).max(name_floor);revive_slot=Some((RECT{left:revive_left,top:r.top,right:cursor,bottom:r.bottom-2},status,color));
    }else{
        rate_left=right-rate_w;right=rate_left-metric_gap;total_left=right-total_w;
    }
    let content_left=if let Some((slot,_,_))=&revive_slot{slot.left}else{total_left};let name_right=(content_left-dps_adaptive_logical_px(3,scale)).max(name_floor);
    SelectObject(hdc,dps_secondary_font(state));SetTextColor(hdc,base);draw(hdc,&rank.to_string(),RECT{left:r.left+3,top:r.top,right:r.left+rank_w,bottom:r.bottom-2},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);
    SelectObject(hdc,dps_primary_font(state));SetTextColor(hdc,if row.is_dead{rgb(255,120,120)}else{base});draw(hdc,&row.name,RECT{left:r.left+rank_w,top:r.top,right:name_right,bottom:r.bottom-2},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);
    if let Some((slot,status,color))=revive_slot{SetTextColor(hdc,color);draw(hdc,&status,slot,DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}
    let total_value=match state.sort_mode{SortMode::Damage=>row.damage,SortMode::Heal=>row.healing,SortMode::Tank=>row.damage_taken};let total=compact(total_value as f64);let rate=compact(if snapshot.encounter_ms>0{total_value as f64*1000.0/snapshot.encounter_ms as f64}else{0.0});SetTextColor(hdc,base);
    if show_total{draw(hdc,&total,RECT{left:total_left,top:r.top,right:total_left+total_w,bottom:r.bottom-2},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}if show_rate{draw(hdc,&rate,RECT{left:rate_left,top:r.top,right:rate_left+rate_w,bottom:r.bottom-2},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}
    if share_w>0{if let Some((share,color))=active_share(row,state.sort_mode,settings){SetTextColor(hdc,color);draw(hdc,&format!("{share:.1}%"),RECT{left:share_left,top:r.top,right:share_left+share_w,bottom:r.bottom-2},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}}
    if death_w>0&&row.deaths>0{SetTextColor(hdc,crate::ui_theme::CRITICAL);draw(hdc,&format!("D{}",row.deaths),RECT{left:death_left,top:r.top,right:r.right-3,bottom:r.bottom-2},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}
    let metric=mode_metric(row,state.sort_mode);let bar=RECT{left:r.left,top:r.bottom-2,right:r.right,bottom:r.bottom};fill(hdc,&bar,rgb(20,24,29));if metric>0{let width=(r.right-r.left).max(0)as i64;let filled=(width.saturating_mul(metric.min(leader))/leader.max(1)).clamp(0,width)as i32;let color=match state.sort_mode{SortMode::Damage=>rgb(235,74,74),SortMode::Heal=>rgb(55,205,105),SortMode::Tank=>rgb(65,145,235)};fill(hdc,&RECT{left:r.left,top:r.bottom-2,right:r.left+filled,bottom:r.bottom},color);}
}

"#,
        "raid compact metrics and revive priority",
    );

    fs::write(&overlay,source).expect("write v1.22.1 raid compatibility fixes");
    println!("cargo:rerun-if-changed=build/legacy/build_v1220_fix.rs");
}
