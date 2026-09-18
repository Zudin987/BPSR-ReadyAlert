use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1359_non_meter_layout_fixes.rs");
    pub fn run() { main(); }
}

fn once(source: &mut String, old: &str, new: &str, label: &str) {
    let count = source.matches(old).count();
    assert_eq!(count, 1, "mode height {label}: expected one anchor, found {count}");
    *source = source.replacen(old, new, 1);
}
fn in_fn(source: &mut String, start: &str, end: &str, old: &str, new: &str, label: &str) {
    let a = source.find(start).expect("mode height function start");
    let b = a + source[a..].find(end).expect("mode height function end");
    let mut fragment = source[a..b].to_owned();
    once(&mut fragment, old, new, label);
    source.replace_range(a..b, &fragment);
}

fn main() {
    previous::run();
    let path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"))
        .join("feature_overlays_v170_fixed.rs");
    let mut src = fs::read_to_string(&path).expect("generated overlay source");

    // The automatic mechanics height is ephemeral. The user's manually chosen
    // baseline rect remains independent of tracker activity and survives restart.
    once(&mut src, "hover_y: i32,\n}", "hover_y: i32,\nmechanics_base: RECT,\nmechanics_drag: Option<i32>,\n}", "state fields");
    once(&mut src, "hover_y:0});", "hover_y:0,mechanics_base:RECT{left:0,top:0,right:0,bottom:0},mechanics_drag:None});", "live state initialization");
    once(&mut src, "hover_y:0}}", "hover_y:0,mechanics_base:RECT{left:0,top:0,right:0,bottom:0},mechanics_drag:None}}", "legacy test state");
    once(&mut src, "hover_text: None, hover_x: 0, hover_y: 0,\n", "hover_text: None, hover_x: 0, hover_y: 0,\n            mechanics_base: RECT { left: 0, top: 0, right: 0, bottom: 0 }, mechanics_drag: None,\n", "hover test state");

    // WM_GETMINMAXINFO prevents keyboard/OS and drag resize from changing raid
    // height; the hit test removes vertical and corner resize cursors as well.
    once(&mut src,
        "0x0024=>{if !ptr.is_null()&&!(*ptr).collapsed{crate::ui::min_window(hwnd,lparam,reference_overlay_min_width(&*ptr,(*ptr).scale_percent),overlay_min_height_mode((*ptr).kind,(*ptr).scale_percent,(*ptr).compact_mode));}0},",
        "0x0024=>{if !ptr.is_null()&&!(*ptr).collapsed{let state=&*ptr;let raid=state.kind==Kind::Dps&&raid_active(state);let height=if raid{raid_height_for_window(hwnd,state)}else{overlay_min_height_mode(state.kind,state.scale_percent,state.compact_mode)};crate::ui::min_window(hwnd,lparam,reference_overlay_min_width(state,state.scale_percent),height);if raid{let info=lparam as *mut windows_sys::Win32::UI::WindowsAndMessaging::MINMAXINFO;if !info.is_null(){(*info).ptMinTrackSize.y=height;(*info).ptMaxTrackSize.y=height;}}}0},",
        "raid fixed min/max track height");
    in_fn(&mut src,"unsafe fn resize_hit_test(","// Keep a readable physical",
        "    reference_legacy_resize_hit_test(hwnd, lparam)",
        "    let hit=reference_legacy_resize_hit_test(hwnd,lparam);\n    if !ptr.is_null() && (*ptr).kind==Kind::Dps && raid_active(&*ptr) && !(*ptr).collapsed { raid_horizontal_hit(hit) } else { hit }",
        "horizontal-only raid hit testing");
    in_fn(&mut src,"unsafe fn toggle_raid_mode(","fn raid_consumable_color(",
        "(candidate.bottom-candidate.top).max(reference_raid_full_height(state))",
        "raid_height_for_window(hwnd,state)",
        "raid entry ignores outdated saved heights");
    in_fn(&mut src,"unsafe fn toggle_compact_mode(","fn format_time(",
        "size.1.max(min_h)", "if raid{raid_height_for_window(hwnd,state)}else{size.1.max(min_h)}",
        "raid compact switch fixes height");
    in_fn(&mut src,"unsafe fn expand_state(","unsafe fn save_bounds(",
        "bottom:state.expanded.top.saturating_add((state.expanded.bottom-state.expanded.top).max(min_h))",
        "bottom:state.expanded.top.saturating_add(if state.kind==Kind::Dps&&raid_active(state){raid_height_for_window(hwnd,state)}else{(state.expanded.bottom-state.expanded.top).max(min_h)})",
        "raid re-expansion fixes height");
    in_fn(&mut src,"unsafe fn expand_state(","unsafe fn save_bounds(",
        "state.expanded=rect;persist_overlay_rect(state,rect);sync_consumable_popup(hwnd,state);",
        "state.expanded=rect;if state.kind==Kind::Mechanics{sync_mechanics_height(hwnd,state);}else{persist_overlay_rect(state,rect);}sync_consumable_popup(hwnd,state);",
        "mechanics expansion keeps user baseline");

    // Measure the same grouped rows the painter uses. A 200-ms existing timer
    // handles expiring tracker entries even when no telemetry snapshot arrives.
    once(&mut src,
        "pub unsafe fn update_mechanics(hwnd:HWND,snapshot:MechanicSnapshot){with_state(hwnd,|state|{let new_urgent=snapshot.rows.iter().any(|next|next.priority>=3&&!state.mechanics.rows.iter().any(|old|old.priority>=3&&old.key==next.key));state.mechanics=snapshot;if new_urgent{state.scroll=0;}clamp_scroll(hwnd,state);});InvalidateRect(hwnd,null(),0);}",
        "pub unsafe fn update_mechanics(hwnd:HWND,snapshot:MechanicSnapshot){with_state(hwnd,|state|{let new_urgent=snapshot.rows.iter().any(|next|next.priority>=3&&!state.mechanics.rows.iter().any(|old|old.priority>=3&&old.key==next.key));state.mechanics=snapshot;if new_urgent{state.scroll=0;}sync_mechanics_height(hwnd,state);clamp_scroll(hwnd,state);});InvalidateRect(hwnd,null(),0);}",
        "resize on mechanics update");
    once(&mut src,
        "WM_TIMER=>{if !ptr.is_null()&&(*ptr).kind==Kind::Mechanics&&wparam==MECH_CONSUMABLE_TIMER_ID{InvalidateRect(hwnd,null(),0);}0},",
        "WM_TIMER=>{if !ptr.is_null()&&(*ptr).kind==Kind::Mechanics&&wparam==MECH_CONSUMABLE_TIMER_ID{sync_mechanics_height(hwnd,&mut*ptr);clamp_scroll(hwnd,&mut*ptr);InvalidateRect(hwnd,null(),0);}0},",
        "resize on tracker expiry");
    once(&mut src,
        "WM_EXITSIZEMOVE=>{if !ptr.is_null(){save_bounds(hwnd,&mut*ptr);clamp_scroll(hwnd,&mut*ptr);}0},",
        "0x0231=>{if !ptr.is_null()&&(*ptr).kind==Kind::Mechanics{let mut r:RECT=std::mem::zeroed();if GetWindowRect(hwnd,&mut r)!=0{(*ptr).mechanics_drag=Some(r.bottom-r.top);}}0},\nWM_EXITSIZEMOVE=>{if !ptr.is_null(){save_bounds(hwnd,&mut*ptr);clamp_scroll(hwnd,&mut*ptr);}0},",
        "capture manually resized baseline");
    once(&mut src,
        "unsafe fn save_bounds(hwnd:HWND,state:&mut State){if state.collapsed{return;}let mut rect:RECT=std::mem::zeroed();if GetWindowRect(hwnd,&mut rect)==0{return;}persist_overlay_rect(state,rect);}",
        "unsafe fn save_bounds(hwnd:HWND,state:&mut State){if state.collapsed{return;}let mut rect:RECT=std::mem::zeroed();if GetWindowRect(hwnd,&mut rect)==0{return;}if state.kind==Kind::Mechanics{let prior=(state.mechanics_base.bottom-state.mechanics_base.top).max(0);let actual=(rect.bottom-rect.top).max(1);let changed=state.mechanics_drag.take().map(|start|start!=actual).unwrap_or(false);let baseline=if prior==0||changed{actual}else{prior};state.mechanics_base=RECT{bottom:rect.top.saturating_add(baseline),..rect};persist_overlay_rect(state,state.mechanics_base);state.expanded=rect;if changed{sync_mechanics_height(hwnd,state);}}else{persist_overlay_rect(state,rect);}}",
        "do not persist temporary tracker growth");

    in_fn(&mut src,"unsafe fn persist_current_overlay_scale(","fn scale_scroll_should_persist(",
        "persist_overlay_rect(overlay,rect);}",
        "if overlay.kind==Kind::Mechanics{persist_overlay_rect(overlay,overlay.mechanics_base);overlay.expanded=rect;}else{persist_overlay_rect(overlay,rect);}}",
        "scale finalization keeps mechanics baseline");
    in_fn(&mut src,"unsafe fn set_overlay_scale(","unsafe fn change_overlay_scale(",
        "let target_h=rescale_px((source.bottom-source.top).max(1),old,new).max(overlay_min_height_mode(overlay.kind,new,overlay.compact_mode));",
        "let target_h=if overlay.kind==Kind::Dps&&raid_active(overlay){raid_height_at(new,overlay.compact_mode).min((work.bottom-work.top).max(1))}else{rescale_px((source.bottom-source.top).max(1),old,new).max(overlay_min_height_mode(overlay.kind,new,overlay.compact_mode))};",
        "scale recalculates raid height");
    in_fn(&mut src,"unsafe fn set_overlay_scale(","unsafe fn change_overlay_scale(",
        "overlay.scale_percent=new;overlay.hover_text=None;overlay.expanded=target;",
        "let old_base=(overlay.mechanics_base.bottom-overlay.mechanics_base.top).max(1);overlay.scale_percent=new;overlay.hover_text=None;overlay.expanded=target;if overlay.kind==Kind::Mechanics{overlay.mechanics_base=RECT{bottom:target.top.saturating_add(rescale_px(old_base,old,new)),..target};}",
        "rescale mechanics baseline independently");
    in_fn(&mut src,"unsafe fn set_overlay_scale(","unsafe fn change_overlay_scale(",
        "if persist{persist_overlay_rect(overlay,target);}",
        "if persist{if overlay.kind==Kind::Mechanics{persist_overlay_rect(overlay,overlay.mechanics_base);overlay.expanded=target;}else{persist_overlay_rect(overlay,target);}}",
        "collapsed scale persistence");
    in_fn(&mut src,"unsafe fn set_overlay_scale(","unsafe fn change_overlay_scale(",
        "if persist{persist_overlay_rect(overlay,target);}",
        "if persist{if overlay.kind==Kind::Mechanics{persist_overlay_rect(overlay,overlay.mechanics_base);overlay.expanded=target;}else{persist_overlay_rect(overlay,target);}}",
        "expanded scale persistence");
    in_fn(&mut src,"unsafe fn set_overlay_scale(","unsafe fn change_overlay_scale(",
        "clamp_scroll(state.parent,overlay);sync_consumable_popup(state.parent,overlay);",
        "if overlay.kind==Kind::Mechanics{sync_mechanics_height(state.parent,overlay);}clamp_scroll(state.parent,overlay);sync_consumable_popup(state.parent,overlay);",
        "resync mechanics after scaling");

    src.push_str(r#"
// Raid dimensions follow ten complete rows per column. Only the width is user-resizable.
fn raid_height_at(scale:i32,compact:bool)->i32 {
    let row=if compact{dps_compact_row_h(scale)}else{dps_row_h(scale)};
    scale_px(dps_rows_top()+RAID_ROWS_PER_COLUMN as i32*row+8,scale)
}
unsafe fn raid_height_for_window(hwnd:HWND,state:&State)->i32 {
    let work=crate::ui::work_area(hwnd);
    raid_height_at(state.scale_percent,state.compact_mode).min((work.bottom-work.top).max(1))
}
fn raid_horizontal_hit(hit:LRESULT)->LRESULT {
    match hit as i32 {
        HTTOPLEFT|HTBOTTOMLEFT=>HTLEFT as LRESULT,
        HTTOPRIGHT|HTBOTTOMRIGHT=>HTRIGHT as LRESULT,
        HTTOP|HTBOTTOM=>HTCLIENT as LRESULT,
        _=>hit,
    }
}
// The baseline is the last manually chosen height, not the last auto-expanded height.
fn mechanics_fit_height(base:i32,scale:i32,attributes:usize,rows:usize,work:i32)->i32 {
    let top=TOOLBAR_H+5+mechanic_attr_rows(attributes) as i32*MECH_ATTR_H+MECH_CONSUMABLE_H;
    let needed=scale_px(top+rows.min(MAX_MECHANIC_ROWS_VISIBLE) as i32*MECH_ROW_H+6,scale);
    base.max(needed).min(work.max(1))
}
unsafe fn sync_mechanics_height(hwnd:HWND,state:&mut State){
    if state.kind!=Kind::Mechanics||state.collapsed||state.mechanics_drag.is_some(){return;}
    let mut current:RECT=std::mem::zeroed();if GetWindowRect(hwnd,&mut current)==0{return;}
    let mut base=state.mechanics_base;
    if base.right<=base.left||base.bottom<=base.top{base=current;state.mechanics_base=base;}
    let count=state.features.read().map(|f|f.mechanic_attributes.tracked.iter().filter(|id|state.mechanics.tracked_attributes.iter().any(|attr|attr.attr_id==**id)).count()).unwrap_or(0);
    let rows=ordered_mechanic_rows(state,now_ms()).len();
    let work=crate::ui::work_area(hwnd);
    let width=(current.right-current.left).max(1);
    let height=mechanics_fit_height((base.bottom-base.top).max(1),state.scale_percent,count,rows,work.bottom-work.top);
    let preferred=preferred_axis_anchor(base.top,base.bottom,work.top,work.bottom);
    let origin=RECT{left:base.left,top:base.top,right:base.left.saturating_add(width),bottom:base.bottom};
    let target=anchored_scaled_rect(origin,work,width,height,preferred_axis_anchor(base.left,base.right,work.left,work.right),preferred);
    if current.left!=target.left||current.top!=target.top||current.right!=target.right||current.bottom!=target.bottom{
        SetWindowPos(hwnd,HWND_TOPMOST,target.left,target.top,(target.right-target.left).max(1),(target.bottom-target.top).max(1),SWP_NOACTIVATE);
    }
    state.expanded=target;
}
#[cfg(test)]
mod mode_height_regressions {
    use super::*;
    #[test]fn raid_height_is_mode_specific_and_fits_ten_rows(){
        for scale in [50,75,100,150,200]{
            for compact in [false,true]{
                let h=raid_height_at(scale,compact);
                let logical=physical_extent_to_logical(h,scale);
                let row=if compact{dps_compact_row_h(scale)}else{dps_row_h(scale)};
                assert!(logical>=dps_rows_top()+10*row+8);
            }
        }
    }
    #[test]fn only_raid_horizontal_edges_remain_resizable(){
        assert_eq!(raid_horizontal_hit(HTTOPLEFT as LRESULT),HTLEFT as LRESULT);
        assert_eq!(raid_horizontal_hit(HTBOTTOMRIGHT as LRESULT),HTRIGHT as LRESULT);
        assert_eq!(raid_horizontal_hit(HTTOP as LRESULT),HTCLIENT as LRESULT);
        assert_eq!(raid_horizontal_hit(HTBOTTOM as LRESULT),HTCLIENT as LRESULT);
        assert_eq!(raid_horizontal_hit(HTRIGHT as LRESULT),HTRIGHT as LRESULT);
    }
    #[test]fn mechanics_expands_then_returns_to_manual_baseline(){
        let baseline=260;
        let grown=mechanics_fit_height(baseline,100,4,6,900);
        assert!(grown>baseline);
        assert_eq!(mechanics_fit_height(baseline,100,4,0,900),baseline);
        assert_eq!(mechanics_fit_height(baseline,100,4,6,300),300);
        assert_eq!(mechanics_fit_height(400,100,0,2,900),400);
    }
}
"#);
    fs::write(&path,src).expect("write mode-specific overlay height behavior");
    println!("cargo:rerun-if-changed=build/legacy/build_v1360_mode_height_rules.rs");
}
