use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1344_meter_header_history_ux.rs");
    pub fn run() { main(); }
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("feature renderer");
    let from = "    if right < 360 { preferred.min(94) } else { preferred }\n";
    let to = "    preferred\n";
    let count = source.matches(from).count();
    assert_eq!(count, 1, "narrow header selector anchor changed");
    source = source.replacen(from, to, 1);
    fs::write(path, source).expect("write narrow header fit");
    println!("cargo:rerun-if-changed=build/legacy/build_v1345_narrow_header_fit.rs");
}
