mod current {
    include!("build/legacy/build_v1250_chat_archive.rs");
    pub fn run() { main(); }
}

fn main() {
    current::run();
    // Historical generated-source compatibility remains assertion-guarded. The
    // chat archive patch keeps the existing append-only logs and only changes
    // the Open Chat Logs action to build a local, offline HTML browser view.
    println!("cargo:rerun-if-changed=build/legacy");
}
