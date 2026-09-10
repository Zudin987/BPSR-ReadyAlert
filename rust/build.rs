mod current {
    include!("build/legacy/build_v1178.rs");
    pub fn run() { main(); }
}

fn main() {
    current::run();
    // The historical generator is intentionally isolated under build/legacy.
    // Watch the folder as a whole because the legacy stages include one another.
    println!("cargo:rerun-if-changed=build/legacy");
}
