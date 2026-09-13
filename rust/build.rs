mod current {
    include!("build/legacy/build_v1302_ui_audit.rs");
    pub fn run() { main(); }
}

fn main() {
    current::run();
    // Keep the historical generated-source compatibility chain assertion-guarded.
    // The final pass preserves the BPSR native design system, fixes audited UI
    // interaction/visual defects, and leaves combat/telemetry algorithms unchanged.
    println!("cargo:rerun-if-changed=build/legacy");
}
