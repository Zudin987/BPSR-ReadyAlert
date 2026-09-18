use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1360_mode_height_rules.rs");
    pub fn run() { main(); }
}

fn main() {
    previous::run();
    let path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"))
        .join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("generated overlay source");
    // windows-sys 0.59 declares HTTOPLEFT, HTBOTTOMLEFT, and other HT* constants
    // as u32. Match against that type rather than the i32 hit-test value.
    let old = "fn raid_horizontal_hit(hit:LRESULT)->LRESULT {\n    match hit as i32 {";
    let new = "fn raid_horizontal_hit(hit:LRESULT)->LRESULT {\n    match hit as u32 {";
    assert_eq!(source.matches(old).count(), 1, "expected one raid hit-test match");
    source = source.replacen(old, new, 1);
    fs::write(path, source).expect("write hit-test type fix");
    println!("cargo:rerun-if-changed=build/legacy/build_v1361_hit_test_type_fix.rs");
}
