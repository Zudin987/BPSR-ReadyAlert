mod current {
    include!("build/legacy/build_v1316_ui_controls_polish_fix.rs");
    pub fn run() { main(); }
}

fn main() {
    current::run();
    // Presentation-only QA stages deliberately run after the behavior/keyboard/dialog
    // audit so visual defect fixes cannot regress ReadyAlert workflows.
    println!("cargo:rerun-if-changed=build/legacy");
}
