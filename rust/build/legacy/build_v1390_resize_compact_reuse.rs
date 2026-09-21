// Compact headers and rows must use the same measured geometry without
// rescanning every participant and remeasuring every numeric value per row.
// Retain v1389's independent left/right Raid sizing and all native behavior.
use std::{env,fs,path::PathBuf};
mod previous {include!("build_v1389_resize_compact_corrected.rs");pub fn run(){main();}}
fn once(part:&mut String,old:&str,new:&str,id:&str){assert_eq!(part.matches(old).count(),1,"compact reuse {id} anchor");*part=part.replacen(old,new,1);}
fn section(src:&mut String,start:&str,end:&str,edits:&[(&str,&str,&str)]){
    assert_eq!(src.matches(start).count(),1,"compact reuse ambiguous start {start}");
    let a=src.find(start).unwrap();let b=a+src[a..].find(end).expect("compact reuse section end");
    let mut part=src[a..b].to_owned();for (old,new,id) in edits {once(&mut part,old,new,id);}
    src.replace_range(a..b,&part);
}
fn main(){
    previous::run();
    let path=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("feature_overlays_v170_fixed.rs");
    let mut src=fs::read_to_string(&path).expect("native meter");
    section(&mut src,"unsafe fn paint_compact_rows(","unsafe fn paint_grouped_mechanic_label(",&[
        ("let show_total=audit_compact_columns(hdc,state,RECT{left:6,top:0,right:rc.right-8,bottom:0},&rows).show_total;",
         "let columns=audit_compact_columns(hdc,state,RECT{left:6,top:0,right:rc.right-8,bottom:0},&rows);","single-panel measure once"),
        ("paint_compact_player(hdc,state,row,*rank,r,show_total);","paint_compact_player(hdc,state,row,*rank,r,columns);","single-panel reuse"),
    ]);
    section(&mut src,"unsafe fn paint_compact_raid_rows(","fn raid_painted_player_column(",&[
        ("let left_total=audit_compact_columns(hdc,state,left,&all_rows).show_total;\n    let right_total=audit_compact_columns(hdc,state,right,&all_rows).show_total;",
         "let left_cols=audit_compact_columns(hdc,state,left,&all_rows);\n    let right_cols=audit_compact_columns(hdc,state,right,&all_rows);","independent Raid measurements"),
        ("                left_total,","                left_cols,","left column reuse"),
        ("                right_total,","                right_cols,","right column reuse"),
    ]);
    section(&mut src,"unsafe fn paint_compact_player(","unsafe fn paint_compact_raid_rows(",&[
        ("    show_total: bool,","    columns: AuditCompactColumns,","pass precomputed columns"),
        ("    let cols=audit_compact_columns(hdc,state,r,&meter_rows(state));","    let cols=columns;","remove per-row roster scan"),
        ("let status_w=reference_compact_status_width(r.right-r.left,scale,show_total,row.is_dead);",
         "let status_w=reference_compact_status_width(r.right-r.left,scale,cols.show_total,row.is_dead);","status uses passed visibility"),
        ("let total_w=if show_total {cols.total_w}else{0};","let total_w=if cols.show_total {cols.total_w}else{0};","total uses passed visibility"),
    ]);
    fs::write(&path,src).expect("write compact measurement reuse");
    println!("cargo:rerun-if-changed=build/legacy/build_v1390_resize_compact_reuse.rs");
}
