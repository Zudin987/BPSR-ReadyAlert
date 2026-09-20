// Resize audit W01-W04: repair compiled native Rust, not the HTML report.
use std::{env,fs,path::PathBuf};
mod previous { include!("build_v1386_followup_test_contract.rs"); pub fn run() { main(); } }
fn exact(src:&mut String,old:&str,new:&str,id:&str){assert_eq!(src.matches(old).count(),1,"resize {id}: expected one anchor");*src=src.replacen(old,new,1);}
fn in_function(src:&mut String,start:&str,end:&str,old:&str,new:&str,id:&str){assert_eq!(src.matches(start).count(),1,"resize {id}: ambiguous function");let a=src.find(start).unwrap();let b=a+src[a..].find(end).expect("function end must follow function start");let mut section=src[a..b].to_owned();exact(&mut section,old,new,id);src.replace_range(a..b,&section);}
fn main(){previous::run();let path=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("feature_overlays_v170_fixed.rs");let mut src=fs::read_to_string(&path).expect("generated meter");
// The old normal layout capped class metadata to 150px, spent spare width on
// the name gutter, and then discarded all scores behind a separate 170px gate.
in_function(&mut src,"unsafe fn dps_row_layout_responsive(","unsafe fn paint_dps_secondary_colored(","    let class_min = dps_adaptive_logical_px(62, scale);","    let class_min = dps_adaptive_logical_px(62, scale);\n    let old_secondary = SelectObject(hdc, _secondary_font);\n    let metadata = dps_secondary(_row);\n    let measured_class = dps_text_width(hdc, &metadata, identity_text_px(&metadata, false)) + 10;\n    SelectObject(hdc, old_secondary);\n    let class_preferred = measured_class.max(dps_adaptive_logical_px(178,scale)).min(dps_adaptive_logical_px(290,scale));","W01 measure metadata with final font");
in_function(&mut src,"unsafe fn dps_row_layout_responsive(","unsafe fn paint_dps_secondary_colored(","    let mut extra = available - active_span - total_span - name_min - class_min - gap;","    let mut extra = available - active_span - total_span - name_min - class_preferred - gap;","W02 metadata precedes optional metrics");
in_function(&mut src,"unsafe fn dps_row_layout_responsive(","unsafe fn paint_dps_secondary_colored(","    let spare_after_class = identity_w - name_min\n        - if has_class { class_min + gap } else { 0 };","    let spare_after_class = identity_w - name_min\n        - if has_class { class_preferred + gap } else { 0 };","W02 badges after metadata");
in_function(&mut src,"unsafe fn dps_row_layout_responsive(","unsafe fn paint_dps_secondary_colored(","    let class_w = if has_class {\n        let spare = (identity_space - name_min - class_min - gap).max(0);\n        class_min + (spare / 2).min(dps_adaptive_logical_px(150, scale) - class_min)\n    } else { 0 };","    // Reclaim empty name gutters before discarding complete metadata tokens.\n    let class_w = if has_class {\n        (identity_space - name_min - gap).max(0).min(class_preferred)\n    } else { 0 };","W02 reclaim name gutter and remove 150px cap");
exact(&mut src,"unsafe fn audit_shared_secondary(hdc:HDC,row:&DpsRow,width:i32)->String{if width<170{dps_spec(row)}else{dps_secondary_compact_measured(hdc,row,width)}}","unsafe fn audit_shared_secondary(hdc:HDC,row:&DpsRow,width:i32)->String{dps_secondary_compact_measured(hdc,row,width)}","W01/W04 remove class-only cliff");
// The compact painter occurs BEFORE the Raid painter in the generated unit;
// scoping Raid by that earlier marker panicked in CI. Use its following module.
in_function(&mut src,"unsafe fn paint_raid_player(","mod white_header_icons {", "        right: if row.is_dead { layout.spec_right } else if settings.meter.show_consumables { identity.right.min(r.right - 48) } else { identity.right },","        right: if row.is_dead { layout.spec_right } else if settings.meter.show_consumables { r.right - 48 } else { r.right - 4 },","W03 second line reserves only F/S, not first-line columns");
src.push_str(r#"
#[cfg(test)] mod resize_metadata_tests {
    use super::*;
    #[test] fn measured_tiers_retain_score_before_strength() { unsafe {
        let hdc=GetDC(std::ptr::null_mut());assert!(!hdc.is_null());
        let old=SelectObject(hdc,dps_cached_font(100,false));
        let row=DpsRow{name:"KuruRyn".into(),subprofession_name:"Moonstrike".into(),ability_score:56_000,illusion_break:3210,..DpsRow::default()};
        let full=dps_secondary(&row);let medium="Moonstrike · 56k";
        let full_w=dps_text_width(hdc,&full,0);
        let medium_w=dps_text_width(hdc,medium,0);
        assert_eq!(audit_shared_secondary(hdc,&row,full_w),full);
        assert_eq!(audit_shared_secondary(hdc,&row,medium_w),medium);
        assert_eq!(audit_shared_secondary(hdc,&row,medium_w-1),"Moonstrike");
        let missing=DpsRow{illusion_break:0,..row};
        assert_eq!(dps_secondary(&missing),medium);
        SelectObject(hdc,old);ReleaseDC(std::ptr::null_mut(),hdc);
    }}
    #[test] fn class_budget_precedes_empty_name_gutter() {
        for scale in [60,100] {
            let name=dps_adaptive_logical_px(94,scale);
            let preferred=dps_adaptive_logical_px(178,scale);
            for surplus in 0..=200 {
                let usable=name+preferred+6+surplus;
                let chosen=(usable-name-6).min(preferred);
                assert_eq!(chosen,preferred,"extra width cannot starve metadata");
            }
        }
    }
}
"#);
fs::write(&path,src).expect("write measured metadata");println!("cargo:rerun-if-changed=build/legacy/build_v1387_resize_metadata.rs");}
