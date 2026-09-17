use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1341_reference_audit.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, before: &str, after: &str, label: &str) {
    assert_eq!(source.matches(before).count(), 1, "dungeon end: unexpected {label} source anchor");
    *source = source.replacen(before, after, 1);
}

fn main() {
    previous::run();
    let path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"))
        .join("telemetry_v170_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .expect("read generated telemetry for dungeon lifecycle support");

    replace_once(&mut source,
        "const SYNC_CONTAINER_DATA: u32 = 0x15;\nconst SYNC_NEAR_DELTA_INFO: u32 = 0x2d;",
        "const SYNC_CONTAINER_DATA: u32 = 0x15;\nconst SYNC_DUNGEON_DATA: u32 = 0x17;\nconst SYNC_DUNGEON_DIRTY: u32 = 0x18;\nconst SYNC_NEAR_DELTA_INFO: u32 = 0x2d;",
        "method constants");
    replace_once(&mut source,
        "    current_scene_id: i32,\n    encounter_started: Option<Instant>,",
        "    current_scene_id: i32,\n    // Require an observed Playing state in this scene before accepting a final state.\n    dungeon_flow_playing: bool,\n    encounter_started: Option<Instant>,",
        "flow tracking field");
    replace_once(&mut source,
        "            current_scene_id: 0,\n            encounter_started: None,",
        "            current_scene_id: 0,\n            dungeon_flow_playing: false,\n            encounter_started: None,",
        "flow tracking init");
    replace_once(&mut source,
        "        self.current_scene_id = 0;\n        crate::event_tracker::clear_scene();",
        "        self.current_scene_id = 0;\n        self.dungeon_flow_playing = false;\n        crate::event_tracker::clear_scene();",
        "scene flow reset");
    replace_once(&mut source,
        "            SYNC_CONTAINER_DATA => self.handle_container_snapshot(body),\n            SYNC_NEAR_DELTA_INFO => {",
        "            SYNC_CONTAINER_DATA => self.handle_container_snapshot(body),\n            SYNC_DUNGEON_DATA => {\n                if let Some(flow) = dungeon_full_flow_state(body) { self.observe_dungeon_flow(flow); }\n            }\n            SYNC_DUNGEON_DIRTY => {\n                if let Some(flow) = dungeon_dirty_flow_state(body) { self.observe_dungeon_flow(flow); }\n            }\n            SYNC_NEAR_DELTA_INFO => {",
        "world notify dispatcher");
    replace_once(&mut source,
        "    /// User-requested training-dummy reset. Keeps roster and current scene context.",
        "    // Only a confirmed Playing -> End/Settlement transition with active combat\n    // and no pending wipe boundary is even a candidate for finalization.\n    // Live packets and scene/run correlation still need validation before merge.\n    fn observe_dungeon_flow(&mut self, state: i32) {\n        match state {\n            3 => self.dungeon_flow_playing = true,\n            4 | 5 => {\n                let should_finish = self.dungeon_flow_playing\n                    && self.pending_boundary.is_none()\n                    && self.encounter_started.is_some()\n                    && self.combat.values().any(|actor| {\n                        actor.damage > 0 || actor.healing > 0 || actor.damage_taken > 0\n                    });\n                self.dungeon_flow_playing = false;\n                if should_finish {\n                    crate::logging::write(\"encounter-boundary: observed Playing -> dungeon end; archiving active encounter\");\n                    self.reset_encounter_keep_roster();\n                    self.emit_dps();\n                }\n            }\n            0..=2 | 6 => self.dungeon_flow_playing = false,\n            _ => {}\n        }\n    }\n\n    /// User-requested training-dummy reset. Keeps roster and current scene context.",
        "guarded state transition");
    replace_once(&mut source,
        "    fn reset_encounter_keep_roster(&mut self) {\n        self.combat.clear();",
        "    fn reset_encounter_keep_roster(&mut self) {\n        // Disarm stale End/Settlement after manual reset or a confirmed wipe.\n        self.dungeon_flow_playing = false;\n        self.combat.clear();",
        "flow reset on encounter boundary");

    source.push_str(include_str!("../../src/dungeon_end_v1339.rs"));
    fs::write(path, source).expect("write dungeon lifecycle telemetry");
    println!("cargo:rerun-if-changed=build/legacy/build_v1342_dungeon_end.rs");
    println!("cargo:rerun-if-changed=src/dungeon_end_v1339.rs");
}
