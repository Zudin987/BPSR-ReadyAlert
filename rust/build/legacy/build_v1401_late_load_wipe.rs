// Preserve packet semantics while separating confirmed wipe accounting from
// display retention and refusing destructive duplicate/partial scene resets.
use std::{env, fs, path::PathBuf};
mod previous { include!("build_v1400_dead_identity_all_layouts.rs"); pub fn run() { main(); } }

fn patch(text: &mut String, old: &str, new: &str, label: &str) {
    assert_eq!(text.matches(old).count(), 1, "v1401: {label} anchor changed");
    *text = text.replacen(old, new, 1);
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let telemetry_path = out.join("telemetry_v170_fixed.rs");
    let mut t = fs::read_to_string(&telemetry_path).expect("telemetry generated source");
    patch(&mut t, "    current_scene_id: i32,\n    encounter_started: Option<Instant>,",
        "    current_scene_id: i32,\n    current_run_uuid: Option<i64>,\n    run_identity_changed: bool,\n    encounter_started: Option<Instant>,", "scene fields");
    patch(&mut t, "            current_scene_id: 0,\n            encounter_started: None,",
        "            current_scene_id: 0,\n            current_run_uuid: None,\n            run_identity_changed: false,\n            encounter_started: None,", "scene initialization");
    patch(&mut t,
        "            proto::ENTER_SCENE_METHOD => {\n                self.reset_scene();\n                self.handle_enter_scene(body);\n            }",
        r#"            proto::ENTER_SCENE_METHOD => {
                // Late/malformed scene notifications must not erase an ongoing
                // encounter. Same-scene run changes require corroborating UUID
                // observations; dungeon flow alone NEVER triggers a reset.
                let scene = proto::get_len_field(body, 1)
                    .and_then(|info| proto::get_len_field(info, 1))
                    .and_then(scene_id_from_attrs);
                match scene {
                    None => crate::logging::write("encounter: incomplete scene notification; keeping current attempt"),
                    Some(id) => {
                        if self.current_scene_id == id && !self.run_identity_changed {
                            crate::logging::write(format!("encounter: repeated scene={id}; refreshing metadata without clearing combat"));
                        } else {
                            self.reset_scene();
                            self.run_identity_changed = false;
                            self.current_run_uuid = None;
                            crate::telemetry::clear_capture_partial();
                        }
                        // Reconstruct local identity, buffs and roster regardless
                        // of whether this was a genuine or duplicate scene entry.
                        self.handle_enter_scene(body);
                    }
                }
            }"#, "guard scene resets");
    patch(&mut t,
        "                    crate::logging::write(format!(\"dungeon-flow-diag: source=full scene_uuid={} scene_id={} state={}({})\", scene_uuid, self.current_scene_id, flow, dungeon_flow_state_name(flow)));",
        "                    self.observe_run_identity(scene_uuid);\n                    crate::logging::write(format!(\"dungeon-flow-diag: source=full scene_uuid={} scene_id={} state={}({})\", scene_uuid, self.current_scene_id, flow, dungeon_flow_state_name(flow)));", "observe full run identity");
    patch(&mut t,
        "                    let scene_uuid = scene_uuid.map(|value| value.to_string()).unwrap_or_else(|| \"delta\".to_string());",
        "                    if let Some(run) = scene_uuid { self.observe_run_identity(run); }\n                    let scene_uuid = scene_uuid.map(|value| value.to_string()).unwrap_or_else(|| \"delta\".to_string());", "observe dirty run identity");
    patch(&mut t,
        "    /// User-requested training-dummy reset. Keeps roster and current scene context.",
        r#"    fn observe_run_identity(&mut self, run: i64) {
        if run <= 0 { return; }
        if let Some(old) = self.current_run_uuid {
            if old != run {
                self.run_identity_changed = true;
                crate::logging::write("encounter: run UUID changed; waiting for scene entry confirmation");
            }
        }
        self.current_run_uuid = Some(run);
    }

    /// User-requested training-dummy reset. Keeps roster and current scene context."#,
        "observe run identity helper");
    patch(&mut t,
        "        self.reset_encounter_keep_roster();\n        self.pending_boundary = None;\n        self.emit_dps();",
        "        self.reset_encounter_keep_roster();\n        self.pending_boundary = None;\n        self.run_identity_changed = false;\n        crate::telemetry::clear_capture_partial();\n        self.emit_dps();", "manual reset status");
    patch(&mut t, "const SEGMENT_BOUNDARY_DELAY: Duration = Duration::from_secs(3);",
        "#[cfg(test)] const SEGMENT_BOUNDARY_DELAY: Duration = Duration::from_secs(3);", "legacy delay fixture");
    patch(&mut t,
        "    fn prepare_segment(&mut self, now: Instant) {\n        // Do not consume a fresh boundary before its guard delay has elapsed.\n        // v1.8 used take() here, so a fast post-wipe hit could permanently lose\n        // the pending reset and merge two pulls.\n        if self.pending_boundary.is_some_and(|boundary| {\n            now.saturating_duration_since(boundary) >= SEGMENT_BOUNDARY_DELAY\n        }) {\n            if self.encounter_started.is_some() { self.emit_dps(); }\n            self.reset_encounter_keep_roster();\n        }\n        self.encounter_started.get_or_insert(now);\n    }",
        r#"    fn prepare_segment(&mut self, now: Instant) {
        // A legacy pending boundary is consumed before the FIRST repull hit,
        // not after a three-second period of misattributed damage.
        if self.pending_boundary.is_some() {
            if self.encounter_started.is_some() { self.emit_dps(); }
            self.reset_encounter_keep_roster();
        }
        self.encounter_started.get_or_insert(now);
    }"#, "rapid repull");
    patch(&mut t,
        "    fn arm_boundary(&mut self) {\n        if self.encounter_started.is_some() && self.pending_boundary.is_none() {\n            self.pending_boundary = Some(Instant::now());\n        }\n    }",
        r#"    fn arm_boundary(&mut self) {
        if self.encounter_started.is_none() || self.pending_boundary.is_some() { return; }
        // Unthrottled final snapshot then an empty boundary: history archives
        // immediately even if no new damage ever arrives. Repeated markers
        // cannot archive twice because the active encounter is now empty.
        self.emit_dps();
        self.reset_encounter_keep_roster();
        crate::telemetry::clear_capture_partial();
        crate::telemetry::mark_wipe_display_hold();
        self.emit_dps();
    }"#, "immediate wipe finalization");

    // Existing regressions asserted the old delayed semantics; update those
    // contradictory expectations while retaining the repository's normal gates.
    assert_eq!(t.matches("assert!(runtime.pending_boundary.is_some());").count(), 4);
    t = t.replace("assert!(runtime.pending_boundary.is_some());", "assert!(runtime.encounter_started.is_none());");
    patch(&mut t,
        "        runtime.prepare_segment(Instant::now());\n        assert!(runtime.encounter_started.is_none());",
        "        runtime.prepare_segment(Instant::now());\n        assert!(runtime.pending_boundary.is_none());", "early repull consumed");
    patch(&mut t, "fn early_repull_preserves_pending_boundary_and_accumulated_damage()",
        "fn early_repull_splits_before_new_damage()", "repull test name");
    patch(&mut t,
        "        assert_eq!(runtime.combat.get(&42).unwrap().damage, 123);\n        assert!(dps_snapshots(rx).is_empty());\n    }\n    #[test]\n    fn elapsed_boundary_flushes_old_pull_before_reset() {",
        "        assert_eq!(runtime.combat.get(&42).unwrap().damage, 0);\n        assert_eq!(dps_snapshots(rx)[0].total_damage, 123);\n    }\n    #[test]\n    fn elapsed_boundary_flushes_old_pull_before_reset() {", "repull expectation");
    fs::write(&telemetry_path, t).expect("write telemetry");

    let overlay_path = out.join("feature_overlays_v170_fixed.rs");
    let mut ui = fs::read_to_string(&overlay_path).expect("overlay source");
    patch(&mut ui,
        "pub unsafe fn update_dps(hwnd:HWND,mut snapshot:DpsSnapshot){\n    remember_mechanic_owner(&snapshot);",
        r#"pub unsafe fn update_dps(hwnd:HWND,mut snapshot:DpsSnapshot){
    // History receives the empty wipe event; only the live display retains
    // the finished attempt until the first snapshot of the next pull.
    if snapshot.encounter_ms == 0 && snapshot.total_damage == 0
        && snapshot.total_healing == 0 && snapshot.total_damage_taken == 0
        && crate::telemetry::take_wipe_display_hold() { return; }
    remember_mechanic_owner(&snapshot);"#, "wipe view retention");
    patch(&mut ui,
        "    let title = if state.history_index.is_some() { format!(\"HISTORY · {title}\") } else if snapshot.encounter_ms == 0",
        "    let title = if state.history_index.is_none() && crate::telemetry::capture_partial() { format!(\"PARTIAL CAPTURE · {title}\") } else { title };\n    let title = if state.history_index.is_some() { format!(\"HISTORY · {title}\") } else if snapshot.encounter_ms == 0", "partial-capture label");
    fs::write(&overlay_path, ui).expect("write overlay");

    let capture_path = out.join("capture_v185.rs");
    let mut capture = fs::read_to_string(&capture_path).expect("capture source");
    patch(&mut capture,
        "        let mut processor = CaptureProcessor::new(tx.clone(), identity.clone(), chat_runtime.clone());",
        "        crate::telemetry::mark_capture_partial();\n        let mut processor = CaptureProcessor::new(tx.clone(), identity.clone(), chat_runtime.clone());", "late startup");
    patch(&mut capture,
        "            if reopen {\n                failures += 1;",
        "            if reopen {\n                crate::telemetry::mark_capture_partial();\n                let _ = tx.send(AppEvent::CaptureStatus(\"Resyncing - capture interrupted; encounter evidence incomplete\".into()));\n                failures += 1;", "capture reopen");
    patch(&mut capture,
        "                        processor.reset_flows();\n                        logging::write(\"capture-recovery: protocol frame stall; reset flows\");",
        "                        processor.reset_flows();\n                        crate::telemetry::mark_capture_partial();\n                        logging::write(\"capture-recovery: protocol frame stall; reset flows\");", "protocol stall");
    fs::write(&capture_path, capture).expect("write capture");
    println!("cargo:rerun-if-changed=build/legacy/build_v1401_late_load_wipe.rs");
}
