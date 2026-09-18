use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1349_audit_raid_consumable_ownership.rs");
    pub fn run() { main(); }
}

fn main() {
    previous::run();
    // The existing renderer QA exercised app scales 100..200, not monitor DPI.
    // Include the audit's 60% screenshots and a representative 80% setting in
    // both native screenshots and the toolbar geometry regression test.
    let path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"))
        .join("ui_meter_qa_tests_raid_v1345.rs");
    let mut source = fs::read_to_string(&path).expect("generated meter QA tests");
    let before = "for scale in [100, 125, 150, 175, 200] {";
    let found = source.matches(before).count();
    assert_eq!(found, 2, "expected toolbar and native screenshot scale loops, got {found}");
    source = source.replace(before, "for scale in [60, 80, 100, 125, 150, 175, 200] {");
    let old_name = "native_meter_rendered_dpi_matrix_covers_narrow_and_wide_states";
    assert_eq!(source.matches(old_name).count(), 1, "native QA function missing");
    source = source.replacen(old_name, "native_meter_rendered_app_scale_matrix_covers_narrow_and_wide_states", 1);
    fs::write(&path, source).expect("write extended meter QA tests");
    println!("cargo:rerun-if-changed=build/legacy/build_v1350_audit_small_scale_matrix.rs");
}
