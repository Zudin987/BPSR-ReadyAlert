use std::{fs, path::Path};
fn replace_once(source:&mut String,from:&str,to:&str,label:&str){let count=source.matches(from).count();assert_eq!(count,1,"compact meter patch {label} expected one match, found {count}");*source=source.replacen(from,to,1);}
fn replace_between(source:&mut String,start:&str,end:&str,replacement:&str,label:&str){let sc=source.matches(start).count();assert_eq!(sc,1,"compact meter patch {label} start expected one match, found {sc}");let begin=source.find(start).expect("compact meter start anchor");let finish=source[begin..].find(end).map(|o|begin+o).expect("compact meter end anchor");source.replace_range(begin..finish,replacement);}
pub fn run(out:&Path){let path=out.join("feature_overlays_v170_fixed.rs");let mut source=fs::read_to_string(&path).expect("read generated feature overlay").replace("\r\n","\n");
replace_between(&mut source,r#"unsafe fn sync_consumable_popup(parent:HWND,state:&mut State){"#,r#"unsafe fn popup_status_at"#,r#"unsafe fn sync_consumable_popup(parent:HWND,state:&mut State){
    if state.kind==Kind::Dps&&(raid_active(state)||state.compact_mode){if !state.consumable_hwnd.is_null(){ShowWindow(state.consumable_hwnd,0);}return;}
    if state.kind!=Kind::Dps||state.consumable_hwnd.is_null()||IsWindow(state.consumable_hwnd)==0{return;}if state.collapsed{ShowWindow(state.consumable_hwnd,0);return;}
    let mut r:RECT=std::mem::zeroed();if GetWindowRect(parent,&mut r)==0{return;}let scale=dps_layout_scale(state);let top=scale_px(dps_rows_top(),state.scale_percent);let popup_w=scale_px(DPS_CONSUMABLE_POPUP_W,dps_readability_percent(scale));let popup_gap=scale_px(DPS_CONSUMABLE_POPUP_GAP,dps_readability_percent(scale));let h=(r.bottom-r.top-top).max(1);SetWindowPos(state.consumable_hwnd,HWND_TOPMOST,r.left-popup_w-popup_gap,r.top+top,popup_w,h,SWP_NOACTIVATE);ShowWindow(state.consumable_hwnd,SW_SHOW);
}
"#,"compact popup hiding");
replace_once(&mut source,r#"fn dps_image_dimensions(client_width:i32,row_count:usize)->(i32,i32){
    let rows=i32::try_from(row_count.max(1)).unwrap_or(i32::MAX/DPS_ROW_H.max(1));
    let width=client_width.max(620);
    let height=dps_rows_top().saturating_add(rows.saturating_mul(DPS_ROW_H)).saturating_add(4).max(180);
    (width,height)
}
"#,r#"fn dps_image_dimensions(client_width:i32,row_count:usize)->(i32,i32){
    let rows=i32::try_from(row_count.max(1)).unwrap_or(i32::MAX/DPS_ROW_H.max(1));
    let width=client_width.max(620);
    let height=dps_rows_top().saturating_add(rows.saturating_mul(DPS_ROW_H)).saturating_add(4).max(180);
    (width,height)
}
fn dps_image_dimensions_for(state:&State,client_width:i32,row_count:usize)->(i32,i32){let raid=raid_active(state);let rows=if raid{row_count.min(RAID_ROWS_PER_COLUMN)}else{row_count}.max(1);let row_h=if state.compact_mode{dps_compact_row_h(100)}else{DPS_ROW_H};let top=if state.compact_mode{TOOLBAR_H+2}else{dps_rows_top()};let width=if raid{client_width.max(if state.compact_mode{760}else{RAID_BASE_WIDTH})}else{client_width.max(if state.compact_mode{360}else{620})};let height=top.saturating_add((rows as i32).saturating_mul(row_h)).saturating_add(6).max(if state.compact_mode{140}else{180});(width,height)}
"#,"image dimensions mode");
replace_once(&mut source,r#"let logical_width=physical_extent_to_logical(client.right-client.left,state.scale_percent);let(width,height)=dps_image_dimensions(logical_width,row_count);"#,r#"let logical_width=physical_extent_to_logical(client.right-client.left,state.scale_percent);let(width,height)=dps_image_dimensions_for(state,logical_width,row_count);"#,"image dimensions call");
replace_once(&mut source,r#"    paint_dps(memory_dc,full,state);"#,r#"    paint_dps_with_raid(memory_dc,full,state);"#,"raid image painter");
replace_once(&mut source,r#"0x0024=>{if !ptr.is_null()&&!(*ptr).collapsed{crate::ui::min_window(hwnd,lparam,overlay_min_width((*ptr).kind,(*ptr).scale_percent),overlay_min_height((*ptr).kind,(*ptr).scale_percent));}0},"#,r#"0x0024=>{if !ptr.is_null()&&!(*ptr).collapsed{crate::ui::min_window(hwnd,lparam,overlay_min_width_mode((*ptr).kind,(*ptr).scale_percent,(*ptr).compact_mode),overlay_min_height_mode((*ptr).kind,(*ptr).scale_percent,(*ptr).compact_mode));}0},"#,"state min wndproc");
fs::write(path,source).expect("write compact meter generated overlay");}
