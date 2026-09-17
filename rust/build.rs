#![recursion_limit = "256"]

mod current {
    include!("build/legacy/build_v1350_audit_small_scale_matrix.rs");
    pub fn run() { main(); }
}

fn main() {
    current::run();
    // Final audit pass runs after the current header/layout/history generators.
    println!("cargo:rerun-if-changed=build/legacy");
}
