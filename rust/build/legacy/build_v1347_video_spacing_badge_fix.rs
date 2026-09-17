use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1346_video_spacing.rs");
    pub fn run() { main(); }
}

fn main() {
    previous::run();
    let path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"))
        .join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("generated meter source");
    let from = "badge_count: badge_count as usize,";
    assert_eq!(source.matches(from).count(), 1, "responsive badge count anchor");
    source = source.replacen(from,
        "badge_count: (badge_count as usize).min(_row.imagines.len()),", 1);
    fs::write(path, source).expect("write actual Imagine count");
    println!("cargo:rerun-if-changed=build/legacy/build_v1347_video_spacing_badge_fix.rs");
}
