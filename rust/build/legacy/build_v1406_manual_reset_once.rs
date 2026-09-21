// Prevent the native meter from archiving the same attempt twice when a
// toolbar Reset saves Live immediately and telemetry later emits final + empty.
use std::{env, fs, path::PathBuf};
mod previous { include!("build_v1405_preserve_preentry_run.rs"); pub fn run() { main(); } }
fn patch(source: &mut String, old: &str, new: &str, reason: &str) {
    assert_eq!(source.matches(old).count(), 1, "v1406: {reason} anchor changed");
    *source = source.replacen(old, new, 1);
}
fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut ui = fs::read_to_string(&path).expect("native overlay generated source");
    patch(&mut ui,
        "wipe_archived: bool,\ndps_updated_unix_ms: i64,",
        "wipe_archived: bool,\n// Reset archives immediately; consume the subsequent telemetry replay once.\nmanual_reset_pending: bool,\ndps_updated_unix_ms: i64,",
        "manual reset state flag");
    assert_eq!(ui.matches("wipe_archived:false,dps_updated_unix_ms:0,").count(), 2,
        "v1406: normal and compact initializers changed");
    ui = ui.replace("wipe_archived:false,dps_updated_unix_ms:0,",
        "wipe_archived:false,manual_reset_pending:false,dps_updated_unix_ms:0,");
    patch(&mut ui,
        "wipe_archived: false,\n            dps_updated_unix_ms: 0,",
        "wipe_archived: false, manual_reset_pending: false,\n            dps_updated_unix_ms: 0,",
        "Windows QA state initializer");
    patch(&mut ui,
        "pub unsafe fn update_dps(hwnd:HWND,mut snapshot:DpsSnapshot){\n    // History receives",
        r#"// An explicit Reset has already saved Live. The capture worker will emit the
// same old final snapshot followed by an empty acknowledgement: neither may
// produce a second archive or resurrect the cleared meter.
fn ignore_manual_reset_replay(waiting: &mut bool, snapshot: &DpsSnapshot) -> bool {
    if !*waiting { return false; }
    if snapshot.encounter_ms == 0 && snapshot.total_damage == 0
        && snapshot.total_healing == 0 && snapshot.total_damage_taken == 0 {
        *waiting = false;
    }
    true
}

pub unsafe fn update_dps(hwnd:HWND,mut snapshot:DpsSnapshot){
    let mut reset_replay = false;
    with_state(hwnd, |state| {
        reset_replay = ignore_manual_reset_replay(&mut state.manual_reset_pending, &snapshot);
    });
    if reset_replay {
        // A wipe hold left in the queue must not apply to a future encounter.
        if snapshot.encounter_ms == 0 && snapshot.total_damage == 0
            && snapshot.total_healing == 0 && snapshot.total_damage_taken == 0 {
            let _ = crate::telemetry::take_wipe_display_hold();
        }
        return;
    }
    // History receives"#,
        "swallow replay of already archived manual Reset");
    patch(&mut ui,
        "            state.dps = DpsSnapshot::default();\n            state.wipe_archived = false;",
        "            state.dps = DpsSnapshot::default();\n            state.wipe_archived = false;\n            state.manual_reset_pending = true;",
        "flag explicit manual reset");
    ui.push_str(r#"
#[cfg(test)]
mod v1406_manual_reset_history_tests {
    use super::*;
    #[test]
    fn toolbar_reset_consumes_old_final_and_empty_once_then_allows_new_combat() {
        let mut waiting = true;
        let mut old = DpsSnapshot::default();
        old.encounter_ms = 1500;
        old.total_damage = 123;
        assert!(ignore_manual_reset_replay(&mut waiting, &old));
        assert!(waiting);
        assert!(ignore_manual_reset_replay(&mut waiting, &DpsSnapshot::default()));
        assert!(!waiting);
        assert!(!ignore_manual_reset_replay(&mut waiting, &old));
    }
}
"#);
    fs::write(&path, ui).expect("write manual reset history deduplication");
    println!("cargo:rerun-if-changed=build/legacy/build_v1406_manual_reset_once.rs");
}
