mod current {
    include!("build/legacy/build_v1231_mechanics.rs");
    pub fn run() { main(); }
}

fn main() {
    current::run();
    // Historical generated-source compatibility remains assertion-guarded. v1.23.1
    // now layers CN-authoritative wipe handling, mechanic-wave dedupe, EnterScene
    // scene detection, and scene-gated dungeon/Raid mechanic parity.
    println!("cargo:rerun-if-changed=build/legacy");
}
