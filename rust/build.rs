mod current {
    include!("build/legacy/build_v1270_benchmark_history.rs");
    pub fn run() { main(); }
}

fn main() {
    current::run();
    // Historical generated-source compatibility remains assertion-guarded.
    // v1.27 keeps the native Win32 overlay while adding benchmark controls and
    // the refreshed offline encounter-history experience.
    println!("cargo:rerun-if-changed=build/legacy");
}
