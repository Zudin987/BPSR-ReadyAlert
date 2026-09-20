// UI audit follow-up: UI copy and migration-free naming changes belong in the
// generated source pipeline, never in OUT_DIR files committed by hand.
use std::{env, fs, path::PathBuf};
mod previous {
    include!("build_v1375_ui_ux_audit.rs");
    pub fn run() { main(); }
}

fn replace_visible_label(src: &mut String, old: &str, new: &str, finding: &str) {
    let found = src.matches(old).count();
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

    // H03: preserve the distinction between a capture failure and tab filter.
    // Accept both apostrophe spellings; only warn if neither source anchor exists.
    let simple = "Recent chat exists, but none matches this tab's channels, level rule or filters";
    let curly = "Recent chat exists, but none matches this tab’s channels, level rule or filters";
    if src.contains(simple) { replace_visible_label(&mut src, simple, "No recent messages match this tab.", "H03 concise chat empty state"); }
    else if src.contains(curly) { replace_visible_label(&mut src, curly, "No recent messages match this tab.", "H03 concise chat empty state"); }
    else { println!("cargo:warning=UI audit H03: neither chat empty-state anchor found; implementation NOT verified"); }

    // T01: maintain the existing stat list and explicit disconnected state.
    replace_visible_label(&mut src, "No active mechanics", "Waiting for mechanics", "T01 idle tracker label");

    // T04: rename only the exact shortened title, not the prefix of an already
    // expanded Tracker & Mechanics title (which would duplicate 'anics').
    let short = "Tracker & Mech\"";
    if src.contains(short) {
        replace_visible_label(&mut src, short, "Tracker & Mechanics\"", "T04 overlay title");
    } else {
        println!("cargo:warning=UI audit T04: abbreviated tracker title not found; check canonical name directly");
    }
    replace_visible_label(&mut src, "Dungeon Mechanics", "Tracker & Mechanics", "T04 settings naming");

    fs::write(&path, src).expect("write audited UI copy");
    println!("cargo:rerun-if-changed=build/legacy/build_v1376_ui_audit_copy.rs");
}
