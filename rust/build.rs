mod current {
    include!("build/legacy/build_v1344_meter_header_history_ux.rs");
    pub fn run() { main(); }
}

fn main() {
    current::run();
    // Final UX pass follows the white-header icon renderer so geometry, native
    // icons, reset behavior and encounter-retention settings stay in sync.
    println!("cargo:rerun-if-changed=build/legacy");
}
