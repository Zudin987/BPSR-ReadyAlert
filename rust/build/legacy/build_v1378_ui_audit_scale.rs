// The verified S03/M07 badge-priority implementation is in v1377.
// Do not reapply its obsolete pre-v1377 anchors: doing so aborts the build.
// The 1:1 geometry experiment also violated existing two-line row and
// minimum-window contracts, so retain the verified scaling behavior until
// a separate fully coordinated geometry redesign has native visual tests.
mod previous {
    include!("build_v1377_ui_audit_verified.rs");
    pub fn run() { main(); }
}
fn main() {
    previous::run();
    println!("cargo:rerun-if-changed=build/legacy/build_v1378_ui_audit_scale.rs");
}
