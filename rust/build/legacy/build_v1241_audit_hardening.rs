use std::{env, fs, path::{Path, PathBuf}};

mod prior {
    include!("build_v1240_focused_mechanics.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.24.1 patch {label} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn replace_count(source: &mut String, from: &str, to: &str, expected: usize, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, expected, "v1.24.1 patch {label} expected {expected} matches, found {count}");
    *source = source.replace(from, to);
}

fn patch_feature_overlay(out: &Path) {
    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read v1.24 feature overlays").replace("\r\n", "\n");

    replace_once(
        &mut source,
        "unsafe fn paint_mechanics(hdc:HDC,rc:RECT,state:&State){",
        r###"#[derive(Clone)]
enum MechanicDisplayRow{
    Tracker(crate::event_tracker::TrackerDisplayRow),
    Mechanic(crate::model::MechanicRow),
}
fn compose_mechanic_rows(mut mechanics:Vec<crate::model::MechanicRow>,tracker:Vec<crate::event_tracker::TrackerDisplayRow>,now:i64)->Vec<MechanicDisplayRow>{
    mechanics.retain(|row|row.persistent||row.expires_unix_ms<=0||row.expires_unix_ms>now);
    mechanics.sort_by(|a,b|{let a_timed=!a.persistent&&a.expires_unix_ms>now;let b_timed=!b.persistent&&b.expires_unix_ms>now;let ae=if a_timed{a.expires_unix_ms}else{i64::MAX};let be=if b_timed{b.expires_unix_ms}else{i64::MAX};b.priority.cmp(&a.priority).then_with(||b_timed.cmp(&a_timed)).then_with(||ae.cmp(&be)).then_with(||b.created_unix_ms.cmp(&a.created_unix_ms)).then_with(||a.key.cmp(&b.key))});
    let urgent=mechanics.iter().take_while(|row|row.priority>=3).count();
    let normal=mechanics.split_off(urgent);
    let mut rows=Vec::with_capacity(mechanics.len().saturating_add(tracker.len()).saturating_add(normal.len()));
    rows.extend(mechanics.into_iter().map(MechanicDisplayRow::Mechanic));
    rows.extend(tracker.into_iter().map(MechanicDisplayRow::Tracker));
    rows.extend(normal.into_iter().map(MechanicDisplayRow::Mechanic));
    rows
}
fn ordered_mechanic_rows(state:&State,now:i64)->Vec<MechanicDisplayRow>{
    compose_mechanic_rows(state.mechanics.rows.clone(),crate::event_tracker::rows(),now)
}
unsafe fn paint_mechanics(hdc:HDC,rc:RECT,state:&State){"###,
        "shared mechanic display ordering",
    );

    replace_once(
        &mut source,
        "let now=now_ms();let tracker=crate::event_tracker::rows();let mut active:Vec<_>=state.mechanics.rows.iter().filter(|row|row.persistent||row.expires_unix_ms<=0||row.expires_unix_ms>now).collect();active.sort_by(|a,b|{let a_timed=!a.persistent&&a.expires_unix_ms>now;let b_timed=!b.persistent&&b.expires_unix_ms>now;let ae=if a_timed{a.expires_unix_ms}else{i64::MAX};let be=if b_timed{b.expires_unix_ms}else{i64::MAX};b.priority.cmp(&a.priority).then_with(||b_timed.cmp(&a_timed)).then_with(||ae.cmp(&be)).then_with(||b.created_unix_ms.cmp(&a.created_unix_ms)).then_with(||a.key.cmp(&b.key))});let total=tracker.len().saturating_add(active.len());let visible=(((rc.bottom-top)/MECH_ROW_H).max(0)as usize).min(MAX_MECHANIC_ROWS_VISIBLE);",
        "let now=now_ms();let rows=ordered_mechanic_rows(state,now);let total=rows.len();let visible=(((rc.bottom-top)/MECH_ROW_H).max(0)as usize).min(MAX_MECHANIC_ROWS_VISIBLE);",
        "mechanic paint uses shared ordering",
    );

    replace_once(
        &mut source,
        r###"for screen_i in 0..visible{let index=state.scroll+screen_i;if index>=total{break;}let y=top+screen_i as i32*MECH_ROW_H;let r=RECT{left:6,top:y,right:rc.right-8,bottom:y+MECH_ROW_H-3};fill(hdc,&r,if screen_i%2==0{crate::ui_theme::RAISED}else{crate::ui_theme::SURFACE});if index<tracker.len(){let row=&tracker[index];fill(hdc,&RECT{left:r.left,top:r.top,right:r.left+3,bottom:r.bottom},crate::ui_theme::WARNING);let detail=if row.detail.trim().is_empty(){String::new()}else{format!("  •  {}",row.detail)};SetTextColor(hdc,crate::ui_theme::TEXT);draw(hdc,&format!("Tracker  •  {}{}",row.label,detail),RECT{left:r.left+10,top:r.top,right:r.right-92,bottom:r.bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);let(timer,color)=match row.remaining_ms{Some(ms)=>(format!("{:.1}s",ms.max(0)as f64/1000.0),timer_urgency_color(ms)),None=>("ACTIVE".into(),crate::ui_theme::ACCENT)};SetTextColor(hdc,color);draw(hdc,&timer,RECT{left:r.right-88,top:r.top,right:r.right-8,bottom:r.bottom},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}else{let row=active[index-tracker.len()];let accent=if row.priority>=3{crate::ui_theme::CRITICAL}else{crate::ui_theme::ACCENT};fill(hdc,&RECT{left:r.left,top:r.top,right:r.left+3,bottom:r.bottom},accent);let label=match&row.target{Some(target)if!target.trim().is_empty()=>format!("{}  •  {}",row.label,target),_=>row.label.clone()};SetTextColor(hdc,crate::ui_theme::TEXT);draw(hdc,&label,RECT{left:r.left+10,top:r.top,right:r.right-75,bottom:r.bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);let remaining=if row.persistent||row.expires_unix_ms<=0{None}else{Some(row.expires_unix_ms.saturating_sub(now))};SetTextColor(hdc,remaining.map(timer_urgency_color).unwrap_or(crate::ui_theme::ACCENT));draw(hdc,&mechanic_timer(row.expires_unix_ms,row.persistent,now),RECT{left:r.right-70,top:r.top,right:r.right-8,bottom:r.bottom},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}}"###,
        r###"for screen_i in 0..visible{let index=state.scroll+screen_i;if index>=total{break;}let y=top+screen_i as i32*MECH_ROW_H;let r=RECT{left:6,top:y,right:rc.right-8,bottom:y+MECH_ROW_H-3};fill(hdc,&r,if screen_i%2==0{crate::ui_theme::RAISED}else{crate::ui_theme::SURFACE});match &rows[index]{MechanicDisplayRow::Tracker(row)=>{fill(hdc,&RECT{left:r.left,top:r.top,right:r.left+3,bottom:r.bottom},crate::ui_theme::WARNING);let detail=if row.detail.trim().is_empty(){String::new()}else{format!("  •  {}",row.detail)};SetTextColor(hdc,crate::ui_theme::TEXT);draw(hdc,&format!("Tracker  •  {}{}",row.label,detail),RECT{left:r.left+10,top:r.top,right:r.right-92,bottom:r.bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);let(timer,color)=match row.remaining_ms{Some(ms)=>(format!("{:.1}s",ms.max(0)as f64/1000.0),timer_urgency_color(ms)),None=>("ACTIVE".into(),crate::ui_theme::ACCENT)};SetTextColor(hdc,color);draw(hdc,&timer,RECT{left:r.right-88,top:r.top,right:r.right-8,bottom:r.bottom},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);},MechanicDisplayRow::Mechanic(row)=>{let accent=if row.priority>=3{crate::ui_theme::CRITICAL}else{crate::ui_theme::ACCENT};fill(hdc,&RECT{left:r.left,top:r.top,right:r.left+3,bottom:r.bottom},accent);let label=match&row.target{Some(target)if!target.trim().is_empty()=>format!("{}  •  {}",row.label,target),_=>row.label.clone()};SetTextColor(hdc,crate::ui_theme::TEXT);draw(hdc,&label,RECT{left:r.left+10,top:r.top,right:r.right-75,bottom:r.bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);let remaining=if row.persistent||row.expires_unix_ms<=0{None}else{Some(row.expires_unix_ms.saturating_sub(now))};SetTextColor(hdc,remaining.map(timer_urgency_color).unwrap_or(crate::ui_theme::ACCENT));draw(hdc,&mechanic_timer(row.expires_unix_ms,row.persistent,now),RECT{left:r.right-70,top:r.top,right:r.right-8,bottom:r.bottom},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}}}"###,
        "mechanic rows render from shared ordering",
    );

    replace_once(
        &mut source,
        r###"let(_,_,top)=mechanics_scroll_metrics(state,rc.bottom);if y>=top{let index=state.scroll+((y-top)/MECH_ROW_H)as usize;let tracker=crate::event_tracker::rows();if let Some(row)=tracker.get(index){return Some(format!("{} • {}",row.label,row.detail));}let now=now_ms();if let Some(row)=state.mechanics.rows.iter().filter(|r|r.persistent||r.expires_unix_ms<=0||r.expires_unix_ms>now).nth(index.saturating_sub(tracker.len())){return Some(format!("{} {}",row.label,row.target.as_deref().unwrap_or("")));}}"###,
        r###"let(_,_,top)=mechanics_scroll_metrics(state,rc.bottom);if y>=top{let index=state.scroll+((y-top)/MECH_ROW_H)as usize;let rows=ordered_mechanic_rows(state,now_ms());if let Some(row)=rows.get(index){return Some(match row{MechanicDisplayRow::Tracker(row)=>format!("{} • {}",row.label,row.detail),MechanicDisplayRow::Mechanic(row)=>format!("{} {}",row.label,row.target.as_deref().unwrap_or(""))});}}"###,
        "mechanic hover follows paint ordering",
    );

    source.push_str(r###"
#[cfg(test)]
mod v1241_mechanic_order_tests{
    use super::*;
    #[test]
    fn urgent_mechanics_precede_tracker_rows_and_normal_mechanics(){
        let urgent=crate::model::MechanicRow{key:"urgent".into(),label:"Urgent".into(),priority:3,persistent:true,..Default::default()};
        let normal=crate::model::MechanicRow{key:"normal".into(),label:"Normal".into(),priority:1,persistent:true,..Default::default()};
        let tracker=crate::event_tracker::TrackerDisplayRow{label:"Custom".into(),active:true,..Default::default()};
        let rows=compose_mechanic_rows(vec![normal,urgent],vec![tracker],1);
        assert!(matches!(&rows[0],MechanicDisplayRow::Mechanic(row) if row.priority>=3));
        assert!(matches!(&rows[1],MechanicDisplayRow::Tracker(_)));
        assert!(matches!(&rows[2],MechanicDisplayRow::Mechanic(row) if row.priority<3));
    }
}
"###);

    fs::write(path, source).expect("write v1.24.1 feature overlays");
}

fn patch_chat_overlay(out: &Path) {
    let path = out.join("overlay_v150_v1181.rs");
    let mut source = fs::read_to_string(&path).expect("read v1.24 chat overlay").replace("\r\n", "\n");
    replace_count(
        &mut source,
        ".max(one).min(one*if settings.chat.compact_mode{3}else{4})",
        ".max(one)",
        3,
        "full wrapped chat and translation height",
    );
    fs::write(path, source).expect("write v1.24.1 chat overlay");
}

fn patch_capture(out: &Path) {
    let path = out.join("capture_v185.rs");
    let mut source = fs::read_to_string(&path).expect("read v1.24 capture").replace("\r\n", "\n");
    replace_once(
        &mut source,
        "fn retry_delay(failures:u32)->Duration { Duration::from_millis(match failures {0|1=>1000,2=>2000,3=>5000,_=>10000}) }",
        "fn synthetic_chat_message_id(sequence:u64)->i64{-(sequence.min(i64::MAX as u64).max(1)as i64)}\nfn retry_delay(failures:u32)->Duration { Duration::from_millis(match failures {0|1=>1000,2=>2000,3=>5000,_=>10000}) }",
        "zero-id chat synthetic id helper",
    );
    replace_once(
        &mut source,
        "if service==proto::CHAT_SERVICE&&method==proto::CHAT_NOTIFY_NEWEST{self.sequence=self.sequence.wrapping_add(1).max(1);if let Some(message)=proto::parse_chat(body,self.sequence){self.chat.handle(&message);let _=self.tx.send(AppEvent::Chat(message));}return;}",
        "if service==proto::CHAT_SERVICE&&method==proto::CHAT_NOTIFY_NEWEST{self.sequence=self.sequence.wrapping_add(1).max(1);if let Some(mut message)=proto::parse_chat(body,self.sequence){if message.message_id==0{message.message_id=synthetic_chat_message_id(message.sequence_id);}self.chat.handle(&message);let _=self.tx.send(AppEvent::Chat(message));}return;}",
        "zero-id chat no longer false-dedupes",
    );
    source.push_str(r###"
#[cfg(test)]
mod v1241_chat_id_tests{
    use super::*;
    #[test]
    fn zero_id_chat_fallback_is_unique_and_outside_normal_positive_ids(){
        assert!(synthetic_chat_message_id(1)<0);
        assert_ne!(synthetic_chat_message_id(1),synthetic_chat_message_id(2));
        assert_eq!(synthetic_chat_message_id(0),-1);
    }
}
"###);
    fs::write(path, source).expect("write v1.24.1 capture");
}

fn patch_win(out: &Path) {
    let path = out.join("win_v182_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read v1.24 Win UI").replace("\r\n", "\n");
    replace_once(
        &mut source,
        "add_tray(hwnd,&state.capture_status)?;crate::updater::spawn_startup(state.paths.root.clone(),hwnd as isize);apply_visibility(&mut state);",
        "add_tray(hwnd,&state.capture_status)?;apply_visibility(&mut state);crate::updater::mark_startup_healthy();crate::updater::spawn_startup(state.paths.root.clone(),hwnd as isize);",
        "update health marker after tray/UI initialization",
    );
    fs::write(path, source).expect("write v1.24.1 Win UI");
}

fn patch_updater(out: &Path) {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let input = manifest.join("src/updater.rs");
    let output = out.join("updater_v1241.rs");
    let mut source = fs::read_to_string(input).expect("read updater source").replace("\r\n", "\n");

    replace_once(&mut source, "process::Command,", "process::{Child, Command},", "updater child import");
    replace_once(&mut source, "time::Duration,", "time::{Duration, Instant},", "updater deadline import");
    replace_once(
        &mut source,
        "const WAIT_FAILED_CODE: u32 = 0xffff_ffff;\nstatic UPDATE_BUSY: AtomicBool = AtomicBool::new(false);",
        "const WAIT_FAILED_CODE: u32 = 0xffff_ffff;\nconst UPDATE_HEALTH_ENV: &str = \"BPSR_READYALERT_UPDATE_HEALTH\";\nconst UPDATE_HEALTH_TIMEOUT: Duration = Duration::from_secs(20);\nstatic UPDATE_BUSY: AtomicBool = AtomicBool::new(false);",
        "updater health constants",
    );
    replace_once(
        &mut source,
        r###"pub fn load_preferences(root: &Path) -> UpdatePreferences {
    let path = preferences_path(root);
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<UpdatePreferences>(&text).ok())
        .unwrap_or_default()
}"###,
        r###"pub fn load_preferences(root: &Path) -> UpdatePreferences {
    let path = preferences_path(root);
    for (candidate, label) in [
        (path.clone(), "primary"),
        (path.with_extension("json.bak"), "backup"),
        (path.with_extension("json.new"), "pending"),
    ] {
        let Ok(text) = fs::read_to_string(&candidate) else { continue; };
        match serde_json::from_str::<UpdatePreferences>(&text) {
            Ok(prefs) => {
                if label != "primary" {
                    logging::write(format!("updater: recovered update settings from {label}"));
                    let _ = save_preferences(root, &prefs);
                }
                return prefs;
            }
            Err(err) => logging::write(format!("updater: invalid update settings {}: {err}", candidate.display())),
        }
    }
    UpdatePreferences::default()
}"###,
        "update preference recovery",
    );
    replace_once(
        &mut source,
        r###"pub fn cleanup_stale_helper() {
    let helper = helper_path();
    let current = std::env::current_exe().ok();
    if current.as_ref() != Some(&helper) {
        let _ = fs::remove_file(helper);
    }
}"###,
        r###"pub fn cleanup_stale_helper() {
    let helper = helper_path();
    let current = std::env::current_exe().ok();
    if current.as_ref() != Some(&helper) {
        let _ = fs::remove_file(helper);
    }
}

pub fn mark_startup_healthy() {
    let Some(path) = std::env::var_os(UPDATE_HEALTH_ENV) else { return; };
    if let Err(err) = fs::write(PathBuf::from(path), b"ready") {
        logging::write(format!("updater: could not write startup health marker: {err}"));
    }
}"###,
        "startup health marker",
    );
    replace_once(
        &mut source,
        r###"fn apply_update_helper(pid: u32, target: &Path, downloaded: &Path) -> Result<(), String> {
    wait_for_process(pid)?;
    let backup = target.with_extension("exe.old");
    let _ = fs::remove_file(&backup);
    fs::rename(target, &backup).map_err(|e| format!("backup current EXE: {e}"))?;

    if let Err(err) = fs::copy(downloaded, target) {
        let _ = fs::rename(&backup, target);
        return Err(format!("replace EXE: {err}"));
    }

    match Command::new(target).spawn() {
        Ok(_) => {
            let _ = fs::remove_file(&backup);
            let _ = fs::remove_file(downloaded);
            Ok(())
        }
        Err(err) => {
            let _ = fs::remove_file(target);
            let _ = fs::rename(&backup, target);
            let _ = Command::new(target).spawn();
            Err(format!("restart updated app: {err}; previous version restored"))
        }
    }
}"###,
        r###"fn apply_update_helper(pid: u32, target: &Path, downloaded: &Path) -> Result<(), String> {
    wait_for_process(pid)?;
    let backup = target.with_extension("exe.old");
    let health = temp_root().join(format!("update-health-{}.ok", std::process::id()));
    let _ = fs::remove_file(&health);
    let _ = fs::remove_file(&backup);
    fs::rename(target, &backup).map_err(|e| format!("backup current EXE: {e}"))?;

    if let Err(err) = fs::copy(downloaded, target) {
        let _ = fs::rename(&backup, target);
        return Err(format!("replace EXE: {err}"));
    }

    let mut child = match Command::new(target).env(UPDATE_HEALTH_ENV, &health).spawn() {
        Ok(child) => child,
        Err(err) => {
            let _ = fs::remove_file(target);
            let _ = fs::rename(&backup, target);
            let _ = Command::new(target).spawn();
            return Err(format!("restart updated app: {err}; previous version restored"));
        }
    };

    match wait_for_startup_health(&mut child, &health) {
        Ok(()) => {
            let _ = fs::remove_file(&health);
            let _ = fs::remove_file(&backup);
            let _ = fs::remove_file(downloaded);
            Ok(())
        }
        Err(err) => {
            let _ = child.kill();
            let _ = child.wait();
            let _ = fs::remove_file(&health);
            let _ = fs::remove_file(target);
            let restored = fs::rename(&backup, target).map_err(|restore| format!("{err}; rollback failed: {restore}"));
            if restored.is_ok() { let _ = Command::new(target).spawn(); }
            restored.and(Err(format!("{err}; previous version restored")))
        }
    }
}"###,
        "health-checked updater rollback",
    );
    replace_once(
        &mut source,
        "fn wait_for_process(pid: u32) -> Result<(), String> {",
        r###"fn wait_for_startup_health(child: &mut Child, marker: &Path) -> Result<(), String> {
    let deadline = Instant::now() + UPDATE_HEALTH_TIMEOUT;
    loop {
        if marker.exists() {
            if let Some(status) = child.try_wait().map_err(|e| format!("check updated app: {e}"))? {
                return Err(format!("updated app exited during startup with {status}"));
            }
            return Ok(());
        }
        if let Some(status) = child.try_wait().map_err(|e| format!("check updated app: {e}"))? {
            return Err(format!("updated app exited before startup completed with {status}"));
        }
        if Instant::now() >= deadline {
            return Err("updated app did not confirm healthy startup within 20 seconds".into());
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn wait_for_process(pid: u32) -> Result<(), String> {"###,
        "updater startup health wait",
    );
    source.push_str(r###"
#[cfg(test)]
mod v1241_updater_recovery_tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    #[test]
    fn preferences_recover_from_backup_when_primary_is_corrupt() {
        let root=std::env::temp_dir().join(format!("readyalert-updater-pref-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
        fs::create_dir_all(&root).unwrap();
        let path=preferences_path(&root);
        fs::write(&path,b"broken").unwrap();
        let wanted=UpdatePreferences{auto_check:false,auto_download:false};
        fs::write(path.with_extension("json.bak"),serde_json::to_vec(&wanted).unwrap()).unwrap();
        let loaded=load_preferences(&root);
        assert!(!loaded.auto_check&&!loaded.auto_download);
        fs::remove_dir_all(root).unwrap();
    }
}
"###);
    fs::write(output, source).expect("write v1.24.1 updater");
}

fn patch_capture_supervisor(out: &Path) {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let input = manifest.join("src/capture_supervisor.rs");
    let output = out.join("capture_supervisor_v1241.rs");
    let mut source = fs::read_to_string(input).expect("read capture supervisor source").replace("\r\n", "\n");
    replace_once(&mut source, ") -> thread::JoinHandle<()> {", ") -> Result<thread::JoinHandle<()>, String> {", "capture supervisor returns Result");
    replace_once(
        &mut source,
        "        })\n        .expect(\"spawn capture supervisor\")\n}",
        "        })\n        .map_err(|e| format!(\"spawn capture supervisor: {e}\"))\n}",
        "capture supervisor spawn error",
    );
    fs::write(output, source).expect("write v1.24.1 capture supervisor");
}

fn main() {
    prior::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_feature_overlay(&out);
    patch_chat_overlay(&out);
    patch_capture(&out);
    patch_win(&out);
    patch_updater(&out);
    patch_capture_supervisor(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1241_audit_hardening.rs");
    println!("cargo:rerun-if-changed=src/updater.rs");
    println!("cargo:rerun-if-changed=src/capture_supervisor.rs");
}
