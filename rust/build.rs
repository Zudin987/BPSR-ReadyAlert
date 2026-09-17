#![recursion_limit = "256"]

mod current {
    include!("build/legacy/build_v1354_audit_font_floors.rs");
    pub fn run() { main(); }
}

fn main() {
    current::run();
    // Final audit pass runs after the current header/layout/history generators.
    println!("cargo:rerun-if-changed=build/legacy");
}
