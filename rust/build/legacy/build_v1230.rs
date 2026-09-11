use std::{env, fs, path::{Path, PathBuf}};

mod prior {
    include!("build_v1220_fix.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.23.0 patch {label} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn replace_count(source: &mut String, from: &str, to: &str, expected: usize, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, expected, "v1.23.0 patch {label} expected {expected} matches, found {count}");
    *source = source.replace(from, to);
}

fn patch_chat(out: &Path) {
    let path = out.join("overlay_v150_v1181.rs");
    let mut source = fs::read_to_string(&path).expect("read v1.22.1 chat overlay").replace("\r\n", "\n");

    replace_once(&mut source,
        "    scroll_from_bottom: usize,\n    collapsed: bool,",
        "    scroll_from_bottom: usize,\n    unseen_messages: usize,\n    collapsed: bool,",
        "separate chat unseen state");
    replace_once(&mut source,
        "        scroll_from_bottom: 0,\n        collapsed: false,",
        "        scroll_from_bottom: 0,\n        unseen_messages: 0,\n        collapsed: false,",
        "initialize chat unseen state");
    replace_once(&mut source,
        r###"    if state.scroll_from_bottom > 0 && visible_now {
        state.scroll_from_bottom = state.scroll_from_bottom.saturating_add(1);
    }"###,
        r###"    if state.scroll_from_bottom > 0 && visible_now {
        state.scroll_from_bottom = state.scroll_from_bottom.saturating_add(1);
        state.unseen_messages = state.unseen_messages.saturating_add(1);
    }"###,
        "count only actually unseen incoming messages");
    replace_once(&mut source,
        r###"    while state.items.len() > snapshot.chat.max_history.max(10) {
        state.items.pop_front();
        state.scroll_from_bottom = state.scroll_from_bottom.saturating_sub(1);
    }
    InvalidateRect(hwnd, null(), 0);"###,
        r###"    while state.items.len() > snapshot.chat.max_history.max(10) {
        state.items.pop_front();
        state.scroll_from_bottom = state.scroll_from_bottom.saturating_sub(1);
    }
    state.unseen_messages = state.unseen_messages.min(snapshot.chat.max_history.max(10));
    InvalidateRect(hwnd, null(), 0);"###,
        "bound unseen count with retained history");
    replace_count(&mut source,
        "state.scroll_from_bottom=0;",
        "state.scroll_from_bottom=0;state.unseen_messages=0;",
        5,
        "reset unseen count whenever returning to tail");
    replace_once(&mut source,
        "                else if delta < 0 { state.scroll_from_bottom = state.scroll_from_bottom.saturating_sub(1); }",
        "                else if delta < 0 { state.scroll_from_bottom = state.scroll_from_bottom.saturating_sub(1); if state.scroll_from_bottom==0 { state.unseen_messages=0; } }",
        "wheel return-to-tail clears unseen count");
    replace_once(&mut source,
        "        let label = format!(\"↓ {} new message{}\", state.scroll_from_bottom, if state.scroll_from_bottom == 1 { \"\" } else { \"s\" });",
        "        let label = if state.unseen_messages>0 { format!(\"↓ {} new message{}\", state.unseen_messages, if state.unseen_messages == 1 { \"\" } else { \"s\" }) } else { \"↓ Back to latest\".to_string() };",
        "accurate new-message badge");

    // Long translated messages were hard-capped at two lines. Give normal rows
    // four lines and compact rows three while keeping a deterministic row cap.
    replace_count(&mut source,
        ".min(one*2)",
        ".min(one*if settings.chat.compact_mode{3}else{4})",
        3,
        "chat and translation line budget");

    fs::write(path, source).expect("write v1.23 chat overlay");
}

fn patch_settings(out: &Path) {
    let path = out.join("settings_ui_v1160_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read v1.22.1 settings UI").replace("\r\n", "\n");
    replace_once(&mut source,
        "        ID_UPDATE_CHECK_NOW => crate::updater::check_interactive(state.paths.root.clone()),",
        "        ID_UPDATE_CHECK_NOW => crate::updater::check_interactive(state.paths.root.clone(),state.main_hwnd as isize),",
        "manual updater owner HWND");
    replace_once(&mut source,
        "    info(hwnd,state,PAGE_SPEECH,\"TTS is limited to Guild and Party / Team; World chat is never spoken.\",174,316,580,22);",
        "    info(hwnd,state,PAGE_SPEECH,\"TTS is limited to Guild and Party / Team; World chat is never spoken. Eligible Translation/TTS chat text is sent to Google services.\",174,306,580,40);",
        "translation privacy disclosure");
    fs::write(path, source).expect("write v1.23 settings UI");
}

fn patch_win(out: &Path) {
    let path = out.join("win_v182_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read v1.22.1 Win UI").replace("\r\n", "\n");
    replace_once(&mut source,
        "        add_tray(hwnd,&state.capture_status)?;crate::updater::spawn_startup(state.paths.root.clone());apply_visibility(&mut state);",
        "        add_tray(hwnd,&state.capture_status)?;crate::updater::spawn_startup(state.paths.root.clone(),hwnd as isize);apply_visibility(&mut state);",
        "startup updater owner HWND");
    fs::write(path, source).expect("write v1.23 Win UI");
}

fn patch_capture(out: &Path) {
    let path = out.join("capture_v185.rs");
    let mut source = fs::read_to_string(&path).expect("read v1.22.1 capture").replace("\r\n", "\n");
    replace_once(&mut source,
        r###"    let d = devices.iter().find(|d| !looks_loopback(d) && !looks_virtual(d))
        .or_else(|| devices.iter().find(|d| !looks_loopback(d))).unwrap_or(&devices[0]);
    Ok(selected(d, "Auto-selected"))
}

fn selected(d:&NpcapDevice, source:&str) -> SelectedDevice { SelectedDevice { name:d.name.clone(), description:d.description.clone(), source:source.into() } }
fn looks_loopback(d:&NpcapDevice) -> bool { format!("{} {}", d.name,d.description).to_ascii_lowercase().contains("loopback") }
fn looks_virtual(d:&NpcapDevice) -> bool {
    let text=format!("{} {}",d.name,d.description).to_ascii_lowercase();
    ["virtual","vmware","hyper-v","virtualbox","vpn","tap","tunnel","wsl","tailscale","zerotier","wireguard","bluetooth","wan miniport"].iter().any(|x| text.contains(x))
}"###,
        r###"    let d = devices.iter().max_by_key(|d| adapter_score(d)).unwrap_or(&devices[0]);
    Ok(selected(d, "Auto-selected"))
}

fn selected(d:&NpcapDevice, source:&str) -> SelectedDevice { SelectedDevice { name:d.name.clone(), description:d.description.clone(), source:source.into() } }
fn adapter_text(d:&NpcapDevice)->String{format!("{} {}",d.name,d.description).to_ascii_lowercase()}
fn looks_loopback(d:&NpcapDevice)->bool{d.is_loopback()||adapter_text(d).contains("loopback")}
fn looks_vpn(d:&NpcapDevice)->bool{let text=adapter_text(d);["vpn","tap","tunnel","tailscale","zerotier","wireguard"].iter().any(|x|text.contains(x))}
fn looks_virtual(d:&NpcapDevice)->bool{let text=adapter_text(d);["virtual","vmware","hyper-v","virtualbox","wsl","bluetooth","wan miniport"].iter().any(|x|text.contains(x))||looks_vpn(d)}
fn adapter_score(d:&NpcapDevice)->i32{
    let text=adapter_text(d);let mut score=0;
    if looks_loopback(d){score-=10_000}else{score+=1_000}
    if d.is_up(){score+=500;}if d.is_running(){score+=500;}
    if !looks_virtual(d){score+=150;}else if looks_vpn(d){score+=60;}
    if ["wi-fi","wifi","wireless","ethernet"].iter().any(|x|text.contains(x)){score+=50;}
    score
}"###,
        "health-aware Npcap auto adapter ranking");
    fs::write(path, source).expect("write v1.23 capture");
}

fn patch_feature_overlay(out: &Path) {
    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read v1.22.1 feature overlays").replace("\r\n", "\n");

    replace_once(&mut source,
        r###"fn raid_load_rect(state:&State)->Option<RECT>{
    let text=std::fs::read_to_string(raid_layout_path(state)).ok()?;let values:Vec<i32>=text.split_whitespace().filter_map(|v|v.parse().ok()).collect();
    if values.len()!=4{return None;}let r=RECT{left:values[0],top:values[1],right:values[0].saturating_add(values[2]),bottom:values[1].saturating_add(values[3])};raid_rect_valid(r).then_some(r)
}
fn raid_save_rect_file(state:&State,r:RECT){
    if !raid_rect_valid(r){return;}let text=format!("{} {} {} {}\n",r.left,r.top,(r.right-r.left).max(1),(r.bottom-r.top).max(1));
    if let Err(err)=std::fs::write(raid_layout_path(state),text){crate::logging::write(format!("raid layout: save failed: {err}"));}
}"###,
        r###"fn raid_load_rect(hwnd:HWND,state:&State)->Option<RECT>{
    let text=std::fs::read_to_string(raid_layout_path(state)).ok()?;let values:Vec<i32>=text.split_whitespace().filter_map(|v|v.parse().ok()).collect();
    if values.len()!=4&&values.len()!=5{return None;}let saved_scale=if values.len()==5{values[4].clamp(50,200)}else{state.scale_percent.clamp(50,200)};let current_scale=state.scale_percent.clamp(50,200);
    let width=((values[2]as i64)*(current_scale as i64)/(saved_scale as i64)).clamp(1,i32::MAX as i64)as i32;let height=((values[3]as i64)*(current_scale as i64)/(saved_scale as i64)).clamp(1,i32::MAX as i64)as i32;
    let r=RECT{left:values[0],top:values[1],right:values[0].saturating_add(width),bottom:values[1].saturating_add(height)};if !raid_rect_valid(r){return None;}Some(crate::ui::fit_rect(r,unsafe{crate::ui::work_area(hwnd)}))
}
fn raid_save_rect_file(state:&State,r:RECT){
    if !raid_rect_valid(r){return;}let text=format!("{} {} {} {} {}\n",r.left,r.top,(r.right-r.left).max(1),(r.bottom-r.top).max(1),state.scale_percent.clamp(50,200));
    if let Err(err)=std::fs::write(raid_layout_path(state),text){crate::logging::write(format!("raid layout: save failed: {err}"));}
}"###,
        "monitor-safe scale-aware Raid geometry");
    replace_once(&mut source,
        ".or_else(||raid_load_rect(state));",
        ".or_else(||raid_load_rect(hwnd,state));",
        "Raid saved geometry caller");

    // Raid headers say ACTIVE/s, so use the same active-rate metric as normal mode
    // and obey the user's Show active rates preference.
    replace_once(&mut source,
        "let mut show_total=!row.is_dead;let mut show_rate=!row.is_dead;",
        "let mut show_total=!row.is_dead;let mut show_rate=!row.is_dead&&settings.meter.show_active_rates;",
        "Raid active-rate visibility");
    replace_once(&mut source,
        "show_rate=show_total&&room>=revive_w+metric_gap+total_w+metric_gap+rate_w;",
        "show_rate=settings.meter.show_active_rates&&show_total&&room>=revive_w+metric_gap+total_w+metric_gap+rate_w;",
        "Raid dead-row active-rate visibility");
    replace_once(&mut source,
        ",snapshot:&DpsSnapshot,settings:&FeatureSettings){",
        ",_snapshot:&DpsSnapshot,settings:&FeatureSettings){",
        "Raid no longer needs encounter-average input");
    replace_once(&mut source,
        "let total=compact(total_value as f64);let rate=compact(if snapshot.encounter_ms>0{total_value as f64*1000.0/snapshot.encounter_ms as f64}else{0.0});",
        "let total=compact(total_value as f64);let rate=compact(active_rate(row,state.sort_mode));",
        "Raid uses true active rate");

    fs::write(path, source).expect("write v1.23 feature overlays");
}

fn patch_telemetry(out: &Path) {
    let path = out.join("telemetry_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read v1.22.1 telemetry").replace("\r\n", "\n");
    replace_once(&mut source,
        "        let mut tracker_instances = HashSet::new();\n        let player_consumable_snapshot=entity_kind(host)==ENTITY_PLAYER;",
        "        let mut tracker_instances = HashSet::new();\n        let mut snapshot_revive_blocks=HashSet::new();\n        let player_consumable_snapshot=entity_kind(host)==ENTITY_PLAYER;",
        "authoritative revive snapshot state");
    replace_once(&mut source,
        "            let base_id = proto::get_varint_field(info, 2).unwrap_or(0) as i32;\n            if player_consumable_snapshot&&consumable_info(base_id).is_some(){snapshot_consumables.insert(buff_uuid);}",
        "            let base_id = proto::get_varint_field(info, 2).unwrap_or(0) as i32;\n            if player_consumable_snapshot&&base_id==REVIVE_BLOCK_BUFF_ID{snapshot_revive_blocks.insert(buff_uuid);}\n            if player_consumable_snapshot&&consumable_info(base_id).is_some(){snapshot_consumables.insert(buff_uuid);}",
        "collect revive blocks from full buff snapshot");
    replace_once(&mut source,
        "        crate::event_tracker::sync_buff_instances(host, &tracker_instances);\n        marker_source",
        r###"        if player_consumable_snapshot{
            let stale_revive:Vec<(i64,i32)>=self.revive_block_instances.keys().filter(|(entry_host,buff_uuid)|*entry_host==host&&!snapshot_revive_blocks.contains(buff_uuid)).copied().collect();
            if !stale_revive.is_empty(){for key in stale_revive{self.revive_block_instances.remove(&key);}self.emit_dps();}
        }
        crate::event_tracker::sync_buff_instances(host, &tracker_instances);
        marker_source"###,
        "reconcile missed revive-block removal packets");
    fs::write(path, source).expect("write v1.23 telemetry");
}

fn main() {
    prior::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_chat(&out);
    patch_settings(&out);
    patch_win(&out);
    patch_capture(&out);
    patch_feature_overlay(&out);
    patch_telemetry(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1230.rs");
}
