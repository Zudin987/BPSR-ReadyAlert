// Arming while combat is active requests a reset. The reset now emits the
// preceding encounter's final DPS before its empty snapshot; that old final
// DPS must be archived, not mistaken for the benchmark's first attack.
use std::{env, fs, path::PathBuf};
mod previous {
    include!("build_v1373_archive_ack.rs");
    pub fn run() { main(); }
}
fn replace_once(src: &mut String, from: &str, to: &str, label: &str) {
    let count = src.matches(from).count();
    assert_eq!(count, 1, "benchmark arm boundary {label}: expected one anchor, found {count}");
    *src = src.replacen(from, to, 1);
}
fn main() {
    previous::run();
    let path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"))
        .join("telemetry_v1270_lifecycle.rs");
    let mut src = fs::read_to_string(&path).expect("generated benchmark wrapper")
        .replace("\r\n", "\n");
    replace_once(&mut src,
        "    benchmark: Option<BenchmarkRun>,\n    completing: Option<u64>,",
        "    benchmark: Option<BenchmarkRun>,\n    completing: Option<u64>,\n    awaiting_arm_reset: bool,",
        "reset barrier field");
    replace_once(&mut src,
        "            benchmark: None,\n            completing: None,",
        "            benchmark: None,\n            completing: None,\n            awaiting_arm_reset: false,",
        "reset barrier initialization");
    replace_once(&mut src,
        "            Some(BenchmarkCommand::Arm(config)) => {\n                if let Some((snapshot, context)) = self.last.take() {\n                    self.queue_archive(snapshot, context);\n                }\n                self.benchmark = Some(BenchmarkRun::Armed(config));",
        "            Some(BenchmarkCommand::Arm(config)) => {\n                // Wait for the preceding encounter's final DPS and the actual\n                // empty reset before starting the new benchmark timer.\n                self.awaiting_arm_reset = true;\n                self.benchmark = Some(BenchmarkRun::Armed(config));",
        "defer previous encounter archive until final snapshot");
    replace_once(&mut src,
        "            Some(BenchmarkCommand::Cancel) => {\n                ACTIVE_BENCHMARK_ID.store(0, Ordering::Release);\n                self.benchmark = None;",
        "            Some(BenchmarkCommand::Cancel) => {\n                ACTIVE_BENCHMARK_ID.store(0, Ordering::Release);\n                self.awaiting_arm_reset = false;\n                self.benchmark = None;",
        "cancel clears reset barrier");
    replace_once(&mut src,
        "            Some(BenchmarkRun::Armed(config)) => {\n                if !has_benchmark_activity(next) {",
        "            Some(BenchmarkRun::Armed(config)) => {\n                if self.awaiting_arm_reset {\n                    // The preceding encounter's unthrottled final snapshot\n                    // arrives before the empty reset. Keep its last hit.\n                    if next.encounter_ms == 0\n                        && next.total_damage == 0\n                        && next.total_healing == 0\n                        && next.total_damage_taken == 0\n                    {\n                        if let Some((snapshot, context)) = self.last.take() {\n                            self.queue_archive(snapshot, context);\n                        }\n                        self.awaiting_arm_reset = false;\n                    } else if encounter_store::has_combat_data(next) {\n                        let context = self.last.as_ref()\n                            .map(|(_, context)| context.clone())\n                            .unwrap_or_else(encounter_context::snapshot);\n                        self.last = Some((next.clone(), context));\n                    }\n                    return;\n                }\n                if !has_benchmark_activity(next) {",
        "ignore pre-reset activity and flush latest old damage");
    src.push_str(r#"
#[cfg(test)]
mod benchmark_arm_boundary_v1374_tests {
    use super::*;
    use crate::model::DpsRow;

    #[test]
    fn old_final_hit_cannot_start_new_benchmark_and_is_kept_for_history() {
        let (tx, _rx) = mpsc::channel();
        let mut runtime = TelemetryRuntime::new(tx);
        // Disable writes to the real Encounter History folder in this test.
        runtime.archive_tx = None;
        let config = BenchmarkConfig { id: 44, name: "Next run".into(), seconds: 300 };
        runtime.benchmark = Some(BenchmarkRun::Armed(config));
        runtime.awaiting_arm_reset = true;
        runtime.last = Some((DpsSnapshot { encounter_ms: 10_000, total_damage: 100,
            rows: vec![DpsRow { is_local: true, damage: 100, ..Default::default() }],
            ..Default::default() }, EncounterContextSnapshot::default()));
        let final_old = DpsSnapshot { encounter_ms: 10_001, total_damage: 125,
            rows: vec![DpsRow { is_local: true, damage: 125, ..Default::default() }],
            ..Default::default() };
        runtime.observe_snapshot(&final_old);
        assert!(matches!(runtime.benchmark, Some(BenchmarkRun::Armed(_))));
        assert!(runtime.awaiting_arm_reset);
        assert_eq!(runtime.last.as_ref().unwrap().0.total_damage, 125);
        runtime.observe_snapshot(&DpsSnapshot::default());
        assert!(!runtime.awaiting_arm_reset);
        assert!(runtime.last.is_none());
        assert!(matches!(runtime.benchmark, Some(BenchmarkRun::Armed(_))));
    }

    #[test]
    fn no_old_encounter_still_requires_empty_reset_before_benchmark_starts() {
        let (tx, _rx) = mpsc::channel();
        let mut runtime = TelemetryRuntime::new(tx);
        runtime.archive_tx = None;
        runtime.benchmark = Some(BenchmarkRun::Armed(BenchmarkConfig {
            id: 45, name: "Idle start".into(), seconds: 300,
        }));
        runtime.awaiting_arm_reset = true;
        runtime.observe_snapshot(&DpsSnapshot::default());
        assert!(!runtime.awaiting_arm_reset);
        assert!(matches!(runtime.benchmark, Some(BenchmarkRun::Armed(_))));
        assert!(runtime.last.is_none());
    }
}
"#);
    fs::write(path, src).expect("write benchmark arm boundary");
    println!("cargo:rerun-if-changed=build/legacy/build_v1374_benchmark_arm_boundary.rs");
}
