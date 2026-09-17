use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1337_audit_hardening.rs");
    pub fn run() { main(); }
}

fn main() {
    previous::run();

    // The application compiles this generated file, NOT src/feature_overlays.rs.
    // Apply the skin after existing behavior/QA patches without changing packet
    // capture, encounter data, sort behavior or user settings.
    let path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"))
        .join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .expect("read generated meter for reference styling")
        .replace("\r\n", "\n");

    // Retain spec_color() itself and its exact-color regression tests. Only
    // paint-time lookups become muted class-tinted row card backgrounds.
    let card_calls = source.matches("spec_color(row)").count();
    assert!(card_calls > 0 && card_calls <= 12,
        "expected rendered specialization card colors, found {card_calls}");
    source = source.replace("spec_color(row)", "reference_meter_row_background(row)");

    // Local-player outlines and progress colours are applied at their real
    // paint sites in the v1339 and v1341 stages, not by unused literal swaps.
    // Append after substitution: this helper must use the original spec_color
    // rather than accidentally rewriting it into a recursive call.
    source.push_str("\n");
    source.push_str(include_str!("../../src/feature_meter_reference_theme.rs"));
    fs::write(path, source).expect("write screenshot-inspired generated meter");
    println!("cargo:rerun-if-changed=build/legacy/build_v1338_reference_meter.rs");
    println!("cargo:rerun-if-changed=src/feature_meter_reference_theme.rs");
}
