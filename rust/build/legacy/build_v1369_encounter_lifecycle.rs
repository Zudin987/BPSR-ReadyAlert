// Patch the final generated telemetry core, not a legacy input that later
// builders overwrite. These anchors match the post-v1368 generated source.
use std::{env, fs, path::PathBuf};
mod previous {
    include!("build_v1368_audit_test_alignment.rs");
    pub fn run() { main(); }
}
fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "encounter lifecycle: {label}: expected one anchor; got {count}");
    *source = source.replacen(from, to, 1);
}
fn main() {
    previous::run();
    let path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("telemetry_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("generated telemetry core").replace("\r\n", "\n");

    // Emit the complete encounter before either reset clears combat. The
    // existing post-reset empty snapshot must still follow the final one.
    replace_once(&mut source,
        "    pub fn manual_reset(&mut self) {\n        crate::event_tracker::reset_counts();",
        "    pub fn manual_reset(&mut self) {\n        if self.encounter_started.is_some() { self.emit_dps(); }\n        crate::event_tracker::reset_counts();",
        "manual-reset final snapshot");
    replace_once(&mut source,
        "    fn reset_scene(&mut self) {\n        self.current_scene_id = 0;",
        "    fn reset_scene(&mut self) {\n        if self.encounter_started.is_some() { self.emit_dps(); }\n        self.current_scene_id = 0;",
        "scene-reset final snapshot");

    // The base generator already removed the arbitrary 20-minute rollover,
    // already ignores ordinary target despawns and already retains an immature
    // pending wipe boundary. Do not reintroduce older lifecycle behavior.
    assert!(!source.contains("MAX_SEGMENT"), "obsolete time-limit rollover reappeared");
    let despawn = source.split("    fn handle_near_entities(").nth(1)
        .and_then(|part| part.split("    fn handle_delta(").next())
        .expect("near-entities section");
    assert!(!despawn.contains("self.arm_boundary()"), "despawn must not arm an encounter boundary");

    // The first post-wipe hit is the only time prepare_segment is called: send
    // the last complete snapshot BEFORE clearing the old encounter so its
    // buffered last damage cannot disappear when the next pull starts.
    replace_once(&mut source,
        "        }) {\n            self.reset_encounter_keep_roster();\n        }\n        self.encounter_started.get_or_insert(now);",
        "        }) {\n            if self.encounter_started.is_some() { self.emit_dps(); }\n            self.reset_encounter_keep_roster();\n        }\n        self.encounter_started.get_or_insert(now);",
        "post-wipe rollover final snapshot");

    source.push_str(r#"
#[cfg(test)]
mod encounter_lifecycle_v1369_tests {
    use super::*;
    fn active_runtime() -> (TelemetryRuntime, std::sync::mpsc::Receiver<AppEvent>) {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut runtime = TelemetryRuntime::new(tx);
        runtime.local_uid = 42;
        runtime.team.insert(42);
        runtime.encounter_started = Instant::now().checked_sub(Duration::from_secs(2));
        runtime.combat.entry(42).or_default().damage = 123;
        (runtime, rx)
    }
    fn dps_snapshots(rx: std::sync::mpsc::Receiver<AppEvent>) -> Vec<DpsSnapshot> {
        rx.try_iter().filter_map(|event| match event {
            AppEvent::Dps(snapshot) => Some(snapshot),
            _ => None,
        }).collect()
    }
    #[test]
    fn manual_reset_emits_final_damage_before_empty_snapshot() {
        let (mut runtime, rx) = active_runtime();
        runtime.manual_reset();
        let snapshots = dps_snapshots(rx);
        assert_eq!(snapshots.len(), 2);
        assert_eq!(snapshots[0].total_damage, 123);
        assert_eq!(snapshots[1].total_damage, 0);
    }
    #[test]
    fn scene_change_emits_final_damage_before_empty_snapshot() {
        let (mut runtime, rx) = active_runtime();
        runtime.reset_scene();
        let snapshots = dps_snapshots(rx);
        assert_eq!(snapshots.len(), 2);
        assert_eq!(snapshots[0].total_damage, 123);
        assert_eq!(snapshots[1].total_damage, 0);
    }
    #[test]
    fn early_repull_preserves_pending_boundary_and_accumulated_damage() {
        let (mut runtime, rx) = active_runtime();
        runtime.pending_boundary = Some(Instant::now());
        runtime.prepare_segment(Instant::now());
        assert!(runtime.pending_boundary.is_some());
        assert_eq!(runtime.combat.get(&42).unwrap().damage, 123);
        assert!(dps_snapshots(rx).is_empty());
    }
    #[test]
    fn elapsed_boundary_flushes_old_pull_before_reset() {
        let (mut runtime, rx) = active_runtime();
        let now = Instant::now();
        runtime.pending_boundary = now.checked_sub(SEGMENT_BOUNDARY_DELAY + Duration::from_millis(1));
        runtime.prepare_segment(now);
        assert!(runtime.pending_boundary.is_none());
        assert_eq!(runtime.combat.get(&42).unwrap().damage, 0);
        let snapshots = dps_snapshots(rx);
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].total_damage, 123);
    }
    #[test]
    fn long_encounter_without_boundary_survives() {
        let (mut runtime, rx) = active_runtime();
        let old = Instant::now().checked_sub(Duration::from_secs(21 * 60)).unwrap();
        runtime.encounter_started = Some(old);
        runtime.prepare_segment(Instant::now());
        assert_eq!(runtime.encounter_started, Some(old));
        assert_eq!(runtime.combat.get(&42).unwrap().damage, 123);
        assert!(dps_snapshots(rx).is_empty());
    }
}
"#);
    fs::write(path, source).expect("write encounter-safe telemetry");
    println!("cargo:rerun-if-changed=build/legacy/build_v1369_encounter_lifecycle.rs");
}
