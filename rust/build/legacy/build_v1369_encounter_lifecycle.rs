// Encounter-safety changes are applied to the final generated core, not to an
// obsolete legacy source that earlier builders overwrite.
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

    // The UI/archive layer must see the final, unthrottled snapshot BEFORE
    // receiving the empty snapshot. Otherwise the last 100 ms of combat can be
    // absent from history when a scene change or manual reset arrives.
    replace_once(&mut source,
        "    pub fn manual_reset(&mut self) {\n        self.reset_encounter_keep_roster();",
        "    pub fn manual_reset(&mut self) {\n        if self.encounter_started.is_some() { self.emit_dps(); }\n        self.reset_encounter_keep_roster();",
        "manual-reset final snapshot");
    replace_once(&mut source,
        "    fn reset_scene(&mut self) {\n        self.entities.clear();",
        "    fn reset_scene(&mut self) {\n        if self.encounter_started.is_some() { self.emit_dps(); }\n        self.entities.clear();",
        "scene-reset final snapshot");

    // A time limit is not proof that a dungeon/pull ended. Long encounters
    // remain intact until an observed boundary or an explicit user reset.
    replace_once(&mut source,
        "        if self.encounter_started.is_some_and(|started| started.elapsed() >= MAX_SEGMENT) {\n            self.reset_encounter_keep_roster();\n        }\n        if let Some(boundary) = self.pending_boundary.take() {\n            if now.saturating_duration_since(boundary) >= SEGMENT_BOUNDARY_DELAY {\n                self.reset_encounter_keep_roster();\n            }\n        }",
        "        // Never clear solely because MAX_SEGMENT elapsed.\n        if self.pending_boundary.is_some_and(|boundary| now.saturating_duration_since(boundary) >= SEGMENT_BOUNDARY_DELAY) {\n            // Preserve the final hit even if DPS snapshots were throttled.\n            self.emit_dps();\n            self.reset_encounter_keep_roster();\n            self.pending_boundary = None;\n        }",
        "guarded encounter rollover");

    // A target may despawn during ordinary boss phases or on distance/streaming
    // changes. Only an observed death is enough evidence for a boundary here.
    replace_once(&mut source,
        "            if uuid == self.last_target && self.encounter_started.is_some() {\n                self.arm_boundary();\n            }\n            self.entities.remove(&uuid);",
        "            if uuid == self.last_target && self.encounter_started.is_some()\n                && self.entities.get(&uuid).is_some_and(|meta| meta.actor_state == ACTOR_STATE_DEAD || (meta.max_hp > 0 && meta.hp <= 0))\n            {\n                self.arm_boundary();\n            }\n            self.entities.remove(&uuid);",
        "disappearing-target death evidence");

    // These test fixtures exercise edge conditions rather than waiting 20 min.
    source.push_str(r#"
#[cfg(test)]
mod encounter_lifecycle_v1369_tests {
    use super::*;
    #[test]
    fn early_repull_does_not_consume_pending_boundary() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let mut runtime = TelemetryRuntime::new(tx);
        runtime.encounter_started = Some(Instant::now());
        runtime.pending_boundary = Some(Instant::now());
        runtime.prepare_segment(Instant::now());
        assert!(runtime.pending_boundary.is_some());
    }
    #[test]
    fn elapsed_boundary_resets_a_new_pull() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let mut runtime = TelemetryRuntime::new(tx);
        runtime.encounter_started = Some(Instant::now());
        runtime.pending_boundary = Instant::now().checked_sub(SEGMENT_BOUNDARY_DELAY + Duration::from_millis(1));
        runtime.prepare_segment(Instant::now());
        assert!(runtime.pending_boundary.is_none());
    }
    #[test]
    fn long_encounter_without_boundary_survives() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let mut runtime = TelemetryRuntime::new(tx);
        let old = Instant::now().checked_sub(Duration::from_secs(21 * 60)).unwrap();
        runtime.encounter_started = Some(old);
        runtime.prepare_segment(Instant::now());
        assert_eq!(runtime.encounter_started, Some(old));
    }
}
"#);
    fs::write(path, source).expect("write encounter-safe telemetry");
    println!("cargo:rerun-if-changed=build/legacy/build_v1369_encounter_lifecycle.rs");
}
