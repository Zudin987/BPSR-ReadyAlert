mod current {
    include!("build/legacy/build_v1300_idmap.rs");
    pub fn run() { main(); }
}

fn main() {
    current::run();
    // Keep the historical generated-source compatibility chain assertion-guarded.
    // The final pass preserves the BPSR native design system and supplements
    // player-facing game-data names without changing meter algorithms.
    println!("cargo:rerun-if-changed=build/legacy");
}
