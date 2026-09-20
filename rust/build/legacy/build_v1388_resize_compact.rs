// Resize audit W05/W06: use one font-measured Compact geometry for header and rows.
use std::{env,fs,path::PathBuf};
mod previous { include!("build_v1387_resize_metadata.rs");pub fn run(){main();} }
fn exact(src:&mut String,old:&str,new:&str,n:usize,id:&str){assert_eq!(src.matches(old).count(),n,"resize {id}: expected {n} anchors");*src=src.replace(old,new);}
fn part(src:&mut String,start:&str,end:&str,old:&str,new:&str,n:usize,id:&str){assert_eq!(src.matches(start).count(),1,"resize {id}: ambiguous function");let a=src.find(start).unwrap();let b=a+src[a..].find(end).expect("function end");let mut section=src[a..b].to_owned();exact(&mut section,old,new,n,id);src.replace_range(a..b,&section);}
fn main(){previous::run();let path=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("feature_overlays_v170_fixed.rs");let mut src=fs::read_to_string(&path).expect("generated meter");
// Compact player rows and header must agree on actual enabled metric widths.
part(&mut src,"unsafe fn paint_compact_player(","unsafe fn paint_compact_raid_rows(","    let rank_w = dps_adaptive_logical_px(24, scale);\n    let gap = dps_adaptive_logical_px(3, scale);\n    let status_w = reference_compact_status_width(r.right - r.left, scale, show_total, row.is_dead);\n    let rate_nominal = dps_adaptive_logical_px(64, scale);\n    let total_nominal = if show_total { dps_adaptive_logical_px(70, scale) } else { 0 };\n    let metric_budget = r.right - r.left - 8 - rank_w - gap - status_w\n        - if row.is_dead { gap + dps_adaptive_logical_px(72, scale) } else { 0 };\n    let total_w = if !row.is_dead || (show_total && metric_budget >= total_nominal + gap) {\n        total_nominal\n    } else { 0 };\n    let rate_w = if !row.is_dead || metric_budget >= total_w\n        + if total_w > 0 { gap } else { 0 } + rate_nominal + gap {\n        rate_nominal\n    } else { 0 };\n    let right = r.right - 4;\n    let rate_right = right;\n    let rate_left = rate_right - rate_w;\n    let total_right = rate_left - if rate_w > 0 { gap } else { 0 };\n    let total_left = total_right - total_w;\n    let identity_right = (if total_w > 0 { total_left } else if rate_w > 0 { rate_left } else { right }) - gap;","    let columns = audit_compact_columns(hdc,state,r,&meter_rows(state));\n    let rank_w=columns.rank_w;let gap=columns.gap;\n    let status_w=reference_compact_status_width(r.right-r.left,scale,show_total,row.is_dead);\n    let total_w=if show_total {columns.total_w}else{0};\n    let rate_w=columns.rate_w;\n    let rate_right=columns.rate_right;let rate_left=rate_right-rate_w;\n    let total_right=rate_left-gap;let total_left=total_right-total_w;\n    let identity_right=(if total_w>0{total_left}else{rate_left})-gap;",1,"W05 share measured header/row columns");
// Compact (one panel) checks the SAME rectangle used to paint its rows.
part(&mut src,"unsafe fn paint_compact_rows(","unsafe fn paint_detail_overlay(","let show_total=reference_compact_show_total((rc.right-14).max(1),scale);","let show_total=audit_compact_columns(hdc,state,RECT{left:6,top:0,right:rc.right-8,bottom:0},&rows).show_total;",1,"W06 Compact actual Total fit");
// Compact Raid halves may be 1px different widths: budget separately.
part(&mut src,"unsafe fn paint_compact_raid_rows(","fn raid_painted_player_column(","    let show_total = reference_compact_show_total((left.right - left.left).max(1), scale);","    let left_total=audit_compact_columns(hdc,state,left,&all_rows).show_total;\n    let right_total=audit_compact_columns(hdc,state,right,&all_rows).show_total;",1,"W06 independent Compact Raid halves");
// The only two standalone show_total arguments here are per-panel row calls.
part(&mut src,"unsafe fn paint_compact_raid_rows(","fn raid_painted_player_column(","                show_total,","                left_total,",1,"W06 left half");
part(&mut src,"unsafe fn paint_compact_raid_rows(","fn raid_painted_player_column(","                show_total,","                right_total,",1,"W06 right half");
// Header and row rectangles are now identical; do not render a 100px heading
// into a hard-coded 64px cell or use a different old fixed-width threshold.
part(&mut src,"unsafe fn paint_reference_headers(","unsafe fn paint_dps(","            let rank_w = dps_adaptive_logical_px(24, scale);\n            let rank_gap = dps_adaptive_logical_px(4, scale);\n            let rate_right = r.right - 4;\n            let rate_left = rate_right - dps_adaptive_logical_px(64, scale);\n            let show_total = reference_compact_show_total(r.right - r.left, scale);\n            let total_right = rate_left - dps_adaptive_logical_px(3, scale);\n            let total_left = total_right - dps_adaptive_logical_px(70, scale);\n            let name_right = if show_total { total_left } else { rate_left } - 4;","            let cols=audit_compact_columns(hdc,state,r,&all_rows);\n            let rank_w=cols.rank_w;let rank_gap=cols.gap;\n            let rate_right=cols.rate_right;let rate_left=cols.rate_left;\n            let show_total=cols.show_total;\n            let total_right=cols.total_right;let total_left=cols.total_left;\n            let name_right=cols.name_right;",1,"W05 headers use measured column boundaries");
part(&mut src,"unsafe fn paint_reference_headers(","unsafe fn paint_dps(","                    reference_meter_rate_label(state.sort_mode),","                    cols.rate_label,",1,"W05 actual measured rate heading");
// Append the shared geometry; all coordinates are logical client units, as
// supplied by the original paint transform. Measure both final GDI fonts.
src.push_str(r#"
#[derive(Clone,Copy,Debug)]struct AuditCompactColumns {
    rank_w:i32,gap:i32,rate_left:i32,rate_right:i32,rate_w:i32,
    total_left:i32,total_right:i32,total_w:i32,name_right:i32,
    show_total:bool,rate_label:&'static str,required_both:i32,
}
fn audit_compact_required(rank:i32,name:i32,total:i32,rate:i32,gap:i32)->i32 {
    3+rank+gap+name+gap+total+gap+rate+4
}
fn audit_short_rate(mode:SortMode)->&'static str {
    match mode {SortMode::Damage=>"Act. DPS",SortMode::Heal=>"Act. HPS",SortMode::Tank=>"Act. DTPS"}
}
unsafe fn audit_compact_columns(hdc:HDC,state:&State,r:RECT,rows:&[&DpsRow])->AuditCompactColumns {
    let scale=dps_layout_scale(state);let rank=dps_adaptive_logical_px(24,scale);
    let gap=dps_adaptive_logical_px(6,scale).max(8);
    let min_name=dps_adaptive_logical_px(98,scale);
    let old=SelectObject(hdc,dps_primary_font(state));
    let mut value_total=dps_text_width(hdc,"0",10);
    let mut value_rate=dps_text_width(hdc,"0",10);
    // Measure all eligible participants, not the fluctuating visible page.
    for row in rows {
        let total=compact(mode_metric(row,state.sort_mode) as f64);
        let rate=compact(active_rate(row,state.sort_mode));
        value_total=value_total.max(dps_text_width(hdc,&total,identity_text_px(&total,true)));
        value_rate=value_rate.max(dps_text_width(hdc,&rate,identity_text_px(&rate,true)));
    }
    SelectObject(hdc,dps_secondary_font(state));
    let full=reference_meter_rate_label(state.sort_mode);
    let short=audit_short_rate(state.sort_mode);
    let total_w=value_total.max(dps_text_width(hdc,"Total",40))+8;
    let rate_full=value_rate.max(dps_text_width(hdc,full,64))+8;
    let rate_short=value_rate.max(dps_text_width(hdc,short,50))+8;
    SelectObject(hdc,old);
    let width=(r.right-r.left).max(0);
    let full_required=audit_compact_required(rank,min_name,total_w,rate_full,gap);
    let short_required=audit_compact_required(rank,min_name,total_w,rate_short,gap);
    let (show_total,rate_w,rate_label,required_both)=if width>=full_required {
        (true,rate_full,full,full_required)
    } else if width>=short_required {
        (true,rate_short,short,short_required)
    } else {(false,rate_full,full,full_required)};
    let rate_right=r.right-4;let rate_left=rate_right-rate_w;
    let total_right=rate_left-gap;let total_left=total_right-total_w;
    let name_right=(if show_total {total_left}else{rate_left})-gap;
    AuditCompactColumns {rank_w:rank,gap,rate_left,rate_right,rate_w,
        total_left,total_right,total_w,name_right,show_total,rate_label,required_both}
}
#[cfg(test)]mod resize_compact_budget_tests {use super::*;
    #[test]fn symmetric_exact_one_pixel_fit() {
        for (scale,total,rate) in [(60,72,100),(100,65,76)] {
            let rank=dps_adaptive_logical_px(24,scale);let name=dps_adaptive_logical_px(98,scale);
            let gap=dps_adaptive_logical_px(6,scale).max(8);
            let fit=audit_compact_required(rank,name,total,rate,gap);
            for width in (fit-2)..=(fit+2) {
                let fit_from_open=width>=fit;let fit_from_shrink=width>=fit;
                assert_eq!(fit_from_open,fit_from_shrink);
                assert_eq!(fit_from_open,width>=fit);
            }
            assert!(fit-1<fit&&fit+1>=fit);
        }
    }
    #[test]fn headers_are_part_of_the_budget() {
        assert!(audit_compact_required(24,98,68,100,8)>audit_compact_required(24,98,68,64,8));
    }
}
"#);
fs::write(&path,src).expect("write compact columns");println!("cargo:rerun-if-changed=build/legacy/build_v1388_resize_compact.rs");}
