mod current {
    include!("build/legacy/build_v1310_pixel_material.rs");
    pub fn run() { main(); }
}

fn main() {
    current::run();
    // Keep every behavior-producing historical stage intact, then apply one
    // presentation-only Pixel/Material finish to the generated native surfaces.
    // The visual pass does not alter telemetry, commands, layout algorithms,
    // shortcuts, navigation, compact/raid behavior or the Win32 architecture.
    println!("cargo:rerun-if-changed=build/legacy");
}
