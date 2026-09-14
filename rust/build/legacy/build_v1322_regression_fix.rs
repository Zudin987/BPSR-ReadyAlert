use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1322_final_hardening.rs");
    pub fn run() { main(); }
}

fn main() {
    previous::run();

    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let path = out.join("event_tracker_v1321_generated.rs");
    let mut source = fs::read_to_string(&path)
        .expect("read generated v1.32.2 event tracker for regression fixture fix")
        .replace("\r\n", "\n");

    let from = r#"        assert!(state.attribute(&settings, 11_440, 123, 55, 1_000));
        assert!(!state.attribute(&settings, 11_440, 123, 55, 1_001));
        assert!(state.attribute(&settings, 11_440, 123, 56, 1_002));"#;
    let to = r#"        assert!(state.attribute(&settings, 11_440, 123, 123, 55, 1_000));
        assert!(!state.attribute(&settings, 11_440, 123, 123, 55, 1_001));
        assert!(state.attribute(&settings, 11_440, 123, 123, 56, 1_002));"#;
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.32.2 inherited Attribute regression fixture expected one match, found {count}");
    source = source.replacen(from, to, 1);

    fs::write(path, source).expect("write v1.32.2 regression fixture fix");
    println!("cargo:rerun-if-changed=build/legacy/build_v1322_regression_fix.rs");
}
