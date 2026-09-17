use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1341_reference_audit.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, before: &str, after: &str, label: &str) {
    assert_eq!(source.matches(before).count(), 1, "dungeon diagnostics: unexpected {label} source anchor");
    *source = source.replacen(before, after, 1);
}

fn main() {
    previous::run();
    let path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"))
        .join("telemetry_v170_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .expect("read generated telemetry for dungeon flow diagnostics");

    replace_once(&mut source,
        "const SYNC_CONTAINER_DATA: u32 = 0x15;\nconst SYNC_NEAR_DELTA_INFO: u32 = 0x2d;",
        "const SYNC_CONTAINER_DATA: u32 = 0x15;\nconst SYNC_DUNGEON_DATA: u32 = 0x17;\nconst SYNC_DUNGEON_DIRTY: u32 = 0x18;\nconst SYNC_NEAR_DELTA_INFO: u32 = 0x2d;",
        "method constants");

    replace_once(&mut source,
        "            SYNC_CONTAINER_DATA => self.handle_container_snapshot(body),\n            SYNC_NEAR_DELTA_INFO => {",
        "            SYNC_CONTAINER_DATA => self.handle_container_snapshot(body),\n            SYNC_DUNGEON_DATA => {\n                if let Some((scene_uuid, flow)) = dungeon_full_flow_state(body) {\n                    crate::logging::write(format!(\"dungeon-flow-diag: source=full scene_uuid={} scene_id={} state={}({})\", scene_uuid, self.current_scene_id, flow, dungeon_flow_state_name(flow)));\n                }\n            }\n            SYNC_DUNGEON_DIRTY => {\n                if let Some((scene_uuid, flow)) = dungeon_dirty_flow_state(body) {\n                    let scene_uuid = scene_uuid.map(|value| value.to_string()).unwrap_or_else(|| \"delta\".to_string());\n                    crate::logging::write(format!(\"dungeon-flow-diag: source=dirty scene_uuid={} scene_id={} state={}({})\", scene_uuid, self.current_scene_id, flow, dungeon_flow_state_name(flow)));\n                }\n            }\n            SYNC_NEAR_DELTA_INFO => {",
        "world notify dispatcher");

    source.push_str(include_str!("../../src/dungeon_flow_diag_v1339.rs"));
    fs::write(path, source).expect("write dungeon flow diagnostics telemetry");
    println!("cargo:rerun-if-changed=build/legacy/build_v1342_dungeon_flow_diag.rs");
    println!("cargo:rerun-if-changed=src/dungeon_flow_diag_v1339.rs");
}
