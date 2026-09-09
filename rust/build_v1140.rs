use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1130.rs");
    pub fn run() { main(); }
}

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){let count=source.matches(from).count();assert_eq!(count,1,"v1.14.0 patch `{label}` expected one match, found {count}");*source=source.replacen(from,to,1);}
fn replace_between(source:&mut String,start:&str,end:&str,replacement:&str,label:&str){let count=source.matches(start).count();assert_eq!(count,1,"v1.14.0 patch `{label}` start expected one match, found {count}");let begin=source.find(start).expect("v1.14 start checked");let rel_end=source[begin..].find(end).unwrap_or_else(||panic!("v1.14.0 patch `{label}` end anchor missing"));source.replace_range(begin..begin+rel_end,replacement);}

fn main(){
    previous::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));

    // Feed already-decoded canonical events into the tracker. Do not add a
    // packet-parser wrapper or infer party membership from delayed DPS rows.
    let telemetry_path = out.join("telemetry_v170_fixed.rs");
    let mut telemetry = fs::read_to_string(&telemetry_path).expect("read telemetry");
    replace_once(&mut telemetry, "    fn reset_scene(&mut self) {", "    fn reset_scene(&mut self) {\n        crate::event_tracker::clear_scene();", "tracker scene reset");
    replace_once(&mut telemetry, "    pub fn manual_reset(&mut self) {", "    pub fn manual_reset(&mut self) {\n        crate::event_tracker::reset_counts();", "tracker manual reset");
    replace_once(&mut telemetry, "                    self.local_uid = uuid >> 16;", "                    self.local_uid = uuid >> 16;\n                    crate::event_tracker::set_local_uid(self.local_uid);", "tracker local identity");
    replace_once(&mut telemetry, "            self.local_uid = char_id;", "            self.local_uid = char_id;\n            crate::event_tracker::set_local_uid(self.local_uid);", "tracker container identity");
    replace_once(&mut telemetry, "            self.entities.remove(&uuid);", "            self.entities.remove(&uuid);\n            crate::event_tracker::remove_entity(uuid);", "tracker despawn");
    replace_once(&mut telemetry,
        "        let skill = proto::get_varint_field(damage, 12).unwrap_or(0) as i32;",
        "        let skill = proto::get_varint_field(damage, 12).unwrap_or(0) as i32;\n        crate::event_tracker::observe_skill(skill, attacker_uid, if entity_kind(target_uuid) == ENTITY_PLAYER { target_uuid >> 16 } else { 0 });",
        "tracker resolved skill hit");
    replace_once(&mut telemetry,
        "    fn observe_consumable(&mut self, host: i64, buff_uuid: i32, base_id: i32, info: &[u8]) {",
        "    fn observe_consumable(&mut self, host: i64, buff_uuid: i32, base_id: i32, info: &[u8]) {\n        crate::event_tracker::observe_buff(host, if entity_kind(host) == ENTITY_PLAYER { host >> 16 } else { 0 }, buff_uuid, base_id, false, observed_buff_expiry(info));",
        "tracker buff timestamp");
    replace_once(&mut telemetry, "        if event_type == 2 {", "        if event_type == 2 {\n            crate::event_tracker::observe_buff(host, 0, buff_uuid, 0, true, 0);", "tracker buff removal");
    replace_once(&mut telemetry, "        let mut marker_source = None;\n        for info in proto::len_fields(sync, 2) {", "        let mut marker_source = None;\n        let mut tracker_instances = HashSet::new();\n        for info in proto::len_fields(sync, 2) {", "tracker full snapshot");
    replace_once(&mut telemetry, "            let buff_uuid = proto::get_varint_field(info, 1).unwrap_or(0) as i32;", "            let buff_uuid = proto::get_varint_field(info, 1).unwrap_or(0) as i32;\n            tracker_instances.insert(buff_uuid);", "tracker snapshot instances");
    replace_once(&mut telemetry, "        marker_source\n    }", "        crate::event_tracker::sync_buff_instances(host, &tracker_instances);\n        marker_source\n    }", "tracker snapshot reconciliation");
    fs::write(&telemetry_path, telemetry).expect("write tracker telemetry hooks");

    let overlay_path=out.join("feature_overlays_v170_fixed.rs");
    let mut overlay=fs::read_to_string(&overlay_path).expect("read generated v1.13 overlay");

    // Make the compact toolbar setting affordance understandable without relying
    // on a tooltip or remembering what a one-letter S means.
    replace_once(
        &mut overlay,
        "draw(hdc,\"S\",RECT{left:rc.right-BUTTON_W*3,top:0,right:rc.right-BUTTON_W*2,bottom:TOOLBAR_H},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);",
        "draw(hdc,\"Set\",RECT{left:rc.right-BUTTON_W*3,top:0,right:rc.right-BUTTON_W*2,bottom:TOOLBAR_H},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);",
        "clear settings toolbar label",
    );

    replace_between(
        &mut overlay,
        "unsafe fn paint_mechanics(hdc:HDC,rc:RECT,state:&State){",
        "fn mechanic_timer(",
        include_str!("overlay_v1140_mechanics_patch.txt"),
        "custom tracker mechanics renderer",
    );

    // v1.8.4 made attributes two physical rows when >3, and v1.8.3 can add a
    // fixed tracked-buff section. Account for both when clamping the scroll list,
    // then include custom tracker rows in the same scrollable virtual list.
    replace_between(
        &mut overlay,
        "unsafe fn clamp_scroll(hwnd:HWND,state:&mut State){",
        "unsafe fn open_detail",
        r#"fn mechanics_scroll_metrics(state:&State,bottom:i32)->(usize,usize,i32){let tracked=state.features.read().map(|f|f.mechanic_attributes.tracked.iter().filter(|id|state.mechanics.tracked_attributes.iter().any(|a|a.attr_id==**id)).count()).unwrap_or(0);let attr_rows=(tracked+2)/3;let top=TOOLBAR_H+5+attr_rows as i32*MECH_ATTR_H+MECH_CONSUMABLE_H;let now=now_ms();let mechanics=state.mechanics.rows.iter().filter(|row|row.persistent||row.expires_unix_ms<=0||row.expires_unix_ms>now).count();let total=crate::event_tracker::rows().len().saturating_add(mechanics);let visible=((bottom-top)/MECH_ROW_H).max(0)as usize;(total,visible,top)}
unsafe fn clamp_scroll(hwnd:HWND,state:&mut State){let mut rc:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rc);let(total,visible)=if state.kind==Kind::Dps{(meter_rows(state).len(),visible_dps_rows(rc.bottom))}else{let(total,visible,_)=mechanics_scroll_metrics(state,rc.bottom);(total,visible)};state.scroll=state.scroll.min(total.saturating_sub(visible.max(1)));}
"#,
        "mechanics scroll geometry",
    );

    replace_between(&mut overlay,"struct SettingsState {", "pub unsafe fn create_dps", "struct SettingsState {\nkind: Kind,\nparent: HWND,\npaths: AppPaths,\nfeatures: Arc<RwLock<FeatureSettings>>,\nform: crate::ui::ScrollForm,\nbackground: windows_sys::Win32::Graphics::Gdi::HBRUSH,\n}\n", "native settings form state");
    replace_between(&mut overlay,"unsafe fn open_feature_settings(parent:HWND,state:&mut State){", "fn mutate_features", include_str!("overlay_v1140_settings_patch.txt"), "native combat settings forms");

    replace_once(&mut overlay,"const BUTTON_W: i32 = 32;","const BUTTON_W: i32 = 52;","larger toolbar hit targets");
    replace_once(&mut overlay,"draw(hdc,\"Set\",", "draw(hdc,\"Settings\",", "settings toolbar label");
    replace_once(&mut overlay,"unsafe fn paint(hwnd:HWND,state:&mut State){", "unsafe fn paint(hwnd:HWND,state:&mut State){clamp_scroll(hwnd,state);", "clamp scroll after expiring rows");
    replace_once(&mut overlay,"row: DpsRow,\nscroll: usize,\nfont: HFONT,", "row: DpsRow,\nscroll: usize,\nhscroll: i32,\nfont: HFONT,", "inspector horizontal scroll state");
    replace_once(&mut overlay,"DetailState{row:row.clone(),scroll:0,font:","DetailState{row:row.clone(),scroll:0,hscroll:0,font:","inspector scroll initialization");
    replace_between(&mut overlay,"    let screen_w=windows_sys::Win32::UI::WindowsAndMessaging::GetSystemMetrics(0).max(800);", "    let ptr=Box::into_raw(Box::new(DetailState", r#"    let work=crate::ui::work_area(parent);
    let width=(work.right-work.left-16).clamp(1,1120);
    let height=(work.bottom-work.top-16).clamp(1,680);
    let x=work.left+(work.right-work.left-width)/2;
    let y=work.top+(work.bottom-work.top-height)/2;
"#, "inspector monitor work area");
    replace_once(&mut overlay,"WS_POPUP|WS_THICKFRAME,x,y,width,height,parent", "WS_POPUP|WS_THICKFRAME|0x00200000|0x00100000,x,y,width,height,parent", "inspector native scrollbars");
    replace_between(&mut overlay,"unsafe extern \"system\" fn detail_wnd_proc", "fn detail_attr", include_str!("overlay_v1140_inspector_proc.txt"), "inspector keyboard and scroll geometry");
    replace_once(&mut overlay,"(*ptr).scroll=(*ptr).scroll.min(detail_item_count(&*ptr).saturating_sub(1));", "sync_detail_scroll(hwnd,&mut *ptr);", "clamp refreshed inspector");
    replace_once(&mut overlay,"    match state.mode{DetailMode::Damage|DetailMode::Heal=>paint_detail_skills(hdc,rc,state),DetailMode::Tank=>paint_detail_taken(hdc,rc,state),DetailMode::Buffs=>paint_detail_buffs(hdc,rc,state),DetailMode::Deaths=>paint_detail_deaths(hdc,rc,state)}", r#"    let saved=windows_sys::Win32::Graphics::Gdi::SaveDC(hdc);
    windows_sys::Win32::Graphics::Gdi::IntersectClipRect(hdc,0,281,rc.right,rc.bottom);
    windows_sys::Win32::Graphics::Gdi::SetViewportOrgEx(hdc,-state.hscroll,0,null_mut());
    let table=RECT{right:rc.right.max(1100),..rc};
    match state.mode{DetailMode::Damage|DetailMode::Heal=>paint_detail_skills(hdc,table,state),DetailMode::Tank=>paint_detail_taken(hdc,table,state),DetailMode::Buffs=>paint_detail_buffs(hdc,table,state),DetailMode::Deaths=>paint_detail_deaths(hdc,table,state)}
    windows_sys::Win32::Graphics::Gdi::RestoreDC(hdc,saved);"#, "keep every inspector column reachable");

    replace_once(&mut overlay,"match msg{WM_NCCALCSIZE=>0,",r#"match msg{0x0024=>{if !ptr.is_null()&&!(*ptr).collapsed{crate::ui::min_window(hwnd,lparam,if (*ptr).kind==Kind::Dps{620}else{340},220);}0},0x0100=>{if !ptr.is_null(){overlay_key(hwnd,&mut *ptr,wparam);}0},WM_NCCALCSIZE=>0,"#,"overlay minimum size and keyboard");
    replace_once(&mut overlay,"unsafe fn on_click(hwnd:HWND,state:&mut State,lparam:LPARAM){", "unsafe fn on_click(hwnd:HWND,state:&mut State,lparam:LPARAM){crate::ui::SetFocus(hwnd);", "overlay click keyboard focus");
    replace_once(&mut overlay,"}apply_opacity(hwnd,layout.opacity);",r#"}let mut bounds:RECT=std::mem::zeroed();GetWindowRect(hwnd,&mut bounds);let work=crate::ui::work_area(hwnd);let w=(bounds.right-bounds.left).max(if kind==Kind::Dps{620}else{340}).min(work.right-work.left);SetWindowPos(hwnd,null_mut(),bounds.left,bounds.top,w,(bounds.bottom-bounds.top).min(work.bottom-work.top),SWP_NOACTIVATE|windows_sys::Win32::UI::WindowsAndMessaging::SWP_NOZORDER);crate::ui::fit_window(hwnd,hwnd,false);with_state(hwnd,|s|save_bounds(hwnd,s));apply_opacity(hwnd,layout.opacity);"#,"usable initial overlay bounds");
    replace_once(&mut overlay,"unsafe fn expand_state(hwnd:HWND,state:&mut State){let rect=state.expanded;", "unsafe fn expand_state(hwnd:HWND,state:&mut State){let rect=crate::ui::fit_rect(state.expanded,crate::ui::work_area(hwnd));", "recover expanded overlays after monitor changes");
    replace_once(&mut overlay,"}}InvalidateRect(hwnd,null(),0);}return;}if state.kind==Kind::Dps", "}}refresh_view_detail(state);InvalidateRect(hwnd,null(),0);}return;}if state.kind==Kind::Dps", "history inspector follows selected view");
    replace_once(&mut overlay,"if let Some(row)=dps_name_row_at(hwnd,state,x,y){open_detail(hwnd,state,row);}","if let Some(row)=dps_row_at(hwnd,state,y){open_detail(hwnd,state,row);}","whole DPS row opens inspector");
    replace_once(&mut overlay,"let next=hover_badge_at(hwnd,state,x,y);", "let next=overlay_help(hwnd,state,x,y).or_else(||hover_badge_at(hwnd,state,x,y));", "toolbar and row help");
    replace_once(&mut overlay,"let rows=sorted_rows(&state.dps,state.sort_mode);let row=*rows.get(state.scroll+screen_i)?;","let rows=meter_rows(state);let row=*rows.get(state.scroll+screen_i)?;","badge hover follows history and filters");
    overlay.push_str(include_str!("overlay_v1140_interactions.txt"));

    fs::write(&overlay_path,overlay).expect("write v1.14 generated overlay");

    let win_path=out.join("win_v182_fixed.rs");
    let mut win=fs::read_to_string(&win_path).expect("read generated v1.13 win source");
    replace_once(
        &mut win,
        "TrayAction::OpenSettings=>",
        "TrayAction::OpenEventTracker=>{crate::event_tracker_ui::show(hwnd);},TrayAction::OpenSettings=>",
        "tray event tracker action",
    );
    replace_once(&mut win,"while GetMessageW(&mut msg,null_mut(),0,0)>0{TranslateMessage(&msg);", "while GetMessageW(&mut msg,null_mut(),0,0)>0{if crate::ui::dialog_message(&msg){continue;}TranslateMessage(&msg);", "native form keyboard navigation");
    replace_once(&mut win,"MOD_CONTROL_|MOD_SHIFT_,VK_F10_", "MOD_CONTROL_|MOD_SHIFT_|0x4000,VK_F10_", "ignore hotkey autorepeat");
    replace_once(&mut win,"if base.chat_overlay_enabled{overlay::expand_if_collapsed(state.chat_overlay);ShowWindow", "if base.chat_overlay_enabled{ShowWindow", "preserve collapsed chat on unrelated changes");
    replace_once(&mut win,"if features.dps_overlay_enabled{feature_overlays::expand(state.dps_overlay);ShowWindow", "if features.dps_overlay_enabled{ShowWindow", "preserve collapsed DPS");
    replace_once(&mut win,"if features.mechanics_overlay_enabled{feature_overlays::expand(state.mechanics_overlay);ShowWindow", "if features.mechanics_overlay_enabled{ShowWindow", "preserve collapsed mechanics");
    replace_once(&mut win,"if command==CMD_EXIT{", "if command==1024{update_settings(state,|s|s.chat_overlay_enabled=false);ShowWindow(state.chat_overlay,SW_HIDE);return;}if command==CMD_EXIT{", "chat hide updates tray state");
    replace_once(&mut win,"state.paths.clone(),1,false)", "state.paths.clone(),(lparam as usize).min(7),false)", "settings opens requested page");

    fs::write(&win_path,win).expect("write v1.14 generated win source");

    println!("cargo:rerun-if-changed=build_v1140.rs");
    println!("cargo:rerun-if-changed=overlay_v1140_mechanics_patch.txt");
    println!("cargo:rerun-if-changed=overlay_v1140_settings_patch.txt");
    println!("cargo:rerun-if-changed=overlay_v1140_inspector_proc.txt");
    println!("cargo:rerun-if-changed=overlay_v1140_interactions.txt");
    println!("cargo:rerun-if-changed=src/event_tracker.rs");
    println!("cargo:rerun-if-changed=src/event_tracker_ui.rs");
    println!("cargo:rerun-if-changed=src/settings_ui.rs");
}
