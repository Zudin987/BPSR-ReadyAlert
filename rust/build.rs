#![recursion_limit = "256"]

mod current {
    include!("build/legacy/build_v1407_ignore_finished_run_replays.rs");
    pub fn run() { main(); }
}

fn main() {
    current::run();
    // Changes to generated sources must originate in checked-in builders.
    println!("cargo:rerun-if-changed=build/legacy");
}
