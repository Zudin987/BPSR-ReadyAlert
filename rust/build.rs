mod current {
    include!("build/legacy/build_v1290_compact_meter.rs");
    pub fn run() { main(); }
}

fn main() {
    current::run();
    // Keep the historical generated-source compatibility chain assertion-guarded.
    // The final pass applies the BPSR Mist Glass / Dark Glass native design system
    // after Compact/Raid behavior is generated, without changing protected meter
    // algorithms, density, state mapping or the lightweight Win32 architecture.
    println!("cargo:rerun-if-changed=build/legacy");
}
