// Apply the same real-scene boundary to the generic mechanic tracker, and
// surface actual packet loss as incomplete encounter evidence.
use std::{env, fs, path::PathBuf};
mod previous { include!("build_v1401_late_load_wipe.rs"); pub fn run() { main(); } }
fn patch(source: &mut String, old: &str, new: &str, reason: &str) {
    assert_eq!(source.matches(old).count(), 1, "v1402: {reason} anchor changed");
    *source = source.replacen(old, new, 1);
}
fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let telemetry_path = out.join("telemetry_v170_fixed.rs");
    let mut t = fs::read_to_string(&telemetry_path).expect("generated telemetry");
    patch(&mut t,
        "    fn reset_scene(&mut self) {\n        if self.encounter_started.is_some() { self.emit_dps(); }",
        "    fn reset_scene(&mut self) {\n        crate::telemetry::mark_scene_replaced();\n        if self.encounter_started.is_some() { self.emit_dps(); }",
        "real scene reset notification");
    fs::write(&telemetry_path, t).expect("write scene notification");

    let future_path = out.join("future_mechanics_v1321_fixed.rs");
    let mut future = fs::read_to_string(&future_path).expect("future tracker");
    patch(&mut future,
        "            proto::ENTER_SCENE_METHOD => {\n                changed |= !self.rows.is_empty();",
        r#"            proto::ENTER_SCENE_METHOD => {
                if !crate::telemetry::take_scene_replaced() {
                    // A delayed/partial EnterScene may still carry a fresh local
                    // identity or attributes, but cannot clear tracked mechanics.
                    if let Some(info) = proto::get_len_field(body, 1) {
                        if let Some(player) = proto::get_len_field(info, 2) {
                            let uuid = signed(proto::get_varint_field(player, 1).unwrap_or(0));
                            if uuid != 0 {
                                self.local_uid = player_uid(uuid);
                                changed |= self.observe_attrs(uuid, proto::get_len_field(player, 3));
                            }
                        }
                    }
                    return changed;
                }
                changed |= !self.rows.is_empty();"#,
        "non-destructive generic mechanics scene refresh");
    fs::write(&future_path, future).expect("write future mechanics");

    let capture_path = out.join("capture_v185.rs");
    let mut capture = fs::read_to_string(&capture_path).expect("capture source");
    patch(&mut capture,
        "                        let new_drops=drop_total.saturating_sub(last_drop_total);\n                        last_drop_total=drop_total;",
        "                        let new_drops=drop_total.saturating_sub(last_drop_total);\n                        if new_drops > 0 { crate::telemetry::mark_capture_partial(); }\n                        last_drop_total=drop_total;",
        "Npcap dropped packet uncertainty");
    fs::write(&capture_path, capture).expect("write capture health");
    println!("cargo:rerun-if-changed=build/legacy/build_v1402_scene_tracker_consistency.rs");
}
