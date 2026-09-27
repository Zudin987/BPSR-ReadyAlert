// The encounter attribute sampler decorates emitted DPS snapshots, but the
// lower-level telemetry constructor must still initialize the new model field.
// Keep this generated-source change in a checked-in builder and fail loudly if
// the upstream constructor changes rather than silently missing the patch.
use std::{env, fs, path::PathBuf};
mod previous {
    include!("build_v1407_ignore_finished_run_replays.rs");
    pub fn run() { main(); }
}
fn main() {
    previous::run();
    let path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"))
        .join("telemetry_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("generated telemetry");
    let anchor = "            rows.push(DpsRow {\n";
    assert_eq!(source.matches(anchor).count(), 1,
        "v1408: DPS row constructor anchor changed");
    source = source.replacen(anchor,
        "            rows.push(DpsRow {\n                encounter_attributes: Vec::new(),\n", 1);
    fs::write(&path, source).expect("write encounter attribute row initializer");
    println!("cargo:rerun-if-changed=build/legacy/build_v1408_encounter_attribute_row.rs");
}
