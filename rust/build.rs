mod current {
    include!("build/legacy/build_v1322_final_hardening.rs");
    pub fn run() { main(); }
}

fn main() {
    current::run();
    // Presentation and compatibility QA stages deliberately run after the
    // behavior/keyboard/dialog audit so future-season fallbacks cannot regress
    // existing ReadyAlert workflows.
    println!("cargo:rerun-if-changed=build/legacy");
}
