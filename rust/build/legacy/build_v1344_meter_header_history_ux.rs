use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1343_white_header_icons.rs");
    pub fn run() { main(); }
}

fn once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.34.4 UX patch {label:?}: expected one anchor, found {count}");
    *source = source.replacen(from, to, 1);
}

fn between(source: &mut String, start: &str, end: &str, replacement: &str, label: &str) {
    let count = source.matches(start).count();
    assert_eq!(count, 1, "v1.34.4 UX patch {label:?}: expected one start, found {count}");
    let begin = source.find(start).expect("v1.34.4 start anchor");
    let finish = begin + source[begin..].find(end).expect("v1.34.4 end anchor");
    source.replace_range(begin..finish, replacement);
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("feature renderer").replace("\r\n", "\n");

    // Replace the five right-side controls with responsive spacing. Wide meters
    // get breathing room; narrow meters progressively collapse the gaps and icon width.
    between(
        &mut source,
        "fn reference_button_w(width: i32) -> i32 {",
        "fn reference_action_enabled(state: &State, action: ToolbarAction) -> bool {",
        r#"fn reference_button_w(width: i32) -> i32 {
    if width < 340 { 20 } else if width < 440 { 24 } else { 28 }
}
fn reference_button_gap(width: i32) -> i32 {
    if width < 400 { 0 } else if width < 560 { 3 } else { 6 }
}
fn reference_system_rect(width: i32, index: i32) -> RECT {
    let w = reference_button_w(width);
    let gap = reference_button_gap(width);
    let right = width - 4 - (2 - index) * (w + gap);
    RECT { left: right - w, top: 3, right, bottom: TOOLBAR_H - 3 }
}
fn reference_contains(r: RECT, x: i32, y: i32) -> bool {
    x >= r.left && x < r.right && y >= r.top && y < r.bottom
}
fn reference_layout_label(state: &State) -> &'static str {
    match (state.compact_mode, raid_active(state)) {
        (false, false) => "Normal ▾",
        (true, false) => "Compact ▾",
        (false, true) => "Raid ▾",
        (true, true) => "Compact Raid ▾",
    }
}
fn reference_selector_w(state: &State, right: i32) -> i32 {
    let preferred = match (state.compact_mode, raid_active(state)) {
        (true, true) => 108,
        (true, false) => 86,
        (false, true) => 70,
        _ => 82,
    };
    if right < 360 { preferred.min(94) } else { preferred }
}
fn toolbar_items(state: &State, right: i32) -> Vec<ToolbarItem> {
    if state.kind != Kind::Dps { return Vec::new(); }
    let w = reference_button_w(right);
    let gap = reference_button_gap(right);
    let selector_w = reference_selector_w(state, right);
    let selector_right = 6 + selector_w;
    let system_left = reference_system_rect(right, 0).left;
    let actions_left = system_left - gap - (2 * w + gap);
    let live_w = if right < 340 { 38 } else { 42 };
    let nav_w = 2 * w + live_w;
    let nav_left = (right / 2 - nav_w / 2)
        .min(actions_left - nav_w - gap.max(3))
        .max(selector_right + gap.max(3));
    let rect = |left, width| RECT { left, top: 3, right: left + width, bottom: TOOLBAR_H - 3 };
    vec![
        ToolbarItem { action: ToolbarAction::More, rect: rect(6, selector_w), label: reference_layout_label(state) },
        ToolbarItem { action: ToolbarAction::Older, rect: rect(nav_left, w), label: "‹" },
        ToolbarItem { action: ToolbarAction::Live, rect: rect(nav_left + w, live_w), label: "Live" },
        ToolbarItem { action: ToolbarAction::Newer, rect: rect(nav_left + w + live_w, w), label: "›" },
        ToolbarItem { action: ToolbarAction::Copy, rect: rect(actions_left, w), label: "" },
        ToolbarItem { action: ToolbarAction::Reset, rect: rect(actions_left + w + gap, w), label: "" },
    ]
}
"#,
        "responsive toolbar geometry",
    );

    once(&mut source,
        "wide(\"Player count and visibility…\").as_ptr(),",
        "wide(\"Settings\").as_ptr(),",
        "meter menu Settings label");

    // Reset is deliberately one click. Archive the current snapshot first, then
    // clear only the live meter and request a telemetry reset. Match the complete
    // dispatch block so the Reset icon-paint match arm cannot be mistaken for it.
    once(
        &mut source,
        r#"        ToolbarAction::Reset => {
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                MessageBoxW, IDYES, MB_DEFBUTTON2, MB_ICONQUESTION, MB_YESNO,
            };
            if MessageBoxW(hwnd,wide("Reset the live encounter? Its current results will remain in encounter history.").as_ptr(),wide("Reset encounter").as_ptr(),MB_YESNO|MB_ICONQUESTION|MB_DEFBUTTON2)==IDYES {
                archive_live_snapshot(state);crate::telemetry::request_manual_reset();
                state.dps=DpsSnapshot::default();state.history_index=None;state.scroll=0;
            }
        }"#,
        r#"        ToolbarAction::Reset => {
            archive_live_snapshot(state);
            crate::telemetry::request_manual_reset();
            state.dps = DpsSnapshot::default();
            state.history_index = None;
            state.scroll = 0;
        }"#,
        "one-click reset",
    );

    // Replace count-based retention with a time policy. Existing history files are
    // compatible and remain readable; 7 days is the migration/default choice.
    once(&mut source,
        "let history_records=if kind==Kind::Dps{history::load_recent(&paths.root,snapshot.meter.history_limit)}else{Vec::new()};",
        "let history_records=if kind==Kind::Dps{meter_history_retention::load(&paths.root,meter_history_retention::load_days(&paths))}else{Vec::new()};",
        "history startup load");

    between(
        &mut source,
        "fn archive_live_snapshot(state:&mut State){",
        "fn history_older(state:&mut State)",
        r#"fn archive_live_snapshot(state:&mut State){
    let days=meter_history_retention::load_days(&state.paths);
    match meter_history_retention::archive(&state.paths.root,&state.dps,days){
        Ok(Some(record))=>{
            if let Some(index)=state.history_index{state.history_index=Some(index.saturating_add(1));}
            state.history.insert(0,record);
            if let Some(index)=state.history_index{if index>=state.history.len(){state.history_index=state.history.len().checked_sub(1);}}
        },
        Ok(None)=>{},
        Err(err)=>crate::logging::write(format!("history: archive failed: {err}")),
    }
}
"#,
        "time-based archive",
    );

    // DPS Settings: replace +/- saved-count controls with the same style of
    // retention selector users already know from local chat logs, plus Never clear.
    once(&mut source,
        "feature_label(hwnd,7011,\"\",286,322,150,22);feature_button(hwnd,7012,\"−\",438,316,36);feature_button(hwnd,7013,\"+\",480,316,36);",
        "feature_label(hwnd,0,\"Encounter history\",286,322,116,22);feature_combo_sized(hwnd,7014,404,316,142,&[\"1 day\",\"3 days\",\"7 days\",\"Never clear\"]);",
        "history retention controls");
    once(&mut source,
        "windows_sys::Win32::UI::WindowsAndMessaging::SetWindowTextW(fc(hwnd,7011),wide(&format!(\"Saved encounters: {}\",f.meter.history_limit)).as_ptr());",
        "let retention=match meter_history_retention::load_days(&state.paths){1=>0,3=>1,0=>3,_=>2};SendMessageW(fc(hwnd,7014),0x014e,retention,0);",
        "history retention refresh");
    once(&mut source,
        "if matches!(id,7003|7010)&&code!=1{return 0;}",
        "if matches!(id,7003|7010|7014)&&code!=1{return 0;}",
        "retention combo notifications");
    once(&mut source,
        "            let checked=SendMessageW(fc(hwnd,id),0x00f0,0,0)==1;\n            let selected=SendMessageW(fc(hwnd,id),0x0147,0,0);\n            mutate_features(state,|f|{",
        r#"            let checked=SendMessageW(fc(hwnd,id),0x00f0,0,0)==1;
            let selected=SendMessageW(fc(hwnd,id),0x0147,0,0);
            if id==7014 {
                let days=match selected{0=>1,1=>3,3=>0,_=>7};
                if let Err(err)=meter_history_retention::save_days(&state.paths,days){crate::logging::write(format!("history retention: save failed: {err}"));}
                let ptr=GetWindowLongPtrW(state.parent,GWLP_USERDATA)as *mut State;
                if !ptr.is_null(){let overlay=&mut*ptr;overlay.history=meter_history_retention::load(&state.paths.root,days);overlay.history_index=None;overlay.scroll=0;InvalidateRect(state.parent,null(),0);}
                refresh_feature_form(hwnd,state);
                return 0;
            }
            mutate_features(state,|f|{"#,
        "save retention selection");

    // Old count buttons no longer exist, so their mutation branches must not be
    // reachable or imply count-based deletion semantics.
    once(&mut source,
        "                    7012=>f.meter.history_limit=f.meter.history_limit.saturating_sub(10).max(10),\n                    7013=>f.meter.history_limit=(f.meter.history_limit+10).min(200),\n",
        "",
        "remove count retention handlers");

    // Keep regression tests aligned with the dynamic title.
    once(&mut source,
        "actions.iter().any(|i|i.action==ToolbarAction::More&&i.label==\"Meter ▾\")",
        "actions.iter().any(|i|i.action==ToolbarAction::More&&i.label==\"Normal ▾\")",
        "toolbar title regression");

    source.push_str("\nmod meter_history_retention { include!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/src/feature_meter_history_retention.rs\")); }\n");
    fs::write(&path, source).expect("write v1.34.4 meter UX");

    println!("cargo:rerun-if-changed=build/legacy/build_v1344_meter_header_history_ux.rs");
    println!("cargo:rerun-if-changed=src/feature_meter_history_retention.rs");
}
