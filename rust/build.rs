mod current {
    include!("build/legacy/build_v1260_encounter_archive.rs");
    pub fn run() { main(); }
}

fn main() {
    current::run();
    // Historical generated-source compatibility remains assertion-guarded.
    // v1.26 keeps the native UI path and routes the existing archive action to
    // one offline hub for chat plus saved encounter history.
    println!("cargo:rerun-if-changed=build/legacy");
}
