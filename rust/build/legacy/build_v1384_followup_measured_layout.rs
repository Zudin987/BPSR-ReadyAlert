// Follow-up R01/R02/R06/R10/R11/R12/R13/R16/R17/R20. Exact-anchored
// changes to the final generated native units: no HTML/UI mocks or telemetry.
use std::{env, fs, path::{Path, PathBuf}};
mod previous {
    include!("build_v1383_followup_preferences.rs");
    pub fn run(){main();}
}
fn exact(s:&mut String,old:&str,new:&str,n:usize,id:&str){
    assert_eq!(s.matches(old).count(),n,"follow-up {id}: expected {n} source anchors");
    *s=s.replace(old,new);
    assert!(s.contains(new),"follow-up {id}: compiled replacement absent");
}
fn section(s:&mut String,start:&str,end:&str,old:&str,new:&str,id:&str){
    assert_eq!(s.matches(start).count(),1,"follow-up {id}: ambiguous function start");
    let first=s.find(start).unwrap();let last=first+s[first..].find(end).expect("function end");
    let mut part=s[first..last].to_owned();exact(&mut part,old,new,1,id);s.replace_range(first..last,&part);
}
fn edit(out:&Path,name:&str, f:impl FnOnce(&mut String)){
    let path=out.join(name);let mut s=fs::read_to_string(&path).expect("read generated Rust");
    f(&mut s);fs::write(path,s).expect("write generated Rust");
}
fn main(){
    previous::run();let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    edit(&out,"feature_overlays_v170_fixed.rs",|s|{
        exact(s,"const REFERENCE_TABS_H: i32 = 28;","const REFERENCE_TABS_H: i32 = 40;",1,"R01 measured tab-height allocation");
        exact(s,"fn reference_selector_w(state: &State, _right: i32) -> i32 {\n    match (state.compact_mode, raid_active(state)) {\n        (true, true) => 148,\n        (true, false) => 96,\n        (false, true) => 76,\n        _ => 96,\n    }\n}",
            "fn reference_selector_w(state: &State, _right: i32) -> i32 {\n    audit_label_width(state,reference_layout_label(state),24).max(96)\n}",1,"R02 selector measured with paint font");
        exact(s,"let live_w = if right < 340 { 38 } else { 42 };",
            "let live_w = audit_label_width(state,\"Live\",16).max(44);",1,"R03 Live width uses selected font");
        exact(s,"reference_contains(reference_tab_rect(index as i32), x, y)",
            "reference_contains(audit_tab_rect(state,index as i32), x, y)",1,"R01 tab hit rectangles match paint");
        exact(s,"let r = reference_tab_rect(index as i32);",
            "let r = audit_tab_rect(state,index as i32);",1,"R01 measured tab painting");
        section(s,"unsafe fn paint_mode_tab(","unsafe fn dps_row_layout_responsive(",
            "bottom: y + 24,","bottom: y + 34,","R01 full tab height");
        section(s,"unsafe fn paint_mode_tab(","unsafe fn dps_row_layout_responsive(",
            "        label,\n        r,\n        DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,",
            "        label,\n        RECT { left: r.left+6, right: r.right-6, ..r },\n        DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,","R01 six-unit text inset");
        exact(s,"let base = overlay_min_width_mode(state.kind, scale, state.compact_mode);\n    if state.kind == Kind::Dps && raid_active(state) {\n        base.max(reference_raid_min_width(scale, state.compact_mode))\n    } else { base }",
            "let base = overlay_min_width_mode(state.kind, scale, state.compact_mode);\n    if state.kind == Kind::Dps {\n        base.max(scale_px(audit_toolbar_min_width(state),scale)).max(if raid_active(state){reference_raid_min_width(scale,state.compact_mode)}else{0})\n    } else { base }",1,"R02 measured toolbar minimum");
        section(s,"unsafe fn dps_row_layout_responsive(","unsafe fn paint_dps_secondary_colored(",
            "else if show_imagines && has_class && spare_after_class >= badge_w + badge_outer_gap + dps_adaptive_logical_px(48, scale) { 1 }",
            "else if show_imagines && _row.imagines.len() == 1 && has_class && spare_after_class >= badge_w + badge_outer_gap + dps_adaptive_logical_px(48, scale) { 1 }", "R10 only one equipped image merits one icon");
        section(s,"unsafe fn dps_row_layout_responsive(","unsafe fn paint_dps_secondary_colored(",
            "let gap = dps_adaptive_logical_px(4, scale);",
            "let gap = dps_adaptive_logical_px(4, scale).max(6);","R12 numeric gap shared plan");
        exact(s,"fn normal_meter_fs_width(scale:i32)->i32{dps_adaptive_logical_px(38,scale).max(30)}",
            "fn normal_meter_fs_width(scale:i32)->i32{dps_adaptive_logical_px(38,scale).max(38)}",1,"R12 food serum reserve");
        exact(s,"let secondary = dps_secondary_compact_measured(hdc, row,",
            "let secondary = audit_shared_secondary(hdc, row,",1,"R11 normal tier-consistent metadata");
        exact(s,"let text = dps_secondary_compact_measured(hdc, row, detail.right - detail.left);",
            "let text = audit_shared_secondary(hdc, row, detail.right - detail.left);",1,"R11 Raid metadata consistent");
        exact(s,"let max_width=(rc.right-8).clamp(160,420);",
            "let max_width=(rc.right-8).clamp(160,280);",1,"R13 bound tooltip size");
        exact(s,"let cards_top=identity.bottom+7;let cards_bottom=(detail_tab_top()-74).max(cards_top+70);",
            "let cards_top=identity.bottom+7;let cards_bottom=(detail_tab_top()-62).max(cards_top+86);",1,"R16 last card line fits");
        section(s,"unsafe fn paint_detail_skills(","unsafe fn paint_detail_taken(",
            "let id_r=58;","let id_r=dps_text_width(hdc,\"9999999999\",84)+16;","R17 exact-font ID width");
        section(s,"unsafe fn paint_detail_skills(","unsafe fn paint_detail_taken(",
            "left:id_r+2,","left:id_r+8,","R17 header gap between ID and skill");
        section(s,"unsafe fn paint_detail_skills(","unsafe fn paint_detail_taken(",
            "left:id_r+3,","left:id_r+8,","R17 row gap between ID and skill");
        exact(s,"let expanded_w=(state.expanded.right-state.expanded.left).max(thick);let expanded_h=(state.expanded.bottom-state.expanded.top).max(thick);",
            "let expanded_w=(state.expanded.right-state.expanded.left).max(thick).min(scale_px(96,state.scale_percent));let expanded_h=(state.expanded.bottom-state.expanded.top).max(thick).min(scale_px(96,state.scale_percent));",1,"R20 resize keeps actual HWND compact");
        exact(s,"let width=(rect.right-rect.left).max(scale_px(100,state.scale_percent));let height=(rect.bottom-rect.top).max(scale_px(80,state.scale_percent));",
            "let width=(rect.right-rect.left).max(scale_px(100,state.scale_percent)).min(scale_px(96,state.scale_percent));let height=(rect.bottom-rect.top).max(scale_px(80,state.scale_percent)).min(scale_px(96,state.scale_percent));",1,"R20 collapse shrinks real HWND");
        s.push_str(r#"
// Measure with precisely the HFONT selected by paint_dps and paint_toolbar.
// Both callsites and WM_GETMINMAXINFO share these logical dimensions.
fn audit_label_width(state:&State,label:&str,padding:i32)->i32{
    unsafe {
        let hdc=GetDC(std::ptr::null_mut());
        if hdc.is_null(){return (label.chars().count() as i32*18+padding).max(1);}
        let old=SelectObject(hdc,dps_primary_font(state));
        let measured=dps_text_width(hdc,label,label.chars().count() as i32*18);
        SelectObject(hdc,old);ReleaseDC(std::ptr::null_mut(),hdc);
        (measured+padding).max(1)
    }
}
fn audit_tab_rect(state:&State,index:i32)->RECT{
    let labels=["Damage","Heal","Tank"];
    let mut left=6;
    for label in labels.iter().take(index.max(0) as usize){left+=audit_label_width(state,label,16).max(70)+4;}
    let label=labels.get(index.max(0) as usize).copied().unwrap_or("Damage");
    let w=audit_label_width(state,label,16).max(70);
    RECT{left,top:reference_tabs_top()+2,right:left+w,bottom:reference_tabs_top()+36}
}
fn audit_toolbar_min_width(state:&State)->i32{
    let w=reference_button_w(480);let gap=reference_button_gap(480);
    6+reference_selector_w(state,480)+6+2*w+audit_label_width(state,"Live",16).max(44)+6+5*w+4*gap+4
}
unsafe fn audit_shared_secondary(hdc:HDC,row:&DpsRow,width:i32)->String{
    if width<170 {dps_spec(row)}else{dps_secondary_compact_measured(hdc,row,width)}
}
#[cfg(test)] mod september_followup_measured_tests{
    use super::*;
    #[test]fn measured_tab_rects_have_padding_and_never_overlap(){
        // Geometry is shared by painting and actual click dispatch.
        let s=State::default();let mut last=0;
        for i in 0..3{let r=audit_tab_rect(&s,i);assert!(r.left>=last);assert!(r.right-r.left>=70);assert_eq!(r.bottom-r.top,34);last=r.right;}
        assert!(audit_toolbar_min_width(&s)>=reference_selector_w(&s,480)+170);
    }
}
"#);
    });
    println!("cargo:rerun-if-changed=build/legacy/build_v1384_followup_measured_layout.rs");
}
