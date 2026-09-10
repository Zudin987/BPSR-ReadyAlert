use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1140.rs");
    pub fn run() { main(); }
}

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){let count=source.matches(from).count();assert_eq!(count,1,"v1.15.0 patch `{label}` expected one match, found {count}");*source=source.replacen(from,to,1);}
fn replace_between(source:&mut String,start:&str,end:&str,replacement:&str,label:&str){let count=source.matches(start).count();assert_eq!(count,1,"v1.15.0 patch `{label}` start expected one match, found {count}");let begin=source.find(start).expect("v1.15 start checked");let rel_end=source[begin..].find(end).unwrap_or_else(||panic!("v1.15.0 patch `{label}` end anchor missing"));source.replace_range(begin..begin+rel_end,replacement);}
fn replace_tail(source:&mut String,start:&str,replacement:&str,label:&str){let count=source.matches(start).count();assert_eq!(count,1,"v1.15.0 patch `{label}` expected one tail anchor, found {count}");let begin=source.find(start).expect("tail checked");source.truncate(begin);source.push_str(replacement);}

fn main(){
    previous::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let overlay_path=out.join("feature_overlays_v170_fixed.rs");
    let mut overlay=fs::read_to_string(&overlay_path).expect("read generated v1.14 overlay");

    // ReadyAlert's combat surface uses a taller, labeled information hierarchy.
    replace_once(&mut overlay,"const DPS_CONTROL_H: i32 = 31;","const DPS_CONTROL_H: i32 = 80;","meter header height");
    replace_once(&mut overlay,"const DPS_ROW_H: i32 = 32;","const DPS_ROW_H: i32 = 42;","readable meter rows");
    replace_once(&mut overlay,"if (*ptr).kind==Kind::Dps{620}else{340},220","if (*ptr).kind==Kind::Dps{620}else{340},260","meter minimum height");

    replace_between(
        &mut overlay,
        "unsafe fn paint_toolbar(hdc:HDC,rc:RECT,state:&State){",
        "fn toolbar_title(state:&State)->String{",
        r#"unsafe fn paint_toolbar(hdc:HDC,rc:RECT,state:&State){let toolbar=RECT{left:0,top:0,right:rc.right,bottom:TOOLBAR_H};fill(hdc,&toolbar,rgb(20,27,33));fill(hdc,&RECT{left:0,top:TOOLBAR_H-2,right:rc.right,bottom:TOOLBAR_H},rgb(66,211,190));let title_right=if state.kind==Kind::Dps{toolbar_action_rects(rc.right)[0].0.left-5}else{rc.right-BUTTON_W*3-5};SetTextColor(hdc,rgb(235,242,245));draw(hdc,&toolbar_title(state),RECT{left:10,top:0,right:title_right.max(80),bottom:TOOLBAR_H},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);if state.kind==Kind::Dps{for(index,(r,label))in toolbar_action_rects(rc.right).iter().enumerate(){let active=index==1&&state.history_index.is_none();paint_tab(hdc,r.left,r.top,r.right-r.left,label,active);}}SetTextColor(hdc,rgb(170,187,198));draw(hdc,"Settings",RECT{left:rc.right-BUTTON_W*3,top:0,right:rc.right-BUTTON_W*2,bottom:TOOLBAR_H},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);draw(hdc,"◀",RECT{left:rc.right-BUTTON_W*2,top:0,right:rc.right-BUTTON_W,bottom:TOOLBAR_H},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);draw(hdc,"×",RECT{left:rc.right-BUTTON_W,top:0,right:rc.right,bottom:TOOLBAR_H},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}
"#,
        "ReadyAlert tactical toolbar",
    );
    replace_between(
        &mut overlay,
        "fn toolbar_title(state:&State)->String{",
        "#[derive(Clone,Copy)]struct DpsRowLayout",
        r#"fn toolbar_title(state:&State)->String{if state.kind==Kind::Mechanics{return format!("READYALERT // MECHANICS   {}",capture_short(state));}if let Some((notice,until))=state.share_notice.as_ref(){if *until>now_ms(){return format!("READYALERT // COMBAT   {notice}");}}let view=match state.history_index{Some(index)=>format!("HISTORY {}/{}",index+1,state.history.len()),None=>"LIVE".into()};let mut title=format!("READYALERT // COMBAT   {view}");if let Some(pb)=personal_best_text(state){title.push_str("   •   ");title.push_str(&pb);}title}
"#,
        "compact branded toolbar title",
    );

    // Replace the packed A/E string with fixed, explicitly labeled columns. The
    // player column is the only flexible area; optional badges are sacrificed
    // before Total / Active / Encounter / Share meaning can disappear.
    let mut meter_patch=include_str!("overlay_v1150_meter_patch.txt").to_string();
    if let Some(begin)=meter_patch.find("unsafe fn dps_row_at("){let end_rel=meter_patch[begin..].find("unsafe fn paint_dps").expect("meter patch dps_row_at end");meter_patch.replace_range(begin..begin+end_rel,"");}
    replace_between(&mut overlay,"#[derive(Clone,Copy)]struct DpsRowLayout","unsafe fn draw_percent(",&meter_patch,"adaptive DPS meter renderer");

    replace_between(
        &mut overlay,
        "unsafe fn on_click(hwnd:HWND,state:&mut State,lparam:LPARAM){",
        "unsafe fn on_wheel",
        r#"unsafe fn on_click(hwnd:HWND,state:&mut State,lparam:LPARAM){crate::ui::SetFocus(hwnd);if state.collapsed{expand_state(hwnd,state);return;}let x=lo_signed(lparam);let y=hi_signed(lparam);let mut rc:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rc);if y<TOOLBAR_H{if x>=rc.right-BUTTON_W{PostMessageW(state.main_hwnd,0x0111,if state.kind==Kind::Dps{CMD_HIDE_DPS as usize}else{CMD_HIDE_MECHANICS as usize},0);}else if x>=rc.right-BUTTON_W*2{collapse(hwnd,state);}else if x>=rc.right-BUTTON_W*3{open_feature_settings(hwnd,state);}else if state.kind==Kind::Dps{for(index,(r,_))in toolbar_action_rects(rc.right).iter().enumerate(){if x>=r.left&&x<r.right&&y>=r.top&&y<r.bottom{match index{0=>history_older(state),1=>{state.history_index=None;state.scroll=0;},2=>history_newer(state),3=>export_view(state,ExportFormat::Csv),4=>export_view(state,ExportFormat::Json),5=>copy_view(state),6=>{if state.history_index.is_none(){archive_live_snapshot(state);crate::telemetry::request_manual_reset();state.dps=DpsSnapshot::default();state.scroll=0;}else{state.history_index=None;state.scroll=0;}},_=>{}}refresh_view_detail(state);clamp_scroll(hwnd,state);InvalidateRect(hwnd,null(),0);return;}}drag_window(hwnd);}else{drag_window(hwnd);}return;}if state.kind==Kind::Dps&&y<dps_rows_top(){let tabs_top=TOOLBAR_H+35;if y>=tabs_top&&y<tabs_top+23{match x{8..=78=>state.sort_mode=SortMode::Damage,83..=148=>state.sort_mode=SortMode::Heal,153..=218=>state.sort_mode=SortMode::Tank,_=>{}}state.scroll=0;refresh_view_detail(state);clamp_scroll(hwnd,state);InvalidateRect(hwnd,null(),0);}return;}if state.kind==Kind::Dps{if let Some(row)=dps_row_at(hwnd,state,y){open_detail(hwnd,state,row);}}}
"#,
        "toolbar action routing and Reset move",
    );

    // Tier 0 is a real tier and the protocol value is already display-ready.
    replace_between(
        &mut overlay,
        "unsafe fn paint_badge(hdc:HDC,x:i32,y:i32,badge:&ImagineBadge){",
        "fn imagine_asset(skill_id:i32)->Option<&'static [u8]>{",
        r#"unsafe fn paint_badge(hdc:HDC,x:i32,y:i32,badge:&ImagineBadge){let r=RECT{left:x,top:y,right:x+BADGE_W,bottom:y+23};if !draw_imagine_asset(hdc,r,badge.skill_id){fill(hdc,&r,badge_color(&badge.icon_key));SetTextColor(hdc,rgb(248,250,252));let short=if badge.icon_key.trim().is_empty()||badge.icon_key.eq_ignore_ascii_case("BI"){"BI"}else{badge.icon_key.as_str()};draw(hdc,short,r,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}let tier=RECT{left:r.right-11,top:r.bottom-11,right:r.right,bottom:r.bottom};fill(hdc,&tier,rgb(8,11,15));SetTextColor(hdc,rgb(255,102,102));draw(hdc,&crate::model::imagine_tier_display(badge.tier).to_string(),tier,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}
"#,
        "correct T0/T5 Imagine badge",
    );
    replace_between(
        &mut overlay,
        "unsafe fn hover_badge_at(hwnd:HWND,state:&State,x:i32,y:i32)->Option<String>{",
        "unsafe fn paint_hover",
        r#"unsafe fn hover_badge_at(hwnd:HWND,state:&State,x:i32,y:i32)->Option<String>{if state.kind!=Kind::Dps||state.collapsed||!state.features.read().map(|f|f.meter.show_imagines).unwrap_or(true){return None;}let mut rc:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rc);let top=dps_rows_top();if y<top||y>=rc.bottom{return None;}let screen_i=((y-top)/DPS_ROW_H)as usize;if screen_i>=visible_dps_rows(rc.bottom){return None;}let rows=meter_rows(state);let row=*rows.get(state.scroll+screen_i)?;if row.is_dead||(rc.right-14)<700{return None;}let r=RECT{left:6,top:top+screen_i as i32*DPS_ROW_H,right:rc.right-8,bottom:top+screen_i as i32*DPS_ROW_H+DPS_ROW_H-2};let layout=dps_row_layout(r,true,row);let mut bx=layout.badge_left;for badge in row.imagines.iter().take(2){if x>=bx&&x<bx+BADGE_W&&y>=r.top+8&&y<r.top+31{return Some(format!("{} · {}",badge.name,crate::model::imagine_tier_label(badge.tier)));}bx+=BADGE_W+BADGE_GAP;}None}
"#,
        "correct Imagine hover tier",
    );

    // Reframe the inspector as grouped cards, while retaining all detailed tabs.
    let detail_patch=include_str!("overlay_v1150_detail_patch.txt")
        .replace("let saved=SaveDC(hdc);IntersectClipRect(hdc,0,281,rc.right,rc.bottom);SetViewportOrgEx(hdc,-state.hscroll,0,null_mut());","let saved=windows_sys::Win32::Graphics::Gdi::SaveDC(hdc);windows_sys::Win32::Graphics::Gdi::IntersectClipRect(hdc,0,281,rc.right,rc.bottom);windows_sys::Win32::Graphics::Gdi::SetViewportOrgEx(hdc,-state.hscroll,0,null_mut());")
        .replace("RestoreDC(hdc,saved);EndPaint(hwnd,&ps);","windows_sys::Win32::Graphics::Gdi::RestoreDC(hdc,saved);EndPaint(hwnd,&ps);");
    replace_between(&mut overlay,"unsafe fn paint_detail(hwnd:HWND,state:&DetailState){","unsafe fn paint_detail_skills",&detail_patch,"grouped Entity Inspector summary");

    // Make combat settings read as sections rather than a checkbox wall.
    replace_between(&mut overlay,"unsafe fn build_feature_form(hwnd:HWND,state:&mut SettingsState) {","unsafe fn refresh_feature_form",include_str!("overlay_v1150_settings_patch.txt"),"grouped combat settings");
    replace_once(&mut overlay,"WS_POPUP|WS_THICKFRAME|0x00c00000|0x00080000|0x00200000|0x00100000,0,0,w,h", "WS_POPUP|WS_THICKFRAME|0x00c00000|0x00080000|0x00200000|0x00100000|0x02000000,0,0,w,h", "clip feature settings children");
    replace_once(&mut overlay,"WM_ERASEBKGND=>{let mut rc:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rc);FillRect(wparam as HDC,&rc,state.background);1}","WM_ERASEBKGND=>1,\n        WM_PAINT=>{let mut ps:PAINTSTRUCT=std::mem::zeroed();let hdc=BeginPaint(hwnd,&mut ps);if !hdc.is_null(){let mut rc:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rc);FillRect(hdc,&rc,state.background);}EndPaint(hwnd,&ps);0}","stable feature settings background paint");

    // Apply the ReadyAlert accent consistently to custom-drawn combat surfaces.
    overlay=overlay.replace("rgb(63,133,255)","rgb(66,211,190)");
    overlay=overlay.replace("rgb(99,199,255)","rgb(66,211,190)");

    replace_tail(
        &mut overlay,
        "unsafe fn overlay_help(hwnd:HWND,state:&State,x:i32,y:i32)->Option<String>{",
        r#"unsafe fn overlay_help(hwnd:HWND,state:&State,x:i32,y:i32)->Option<String>{
    if state.collapsed{return Some("Click, Enter or Space to expand".into());}
    let mut rc:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rc);
    if y<TOOLBAR_H{
        if x>=rc.right-BUTTON_W{return Some("Hide overlay • restore from tray or Ctrl+Shift+F10".into());}
        if x>=rc.right-BUTTON_W*2{return Some("Collapse to the configured screen edge".into());}
        if x>=rc.right-BUTTON_W*3{return Some("Open ReadyAlert overlay settings (S)".into());}
        if state.kind==Kind::Dps{for(index,(r,_))in toolbar_action_rects(rc.right).iter().enumerate(){if x>=r.left&&x<r.right{return Some(match index{0=>"Older encounter",1=>"Return to live encounter",2=>"Newer encounter",3=>"Export selected encounter as CSV",4=>"Export selected encounter as JSON",5=>"Copy selected encounter summary",6=>"Reset the live encounter",_=>"Combat action"}.into());}}}
        return Some(toolbar_title(state));
    }
    if state.kind==Kind::Dps {
        if hover_badge_at(hwnd,state,x,y).is_some(){return None;}
        let target_top=TOOLBAR_H+4;if y>=target_top&&y<target_top+27{return Some("Target context • name/ID, HP and encounter time".into());}
        let tabs_top=TOOLBAR_H+35;if y>=tabs_top&&y<tabs_top+23{return Some("Damage / Heal / Tank • keyboard 1 / 2 / 3".into());}
        if let Some(row)=dps_row_at(hwnd,state,y){return Some(format!("{} • Total / Active / Encounter / Share • Click to inspect",dps_identity(&row)));}
    } else {
        let (_,_,top)=mechanics_scroll_metrics(state,rc.bottom);if y>=top{let index=state.scroll+((y-top)/MECH_ROW_H)as usize;let tracker=crate::event_tracker::rows();if let Some(row)=tracker.get(index){return Some(format!("{} • {}",row.label,row.detail));}let now=now_ms();if let Some(row)=state.mechanics.rows.iter().filter(|r|r.persistent||r.expires_unix_ms<=0||r.expires_unix_ms>now).nth(index.saturating_sub(tracker.len())){return Some(format!("{} {}",row.label,row.target.as_deref().unwrap_or("")));}}
    }
    None
}
"#,
        "updated ReadyAlert help targets",
    );

    fs::write(&overlay_path,overlay).expect("write v1.15 generated overlay");
    println!("cargo:rerun-if-changed=build_v1150.rs");
    println!("cargo:rerun-if-changed=overlay_v1150_meter_patch.txt");
    println!("cargo:rerun-if-changed=overlay_v1150_detail_patch.txt");
    println!("cargo:rerun-if-changed=overlay_v1150_settings_patch.txt");
    println!("cargo:rerun-if-changed=src/model_v170.rs");
    println!("cargo:rerun-if-changed=src/ui.rs");
}
