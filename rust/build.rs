mod current {
    include!("build/legacy/build_v1240_focused_mechanics.rs");
    pub fn run() { main(); }
}

fn main() {
    current::run();
    // Historical generated-source compatibility remains assertion-guarded. v1.24.0
    // keeps all v1.23.1 CN-derived scene mechanics intact, then layers only
    // verified non-spatial dungeon gaps plus a compact priority-first mechanic UI.
    println!("cargo:rerun-if-changed=build/legacy");
}
