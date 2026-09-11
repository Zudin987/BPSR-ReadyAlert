use std::{env,fs,path::PathBuf};

mod prior {
    include!("build_v1220_fix.rs");
    pub fn run(){main();}
}

fn replace_between(source:&mut String,start:&str,end:&str,replacement:&str,label:&str){
    let count=source.matches(start).count();
    assert_eq!(count,1,"v1.22.1 patch {label} start expected one match, found {count}");
    let begin=source.find(start).expect("v1.22.1 start anchor");
    let rel_end=source[begin..].find(end).unwrap_or_else(||panic!("v1.22.1 patch {label} end anchor missing"));
    source.replace_range(begin..begin+rel_end,replacement);
}

fn main(){
    prior::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let overlay=out.join("feature_overlays_v170_fixed.rs");
    let mut source=fs::read_to_string(&overlay).expect("read v1.22.0 generated overlays").replace("\r\n","\n");

    replace_between(
        &mut source,
        "unsafe fn paint_raid_player(",
        "unsafe fn paint_raid_rows(",
        r#"fn raid_revive_metric_plan(available:i32,revive_w:i32,total_w:i32,active_w:i32,gap:i32)->(bool,bool){
    let after_revive=available.saturating_sub(revive_w);
    let show_total=after_revive>=gap.saturating_add(total_w);
    let after_total=after_revive.saturating_sub(if show_total{gap.saturating_add(total_w)}else{0});
    let show_active=show_total&&after_total>=gap.saturating_add(active_w);
    (show_total,show_active)
}

unsafe fn paint_raid_player(hdc:HDC,state:&State,row:&DpsRow,rank:usize,r:RECT,leader:i64,_snapshot:&DpsSnapshot,settings:&FeatureSettings){
    let bg=if row.is_dead{rgb(105,28,34)}else{spec_color(row)};fill(hdc,&r,bg);if row.is_local{outline(hdc,r,rgb(212,175,55),2);}let base=text_on(bg);let scale=dps_layout_scale(state);
    let rank_w=dps_adaptive_logical_px(24,scale);let metric_gap=dps_adaptive_logical_px(1,scale).max(1);let name_left=r.left+rank_w;
    let total_value=match state.sort_mode{SortMode::Damage=>row.damage,SortMode::Heal=>row.healing,SortMode::Tank=>row.damage_taken};let total=compact(total_value as f64);
    let active_value=match state.sort_mode{SortMode::Damage=>row.active_dps,SortMode::Heal=>row.active_hps,SortMode::Tank=>row.active_dtps};let active=compact(active_value);
    let old_font=SelectObject(hdc,dps_primary_font(state));
    let total_w=(dps_text_width(hdc,&total,dps_adaptive_logical_px(52,scale))+4).clamp(dps_adaptive_logical_px(42,scale),dps_adaptive_logical_px(68,scale));
    let active_w=(dps_text_width(hdc,&active,dps_adaptive_logical_px(44,scale))+4).clamp(dps_adaptive_logical_px(36,scale),dps_adaptive_logical_px(58,scale));

    SelectObject(hdc,dps_secondary_font(state));SetTextColor(hdc,base);draw(hdc,&rank.to_string(),RECT{left:r.left+3,top:r.top,right:r.left+rank_w,bottom:r.bottom-2},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);
    SelectObject(hdc,dps_primary_font(state));

    if row.is_dead{
        // Revive state is the highest-priority information in Raid Mode. It owns the
        // metric block first; TOTAL is added only if it fits, then ACTIVE/s last.
        // Share/death reservations are intentionally dropped on a dead row so they
        // can never crowd out CAN REVIVE / REVIVE IN.
        let(status,status_color)=revive_status_text(row,now_ms());
        let min_name_w=dps_adaptive_logical_px(34,scale);let hard_metric_left=(name_left+min_name_w+4).min(r.right-8);
        let max_revive_w=(r.right-4-hard_metric_left).max(dps_adaptive_logical_px(44,scale));
        let revive_w=(dps_text_width(hdc,&status,dps_adaptive_logical_px(88,scale))+6).clamp(dps_adaptive_logical_px(58,scale),dps_adaptive_logical_px(132,scale)).min(max_revive_w);
        let available=(r.right-4-hard_metric_left).max(revive_w);
        let(show_total,show_active)=raid_revive_metric_plan(available,revive_w,total_w,active_w,metric_gap);
        let block_w=revive_w+if show_total{metric_gap+total_w}else{0}+if show_active{metric_gap+active_w}else{0};
        let block_left=(r.right-4-block_w).max(name_left+8);let name_right=(block_left-4).max(name_left+8);
        SetTextColor(hdc,rgb(255,120,120));draw(hdc,&row.name,RECT{left:name_left,top:r.top,right:name_right,bottom:r.bottom-2},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);
        let mut x=block_left;SetTextColor(hdc,status_color);draw(hdc,&status,RECT{left:x,top:r.top,right:x+revive_w,bottom:r.bottom-2},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);x+=revive_w;
        if show_total{x+=metric_gap;SetTextColor(hdc,base);draw(hdc,&total,RECT{left:x,top:r.top,right:x+total_w,bottom:r.bottom-2},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);x+=total_w;}
        if show_active{x+=metric_gap;SetTextColor(hdc,base);draw(hdc,&active,RECT{left:x,top:r.top,right:(x+active_w).min(r.right-4),bottom:r.bottom-2},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}
    }else{
        let share_w=if dps_mode_share_enabled(settings,state.sort_mode){dps_adaptive_logical_px(48,scale)}else{0};let death_w=if settings.meter.show_deaths{dps_adaptive_logical_px(24,scale)}else{0};
        let mut right=r.right-4;let death_left=right-death_w;if death_w>0{right=death_left-metric_gap;}let share_left=right-share_w;if share_w>0{right=share_left-metric_gap;}let active_left=right-active_w;right=active_left-metric_gap;let total_left=right-total_w;let name_right=(total_left-4).max(name_left+30);
        SetTextColor(hdc,base);draw(hdc,&row.name,RECT{left:name_left,top:r.top,right:name_right,bottom:r.bottom-2},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);
        SetTextColor(hdc,base);draw(hdc,&total,RECT{left:total_left,top:r.top,right:total_left+total_w,bottom:r.bottom-2},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);draw(hdc,&active,RECT{left:active_left,top:r.top,right:active_left+active_w,bottom:r.bottom-2},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);
        if share_w>0{if let Some((share,color))=active_share(row,state.sort_mode,settings){SetTextColor(hdc,color);draw(hdc,&format!("{share:.1}%"),RECT{left:share_left,top:r.top,right:share_left+share_w,bottom:r.bottom-2},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}}
        if death_w>0&&row.deaths>0{SetTextColor(hdc,crate::ui_theme::CRITICAL);draw(hdc,&format!("D{}",row.deaths),RECT{left:death_left,top:r.top,right:r.right-3,bottom:r.bottom-2},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}
    }
    SelectObject(hdc,old_font);
    let metric=mode_metric(row,state.sort_mode);let bar=RECT{left:r.left,top:r.bottom-2,right:r.right,bottom:r.bottom};fill(hdc,&bar,rgb(20,24,29));if metric>0{let width=(r.right-r.left).max(0)as i64;let filled=(width.saturating_mul(metric.min(leader))/leader.max(1)).clamp(0,width)as i32;let color=match state.sort_mode{SortMode::Damage=>rgb(235,74,74),SortMode::Heal=>rgb(55,205,105),SortMode::Tank=>rgb(65,145,235)};fill(hdc,&RECT{left:r.left,top:r.bottom-2,right:r.left+filled,bottom:r.bottom},color);}
}

"#,
        "raid player metrics",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1221_raid_metric_priority_tests{
    use super::*;
    #[test]
    fn revive_survives_before_total_or_active(){
        assert_eq!(raid_revive_metric_plan(80,80,50,45,1),(false,false));
        assert_eq!(raid_revive_metric_plan(131,80,50,45,1),(true,false));
        assert_eq!(raid_revive_metric_plan(177,80,50,45,1),(true,true));
    }
    #[test]
    fn active_is_dropped_before_total_when_space_tightens(){
        assert_eq!(raid_revive_metric_plan(150,80,50,45,1),(true,false));
    }
}
"#);

    fs::write(&overlay,source).expect("write v1.22.1 raid metric priority patch");
    println!("cargo:rerun-if-changed=build/legacy/build_v1221.rs");
}
