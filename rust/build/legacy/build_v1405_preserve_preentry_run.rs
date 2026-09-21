// Some full dungeon Playing packets arrive before EnterScene; dropping their
// identity on a harmless scene transition made later Settlement uncorrelatable.
// Carry only a nonterminal Playing identity, never the completed old run.
use std::{env, fs, path::PathBuf};
mod previous { include!("build_v1404_confirmed_raid_reentry.rs"); pub fn run() { main(); } }
fn patch(source: &mut String, old: &str, new: &str, reason: &str) {
    assert_eq!(source.matches(old).count(), 1, "v1405: {reason} anchor changed");
    *source = source.replacen(old, new, 1);
}
fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let path = out.join("telemetry_v170_fixed.rs");
    let mut t = fs::read_to_string(&path).expect("generated telemetry");
    patch(&mut t,
        "                            self.reset_scene();\n                            self.run_identity_changed = false;\n                            self.current_run_uuid = None;\n                            self.playing_run_uuid = None;\n                            self.completed_run_uuid = None;",
        r#"                            // Preserve a fresh Playing identity when it was
                            // observed before scene entry (slow loader / reordered
                            // full sync). Never carry a completed old run or an
                            // old active encounter into a new scene.
                            let entering_run = if self.completed_run_uuid.is_none()
                                && (self.encounter_started.is_none() || self.run_identity_changed)
                                && self.current_run_uuid.is_some()
                                && self.current_run_uuid == self.playing_run_uuid {
                                self.current_run_uuid
                            } else { None };
                            self.reset_scene();
                            self.run_identity_changed = false;
                            self.current_run_uuid = entering_run;
                            self.playing_run_uuid = entering_run;
                            self.completed_run_uuid = None;"#,
        "preserve pre-entry Playing correlation");
    t.push_str(r#"
#[cfg(test)]
mod v1405_preentry_run_tests {
    use super::*;
    #[test]
    fn playing_seen_before_enter_scene_can_still_correlate_settlement() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let mut runtime = TelemetryRuntime::new(tx);
        runtime.observe_run_flow(21, 3);
        let scene = [0x0a, 10, 0x0a, 8, 0x12, 6, 0x08, 0xd5, 0x02, 0x12, 1, 77];
        runtime.handle_notify(proto::WORLD_SERVICE, proto::ENTER_SCENE_METHOD, &scene);
        assert_eq!(runtime.current_run_uuid, Some(21));
        assert_eq!(runtime.playing_run_uuid, Some(21));
        runtime.encounter_started = Instant::now().checked_sub(Duration::from_secs(2));
        runtime.combat.entry(42).or_default().damage = 123;
        runtime.observe_run_flow(21, 5);
        assert_eq!(runtime.completed_run_uuid, Some(21));
    }
}
"#);
    fs::write(&path, t).expect("write late-entry correlation");
    println!("cargo:rerun-if-changed=build/legacy/build_v1405_preserve_preentry_run.rs");
}
