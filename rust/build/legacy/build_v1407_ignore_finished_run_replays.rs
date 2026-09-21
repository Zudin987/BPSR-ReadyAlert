// A delayed full/dirty flow update for a completed previous run must not
// overwrite the new run's identity, even if the update is Playing/Active.
use std::{env, fs, path::PathBuf};
mod previous { include!("build_v1406_manual_reset_once.rs"); pub fn run() { main(); } }
fn patch(source: &mut String, old: &str, new: &str, reason: &str) {
    assert_eq!(source.matches(old).count(), 1, "v1407: {reason} anchor changed");
    *source = source.replacen(old, new, 1);
}
fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let path = out.join("telemetry_v170_fixed.rs");
    let mut t = fs::read_to_string(&path).expect("generated telemetry");
    patch(&mut t,
        "    playing_run_uuid: Option<i64>,\n    completed_run_uuid: Option<i64>,",
        "    playing_run_uuid: Option<i64>,\n    completed_run_uuid: Option<i64>,\n    last_finished_run_uuid: Option<i64>,",
        "remember finalized run identity");
    patch(&mut t,
        "            playing_run_uuid: None,\n            completed_run_uuid: None,",
        "            playing_run_uuid: None,\n            completed_run_uuid: None,\n            last_finished_run_uuid: None,",
        "finalized run initialization");
    patch(&mut t,
        "                            self.reset_scene();\n                            self.run_identity_changed = false;",
        "                            if let Some(completed) = self.completed_run_uuid {\n                                self.last_finished_run_uuid = Some(completed);\n                            }\n                            self.reset_scene();\n                            self.run_identity_changed = false;",
        "retain completed run across scene reentry");
    patch(&mut t,
        "    fn observe_run_flow(&mut self, run: i64, flow: i32) {\n        if run <= 0 { return; }",
        r#"    fn observe_run_flow(&mut self, run: i64, flow: i32) {
        if run <= 0 { return; }
        // A delayed Playing/Active/full-sync for the just-finished raid must
        // not rewind the new raid's UUID and provoke a false scene reset.
        if self.last_finished_run_uuid == Some(run)
            && self.current_run_uuid.is_some_and(|current| current != run) {
            crate::logging::write("encounter: stale finished-run dungeon flow ignored");
            return;
        }"#,
        "drop replayed old-run Playing and other flow");
    patch(&mut t,
        "            self.arm_boundary();\n            self.completed_run_uuid = None;\n            crate::logging::write(\"encounter: completed raid followed by new run Playing; splitting before new damage\");",
        "            self.last_finished_run_uuid = self.completed_run_uuid;\n            self.arm_boundary();\n            self.completed_run_uuid = None;\n            crate::logging::write(\"encounter: completed raid followed by new run Playing; splitting before new damage\");",
        "latch old UUID on new Playing");
    patch(&mut t,
        "        self.run_identity_changed = false;\n        self.completed_run_uuid = None;\n        crate::telemetry::clear_capture_partial();",
        "        self.run_identity_changed = false;\n        self.completed_run_uuid = None;\n        self.last_finished_run_uuid = None;\n        crate::telemetry::clear_capture_partial();",
        "clear old run latch on manual reset");
    t.push_str(r#"
#[cfg(test)]
mod v1407_finished_run_replay_tests {
    use super::*;
    #[test]
    fn delayed_old_playing_and_active_cannot_rewind_new_raid_identity() {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut runtime = TelemetryRuntime::new(tx);
        runtime.current_scene_id = 77;
        runtime.encounter_started = Instant::now().checked_sub(Duration::from_secs(2));
        runtime.combat.entry(42).or_default().damage = 123;
        runtime.observe_run_flow(11, 3);
        runtime.observe_run_flow(11, 5);
        runtime.observe_run_flow(12, 3);
        assert_eq!(runtime.current_run_uuid, Some(12));
        assert_eq!(runtime.last_finished_run_uuid, Some(11));
        runtime.encounter_started = Instant::now().checked_sub(Duration::from_secs(2));
        runtime.combat.entry(42).or_default().damage = 75;
        runtime.observe_run_flow(11, 3); // old Playing replay, not a new run
        runtime.observe_run_flow(11, 1); // old full-sync Active replay
        assert_eq!(runtime.current_run_uuid, Some(12));
        assert_eq!(runtime.playing_run_uuid, Some(12));
        assert_eq!(runtime.combat.get(&42).unwrap().damage, 75);
        runtime.observe_run_flow(12, 5);
        assert_eq!(runtime.completed_run_uuid, Some(12));
        assert_eq!(rx.try_iter().filter(|event| matches!(event, AppEvent::Dps(_))).count(), 2);
    }
}
"#);
    fs::write(&path, t).expect("write stale finished-run flow guard");
    println!("cargo:rerun-if-changed=build/legacy/build_v1407_ignore_finished_run_replays.rs");
}
