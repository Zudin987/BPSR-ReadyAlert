mod current {
    include!("build/legacy/build_v1231_text_mechanics.rs");
    pub fn run() { main(); }
}

fn main() {
    current::run();
    // Historical generated-source compatibility remains assertion-guarded. v1.23.1
    // now layers CN-authoritative wipe handling, mechanic-wave dedupe, EnterScene
    // scene detection, scene-gated dungeon/Raid parity, and a text-only renderer
    // policy that excludes CN minimap-only spatial mechanics.
    println!("cargo:rerun-if-changed=build/legacy");
}
