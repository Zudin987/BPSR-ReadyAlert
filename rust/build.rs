#![recursion_limit = "256"]

mod current {
    include!("build/legacy/build_v1351_audit_identity_settings.rs");
    pub fn run() { main(); }
}

fn main() {
    current::run();
    // Final audit pass runs after the current header/layout/history generators.
    println!("cargo:rerun-if-changed=build/legacy");
}
