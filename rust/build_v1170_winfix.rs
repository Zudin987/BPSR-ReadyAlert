use std::{env, fs, path::{Path, PathBuf}};

mod v1170 {
    include!("build_v1170.rs");

    pub fn run_prior() {
        prior::run();
    }

    pub fn apply_patches(out: &Path) {
        patch_settings(out);
        patch_event_tracker(out);
        patch_feature_settings(out);
    }
}

fn normalize_generated(path: &Path) {
    let source = fs::read_to_string(path).expect("read generated v1.17 source for line-ending normalization");
    if source.contains("\r\n") {
        fs::write(path, source.replace("\r\n", "\n")).expect("normalize generated v1.17 source line endings");
    }
}

fn main() {
    // Build the proven v1.16.7 generated sources first, then normalize CRLF
    // before applying v1.17's exact structural patches. GitHub's Windows runner
    // writes generated sources with CRLF while local artifact inspection often
    // transparently normalizes them; keeping this boundary explicit makes the
    // generator deterministic across Windows/local validation.
    v1170::run_prior();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    for name in [
        "settings_ui_v1160_fixed.rs",
        "event_tracker_ui_v1160_fixed.rs",
        "feature_overlays_v170_fixed.rs",
    ] {
        normalize_generated(&out.join(name));
    }
    v1170::apply_patches(&out);

    println!("cargo:rerun-if-changed=build_v1170_winfix.rs");
    println!("cargo:rerun-if-changed=build_v1170.rs");
    println!("cargo:rerun-if-changed=ui_v1170/settings_layout.txt");
    println!("cargo:rerun-if-changed=ui_v1170/event_tracker_layout.txt");
    println!("cargo:rerun-if-changed=ui_v1170/feature_settings_layout.txt");
    println!("cargo:rerun-if-changed=src/ui_theme.rs");
}
