use std::{env, fs, path::PathBuf};

fn main() {
    // The v1.8 telemetry/overlay sources are checked in directly. Keep the
    // include!-based module boundary used by production. Normalize the one
    // Rust-2021 token boundary introduced by compact source formatting before
    // including the overlay from OUT_DIR; runtime behavior remains unchanged.
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));

    let telemetry = fs::read_to_string("src/telemetry_v170.rs")
        .expect("read telemetry_v170.rs");
    fs::write(out.join("telemetry_v170_fixed.rs"), telemetry)
        .expect("write generated telemetry source");

    let overlay = fs::read_to_string("src/feature_overlays_v170.rs")
        .expect("read feature_overlays_v170.rs")
        .replace("return\"", "return \"");
    fs::write(out.join("feature_overlays_v170_fixed.rs"), overlay)
        .expect("write generated overlay source");

    println!("cargo:rerun-if-changed=src/telemetry_v170.rs");
    println!("cargo:rerun-if-changed=src/feature_overlays_v170.rs");

    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("../src/BPSR.ReadyAlert/Assets/App.ico");
        res.set("FileDescription", "BPSR Ready Alert");
        res.set("ProductName", "BPSR Ready Alert");
        res.set("FileVersion", env!("CARGO_PKG_VERSION"));
        res.set("ProductVersion", env!("CARGO_PKG_VERSION"));
        if let Err(err) = res.compile() {
            panic!("failed to embed Windows resources: {err}");
        }
    }
}
