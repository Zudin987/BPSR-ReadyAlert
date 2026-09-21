// Archive a confirmed wipe before holding its completed Live snapshot; consume
// the boundary exactly once and never re-archive it on a rapid repull or Reset.
use std::{env, fs, path::PathBuf};
mod previous { include!("build_v1402_scene_tracker_consistency.rs"); pub fn run() { main(); } }
fn patch(source: &mut String, old: &str, new: &str, reason: &str) {
    assert_eq!(source.matches(old).count(), 1, "v1403: {reason} anchor changed");
    *source = source.replacen(old, new, 1);
}
fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let overlay_path = out.join("feature_overlays_v170_fixed.rs");
    let mut ui = fs::read_to_string(&overlay_path).expect("native meter generated source");
    patch(&mut ui, "dps: DpsSnapshot,\ndps_updated_unix_ms: i64,",
        "dps: DpsSnapshot,\n// The finished Live view is already stored in History; do not archive it again.\nwipe_archived: bool,\ndps_updated_unix_ms: i64,", "state boundary flag");
    assert_eq!(ui.matches("dps:DpsSnapshot::default(),dps_updated_unix_ms:0,").count(), 2,
        "v1403: main and compact QA initializers changed");
    ui = ui.replace("dps:DpsSnapshot::default(),dps_updated_unix_ms:0,",
        "dps:DpsSnapshot::default(),wipe_archived:false,dps_updated_unix_ms:0,");
    patch(&mut ui, "compact_mode: false, dps: DpsSnapshot::default(),",
        "compact_mode: false, dps: DpsSnapshot::default(), wipe_archived: false,",
        "Windows hover QA initializer");
    patch(&mut ui,
        "        && crate::telemetry::take_wipe_display_hold() { return; }",
        r#"        && crate::telemetry::take_wipe_display_hold() {
        // The overlay is the owner of native History. An early return here
        // used to discard the empty boundary and never archive the wipe.
        with_state(hwnd, |state| {
            if !state.wipe_archived && history::encounter_rolled(&state.dps, &snapshot) {
                archive_live_snapshot(state);
                state.wipe_archived = true;
                if !state.features.read().map(|f| f.meter.remember_scroll).unwrap_or(false) {
                    state.scroll = 0;
                }
            }
        });
        return;
    }"#, "archive empty wipe boundary before holding Live");
    patch(&mut ui,
        "        let rolled=history::encounter_rolled(&state.dps,&snapshot);\n        retain_admitted_meter_rows(&state.dps,&mut snapshot,rolled);",
        r#"        // Empty metadata refreshes after a wipe are not a new encounter.
        // Keep the completed Live view until combat really resumes.
        if state.wipe_archived && snapshot.total_damage == 0
            && snapshot.total_healing == 0 && snapshot.total_damage_taken == 0 { return; }
        let rolled = !state.wipe_archived && history::encounter_rolled(&state.dps, &snapshot);
        // A new pull must not inherit admitted players from the prior attempt.
        retain_admitted_meter_rows(&state.dps, &mut snapshot, rolled || state.wipe_archived);
        state.wipe_archived = false;"#, "new pull must not rearchive the held wipe");
    patch(&mut ui,
        "fn archive_live_snapshot(state:&mut State){\n    let days=",
        "fn archive_live_snapshot(state:&mut State){\n    if state.wipe_archived { return; }\n    let days=",
        "manual reset must not duplicate an archived wipe");
    patch(&mut ui,
        "            archive_live_snapshot(state);\n            crate::telemetry::request_manual_reset();\n            state.dps = DpsSnapshot::default();",
        "            archive_live_snapshot(state);\n            crate::telemetry::request_manual_reset();\n            state.dps = DpsSnapshot::default();\n            state.wipe_archived = false;",
        "manual reset clears held boundary");
    fs::write(&overlay_path, ui).expect("write wipe history fix");
    println!("cargo:rerun-if-changed=build/legacy/build_v1403_wipe_history_once.rs");
}
