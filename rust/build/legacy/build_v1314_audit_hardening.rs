use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1313_single_buff_row.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.31.4 patch {label} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn patch_telemetry(out: &PathBuf) {
    let path = out.join("telemetry_v170_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read v1.31.4 telemetry: {e}"))
        .replace("\r\n", "\n");

    replace_once(
        &mut source,
        "                is_party: false,",
        "                is_party: uid == self.local_uid || self.team.contains(&uid),",
        "meter party membership flag",
    );

    source.push_str(r#"
#[cfg(test)]
mod v1314_party_membership_tests {
    use super::*;

    #[test]
    fn dps_party_flag_contract_uses_live_team_membership() {
        let generated = include_str!(concat!(env!("OUT_DIR"), "/telemetry_v170_fixed.rs"));
        assert!(generated.contains("is_party: uid == self.local_uid || self.team.contains(&uid)"));
    }
}
"#);

    assert!(source.contains("is_party: uid == self.local_uid || self.team.contains(&uid)"));
    fs::write(path, source).unwrap_or_else(|e| panic!("write v1.31.4 telemetry: {e}"));
}

fn patch_feature_overlay(out: &PathBuf) {
    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read v1.31.4 feature overlay: {e}"))
        .replace("\r\n", "\n");

    replace_once(
        &mut source,
        "        let total=meter_rows(state).len();\n        state.scroll=state.scroll.min(total.saturating_sub(1));",
        "        clamp_scroll(hwnd,state);",
        "DPS scroll clamps to visible page after row shrink",
    );

    replace_once(
        &mut source,
        "pub unsafe fn update_mechanics(hwnd:HWND,snapshot:MechanicSnapshot){with_state(hwnd,|state|{let new_urgent=snapshot.rows.iter().any(|next|next.priority>=3&&!state.mechanics.rows.iter().any(|old|old.priority>=3&&old.key==next.key));state.mechanics=snapshot;if new_urgent{state.scroll=0;}});InvalidateRect(hwnd,null(),0);}",
        "pub unsafe fn update_mechanics(hwnd:HWND,snapshot:MechanicSnapshot){with_state(hwnd,|state|{let new_urgent=snapshot.rows.iter().any(|next|next.priority>=3&&!state.mechanics.rows.iter().any(|old|old.priority>=3&&old.key==next.key));state.mechanics=snapshot;if new_urgent{state.scroll=0;}clamp_scroll(hwnd,state);});InvalidateRect(hwnd,null(),0);}",
        "mechanics scroll clamps after rows expire",
    );

    replace_once(
        &mut source,
        "unsafe fn clamp_scroll(hwnd:HWND,state:&mut State){let rc=logical_client_rect(hwnd,state.scale_percent);let(total,visible)=if state.kind==Kind::Dps{let physical=visible_dps_rows_scaled_for(state,rc.bottom,dps_layout_scale(state));(meter_rows(state).len(),meter_regular_page_size(state,physical))}else{let(total,visible,_)=mechanics_scroll_metrics(state,rc.bottom);(total,visible)};state.scroll=state.scroll.min(total.saturating_sub(visible.max(1)));}",
        "fn clamp_scroll_offset(total:usize,visible:usize,scroll:usize)->usize{scroll.min(total.saturating_sub(visible.max(1)))}\nunsafe fn clamp_scroll(hwnd:HWND,state:&mut State){let rc=logical_client_rect(hwnd,state.scale_percent);let(total,visible)=if state.kind==Kind::Dps{let physical=visible_dps_rows_scaled_for(state,rc.bottom,dps_layout_scale(state));(meter_rows(state).len(),meter_regular_page_size(state,physical))}else{let(total,visible,_)=mechanics_scroll_metrics(state,rc.bottom);(total,visible)};state.scroll=clamp_scroll_offset(total,visible,state.scroll);}",
        "shared scroll clamp helper",
    );

    source.push_str(r#"
#[cfg(test)]
mod v1314_scroll_clamp_tests {
    use super::*;

    #[test]
    fn shrinking_rows_cannot_leave_blank_meter_or_mechanics_viewport() {
        assert_eq!(clamp_scroll_offset(20, 10, 19), 10);
        assert_eq!(clamp_scroll_offset(4, 10, 8), 0);
        assert_eq!(clamp_scroll_offset(0, 10, 8), 0);
        assert_eq!(clamp_scroll_offset(20, 10, 5), 5);
    }
}
"#);

    for required in ["clamp_scroll(hwnd,state);", "clamp_scroll_offset(total,visible,state.scroll)"] {
        assert!(source.contains(required), "v1.31.4 scroll contract missing {required}");
    }
    fs::write(path, source).unwrap_or_else(|e| panic!("write v1.31.4 feature overlay: {e}"));
}

fn patch_updater(out: &PathBuf) {
    let path = out.join("updater_v1241.rs");
    let mut source = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read v1.31.4 updater: {e}"))
        .replace("\r\n", "\n");

    let old = r#"fn apply_update_helper(pid: u32, target: &Path, downloaded: &Path) -> Result<(), String> {
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
}"#;

    let new = r#"fn stage_update_candidate(downloaded: &Path, target: &Path) -> Result<PathBuf, String> {
    let staged = target.with_extension("exe.new");
    let _ = fs::remove_file(&staged);
    if let Err(err) = fs::copy(downloaded, &staged) {
        let _ = fs::remove_file(&staged);
        return Err(format!("stage update beside current EXE: {err}"));
    }
    let source_hash = sha256_file(downloaded)?;
    let staged_hash = sha256_file(&staged)?;
    if !source_hash.eq_ignore_ascii_case(&staged_hash) {
        let _ = fs::remove_file(&staged);
        return Err("staged update failed local SHA-256 verification".into());
    }
    Ok(staged)
}

fn apply_update_helper(pid: u32, target: &Path, downloaded: &Path) -> Result<(), String> {
    wait_for_process(pid)?;
    // Copy and verify the candidate beside the installed EXE before touching the
    // currently working executable. This prevents a disk-full/I/O failure from
    // leaving a partial replacement at `target` and blocking rollback on Windows.
    let staged = stage_update_candidate(downloaded, target)?;
    let backup = target.with_extension("exe.old");
    let health = temp_root().join(format!("update-health-{}.ok", std::process::id()));
    let _ = fs::remove_file(&health);
    let _ = fs::remove_file(&backup);
    if let Err(err) = fs::rename(target, &backup) {
        let _ = fs::remove_file(&staged);
        return Err(format!("backup current EXE: {err}"));
    }

    if let Err(err) = fs::rename(&staged, target) {
        let rollback = fs::rename(&backup, target);
        let _ = fs::remove_file(&staged);
        return match rollback {
            Ok(()) => Err(format!("commit staged update: {err}; previous version restored")),
            Err(restore) => Err(format!("commit staged update: {err}; rollback failed: {restore}")),
        };
    }

    let mut child = match Command::new(target).env(UPDATE_HEALTH_ENV, &health).spawn() {
        Ok(child) => child,
        Err(err) => {
            let _ = fs::remove_file(target);
            let restored = fs::rename(&backup, target);
            if restored.is_ok() { let _ = Command::new(target).spawn(); }
            return match restored {
                Ok(()) => Err(format!("restart updated app: {err}; previous version restored")),
                Err(restore) => Err(format!("restart updated app: {err}; rollback failed: {restore}")),
            };
        }
    };

    match wait_for_startup_health(&mut child, &health) {
        Ok(()) => {
            let _ = fs::remove_file(&health);
            let _ = fs::remove_file(&backup);
            let _ = fs::remove_file(&staged);
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
}"#;

    replace_once(&mut source, old, new, "transactional updater staging");

    source.push_str(r#"
#[cfg(test)]
mod v1314_updater_transaction_tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn root(label:&str)->PathBuf{
        let nonce=SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root=std::env::temp_dir().join(format!("readyalert-v1314-{label}-{}-{nonce}",std::process::id()));
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn staging_never_touches_working_exe_before_commit(){
        let root=root("stage");
        let target=root.join("BPSR-ReadyAlert.exe");
        let downloaded=root.join("download.exe");
        fs::write(&target,b"known-good").unwrap();
        fs::write(&downloaded,b"candidate").unwrap();
        let staged=stage_update_candidate(&downloaded,&target).unwrap();
        assert_eq!(fs::read(&target).unwrap(),b"known-good");
        assert_eq!(fs::read(&staged).unwrap(),b"candidate");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_staging_preserves_working_exe(){
        let root=root("missing");
        let target=root.join("BPSR-ReadyAlert.exe");
        fs::write(&target,b"known-good").unwrap();
        assert!(stage_update_candidate(&root.join("missing.exe"),&target).is_err());
        assert_eq!(fs::read(&target).unwrap(),b"known-good");
        fs::remove_dir_all(root).unwrap();
    }
}
"#);

    for required in ["stage_update_candidate", "staged update failed local SHA-256 verification", "commit staged update"] {
        assert!(source.contains(required), "v1.31.4 updater contract missing {required}");
    }
    fs::write(path, source).unwrap_or_else(|e| panic!("write v1.31.4 updater: {e}"));
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_telemetry(&out);
    patch_feature_overlay(&out);
    patch_updater(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1314_audit_hardening.rs");
}
