mod current {
    include!("build/legacy/build_v1290_compact_meter.rs");
    pub fn run() { main(); }
}

fn main() {
    current::run();
    // Keep the historical generated-source compatibility chain assertion-guarded.
    // The final UI pass applies compact Pixel / Material 3 + One UI polish while
    // adding the dedicated meter Compact Mode without changing the protected
    // normal-mode footprint or native Win32 architecture.
    println!("cargo:rerun-if-changed=build/legacy");
}
