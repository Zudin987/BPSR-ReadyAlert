mod current {
    include!("build/legacy/build_ui_modernization.rs");
    pub fn run() { main(); }
}

fn main() {
    current::run();
    // Keep the historical generated-source compatibility chain assertion-guarded.
    // The final UI pass applies compact Pixel / Material 3 + One UI polish without
    // changing the protected native window footprints or responsive data density.
    println!("cargo:rerun-if-changed=build/legacy");
}
