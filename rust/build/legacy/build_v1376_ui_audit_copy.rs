// UI audit follow-up: UI copy and migration-free naming changes belong in the
// generated source pipeline, never in OUT_DIR files committed by hand.
use std::{env, fs, path::PathBuf};
mod previous {
    include!("build_v1375_ui_ux_audit.rs");
    pub fn run() { main(); }
}

fn replace_visible_label(src: &mut String, old: &str, new: &str, finding: &str) {
    let found = src.matches(old).count();
    // A missing label is NOT evidence that the finding is implemented. Emit a
    // searchable diagnostic for review, rather than reporting 36/36 complete.
    if found == 0 {
        println!("cargo:warning=UI audit {finding}: label not found ({old}); implementation NOT verified");
    } else {
        *src = src.replace(old, new);
        println!("cargo:warning=UI audit {finding}: replaced {found} source occurrence(s)");
    }
}

fn main() {
    previous::run();
    let path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"))
        .join("feature_overlays_v170_fixed.rs");
    let mut src = fs::read_to_string(&path).expect("generated overlays");

    // H03: keep capture-vs-filter distinction while shortening the explanatory
    // sentence that wrapped underneath the chat overlay's empty-state heading.
    replace_visible_label(&mut src,
        "Recent chat exists, but none matches this tab's channels, level rule or filters",
        "No recent messages match this tab.", "H03 concise chat empty state");
    replace_visible_label(&mut src,
        "Recent chat exists, but none matches this tab’s channels, level rule or filters",
        "No recent messages match this tab.", "H03 curly-apostrophe fallback");

    // T01: waiting for a mechanic is not the same thing as an empty or broken
    // capture session. Keep the existing stats and fixed-size settings intact.
    replace_visible_label(&mut src, "No active mechanics", "Waiting for mechanics", "T01 idle tracker label");

    // T04: align the visible window and settings titles. Event Tracker remains
    // the distinct subfeature; this changes no persisted identifiers.
    replace_visible_label(&mut src, "Tracker & Mech", "Tracker & Mechanics", "T04 tracker naming");
    replace_visible_label(&mut src, "Dungeon Mechanics", "Tracker & Mechanics", "T04 settings naming");

    fs::write(&path, src).expect("write audited UI copy");
    println!("cargo:rerun-if-changed=build/legacy/build_v1376_ui_audit_copy.rs");
}
