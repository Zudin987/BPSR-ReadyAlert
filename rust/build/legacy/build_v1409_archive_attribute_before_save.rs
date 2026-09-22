// The archive layer clones incoming DPS snapshots before the top-level
// telemetry wrapper receives them. Annotate here or saved encounters lose all
// attribute summaries even though the live UI sees them.
use std::{env, fs, path::PathBuf};
mod previous {
    include!("build_v1408_encounter_attribute_row.rs");
    pub fn run() { main(); }
}
fn patch(source: &mut String, old: &str, new: &str, label: &str) {
    assert_eq!(source.matches(old).count(), 1, "v1409: {label} anchor changed");
    *source = source.replacen(old, new, 1);
}
fn main() {
    previous::run();
    let path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"))
        .join("telemetry_v1270_lifecycle.rs");
    let mut source = fs::read_to_string(&path).expect("generated archive telemetry");
    patch(&mut source,
        "    last: Option<(DpsSnapshot, EncounterContextSnapshot)>,",
        "    last: Option<(DpsSnapshot, EncounterContextSnapshot)>,\n    attribute_recorder: crate::telemetry::Recorder,",
        "archive recorder field");
    patch(&mut source,
        "            last: None,",
        "            last: None,\n            attribute_recorder: crate::telemetry::Recorder::default(),",
        "archive recorder initializer");
    patch(&mut source,
        "        while let Ok(event) = self.inner_rx.try_recv() {\n            if let AppEvent::Dps(snapshot) = &event {\n                self.observe_snapshot(snapshot);\n            }\n            let _ = self.tx.send(event);\n        }",
        "        while let Ok(mut event) = self.inner_rx.try_recv() {\n            // Archive and UI must receive exactly the same sampled data.\n            self.annotate_before_archive(&mut event);\n            if let AppEvent::Dps(snapshot) = &event {\n                self.observe_snapshot(snapshot);\n            }\n            let _ = self.tx.send(event);\n        }",
        "annotate before history copies snapshot");
    source.push_str(r#"
impl TelemetryRuntime {
    fn annotate_before_archive(&mut self, event: &mut AppEvent) {
        if let AppEvent::Dps(snapshot) = event {
            self.attribute_recorder.update(snapshot, crate::telemetry::capture_partial());
        }
    }
}

#[cfg(test)]
mod v1409_archive_attribute_order_tests {
    use super::*;
    use crate::{feature_settings, model::{DpsRow, TrackedAttribute}};

    #[test]
    fn archived_snapshot_retains_the_exact_attributes_forwarded_to_ui() {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut runtime = TelemetryRuntime::new(tx);
        let row = DpsRow {
            uid: 42, is_local: true, damage: 100,
            attributes: vec![TrackedAttribute {
                attr_id: feature_settings::ATTR_LUCKY,
                label: "Luck".into(), value: 3_500,
            }],
            ..Default::default()
        };
        let mut event = AppEvent::Dps(DpsSnapshot {
            encounter_ms: 1_000, total_damage: 100, rows: vec![row],
            ..Default::default()
        });
        runtime.annotate_before_archive(&mut event);
        if let AppEvent::Dps(snapshot) = &event {
            runtime.observe_snapshot(snapshot);
        }
        let archived = runtime.last.as_ref().unwrap().0.rows[0].encounter_attributes.clone();
        assert_eq!(archived.len(), 1);
        assert_eq!(archived[0].initial, Some(3_500));
        assert_eq!(archived[0].final_value, Some(3_500));
        runtime.tx.send(event).unwrap();
        let AppEvent::Dps(forwarded) = rx.try_recv().unwrap() else { panic!("missing DPS event") };
        assert_eq!(forwarded.rows[0].encounter_attributes[0].initial, archived[0].initial);
        // Do not create a real history file in this unit test.
        runtime.last.take();
    }
}
"#);
    fs::write(&path, source).expect("write archive attribute ordering fix");
    println!("cargo:rerun-if-changed=build/legacy/build_v1409_archive_attribute_before_save.rs");
}
