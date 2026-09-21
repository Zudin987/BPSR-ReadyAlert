// Split only a corroborated completed raid, not a boss death, idle period,
// ambiguous same-scene refresh, or an uncorrelated dungeon End notification.
use std::{env, fs, path::PathBuf};
mod previous { include!("build_v1403_wipe_history_once.rs"); pub fn run() { main(); } }
fn patch(source: &mut String, old: &str, new: &str, reason: &str) {
    assert_eq!(source.matches(old).count(), 1, "v1404: {reason} anchor changed");
    *source = source.replacen(old, new, 1);
}
fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let path = out.join("telemetry_v170_fixed.rs");
    let mut t = fs::read_to_string(&path).expect("generated telemetry");
    patch(&mut t,
        "    current_run_uuid: Option<i64>,\n    run_identity_changed: bool,",
        "    current_run_uuid: Option<i64>,\n    run_identity_changed: bool,\n    playing_run_uuid: Option<i64>,\n    completed_run_uuid: Option<i64>,",
        "correlated raid fields");
    patch(&mut t,
        "            current_run_uuid: None,\n            run_identity_changed: false,",
        "            current_run_uuid: None,\n            run_identity_changed: false,\n            playing_run_uuid: None,\n            completed_run_uuid: None,",
        "raid field initialization");
    patch(&mut t,
        "if self.current_scene_id == id && !self.run_identity_changed {",
        "if self.current_scene_id == id && !self.run_identity_changed\n                            && !self.current_run_uuid.is_some_and(|run| self.completed_run_uuid == Some(run)) {",
        "same-map completed raid requires a fresh scene entry");
    patch(&mut t,
        "                            self.run_identity_changed = false;\n                            self.current_run_uuid = None;",
        "                            self.run_identity_changed = false;\n                            self.current_run_uuid = None;\n                            self.playing_run_uuid = None;\n                            self.completed_run_uuid = None;",
        "clear correlation on real scene replacement");
    patch(&mut t,
        "                    self.observe_run_identity(scene_uuid);",
        "                    self.observe_run_flow(scene_uuid, flow);",
        "correlate full dungeon flow");
    patch(&mut t,
        "                    if let Some(run) = scene_uuid { self.observe_run_identity(run); }",
        "                    if let Some(run) = scene_uuid { self.observe_run_flow(run, flow); }",
        "correlate only UUID-bearing dirty flow");
    patch(&mut t,
        "    /// User-requested training-dummy reset. Keeps roster and current scene context.",
        r#"    // The terminal flow value alone is never a reset. It must follow Playing
    // for the same nonzero run UUID while combat is active. A fresh same-map
    // EnterScene or a distinct run's Playing is then the second boundary signal.
    fn observe_run_flow(&mut self, run: i64, flow: i32) {
        if run <= 0 { return; }
        // Delayed terminal packets from an older run must not replace the newer
        // identity or turn a live raid into an apparent completed one.
        if matches!(flow, 4 | 5) && self.current_run_uuid.is_some_and(|current| current != run) {
            crate::logging::write("encounter: stale dungeon terminal UUID ignored");
            return;
        }
        if flow == 3 && self.completed_run_uuid.is_some_and(|completed| completed != run)
            && self.encounter_started.is_some() {
            // Completed old run + different UUID now Playing: no EnterScene is
            // required for the meter to archive the previous raid. Use the same
            // final snapshot/empty boundary and Live retention as a confirmed wipe.
            self.arm_boundary();
            self.completed_run_uuid = None;
            crate::logging::write("encounter: completed raid followed by new run Playing; splitting before new damage");
        }
        self.observe_run_identity(run);
        match flow {
            3 if self.completed_run_uuid != Some(run) => self.playing_run_uuid = Some(run),
            // Settlement is used rather than End: End may be intermediate or
            // ambiguous in a multi-phase raid, and boss HP/death is not proof.
            5 if self.playing_run_uuid == Some(run) && self.encounter_started.is_some() => {
                self.completed_run_uuid = Some(run);
                crate::logging::write("encounter: same-run Playing -> Settlement confirmed; awaiting new run or scene entry");
            }
            _ => {}
        }
    }

    /// User-requested training-dummy reset. Keeps roster and current scene context."#,
        "strict completion correlation");
    patch(&mut t,
        "        self.run_identity_changed = false;\n        crate::telemetry::clear_capture_partial();",
        "        self.run_identity_changed = false;\n        self.completed_run_uuid = None;\n        crate::telemetry::clear_capture_partial();",
        "manual reset discards completion latch");
    t.push_str(r#"
#[cfg(test)]
mod v1404_confirmed_raid_reentry_tests {
    use super::*;
    fn active() -> (TelemetryRuntime, std::sync::mpsc::Receiver<AppEvent>) {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut runtime = TelemetryRuntime::new(tx);
        runtime.current_scene_id = 77;
        runtime.local_uid = 42;
        runtime.team.insert(42);
        runtime.encounter_started = Instant::now().checked_sub(Duration::from_secs(2));
        runtime.combat.entry(42).or_default().damage = 123;
        (runtime, rx)
    }
    fn same_scene() -> Vec<u8> {
        // EnterScene.info.scene_attrs.attributes[0] = (0x155, scene=77).
        vec![0x0a, 10, 0x0a, 8, 0x12, 6, 0x08, 0xd5, 0x02, 0x12, 1, 77]
    }
    #[test]
    fn duplicate_scene_and_uncorroborated_end_keep_active_combat() {
        let (mut runtime, _) = active();
        runtime.observe_run_flow(11, 3);
        runtime.observe_run_flow(11, 4); // End alone is not reliable completion.
        runtime.handle_notify(proto::WORLD_SERVICE, proto::ENTER_SCENE_METHOD, &same_scene());
        assert!(runtime.encounter_started.is_some());
        assert_eq!(runtime.combat.get(&42).unwrap().damage, 123);
        assert_eq!(runtime.completed_run_uuid, None);
    }
    #[test]
    fn same_scene_reentry_after_matching_settlement_splits() {
        let (mut runtime, rx) = active();
        runtime.observe_run_flow(11, 3);
        runtime.observe_run_flow(11, 5);
        assert_eq!(runtime.combat.get(&42).unwrap().damage, 123);
        runtime.handle_notify(proto::WORLD_SERVICE, proto::ENTER_SCENE_METHOD, &same_scene());
        assert!(runtime.encounter_started.is_none());
        assert_eq!(runtime.combat.get(&42).unwrap().damage, 0);
        let snapshots: Vec<_> = rx.try_iter().filter_map(|event| match event {
            AppEvent::Dps(snapshot) => Some(snapshot), _ => None,
        }).collect();
        assert_eq!(snapshots.first().unwrap().total_damage, 123);
        assert!(snapshots.iter().any(|snapshot| snapshot.total_damage == 0));
    }
    #[test]
    fn distinct_playing_uuid_after_settlement_splits_once_without_scene_packet() {
        let (mut runtime, rx) = active();
        runtime.observe_run_flow(11, 3);
        runtime.observe_run_flow(11, 5);
        runtime.observe_run_flow(12, 3);
        assert!(runtime.encounter_started.is_none());
        assert_eq!(runtime.combat.get(&42).unwrap().damage, 0);
        runtime.observe_run_flow(12, 3);
        runtime.observe_run_flow(11, 5); // delayed old terminal must not rewind identity
        assert_eq!(runtime.current_run_uuid, Some(12));
        let snapshots: Vec<_> = rx.try_iter().filter_map(|event| match event {
            AppEvent::Dps(snapshot) => Some(snapshot), _ => None,
        }).collect();
        assert_eq!(snapshots.len(), 2);
        assert_eq!(snapshots[0].total_damage, 123);
        assert_eq!(snapshots[1].total_damage, 0);
    }
}
"#);
    fs::write(&path, t).expect("write correlated raid lifecycle");
    println!("cargo:rerun-if-changed=build/legacy/build_v1404_confirmed_raid_reentry.rs");
}
