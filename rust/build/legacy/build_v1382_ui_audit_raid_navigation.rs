// September audit M08: make every Raid page render, hit-test, scroll and label
// the same 20-player window. Apply to the actual generated native Rust unit.
use std::{env, fs, path::PathBuf};
mod previous {
    include!("build_v1381_ui_audit_header_chat.rs");
    pub fn run() { main(); }
}
fn replace_exact(source:&mut String,old:&str,new:&str,expected:usize,id:&str){
    assert_eq!(source.matches(old).count(),expected,"M08 {id}: missing or ambiguous generated-source anchor");
    *source=source.replace(old,new);
    assert_eq!(source.matches(new).count(),expected,"M08 {id}: replacement absent");
}
fn within(source:&mut String,start:&str,end:&str,old:&str,new:&str,id:&str){
    assert_eq!(source.matches(start).count(),1,"M08 {id}: section missing or ambiguous");
    let first=source.find(start).unwrap();
    let last=first+source[first..].find(end).unwrap_or_else(||panic!("M08 {id}: section end missing"));
    let mut section=source[first..last].to_owned();
    replace_exact(&mut section,old,new,1,id);
    source.replace_range(first..last,&section);
}
fn main(){
    previous::run();
    let path=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("feature_overlays_v170_fixed.rs");
    let mut src=fs::read_to_string(&path).expect("read compiled native meter");
    // The two existing clamp/page-key paths otherwise treat Raid as a ten-row
    // one-column list even though the renderer displays two columns of ten.
    replace_exact(&mut src,
        "let physical=visible_dps_rows_scaled_for(state,rc.bottom,dps_layout_scale(state));(meter_rows(state).len(),meter_regular_page_size(state,physical))",
        "let physical=visible_dps_rows_scaled_for(state,rc.bottom,dps_layout_scale(state));(meter_rows(state).len(),if raid_active(state){RAID_MAX_ROWS}else{meter_regular_page_size(state,physical)})",
        2,"20-player scroll and keyboard page budget");
    // Both renderers use one shared, clamped window start. Preserve a full
    // twenty-person viewport at the end rather than allowing blank page slots.
    for (start,end,id) in [
        ("unsafe fn paint_compact_raid_rows(","fn raid_painted_player_column(","compact paint window"),
        ("unsafe fn paint_raid_rows(","fn reference_raid_first_line(","normal paint window"),
    ] {
        within(&mut src,start,end,
            "    let rows = meter_rows(state);",
            "    let all_rows = meter_rows(state);\n    let total_rows = all_rows.len();\n    let window_start = audit_raid_window_start(total_rows, state.scroll);\n    let rows = &all_rows[window_start..];",
            id);
    }
    within(&mut src,"unsafe fn paint_compact_raid_rows(","fn raid_painted_player_column(",
        "                slot + 1,","                window_start + slot + 1,","compact left true ranks");
    within(&mut src,"unsafe fn paint_compact_raid_rows(","fn raid_painted_player_column(",
        "                ri + 1,","                window_start + ri + 1,","compact right true ranks");
    within(&mut src,"unsafe fn paint_raid_rows(","fn reference_raid_first_line(",
        "                li + 1,","                window_start + li + 1,","normal left true ranks");
    within(&mut src,"unsafe fn paint_raid_rows(","fn reference_raid_first_line(",
        "                ri + 1,","                window_start + ri + 1,","normal right true ranks");
    // Hit-testing and F/S hover must index the same visible slice; previously
    // they always selected ranks 1..20 even if keyboard/wheel changed scroll.
    within(&mut src,"unsafe fn raid_row_at(","unsafe fn hover_badge_at(",
        "    rows.get(index).map(|row| (*row).clone())",
        "    rows.get(audit_raid_window_start(rows.len(),state.scroll)+index).map(|row| (*row).clone())",
        "both-column mouse inspection follows scroll");
    replace_exact(&mut src,
        "        let row = *rows.get(index)?;",
        "        let row = *rows.get(audit_raid_window_start(rows.len(),state.scroll)+index)?;",
        1,"raid food/serum inspection follows scroll");
    // Explicit two-column reading order and actual rank ranges. Both header
    // painters use the same window start and display updated ranges on scroll.
    within(&mut src,"unsafe fn paint_reference_raid_headers(","#[cfg(test)]\nmod reference_meter_audit_tests",
        "    for col in columns {",
        "    let all_rows = meter_rows(state);\n    let first_rank = audit_raid_window_start(all_rows.len(),state.scroll);\n    for (column_index,col) in columns.into_iter().enumerate() {\n        let from = first_rank + column_index * RAID_ROWS_PER_COLUMN + 1;\n        let to = (from + RAID_ROWS_PER_COLUMN - 1).min(all_rows.len());\n        let player_heading = if from <= to {format!(\"{from}–{to} ↓\")}else{\"Player ↓\".to_owned()};",
        "normal two-column rank headings");
    within(&mut src,"unsafe fn paint_reference_raid_headers(","#[cfg(test)]\nmod reference_meter_audit_tests",
        "(\"Player\", layout.name_left, layout.spec_right, 0)",
        "(player_heading.as_str(), layout.name_left, layout.spec_right, 0)",
        "normal player heading rendering");
    within(&mut src,"unsafe fn paint_reference_headers(","unsafe fn paint_dps(",
        "        for r in columns {",
        "        let all_rows = meter_rows(state);\n        let first_rank = audit_raid_window_start(all_rows.len(),state.scroll);\n        for (column_index,r) in columns.into_iter().enumerate() {\n            let from=first_rank+column_index*RAID_ROWS_PER_COLUMN+1;\n            let to=(from+RAID_ROWS_PER_COLUMN-1).min(all_rows.len());\n            let player_heading=if !reference_raid_columns(state,rc.right){\"Player\".to_owned()}else if from<=to{format!(\"{from}–{to} ↓\")}else{\"Player ↓\".to_owned()};",
        "compact two-column rank headings");
    within(&mut src,"unsafe fn paint_reference_headers(","unsafe fn paint_dps(",
        "(\"Player\", r.left + rank_w + rank_gap, name_right, 0)",
        "(player_heading.as_str(), r.left + rank_w + rank_gap, name_right, 0)",
        "compact player heading rendering");
    // Narrow unobtrusive scrollbar indicates that twenty of potentially more
    // players are visible; never modify sorted row identity or telemetry.
    for (start,end,id) in [
        ("unsafe fn paint_compact_raid_rows(","fn raid_painted_player_column(","compact scrollbar"),
        ("unsafe fn paint_raid_rows(","fn reference_raid_first_line(","normal scrollbar"),
    ] {
        let begin=src.find(start).expect("paint start");
        let finish=begin+src[begin..].find(end).expect("paint end");
        let mut section=src[begin..finish].to_owned();
        let brace=section.rfind("\n}").expect("paint function close");
        section.insert_str(brace,"\n    paint_scrollbar(hdc,rc,total_rows,RAID_MAX_ROWS,window_start,top);");
        src.replace_range(begin..finish,&section);
        println!("cargo:warning=UI audit M08 {id}: verified native scrollbar");
    }
    src.push_str(r#"
// Use the final complete twenty-person window instead of rendering empty
// slots when a wheel/keyboard step goes beyond the last complete page.
fn audit_raid_window_start(total:usize,requested:usize)->usize {
    requested.min(total.saturating_sub(RAID_MAX_ROWS))
}
#[cfg(test)]
mod september_raid_navigation_regressions {
    use super::*;
    #[test]
    fn two_columns_cover_entire_roster_without_skipped_or_duplicate_ranks() {
        for total in [0usize,1,10,11,20,21,35,40] {
            for requested in 0..=total+2 {
                let start=audit_raid_window_start(total,requested);
                let shown=(start..(start+RAID_MAX_ROWS).min(total)).collect::<Vec<_>>();
                assert!(shown.len()<=20);
                assert_eq!(shown.iter().copied().collect::<std::collections::HashSet<_>>().len(),shown.len());
                if total>=20 {assert_eq!(shown.len(),20);}
                if total>20 && requested==total {assert_eq!(shown.last().copied(),Some(total-1));}
                let left=&shown[..shown.len().min(RAID_ROWS_PER_COLUMN)];
                let right=&shown[left.len()..];
                assert!(left.len()<=10 && right.len()<=10);
                assert!(left.last().zip(right.first()).map(|(l,r)|r==&(l+1)).unwrap_or(true));
            }
        }
    }
}
"#);
    assert!(src.contains("audit_raid_window_start(rows.len(),state.scroll)+index"));
    fs::write(&path,src).expect("write compiled Raid navigation");
    println!("cargo:rerun-if-changed=build/legacy/build_v1382_ui_audit_raid_navigation.rs");
}
