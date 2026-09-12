mod current {
    include!("build/legacy/build_v1241_audit_hardening.rs");
    pub fn run() { main(); }
}

fn main() {
    current::run();
    // Historical generated-source compatibility remains assertion-guarded. v1.24.1
    // adds audited ordering, updater rollback/recovery, chat visibility and release
    // hardening without widening the verified dungeon-mechanic protocol scope.
    println!("cargo:rerun-if-changed=build/legacy");
}
