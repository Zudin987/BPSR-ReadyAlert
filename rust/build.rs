mod current {
    include!("build/legacy/build_v1271_history_button.rs");
    pub fn run() { main(); }
}

fn main() {
    current::run();
    // Historical generated-source compatibility remains assertion-guarded.
    // v1.27.1 polishes the native Benchmark dialog and adds direct Encounter
    // History access to the responsive meter toolbar.
    println!("cargo:rerun-if-changed=build/legacy");
}
