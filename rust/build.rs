mod current {
    include!("build/legacy/build_v1310_true_pixel.rs");
    pub fn run() { main(); }
}

fn main() {
    current::run();
    // Final stage is presentation-only and deliberately runs after the v1.30.2
    // behavior/keyboard/dialog audit so visual work cannot regress those fixes.
    println!("cargo:rerun-if-changed=build/legacy");
}
