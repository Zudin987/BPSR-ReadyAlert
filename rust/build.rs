mod current {
    include!("build/legacy/build_pixel_phase3_regression.rs");
    pub fn run() { main(); }
}

fn main() {
    current::run();
    // Presentation-only final stages deliberately run after the v1.30.2
    // behavior/keyboard/dialog audit so visual work cannot regress those fixes.
    println!("cargo:rerun-if-changed=build/legacy");
}
