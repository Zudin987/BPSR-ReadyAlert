use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1337_audit_hardening.rs");
    pub fn run() { main(); }
}

fn main() {
    previous::run();

    // The application compiles this generated file, NOT src/feature_overlays.rs.
    // Apply the skin after every existing behavior/QA patch so navigation, raid
    // mode, hover hitboxes, scroll handling and settings remain untouched.
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

    // The image uses a mint outline for the pinned local player and a muted
    // coral progress bar. Restrict replacements to the feature overlay rather
    // than altering the global application palette or the mechanics data.
    source = source.replace("rgb(212,175,55)", "rgb(104,224,183)");
    source = source.replace("rgb(235,74,74)", "rgb(207,91,109)");
    source = source.replace("rgb(105,28,34)", "rgb(52,29,36)");

    // Append after substitutions: this helper must call the original spec_color
    // without being rewritten into a recursive call by the replacement above.
    source.push_str("\n");
    source.push_str(include_str!("../../src/feature_meter_reference_theme.rs"));
    fs::write(path, source).expect("write screenshot-inspired generated meter");
    println!("cargo:rerun-if-changed=build/legacy/build_v1338_reference_meter.rs");
    println!("cargo:rerun-if-changed=src/feature_meter_reference_theme.rs");
}
