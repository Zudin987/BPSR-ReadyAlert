use std::{env,fs,path::{Path,PathBuf}};

mod prior {
    include!("build_v1187.rs");
    pub fn run() { main(); }
}

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){
    let count=source.matches(from).count();
    assert_eq!(count,1,"v1.18.8 patch {label} expected one match, found {count}");
    *source=source.replacen(from,to,1);
}
fn replace_between(source:&mut String,start:&str,end:&str,replacement:&str,label:&str){
    let count=source.matches(start).count();
    assert_eq!(count,1,"v1.18.8 patch {label} start expected one match, found {count}");
    let begin=source.find(start).expect("v1.18.8 start anchor");
    let rel_end=source[begin..].find(end).unwrap_or_else(||panic!("v1.18.8 patch {label} end anchor missing"));
    source.replace_range(begin..begin+rel_end,replacement);
}

fn patch_feature_overlay(out:&Path){
    let path=out.join("feature_overlays_v170_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read v1.18.7 generated overlays").replace("\r\n","\n");

    replace_once(
        &mut source,
        r##"#[link(name="gdi32")]extern "system"{fn SetMapMode(hdc:HDC,mode:i32)->i32;fn SetWindowExtEx(hdc:HDC,x:i32,y:i32,prev:*mut c_void)->i32;fn SetViewportExtEx(hdc:HDC,x:i32,y:i32,prev:*mut c_void)->i32;}
"##,
        r##"#[link(name="gdi32")]extern "system"{fn SetMapMode(hdc:HDC,mode:i32)->i32;fn SetWindowExtEx(hdc:HDC,x:i32,y:i32,prev:*mut c_void)->i32;fn SetViewportExtEx(hdc:HDC,x:i32,y:i32,prev:*mut c_void)->i32;}
#[link(name="comctl32")]extern "system"{fn InitCommonControls();}
"##,
        "common-control init",
    );

    replace_once(
        &mut source,
        "fn overlay_min_height(kind:Kind,scale:i32)->i32{scale_px(if kind==Kind::Dps{200}else{260},scale)}\nunsafe fn logical_client_rect",
        r##"fn overlay_min_height(kind:Kind,scale:i32)->i32{scale_px(if kind==Kind::Dps{200}else{260},scale)}
fn snap_overlay_scale(value:i32)->i32{clamp_overlay_scale(((value+5)/10)*10)}
fn scale_is_screen_limited(kind:Kind,scale:i32,work:RECT)->bool{let w=(work.right-work.left).max(1);let h=(work.bottom-work.top).max(1);overlay_min_width(kind,scale)>w||overlay_min_height(kind,scale)>h}
fn preferred_axis_anchor(start:i32,end:i32,work_start:i32,work_end:i32)->i8{const EDGE_ANCHOR_PX:i32=64;let lead=(start-work_start).max(0);let trail=(work_end-end).max(0);if lead<=EDGE_ANCHOR_PX&&lead<=trail{-1}else if trail<=EDGE_ANCHOR_PX&&trail<lead{1}else{0}}
fn anchored_axis(start:i32,end:i32,work_start:i32,work_end:i32,target_extent:i32,preferred:i8)->(i32,i32){
    let work_extent=(work_end-work_start).max(1);let extent=target_extent.max(1).min(work_extent);let max_start=work_end.saturating_sub(extent);let lead=(start-work_start).max(0);let trail=(work_end-end).max(0);
    let raw=match preferred.cmp(&0){std::cmp::Ordering::Less=>work_start.saturating_add(lead.min((work_extent-extent).max(0))),std::cmp::Ordering::Greater=>work_end.saturating_sub(trail.min((work_extent-extent).max(0))).saturating_sub(extent),std::cmp::Ordering::Equal=>{let center=(start as i64+end as i64)/2;center.saturating_sub(extent as i64/2).clamp(i32::MIN as i64,i32::MAX as i64)as i32}};
    let new_start=raw.clamp(work_start,max_start.max(work_start));(new_start,new_start.saturating_add(extent))
}
fn anchored_scaled_rect(source:RECT,work:RECT,target_w:i32,target_h:i32,anchor_x:i8,anchor_y:i8)->RECT{let(left,right)=anchored_axis(source.left,source.right,work.left,work.right,target_w,anchor_x);let(top,bottom)=anchored_axis(source.top,source.bottom,work.top,work.bottom,target_h,anchor_y);RECT{left,top,right,bottom}}
unsafe fn logical_client_rect"##,
        "scale UX helpers",
    );

    replace_once(
        &mut source,
        r##"struct SettingsState {
kind: Kind,
parent: HWND,
paths: AppPaths,
features: Arc<RwLock<FeatureSettings>>,
form: crate::ui::ScrollForm,
background: windows_sys::Win32::Graphics::Gdi::HBRUSH,
}
"##,
        r##"struct SettingsState {
kind: Kind,
parent: HWND,
paths: AppPaths,
features: Arc<RwLock<FeatureSettings>>,
form: crate::ui::ScrollForm,
background: windows_sys::Win32::Graphics::Gdi::HBRUSH,
anchor_x:i8,
anchor_y:i8,
}
"##,
        "settings anchor preference fields",
    );

    replace_once(
        &mut source,
        r##"    let ptr=Box::into_raw(Box::new(SettingsState{kind:state.kind,parent,paths:state.paths.clone(),features:state.features.clone(),form:Default::default(),background:CreateSolidBrush(crate::ui_theme::BG)}));
"##,
        r##"    let mut anchor_rect:RECT=std::mem::zeroed();GetWindowRect(parent,&mut anchor_rect);let anchor_work=crate::ui::work_area(parent);let anchor_x=preferred_axis_anchor(anchor_rect.left,anchor_rect.right,anchor_work.left,anchor_work.right);let anchor_y=preferred_axis_anchor(anchor_rect.top,anchor_rect.bottom,anchor_work.top,anchor_work.bottom);
    let ptr=Box::into_raw(Box::new(SettingsState{kind:state.kind,parent,paths:state.paths.clone(),features:state.features.clone(),form:Default::default(),background:CreateSolidBrush(crate::ui_theme::BG),anchor_x,anchor_y}));
"##,
        "settings initial anchors",
    );

    replace_once(
        &mut source,
        r##"unsafe fn feature_combo(hwnd:HWND,id:i32,x:i32,y:i32,items:&[&str]) {
    let c=feature_control(hwnd,"COMBOBOX",id,"",x,y,155,180,0x00210213);
    for text in items {SendMessageW(c,0x0143,0,wide(text).as_ptr() as isize);}
}
"##,
        r##"unsafe fn feature_combo(hwnd:HWND,id:i32,x:i32,y:i32,items:&[&str]) {
    let c=feature_control(hwnd,"COMBOBOX",id,"",x,y,155,180,0x00210213);
    for text in items {SendMessageW(c,0x0143,0,wide(text).as_ptr() as isize);}
}
unsafe fn feature_combo_sized(hwnd:HWND,id:i32,x:i32,y:i32,w:i32,items:&[&str]) {
    let c=feature_control(hwnd,"COMBOBOX",id,"",x,y,w,180,0x00210213);
    for text in items {SendMessageW(c,0x0143,0,wide(text).as_ptr() as isize);}
}
unsafe fn feature_scale_slider(hwnd:HWND,id:i32,x:i32,y:i32,w:i32)->HWND{
    let c=feature_control(hwnd,"msctls_trackbar32",id,"",x,y,w,20,0x00000010);
    let range=(((OVERLAY_SCALE_MAX as u32)<<16)|(OVERLAY_SCALE_MIN as u32&0xffff))as isize;SendMessageW(c,0x0406,1,range);c
}
"##,
        "slider helpers",
    );

    replace_between(
        &mut source,
        "unsafe fn build_feature_form(hwnd:HWND,state:&mut SettingsState) {",
        "unsafe fn refresh_feature_form",
        r##"unsafe fn build_feature_form(hwnd:HWND,state:&mut SettingsState) {
    InitCommonControls();
    let title=if state.kind==Kind::Dps{"DPS Meter"}else{"Dungeon Mechanics"};let title_hwnd=feature_label(hwnd,0,title,18,14,320,24);crate::ui_theme::set_font(title_hwnd,crate::ui_theme::FontRole::Heading);
    feature_label(hwnd,7000,"",18,50,86,22);feature_button(hwnd,7001,"−",106,44,30);feature_button(hwnd,7002,"+",140,44,30);
    feature_label(hwnd,7004,"",184,50,96,22);feature_button(hwnd,7005,"−",284,44,30);feature_button(hwnd,7006,"+",318,44,30);feature_button(hwnd,7008,"Reset 100%",352,44,88);
    feature_label(hwnd,0,"Collapse",450,50,64,22);feature_combo_sized(hwnd,7003,516,44,88,&["Right","Bottom","Left","Top"]);
    feature_scale_slider(hwnd,7007,184,72,256);feature_label(hwnd,7009,"",450,74,154,22);
    if state.kind==Kind::Dps{
        let section_hwnd=feature_label(hwnd,0,"DISPLAY",18,96,180,18);crate::ui_theme::set_font(section_hwnd,crate::ui_theme::FontRole::Secondary);
        let display=["Damage share %","Healing share %","Tank share %","Death count","Battle Imagine badges","Target / HP summary","Active rate"];
        for(i,label)in display.iter().enumerate(){let col=(i/4)as i32;let row=(i%4)as i32;feature_check(hwnd,7100+i as i32,label,18+col*286,116+row*27,270);}
        feature_label(hwnd,0,"ROSTER & HISTORY",18,232,220,18);
        let roster=["Only contributors","Party only","Always show self","Remember scroll between encounters"];
        for(i,label)in roster.iter().enumerate(){let col=(i/2)as i32;let row=(i%2)as i32;feature_check(hwnd,7107+i as i32,label,18+col*286,254+row*27,270);}
        feature_label(hwnd,0,"Visible rows",18,318,90,22);feature_combo(hwnd,7010,108,312,&["Auto","5","10","20","30","50"]);
        feature_label(hwnd,7011,"",286,318,150,22);feature_button(hwnd,7012,"−",438,312,36);feature_button(hwnd,7013,"+",480,312,36);
        feature_label(hwnd,0,"Changes save immediately.",18,358,220,20);feature_button(hwnd,2,"Close",494,352,92);
    }else{
        feature_label(hwnd,7020,"",18,96,220,22);feature_button(hwnd,7021,"Event Tracker",534,88,150);
        feature_label(hwnd,0,"TRACKED COMBAT ATTRIBUTES",18,130,300,18);
        let rows=(ATTRIBUTE_CATALOG.len()+2)/3;
        for(i,(_,label))in ATTRIBUTE_CATALOG.iter().enumerate(){feature_check(hwnd,7200+i as i32,label,18+(i/rows)as i32*224,152+(i%rows)as i32*26,216);}
        let bottom=152+rows as i32*26;
        feature_label(hwnd,0,"Choose up to 8 stats. Food, Serum and Event Tracker rows appear automatically.",18,bottom+10,520,34);
        feature_button(hwnd,2,"Close",598,bottom+12,92);
    }
    refresh_feature_form(hwnd,state);
}
"##,
        "settings form layout",
    );

    replace_once(
        &mut source,
        r##"    let scale=settings_overlay_scale(state);windows_sys::Win32::UI::WindowsAndMessaging::SetWindowTextW(fc(hwnd,7004),wide(&format!("Scale: {}%",scale)).as_ptr());crate::ui::EnableWindow(fc(hwnd,7005),(scale>OVERLAY_SCALE_MIN)as i32);crate::ui::EnableWindow(fc(hwnd,7006),(scale<OVERLAY_SCALE_MAX)as i32);
"##,
        r##"    let scale=settings_overlay_scale(state);let limited=settings_scale_is_limited(state,scale);
    windows_sys::Win32::UI::WindowsAndMessaging::SetWindowTextW(fc(hwnd,7004),wide(&format!("Scale: {}%",scale)).as_ptr());windows_sys::Win32::UI::WindowsAndMessaging::SetWindowTextW(fc(hwnd,7009),wide(if limited{"Limited by screen"}else{""}).as_ptr());
    SendMessageW(fc(hwnd,7007),0x0405,1,scale as isize);crate::ui::EnableWindow(fc(hwnd,7005),(scale>OVERLAY_SCALE_MIN)as i32);crate::ui::EnableWindow(fc(hwnd,7006),(scale<OVERLAY_SCALE_MAX)as i32);crate::ui::EnableWindow(fc(hwnd,7008),(scale!=OVERLAY_SCALE_DEFAULT)as i32);
"##,
        "settings scale refresh",
    );

    replace_once(
        &mut source,
        r##"unsafe fn settings_overlay_scale(state:&SettingsState)->i32{let ptr=GetWindowLongPtrW(state.parent,GWLP_USERDATA)as *mut State;if ptr.is_null(){scale_slot(&state.paths,state.kind).percent}else{(*ptr).scale_percent}}
"##,
        r##"unsafe fn settings_overlay_scale(state:&SettingsState)->i32{let ptr=GetWindowLongPtrW(state.parent,GWLP_USERDATA)as *mut State;if ptr.is_null(){scale_slot(&state.paths,state.kind).percent}else{(*ptr).scale_percent}}
unsafe fn settings_scale_is_limited(state:&SettingsState,scale:i32)->bool{let work=crate::ui::work_area(state.parent);if scale_is_screen_limited(state.kind,scale,work){return true;}if scale<=100{return false;}let ptr=GetWindowLongPtrW(state.parent,GWLP_USERDATA)as *mut State;if ptr.is_null(){return false;}let overlay=&*ptr;let mut rect=overlay.expanded;if !overlay.collapsed&&GetWindowRect(state.parent,&mut rect)==0{return false;}let work_w=(work.right-work.left).max(1);let work_h=(work.bottom-work.top).max(1);(rect.right-rect.left)>=work_w-1||(rect.bottom-rect.top)>=work_h-1}
"##,
        "screen limit status",
    );

    replace_between(
        &mut source,
        "unsafe fn change_overlay_scale(state:&SettingsState,delta:i32){",
        "unsafe extern \"system\" fn settings_wnd_proc",
        r##"unsafe fn set_overlay_scale(state:&mut SettingsState,requested:i32){
    let ptr=GetWindowLongPtrW(state.parent,GWLP_USERDATA)as *mut State;if ptr.is_null(){return;}let overlay=&mut*ptr;let old=overlay.scale_percent;let new=snap_overlay_scale(requested);if new==old{return;}
    let source=if overlay.collapsed{overlay.expanded}else{let mut r:RECT=std::mem::zeroed();if GetWindowRect(state.parent,&mut r)==0{return;}r};let work=crate::ui::work_area(state.parent);
    let spans_x=source.left<=work.left+1&&source.right>=work.right-1;let spans_y=source.top<=work.top+1&&source.bottom>=work.bottom-1;if !spans_x{state.anchor_x=preferred_axis_anchor(source.left,source.right,work.left,work.right);}if !spans_y{state.anchor_y=preferred_axis_anchor(source.top,source.bottom,work.top,work.bottom);}
    let target_w=rescale_px((source.right-source.left).max(1),old,new).max(overlay_min_width(overlay.kind,new));let target_h=rescale_px((source.bottom-source.top).max(1),old,new).max(overlay_min_height(overlay.kind,new));let target=anchored_scaled_rect(source,work,target_w,target_h,state.anchor_x,state.anchor_y);
    overlay.scale_percent=new;overlay.hover_text=None;
    if overlay.collapsed{persist_overlay_rect(overlay,target);resize_collapsed_for_scale(state.parent,overlay);}else{SetWindowPos(state.parent,HWND_TOPMOST,target.left,target.top,(target.right-target.left).max(1),(target.bottom-target.top).max(1),SWP_NOACTIVATE);persist_overlay_rect(overlay,target);clamp_scroll(state.parent,overlay);sync_consumable_popup(state.parent,overlay);}
    InvalidateRect(state.parent,null(),0);if !overlay.consumable_hwnd.is_null(){InvalidateRect(overlay.consumable_hwnd,null(),0);}
}
unsafe fn change_overlay_scale(state:&mut SettingsState,delta:i32){let current=settings_overlay_scale(state);set_overlay_scale(state,current.saturating_add(delta));}
"##,
        "edge-aware scale changer",
    );

    replace_once(
        &mut source,
        r##"        WM_SIZE=>0,

        crate::ui::WM_REVEAL_FOCUS=>0,
"##,
        r##"        WM_SIZE=>0,
        0x0114=>{if lparam as HWND==fc(hwnd,7007){let raw=SendMessageW(fc(hwnd,7007),0x0400,0,0)as i32;let snapped=snap_overlay_scale(raw);SendMessageW(fc(hwnd,7007),0x0405,1,snapped as isize);set_overlay_scale(state,snapped);refresh_feature_form(hwnd,state);return 0;}DefWindowProcW(hwnd,msg,wparam,lparam)},

        crate::ui::WM_REVEAL_FOCUS=>0,
"##,
        "slider scroll handling",
    );

    replace_once(
        &mut source,
        r##"            if id==7005||id==7006{change_overlay_scale(state,if id==7005{-10}else{10});refresh_feature_form(hwnd,state);return 0;}
"##,
        r##"            if id==7005||id==7006||id==7008{if id==7008{set_overlay_scale(state,OVERLAY_SCALE_DEFAULT);}else{change_overlay_scale(state,if id==7005{-10}else{10});}refresh_feature_form(hwnd,state);return 0;}
"##,
        "scale buttons",
    );

    replace_once(
        &mut source,
        r##"        0x0135|0x0138=>{let hdc=wparam as HDC;windows_sys::Win32::Graphics::Gdi::SetBkColor(hdc,crate::ui_theme::BG);SetTextColor(hdc,crate::ui_theme::TEXT);state.background as isize}
"##,
        r##"        0x0135|0x0138=>{let hdc=wparam as HDC;windows_sys::Win32::Graphics::Gdi::SetBkColor(hdc,crate::ui_theme::BG);let warning=lparam as HWND==fc(hwnd,7009)&&settings_scale_is_limited(state,settings_overlay_scale(state));SetTextColor(hdc,if warning{crate::ui_theme::WARNING}else{crate::ui_theme::TEXT});state.background as isize}
"##,
        "screen limit warning color",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1188_scale_ux_tests{
    use super::*;
    #[test]
    fn slider_scale_snaps_to_ten_percent_steps(){
        assert_eq!(snap_overlay_scale(30),30);
        assert_eq!(snap_overlay_scale(34),30);
        assert_eq!(snap_overlay_scale(35),40);
        assert_eq!(snap_overlay_scale(304),300);
    }
    #[test]
    fn anchor_detection_distinguishes_edges_and_floating_windows(){
        assert_eq!(preferred_axis_anchor(10,510,0,1920),-1);
        assert_eq!(preferred_axis_anchor(1410,1910,0,1920),1);
        assert_eq!(preferred_axis_anchor(710,1210,0,1920),0);
    }
    #[test]
    fn preferred_edge_keeps_its_gap_when_scaling(){
        assert_eq!(anchored_axis(10,510,0,1920,750,-1),(10,760));
        assert_eq!(anchored_axis(1410,1910,0,1920,750,1),(1160,1910));
        assert_eq!(anchored_axis(20,220,0,1080,300,-1),(20,320));
        assert_eq!(anchored_axis(870,1070,0,1080,300,1),(770,1070));
    }
    #[test]
    fn floating_windows_preserve_center(){
        assert_eq!(anchored_axis(710,1210,0,1920,750,0),(585,1335));
    }
    #[test]
    fn screen_limit_status_uses_scaled_minimums(){
        let small=RECT{left:0,top:0,right:1280,bottom:680};
        let full=RECT{left:0,top:0,right:1920,bottom:1080};
        assert!(!scale_is_screen_limited(Kind::Dps,100,small));
        assert!(scale_is_screen_limited(Kind::Dps,300,small));
        assert!(!scale_is_screen_limited(Kind::Dps,300,full));
        assert!(scale_is_screen_limited(Kind::Mechanics,300,small));
    }
    #[test]
    fn oversized_scaled_window_is_safely_clamped_to_work_area(){
        let work=RECT{left:0,top:0,right:1280,bottom:680};
        let source=RECT{left:770,top:300,right:1270,bottom:500};
        let out=anchored_scaled_rect(source,work,1500,600,1,0);
        assert_eq!(out.left,0);
        assert_eq!(out.right,1280);
        assert_eq!(out.bottom-out.top,600);
        assert!(out.top>=0&&out.bottom<=680);
    }
}
"#);

    fs::write(path,source).expect("write v1.18.8 feature overlay");
}

fn main(){
    prior::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_feature_overlay(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1188.rs");
}
