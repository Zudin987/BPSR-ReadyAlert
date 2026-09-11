mod current {
    include!("build/legacy/build_v1230.rs");
    pub fn run() { main(); }
}

fn main() {
    current::run();
    // Historical generated-source compatibility remains assertion-guarded. v1.23
    // consolidates the new audit fixes in one final stage so future source edits
    // fail loudly instead of silently producing a partially patched runtime.
    println!("cargo:rerun-if-changed=build/legacy");
}
