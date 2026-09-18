#![recursion_limit = "256"]

mod current {
    include!("build/legacy/build_v1361_hit_test_type_fix.rs");
    pub fn run() { main(); }
}

fn main() {
    current::run();
    // Final layout fixes run after approved DPS generators; generated outputs
    // are never edited directly without a checked-in source transformation.
    println!("cargo:rerun-if-changed=build/legacy");
}
