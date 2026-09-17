mod current {
    include!("build/legacy/build_v1343_white_header_icons.rs");
    pub fn run() { main(); }
}

fn main() {
    current::run();
    // The native white-header-icon pass follows all behavior and layout stages.
    println!("cargo:rerun-if-changed=build/legacy");
}
