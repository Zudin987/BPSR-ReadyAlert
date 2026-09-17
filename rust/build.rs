mod current {
    include!("build/legacy/build_v1342_dungeon_flow_diag.rs");
    pub fn run() { main(); }
}

fn main() {
    current::run();
    // Presentation and compatibility QA stages deliberately run after the
    // behavior/keyboard/dialog audit so future-season fallbacks cannot regress
    // existing ReadyAlert workflows. The dungeon-flow stage is diagnostic-only:
    // it must not mutate encounter lifecycle state.
    println!("cargo:rerun-if-changed=build/legacy");
}
