mod current {
    include!("build/legacy/build_v1340_reference_hitboxes.rs");
    pub fn run() { main(); }
}

fn main() {
    current::run();
    // Presentation and compatibility QA stages deliberately run after the
    // behavior/keyboard/dialog audit so future-season fallbacks cannot regress
    // existing ReadyAlert workflows.
    println!("cargo:rerun-if-changed=build/legacy");
}
