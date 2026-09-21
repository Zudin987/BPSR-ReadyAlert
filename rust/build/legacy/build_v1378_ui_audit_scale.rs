// Incremental verified September UI audit. v1377 already implements S03/M07;
// do not reapply the outdated badge anchors or the failing 1:1 row experiment.
use std::{env, fs, path::{Path, PathBuf}};
mod previous {
    include!("build_v1377_ui_audit_verified.rs");
    pub fn run() { main(); }
}
fn required(out: &Path, name: &str, old: &str, new: &str, issue: &str) {
    let path = out.join(name);
    let mut src = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{name}: {e}"));
    let count = src.matches(old).count();
    assert_eq!(count, 1, "UI audit {issue}: expected one native source anchor, found {count}");
    src = src.replacen(old, new, 1);
    fs::write(path, src).unwrap_or_else(|e| panic!("{name}: {e}"));
    println!("cargo:warning=UI audit {issue}: verified and applied");
}
fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let chat = "overlay_v150_v1181.rs";
    // I05: a collapsed window must not intercept clicks along the entire
    // former chat height/width. Shrink the ACTUAL HWND, not just its glyph.
    required(&out, chat,
        "let h = expanded_h.min(work.bottom - work.top);",
        "let h = audit_collapsed_tab_extent(expanded_h, work.bottom - work.top);",
        "I05 compact vertical restore handle");
    required(&out, chat,
        "let w = expanded_w.min(work.right - work.left);",
        "let w = audit_collapsed_tab_extent(expanded_w, work.right - work.left);",
        "I05 compact horizontal restore handle");
    let chat_path = out.join(chat);
    let mut source = fs::read_to_string(&chat_path).expect("chat generated source");
    source.push_str(r#"
// The window itself is limited to the visible restore tab; outside this HWND
// Windows delivers mouse events to the game instead of an invisible overlay.
fn audit_collapsed_tab_extent(expanded: i32, monitor: i32) -> i32 {
    expanded.max(1).min(96).min(monitor.max(1))
}
#[cfg(test)]
mod september_compact_restore_tests {
    use super::*;
    #[test]
    fn restore_handle_never_reuses_expanded_window_extent() {
        assert_eq!(audit_collapsed_tab_extent(640, 1080), 96);
        assert_eq!(audit_collapsed_tab_extent(420, 1920), 96);
        assert_eq!(audit_collapsed_tab_extent(640, 64), 64);
        assert_eq!(audit_collapsed_tab_extent(40, 1080), 40);
        assert_eq!(audit_collapsed_tab_extent(640, 0), 1);
    }
}
"#);
    fs::write(chat_path, source).expect("write chat restore tab tests");

    // M04: missing capture/roster status must never assert a buff is inactive.
    required(&out, "feature_overlays_v170_fixed.rs",
        "None=>format!(\"{kind}: not detected\")",
        "None=>format!(\"{kind}: status unknown (not detected)\")",
        "M04 distinguish unknown food and serum from inactive");
    // M05: keep live roster identity, but distinguish an encounter not started
    // from a historical zero or an active encounter. No invented disconnection.
    required(&out, "feature_overlays_v170_fixed.rs",
        "let title = if state.history_index.is_some() { format!(\"HISTORY · {title}\") } else { title };",
        "let title = if state.history_index.is_some() { format!(\"HISTORY · {title}\") } else if snapshot.encounter_ms == 0 && snapshot.total_damage == 0 && snapshot.total_healing == 0 && snapshot.total_damage_taken == 0 { format!(\"WAITING · {title}\") } else { title };",
        "M05 live idle versus historical encounter label");
    // G05: the real expression parser supports OR, AND and regular expressions;
    // show valid examples near the field and in its existing validation path.
    required(&out, "settings_ui_v1160_fixed.rs",
        "Filters support OR, AND and regular expressions.",
        "Example: guild OR party; healer AND buff. Regex supported.",
        "G05 in-context real parser examples");
    required(&out, "settings_ui_v1160_fixed.rs",
        "Use OR / || / spaced |, AND / &&, or a valid regular expression.",
        "Example: guild OR party; healer AND buff. Use OR / || / spaced |, AND / &&, or a valid regular expression.",
        "G05 actionable invalid-expression message");
    println!("cargo:rerun-if-changed=build/legacy/build_v1378_ui_audit_scale.rs");
}
