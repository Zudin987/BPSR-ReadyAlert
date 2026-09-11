mod current {
    include!("build/legacy/build_v1231.rs");
    pub fn run() { main(); }
}

fn main() {
    current::run();
    // Historical generated-source compatibility remains assertion-guarded. v1.23.1
    // adds mechanic-wave dedupe and Raid-safe wipe detection as the final stage.
    println!("cargo:rerun-if-changed=build/legacy");
}
