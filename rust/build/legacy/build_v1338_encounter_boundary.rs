use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1337_audit_hardening.rs");
    pub fn run() { main(); }
}

fn main() {
    previous::run();

    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let path = out.join("telemetry_v170_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .expect("read generated telemetry for encounter boundary regression")
        .replace("\r\n", "\n");

    // v1.33.7 already fixed the early-take bug in generated telemetry. Assert
    // the final production implementation instead of matching obsolete source.
    let fixed = "if self.pending_boundary.is_some_and(|boundary| {\n            now.saturating_duration_since(boundary) >= SEGMENT_BOUNDARY_DELAY\n        })";
    assert_eq!(source.matches(fixed).count(), 1,
        "v1.33.8: generated telemetry must retain the safe boundary guard");
    assert!(!source.contains("self.pending_boundary.take()"),
        "v1.33.8: pending boundary must not be consumed on a fast repull");

    source.push_str(r#"

#[cfg(test)]
mod v1338_boundary_regression_tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn attack_during_grace_does_not_discard_pending_boundary() {
        let (tx, _rx) = mpsc::channel();
        let mut runtime = TelemetryRuntime::new(tx);
        runtime.local_uid = 42;
        runtime.encounter_started = Some(Instant::now());
        runtime.combat.entry(42).or_default().damage = 1_000;
        runtime.arm_boundary();
        let boundary = runtime.pending_boundary.expect("boundary armed");

        runtime.prepare_segment(boundary + Duration::from_millis(500));
        assert_eq!(runtime.pending_boundary, Some(boundary));
        assert_eq!(runtime.combat.get(&42).map(|row| row.damage), Some(1_000));

        runtime.prepare_segment(boundary + SEGMENT_BOUNDARY_DELAY);
        assert!(runtime.pending_boundary.is_none());
        assert_eq!(runtime.combat.get(&42).map(|row| row.damage), Some(0));
    }

    #[test]
    fn repeated_fast_hits_do_not_extend_boundary_deadline() {
        let (tx, _rx) = mpsc::channel();
        let mut runtime = TelemetryRuntime::new(tx);
        runtime.local_uid = 42;
        runtime.encounter_started = Some(Instant::now());
        runtime.arm_boundary();
        let boundary = runtime.pending_boundary.unwrap();
        for millis in [100, 300, 900, 1_500, 2_900] {
            runtime.prepare_segment(boundary + Duration::from_millis(millis));
            assert_eq!(runtime.pending_boundary, Some(boundary));
        }
        runtime.prepare_segment(boundary + SEGMENT_BOUNDARY_DELAY);
        assert!(runtime.pending_boundary.is_none());
    }
}
"#);

    fs::write(&path, source).expect("write v1.33.8 encounter boundary regression");
    println!("cargo:rerun-if-changed=build/legacy/build_v1338_encounter_boundary.rs");
}
