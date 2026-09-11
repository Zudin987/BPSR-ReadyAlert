use std::{env,fs,path::{Path,PathBuf}};

mod prior {
    include!("build_v1211.rs");
    pub fn run(){main();}
}

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){
    let count=source.matches(from).count();
    assert_eq!(count,1,"v1.21.2 patch {label} expected one match, found {count}");
    *source=source.replacen(from,to,1);
}

fn replace_between(source:&mut String,start:&str,end:&str,replacement:&str,label:&str){
    let count=source.matches(start).count();
    assert_eq!(count,1,"v1.21.2 patch {label} start expected one match, found {count}");
    let begin=source.find(start).expect("v1.21.2 start anchor");
    let rel_end=source[begin..].find(end).unwrap_or_else(||panic!("v1.21.2 patch {label} end anchor missing"));
    source.replace_range(begin..begin+rel_end,replacement);
}

fn patch_feature_overlay(out:&Path){
    let path=out.join("feature_overlays_v170_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read v1.21.1 generated overlays").replace("\r\n","\n");

    replace_once(
        &mut source,
        "const DPS_CONSUMABLE_KEY:u32=0x00030201;",
        "const DPS_CONSUMABLE_KEY:u32=0x00030201;\nconst DPS_CONSUMABLE_STACK_H:i32=26;\nconst DPS_CONSUMABLE_SEGMENT_W:i32=5;\nconst DPS_CONSUMABLE_SEGMENT_H:i32=5;\nconst DPS_CONSUMABLE_SEGMENT_GAP:i32=1;\nconst DPS_CONSUMABLE_LABEL_W:i32=8;\nconst DPS_CONSUMABLE_LABEL_GAP:i32=2;\nconst DPS_CONSUMABLE_ORANGE:u32=rgb(238,139,47);",
        "segmented consumable constants",
    );

    replace_once(
        &mut source,
        "// Color-keyed transparent pixels mean there is no overlay background beside\n    // the meter; only the circular F/S indicators are visible.",
        "// Color-keyed transparent pixels mean there is no overlay background beside\n    // the meter; only the compact F/S segmented indicators are visible.",
        "consumable popup comment",
    );

    replace_between(
        &mut source,
        "unsafe fn popup_status_at<'a>(state:&'a State,x:i32,y:i32,now:i64)->Option<(&'a ConsumableStatus,&'static str)>{",
        "unsafe fn paint_consumable_popup",
        r#"unsafe fn popup_status_at<'a>(state:&'a State,x:i32,y:i32,now:i64)->Option<(&'a ConsumableStatus,&'static str)>{
    if y<0{return None;}let scale=dps_layout_scale(state);let row_h=dps_row_h(scale);let stack_h=consumable_stack_height(scale);let screen_i=(y/row_h)as usize;let logical_height=physical_extent_to_logical((state.expanded.bottom-state.expanded.top).max(0),state.scale_percent);let physical=visible_dps_rows_scaled(logical_height,scale);let shown=meter_page_rows(state,physical);let row=shown.get(screen_i).map(|(_,row)|*row)?;let iy=screen_i as i32*row_h+((row_h-2-stack_h)/2).max(0);if y<iy||y>=iy+stack_h{return None;}let max_x=consumable_total_width(scale);if x<0||x>=max_x{return None;}let local_y=y-iy;let line_gap=dps_adaptive_logical_px(2,scale);let line_h=((stack_h-line_gap)/2).max(1);if local_y<line_h{return active_consumable(row.food.as_ref(),now).map(|s|(s,"Food"));}if local_y>=line_h+line_gap{return active_consumable(row.serum.as_ref(),now).map(|s|(s,"Serum"));}None
}
"#,
        "responsive consumable hit testing",
    );

    replace_between(
        &mut source,
        "unsafe fn paint_consumable_popup(hwnd:HWND,state:&ConsumablePopupState){",
        "unsafe fn on_consumable_mouse_move",
        r#"unsafe fn paint_consumable_popup(hwnd:HWND,state:&ConsumablePopupState){
    let parent_ptr=GetWindowLongPtrW(state.parent,GWLP_USERDATA)as *mut State;let mut ps:PAINTSTRUCT=std::mem::zeroed();let hdc=BeginPaint(hwnd,&mut ps);if hdc.is_null(){return;}let selected=if parent_ptr.is_null(){100}else{(*parent_ptr).scale_percent};let layout_scale=if parent_ptr.is_null(){100}else{dps_layout_scale(&*parent_ptr)};let mut physical:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut physical);configure_overlay_dc(hdc,selected);let rc=RECT{left:0,top:0,right:physical_extent_to_logical(physical.right.max(0),selected),bottom:physical_extent_to_logical(physical.bottom.max(0),selected)};fill(hdc,&rc,DPS_CONSUMABLE_KEY);
    if !parent_ptr.is_null(){let parent=&*parent_ptr;if !parent.collapsed{let now=now_ms();let row_h=dps_row_h(layout_scale);let stack_h=consumable_stack_height(layout_scale);let gap=dps_adaptive_logical_px(2,layout_scale);let line_h=((stack_h-gap)/2).max(1);let visible=(rc.bottom/row_h).max(0)as usize;let shown=meter_page_rows(parent,visible);for(screen_i,(_,row))in shown.iter().enumerate(){let row=*row;let y=screen_i as i32*row_h+((row_h-2-stack_h)/2).max(0);if let Some(food)=active_consumable(row.food.as_ref(),now){paint_consumable_segment_line(hdc,2,y,line_h,"F",food,now,layout_scale);}if let Some(serum)=active_consumable(row.serum.as_ref(),now){paint_consumable_segment_line(hdc,2,y+line_h+gap,line_h,"S",serum,now,layout_scale);}}}}
    reset_overlay_dc(hdc);EndPaint(hwnd,&ps);
}
"#,
        "segmented consumable popup paint",
    );

    replace_between(
        &mut source,
        "unsafe fn paint_consumable_ring(hdc:HDC,x:i32,y:i32,label:&str,status:&ConsumableStatus,now:i64){",
        "unsafe fn hover_badge_at",
        r#"fn consumable_segment_count(scale:i32)->usize{let scale=clamp_overlay_scale(scale);if scale>=90{5}else if scale>=65{4}else{3}}
fn consumable_stack_height(scale:i32)->i32{dps_adaptive_logical_px(DPS_CONSUMABLE_STACK_H,scale).max(16)}
fn consumable_segment_width(scale:i32)->i32{dps_adaptive_logical_px(DPS_CONSUMABLE_SEGMENT_W,scale).max(3)}
fn consumable_segment_height(scale:i32)->i32{dps_adaptive_logical_px(DPS_CONSUMABLE_SEGMENT_H,scale).max(3)}
fn consumable_segment_gap(scale:i32)->i32{dps_adaptive_logical_px(DPS_CONSUMABLE_SEGMENT_GAP,scale).max(1)}
fn consumable_label_width(scale:i32)->i32{dps_adaptive_logical_px(DPS_CONSUMABLE_LABEL_W,scale).max(6)}
fn consumable_label_gap(scale:i32)->i32{dps_adaptive_logical_px(DPS_CONSUMABLE_LABEL_GAP,scale).max(1)}
fn consumable_total_width(scale:i32)->i32{let count=consumable_segment_count(scale)as i32;2+consumable_label_width(scale)+consumable_label_gap(scale)+count*consumable_segment_width(scale)+(count-1).max(0)*consumable_segment_gap(scale)}
fn consumable_fraction(status:&ConsumableStatus,now:i64)->f64{if status.expires_unix_ms<=0||status.duration_ms<=0{1.0}else{(status.expires_unix_ms.saturating_sub(now).max(0)as f64/status.duration_ms.max(1)as f64).clamp(0.0,1.0)}}
fn consumable_is_urgent(status:&ConsumableStatus,now:i64)->bool{status.expires_unix_ms>0&&status.expires_unix_ms.saturating_sub(now)>0&&status.expires_unix_ms.saturating_sub(now)<=30_000}
fn consumable_blink_on(now:i64)->bool{(now/400)%2==0}
fn consumable_bar_color(status:&ConsumableStatus,now:i64)->u32{if consumable_is_urgent(status,now){if consumable_blink_on(now){crate::ui_theme::CRITICAL}else{rgb(108,34,34)}}else{DPS_CONSUMABLE_ORANGE}}
unsafe fn paint_consumable_segment_line(hdc:HDC,x:i32,y:i32,line_h:i32,label:&str,status:&ConsumableStatus,now:i64,scale:i32){
    let count=consumable_segment_count(scale);let seg_w=consumable_segment_width(scale);let seg_h=consumable_segment_height(scale).min(line_h.max(1));let seg_gap=consumable_segment_gap(scale);let label_w=consumable_label_width(scale);let label_gap=consumable_label_gap(scale);let bar_x=x+label_w+label_gap;let bar_y=y+((line_h-seg_h)/2).max(0);let fraction=consumable_fraction(status,now);let color=consumable_bar_color(status,now);let track=rgb(62,67,74);let old_bk=SetBkMode(hdc,TRANSPARENT as i32);SelectObject(hdc,GetStockObject(DEFAULT_GUI_FONT));SetTextColor(hdc,if consumable_is_urgent(status,now){crate::ui_theme::CRITICAL}else{crate::ui_theme::TEXT_SECONDARY});draw(hdc,label,RECT{left:x,top:y,right:x+label_w,bottom:y+line_h},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);SetBkMode(hdc,old_bk);for index in 0..count{let left=bar_x+index as i32*(seg_w+seg_gap);let r=RECT{left,top:bar_y,right:left+seg_w,bottom:bar_y+seg_h};fill(hdc,&r,track);let local=(fraction*count as f64-index as f64).clamp(0.0,1.0);if local>0.001{let filled=((seg_w as f64)*local).round().clamp(1.0,seg_w as f64)as i32;fill(hdc,&RECT{left:r.left,top:r.top,right:r.left+filled,bottom:r.bottom},color);}}
}
"#,
        "segmented consumable helpers",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1212_segmented_consumable_tests{
    use super::*;

    fn status(remaining:i64,duration:i64,now:i64)->ConsumableStatus{ConsumableStatus{buff_id:1,name:"Food".into(),expires_unix_ms:now+remaining,duration_ms:duration}}

    #[test]
    fn segment_count_keeps_small_scales_readable(){
        assert_eq!(consumable_segment_count(50),3);
        assert_eq!(consumable_segment_count(75),4);
        assert_eq!(consumable_segment_count(100),5);
    }

    #[test]
    fn warning_begins_at_thirty_seconds(){
        let now=100_000;
        assert!(consumable_is_urgent(&status(30_000,300_000,now),now));
        assert!(!consumable_is_urgent(&status(30_001,300_000,now),now));
    }

    #[test]
    fn normal_bar_is_orange(){
        let now=100_000;
        assert_eq!(consumable_bar_color(&status(120_000,300_000,now),now),DPS_CONSUMABLE_ORANGE);
    }

    #[test]
    fn popup_width_fits_the_full_five_segment_layout(){
        assert!(consumable_total_width(100)<=DPS_CONSUMABLE_POPUP_W);
    }
}
"#);

    fs::write(path,source).expect("write v1.21.2 feature overlay");
}

fn main(){
    prior::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_feature_overlay(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1212.rs");
}
