#![recursion_limit = "256"]

mod current {
    include!("build/legacy/build_v1411_event_tracker_polish.rs");
    pub fn run() { main(); }
}

#[path = "build/design.rs"]
mod design;

fn main() {
    current::run();
    design::apply();
    // Changes to generated sources must originate in checked-in builders.
    println!("cargo:rerun-if-changed=build/legacy");
}
