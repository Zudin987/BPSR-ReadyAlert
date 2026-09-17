#![recursion_limit = "256"]

mod current {
    include!("build/legacy/build_v1345_narrow_header_fit.rs");
    pub fn run() { main(); }
}

fn main() {
    current::run();
    // Final UX pass follows the white-header icon renderer so geometry, native
    // icons, reset behavior and encounter-retention settings stay in sync.
    println!("cargo:rerun-if-changed=build/legacy");
}
