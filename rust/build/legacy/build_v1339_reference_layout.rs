use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1338_reference_meter.rs");
    pub fn run() { main(); }
}

fn replace_exact(source: &mut String, from: &str, to: &str, expected: usize, label: &str) {
    let found = source.matches(from).count();
    assert_eq!(found, expected, "reference meter layout: {label} changed ({found} matches)");
    *source = source.replace(from, to);
}

fn main() {
    previous::run();
    let path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"))
        .join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .expect("read generated reference meter")
        .replace("\r\n", "\n");

    // Normal meter, raid, resize, hover and export derive from shared geometry.
    replace_exact(&mut source, "const DPS_CONTROL_H: i32 = 80;", "const DPS_CONTROL_H: i32 = 96;", 1, "header height");
    replace_exact(&mut source, "const DPS_ROW_H: i32 = 33;", "const DPS_ROW_H: i32 = 40;", 1, "card row height");
    replace_exact(&mut source, "const BADGE_W: i32 = 25;", "const BADGE_W: i32 = 29;", 1, "Imagine icon width");
    replace_exact(&mut source, "const BADGE_GAP: i32 = 3;", "const BADGE_GAP: i32 = 4;", 1, "Imagine spacing");
    replace_exact(&mut source, "dps_adaptive_logical_px(23,scale)", "dps_adaptive_logical_px(27,scale)", 1, "Imagine icon height");

    // Two-line target strip: observed HP and time above; real mode total below.
    replace_exact(&mut source, "right:rc.right-6,bottom:target_top+27", "right:rc.right-6,bottom:target_top+43", 1, "target strip");
    replace_exact(&mut source, "top:target_r.top,right:rc.right-10,bottom:target_r.bottom", "top:target_r.top,right:rc.right-10,bottom:target_r.top+21", 1, "time first line");
    replace_exact(&mut source, "top:target_r.top,right:time_r.left-5,bottom:target_r.bottom", "top:target_r.top,right:time_r.left-5,bottom:target_r.top+21", 1, "HP first line");
    replace_exact(&mut source, "top:target_r.top,right:title_right.max(target_r.left+58),bottom:target_r.bottom", "top:target_r.top,right:title_right.max(target_r.left+58),bottom:target_r.top+21", 1, "target title first line");
    replace_exact(&mut source, "let tabs_top=target_r.bottom+4;", "paint_reference_target_total(hdc,target_r,snapshot,state.sort_mode);let tabs_top=target_r.bottom+4;", 1, "target total");
    replace_exact(&mut source,
        "let total_value=compact(snapshot_mode_total(snapshot,state.sort_mode)as f64);SelectObject(hdc,secondary_font);SetTextColor(hdc,crate::ui_modern::BPSR_MUTED);draw(hdc,mode_label(state.sort_mode),RECT{left:225,top:tabs_top,right:rc.right-70,bottom:tabs_top+23},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);SetTextColor(hdc,crate::ui_modern::BPSR_TEXT_SECONDARY);draw(hdc,&total_value,RECT{left:(rc.right-68).max(245),top:tabs_top,right:rc.right-8,bottom:tabs_top+23},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);",
        "SelectObject(hdc,secondary_font);", 1, "remove redundant tab total");

    // Dark class-tinted rounded cards and a mint self outline at any rank.
    replace_exact(&mut source, "crate::ui_modern::tonal_class_surface(class_bg)", "class_bg", 3, "preserve class tint");
    replace_exact(&mut source, "fill(hdc,&r,bg);", "crate::ui_modern::fill_round_rect(hdc,r,bg,crate::ui_modern::RADIUS_SMALL);", 3, "rounded player cards");
    replace_exact(&mut source,
        "unsafe fn paint_pinned_self_frame(hdc:HDC,r:&RECT){crate::ui_modern::stroke_round_rect(hdc,*r,crate::ui_modern::BPSR_ACCENT,crate::ui_modern::RADIUS_SMALL,1);}",
        "unsafe fn paint_pinned_self_frame(hdc:HDC,r:&RECT){crate::ui_modern::stroke_round_rect(hdc,*r,rgb(104,224,183),crate::ui_modern::RADIUS_SMALL,1);}", 1, "mint local frame");
    replace_exact(&mut source,
        "if row.is_local&&forced_self_row(rows.len(),rank.saturating_sub(1),state.scroll,regular_page){paint_pinned_self_frame(hdc,&r);}",
        "if row.is_local{paint_pinned_self_frame(hdc,&r);}", 1, "frame every local row");
    replace_exact(&mut source, "let regular_page=meter_regular_page_size(state,visible);", "", 1, "remove unused page size");
    replace_exact(&mut source,
        "crate::ui_modern::stroke_round_rect(hdc,r,crate::ui_modern::BPSR_ACCENT,crate::ui_modern::RADIUS_SMALL,1)",
        "crate::ui_modern::stroke_round_rect(hdc,r,rgb(104,224,183),crate::ui_modern::RADIUS_SMALL,1)", 2, "mint compact and raid local frames");
    replace_exact(&mut source,
        "SetTextColor(hdc,if row.is_local{crate::ui_modern::BPSR_DANGER}else{crate::ui_modern::BPSR_TEXT_SECONDARY});draw(hdc,&rank.to_string()",
        "SetTextColor(hdc,if row.is_local{rgb(225,169,129)}else{crate::ui_modern::BPSR_TEXT_SECONDARY});draw(hdc,&rank.to_string()", 1, "warm local rank");

    replace_exact(&mut source,
        "let(total_head,active_head,share_head)=dps_header_labels(tier);",
        "let(total_head,active_head,share_head)=dps_header_labels(tier);let active_head=if matches!(tier,DpsLayoutTier::Comfortable)&&rc.right>=760{reference_meter_rate_label(state.sort_mode)}else{active_head};", 1, "mode-aware rate heading");
    replace_exact(&mut source,
        "draw(hdc,\"PLAYER\",RECT{left:hr.left+25",
        "draw(hdc,\"#\",RECT{left:hr.left+3,top:hr.top,right:hr.left+20,bottom:hr.bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);draw(hdc,\"PLAYER\",RECT{left:hr.left+25", 1, "rank column heading");
    replace_exact(&mut source,
        "    let rows=meter_rows(state);let top=dps_rows_top();let row_h=dps_row_h(scale);",
        "    paint_reference_column_headers(hdc,hr,&layout,tier,scale,settings.meter.show_imagines);let rows=meter_rows(state);let top=dps_rows_top();let row_h=dps_row_h(scale);", 1, "class and Imagine headings");

    source.push_str(include_str!("../../src/feature_meter_reference_layout.rs"));
    fs::write(path, source).expect("write refined native meter layout");
    println!("cargo:rerun-if-changed=build/legacy/build_v1339_reference_layout.rs");
    println!("cargo:rerun-if-changed=src/feature_meter_reference_layout.rs");
}
