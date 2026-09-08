use std::{env, fs, path::PathBuf};

fn patch_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(
        count, 1,
        "v1.8.1 generated-source patch `{label}` expected one match, found {count}"
    );
    *source = source.replacen(from, to, 1);
}

fn main() {
    // v1.8 still includes the compact v1.7 Win32/protocol implementation from
    // OUT_DIR. Apply small, assertion-guarded compatibility patches here rather
    // than silently carrying the v1.8 regressions into the generated source.
    // Each patch must match exactly once so an upstream source edit fails loudly
    // in CI instead of producing subtly different runtime behavior.
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));

    // Git's Windows checkout may materialize CRLF source while these audited
    // patch anchors intentionally use LF. Normalize the in-memory generated-source
    // inputs so CI behavior does not depend on core.autocrlf / runner platform.
    let mut telemetry = fs::read_to_string("src/telemetry_v170.rs")
        .expect("read telemetry_v170.rs")
        .replace("\r\n", "\n");

    patch_once(
        &mut telemetry,
        "const FANTASY_MARKER_BUFF_ID: i32 = 2_199_999;\nconst SEGMENT_BOUNDARY_DELAY: Duration = Duration::from_secs(3);\nconst MAX_SEGMENT: Duration = Duration::from_secs(20 * 60);",
        "const FANTASY_MARKER_BUFF_ID: i32 = 2_199_999;\n// CN Resonance emits WipeDetected when this buff is applied to the local player.\nconst WIPE_BUFF_BASE_ID: i32 = 510_072;\nconst SEGMENT_BOUNDARY_DELAY: Duration = Duration::from_secs(3);",
        "CN wipe id / remove timer reset",
    );

    patch_once(
        &mut telemetry,
        "            if uuid == self.last_target && self.encounter_started.is_some() {\n                self.arm_boundary();\n            }\n            self.entities.remove(&uuid);",
        "            // A target disappearing is normal during add waves, phase swaps and\n            // boss cleanup. CN Resonance does not treat despawn as a wipe/reset.\n            self.entities.remove(&uuid);",
        "do not reset on target despawn",
    );

    patch_once(
        &mut telemetry,
        "                if old_state != ACTOR_STATE_DEAD && new_state == ACTOR_STATE_DEAD && uuid == self.last_target {\n                    self.arm_boundary();\n                }",
        "                if old_state != ACTOR_STATE_DEAD && new_state == ACTOR_STATE_DEAD && uuid == self.last_target {\n                    // Boss/add death is not an encounter boundary. Keep the accumulated\n                    // meter until a real wipe, scene re-entry or manual reset.\n                }",
        "do not reset on target death",
    );

    patch_once(
        &mut telemetry,
        "        // CN-style segment lifecycle: start on eligible outgoing player damage,\n        // split only after an observed boundary (scene/target death/disappear/wipe)\n        // and a short guard delay. Ordinary idle time never wipes the meter.",
        "        // CN-style segment lifecycle: start on eligible outgoing player damage.\n        // Scene re-entry resets immediately; wipes arm a short guard boundary.\n        // Boss/add death, despawn and ordinary idle time never wipe the meter.",
        "segment lifecycle comment",
    );

    patch_once(
        &mut telemetry,
        "    fn prepare_segment(&mut self, now: Instant) {\n        if self.encounter_started.is_some_and(|started| started.elapsed() >= MAX_SEGMENT) {\n            self.reset_encounter_keep_roster();\n        }\n        if let Some(boundary) = self.pending_boundary.take() {\n            if now.saturating_duration_since(boundary) >= SEGMENT_BOUNDARY_DELAY {\n                self.reset_encounter_keep_roster();\n            }\n        }\n        self.encounter_started.get_or_insert(now);\n    }",
        "    fn prepare_segment(&mut self, now: Instant) {\n        // Do not consume a fresh boundary before its guard delay has elapsed.\n        // v1.8 used take() here, so a fast post-wipe hit could permanently lose\n        // the pending reset and merge two pulls.\n        if self.pending_boundary.is_some_and(|boundary| {\n            now.saturating_duration_since(boundary) >= SEGMENT_BOUNDARY_DELAY\n        }) {\n            self.reset_encounter_keep_roster();\n        }\n        self.encounter_started.get_or_insert(now);\n    }",
        "preserve pending wipe boundary",
    );

    patch_once(
        &mut telemetry,
        "            let base_id = proto::get_varint_field(info, 2).unwrap_or(0) as i32;\n            self.observe_consumable(host, buff_uuid, base_id, info);",
        "            let base_id = proto::get_varint_field(info, 2).unwrap_or(0) as i32;\n            if event_type == 1\n                && base_id == WIPE_BUFF_BASE_ID\n                && entity_kind(host) == ENTITY_PLAYER\n                && host >> 16 == self.local_uid\n            {\n                self.arm_boundary();\n            }\n            self.observe_consumable(host, buff_uuid, base_id, info);",
        "CN wipe buff transition",
    );

    patch_once(
        &mut telemetry,
        "        2_032_061..=2_032_286 | 2_032_311..=2_032_383 => (Food, \"Cuisine\"),",
        "        // CN classifies the 2032xxx family from buff metadata/icon data. Keep\n        // the specific labels above, then accept the rest of the cuisine family\n        // so seasonal/new recipes do not silently show as Food: None.\n        2_032_061..=2_032_999 => (Food, \"Cuisine\"),",
        "broaden cuisine family",
    );

    fs::write(out.join("telemetry_v170_fixed.rs"), telemetry)
        .expect("write generated telemetry source");

    let mut overlay = fs::read_to_string("src/feature_overlays_v170.rs")
        .expect("read feature_overlays_v170.rs")
        .replace("\r\n", "\n")
        .replace("return\"", "return \"");

    patch_once(
        &mut overlay,
        "feature_settings::{self, FeatureSettings, ATTRIBUTE_CATALOG},",
        "feature_settings::{self, format_attr_value, FeatureSettings, ATTRIBUTE_CATALOG},",
        "attribute formatter import",
    );
    patch_once(
        &mut overlay,
        "compact_attr(attr.value)",
        "format_attr_value(attr.attr_id, attr.value)",
        "render attributes as percentages",
    );

    // Keep the standard thick-frame resize semantics but make the whole popup a
    // client area. A custom NCHITTEST below restores resize grips without the
    // bright Win11 non-client strip that v1.8 left above the overlay.
    patch_once(
        &mut overlay,
        "SetForegroundWindow, SetLayeredWindowAttributes, SetWindowLongPtrW, SetWindowPos, ShowWindow,",
        "SetForegroundWindow, SetLayeredWindowAttributes, SetWindowLongPtrW, SetWindowPos, ShowWindow, TrackMouseEvent,",
        "TrackMouseEvent import",
    );
    patch_once(
        &mut overlay,
        "CREATESTRUCTW, CW_USEDEFAULT, GWLP_USERDATA, HWND_TOPMOST, IDC_ARROW, LWA_ALPHA,\nSWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_SHOW, WM_ERASEBKGND, WM_EXITSIZEMOVE,\nWM_LBUTTONDOWN, WM_MOUSEWHEEL, WM_NCCREATE, WM_NCDESTROY, WM_PAINT, WM_SIZE,\nWS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_THICKFRAME, WNDCLASSW,",
        "CREATESTRUCTW, CW_USEDEFAULT, GWLP_USERDATA, HWND_TOPMOST, IDC_ARROW, LWA_ALPHA,\nHTBOTTOM, HTBOTTOMLEFT, HTBOTTOMRIGHT, HTCLIENT, HTLEFT, HTRIGHT, HTTOP, HTTOPLEFT, HTTOPRIGHT,\nSWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_SHOW, TME_LEAVE, TRACKMOUSEEVENT, WM_ERASEBKGND, WM_EXITSIZEMOVE,\nWM_LBUTTONDOWN, WM_MOUSELEAVE, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_NCCALCSIZE, WM_NCHITTEST, WM_NCCREATE, WM_NCDESTROY, WM_PAINT, WM_SIZE,\nWS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_THICKFRAME, WNDCLASSW,",
        "borderless-resize and hover imports",
    );

    patch_once(
        &mut overlay,
        "settings_hwnd: HWND,\n}",
        "settings_hwnd: HWND,\nhover_text: Option<String>,\nhover_x: i32,\nhover_y: i32,\n}",
        "hover state fields",
    );
    patch_once(
        &mut overlay,
        "detail_hwnd:null_mut(),detail_uid:0,settings_hwnd:null_mut()}",
        "detail_hwnd:null_mut(),detail_uid:0,settings_hwnd:null_mut(),hover_text:None,hover_x:0,hover_y:0}",
        "hover state initialization",
    );

    patch_once(
        &mut overlay,
        "let ptr=GetWindowLongPtrW(hwnd,GWLP_USERDATA)as *mut State;match msg{WM_ERASEBKGND=>1,WM_PAINT=>",
        "let ptr=GetWindowLongPtrW(hwnd,GWLP_USERDATA)as *mut State;match msg{WM_NCCALCSIZE=>0,WM_NCHITTEST=>{if ptr.is_null(){DefWindowProcW(hwnd,msg,wparam,lparam)}else{resize_hit_test(hwnd,lparam)}},WM_ERASEBKGND=>1,WM_MOUSEMOVE=>{if !ptr.is_null(){on_mouse_move(hwnd,&mut*ptr,lparam);}0},WM_MOUSELEAVE=>{if !ptr.is_null(){(*ptr).hover_text=None;InvalidateRect(hwnd,null(),0);}0},WM_PAINT=>",
        "custom non-client frame and hover messages",
    );

    patch_once(
        &mut overlay,
        "unsafe fn on_wheel(hwnd:HWND,state:&mut State,wparam:WPARAM){if state.collapsed{return;}",
        r#"unsafe fn resize_hit_test(hwnd:HWND,lparam:LPARAM)->LRESULT{let x=lo_signed(lparam);let y=hi_signed(lparam);let mut wr:RECT=std::mem::zeroed();GetWindowRect(hwnd,&mut wr);let b=6;let left=x<wr.left+b;let right=x>=wr.right-b;let top=y<wr.top+b;let bottom=y>=wr.bottom-b;if top&&left{HTTOPLEFT as LRESULT}else if top&&right{HTTOPRIGHT as LRESULT}else if bottom&&left{HTBOTTOMLEFT as LRESULT}else if bottom&&right{HTBOTTOMRIGHT as LRESULT}else if left{HTLEFT as LRESULT}else if right{HTRIGHT as LRESULT}else if top{HTTOP as LRESULT}else if bottom{HTBOTTOM as LRESULT}else{HTCLIENT as LRESULT}}
unsafe fn on_mouse_move(hwnd:HWND,state:&mut State,lparam:LPARAM){let mut track=TRACKMOUSEEVENT{cbSize:std::mem::size_of::<TRACKMOUSEEVENT>() as u32,dwFlags:TME_LEAVE,hwndTrack:hwnd,dwHoverTime:0};TrackMouseEvent(&mut track);let x=lo_signed(lparam);let y=hi_signed(lparam);let next=hover_badge_at(hwnd,state,x,y);if state.hover_text!=next||state.hover_x!=x||state.hover_y!=y{state.hover_text=next;state.hover_x=x;state.hover_y=y;InvalidateRect(hwnd,null(),0);}}
unsafe fn hover_badge_at(hwnd:HWND,state:&State,x:i32,y:i32)->Option<String>{if state.kind!=Kind::Dps||state.collapsed||!state.features.read().map(|f|f.meter.show_imagines).unwrap_or(true){return None;}let mut rc:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rc);let top=dps_rows_top();if y<top||y>=rc.bottom{return None;}let screen_i=((y-top)/DPS_ROW_H)as usize;if screen_i>=visible_dps_rows(rc.bottom){return None;}let rows=sorted_rows(&state.dps,state.sort_mode);let row=*rows.get(state.scroll+screen_i)?;let r=RECT{left:6,top:top+screen_i as i32*DPS_ROW_H,right:rc.right-8,bottom:top+screen_i as i32*DPS_ROW_H+DPS_ROW_H-2};let base_x=r.left+30;let metric_area=235;let identity_right=(r.right-metric_area).max(base_x+80);let badge_count=row.imagines.len().min(2)as i32;let badge_reserve=badge_count*(BADGE_W+BADGE_GAP);let name_right=(base_x+((identity_right-base_x)*45/100)-badge_reserve/2).max(base_x+70);let mut badge_x=name_right+5;for badge in row.imagines.iter().take(2){if x>=badge_x&&x<badge_x+BADGE_W&&y>=r.top+5&&y<r.top+24{let tier=if badge.tier>0{badge.tier.to_string()}else{"?".into()};return Some(format!("{} · Tier {}",badge.name,tier));}badge_x+=BADGE_W+BADGE_GAP;}None}
unsafe fn paint_hover(hdc:HDC,rc:RECT,state:&State){let Some(text)=state.hover_text.as_ref()else{return;};let width=((text.chars().count()as i32)*7+20).clamp(110,340);let height=25;let mut left=state.hover_x+14;let mut top=state.hover_y+16;if left+width>rc.right-4{left=(state.hover_x-width-10).max(4);}if top+height>rc.bottom-4{top=(state.hover_y-height-8).max(4);}let r=RECT{left,top,right:left+width,bottom:top+height};fill(hdc,&r,rgb(12,16,21));outline(hdc,r,rgb(99,199,255),1);SetTextColor(hdc,rgb(238,242,247));draw(hdc,text,RECT{left:r.left+8,top:r.top,right:r.right-8,bottom:r.bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}
unsafe fn on_wheel(hwnd:HWND,state:&mut State,wparam:WPARAM){state.hover_text=None;if state.collapsed{return;}"#,
        "hover tooltip and resize helpers",
    );

    patch_once(
        &mut overlay,
        "match state.kind{Kind::Dps=>paint_dps(hdc,rc,state),Kind::Mechanics=>paint_mechanics(hdc,rc,state)}EndPaint(hwnd,&ps);",
        "match state.kind{Kind::Dps=>paint_dps(hdc,rc,state),Kind::Mechanics=>paint_mechanics(hdc,rc,state)}paint_hover(hdc,rc,state);EndPaint(hwnd,&ps);",
        "paint hover tooltip",
    );

    patch_once(
        &mut overlay,
        "let mut x=r.left+30;if settings.meter.show_imagines{for badge in row.imagines.iter().take(2){paint_badge(hdc,x,r.top+5,badge);x+=BADGE_W+BADGE_GAP;}}let metric_area=235;",
        "let mut x=r.left+30;let metric_area=235;",
        "move Imagine badges after player name",
    );
    patch_once(
        &mut overlay,
        "let name_right=(x+((identity_right-x)*45/100)).max(x+70);",
        "let badge_count=if settings.meter.show_imagines{row.imagines.len().min(2)as i32}else{0};let badge_reserve=badge_count*(BADGE_W+BADGE_GAP);let name_right=(x+((identity_right-x)*45/100)-badge_reserve/2).max(x+70);",
        "reserve identity space for Imagine badges",
    );
    patch_once(
        &mut overlay,
        "draw(hdc,&row.name,RECT{left:x,top:r.top,right:name_right,bottom:r.bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);SetTextColor(hdc,base_text);draw(hdc,&identity_tail(row),RECT{left:name_right+5,top:r.top,right:identity_right,bottom:r.bottom}",
        "draw(hdc,&row.name,RECT{left:x,top:r.top,right:name_right,bottom:r.bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);let mut tail_left=name_right+5;if settings.meter.show_imagines{for badge in row.imagines.iter().take(2){paint_badge(hdc,tail_left,r.top+5,badge);tail_left+=BADGE_W+BADGE_GAP;}}SetTextColor(hdc,base_text);draw(hdc,&identity_tail(row),RECT{left:tail_left+2,top:r.top,right:identity_right,bottom:r.bottom}",
        "render Imagine badges after name",
    );

    fs::write(out.join("feature_overlays_v170_fixed.rs"), overlay)
        .expect("write generated overlay source");

    println!("cargo:rerun-if-changed=src/telemetry_v170.rs");
    println!("cargo:rerun-if-changed=src/feature_overlays_v170.rs");

    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("../src/BPSR.ReadyAlert/Assets/App.ico");
        res.set("FileDescription", "BPSR Ready Alert");
        res.set("ProductName", "BPSR Ready Alert");
        res.set("FileVersion", env!("CARGO_PKG_VERSION"));
        res.set("ProductVersion", env!("CARGO_PKG_VERSION"));
        if let Err(err) = res.compile() {
            panic!("failed to embed Windows resources: {err}");
        }
    }
}
