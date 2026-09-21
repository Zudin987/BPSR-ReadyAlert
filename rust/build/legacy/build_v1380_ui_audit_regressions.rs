// Final-stage checks and regression updates for the September native UI audit.
// Edit the generated Rust actually compiled by the Windows target, not a report.
use std::{env, fs, path::{Path, PathBuf}};
mod previous {
    include!("build_v1379_ui_audit_settings_chat.rs");
    pub fn run() { main(); }
}
fn required(out: &Path, name: &str, old: &str, new: &str, count: usize, id: &str) {
    let path = out.join(name);
    let mut source = fs::read_to_string(&path).unwrap_or_else(|e| panic!("UI audit {id}: cannot read {name}: {e}"));
    assert_eq!(source.matches(old).count(), count, "UI audit {id}: missing or ambiguous generated-source anchor");
    source = source.replace(old, new);
    assert_eq!(source.matches(new).count(), count, "UI audit {id}: replacement was not retained");
    fs::write(&path, source).unwrap_or_else(|e| panic!("UI audit {id}: cannot write {name}: {e}"));
    println!("cargo:warning=UI audit {id}: {count} compiled-source replacement(s) verified");
}
fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let meter = "feature_overlays_v170_fixed.rs";
    // M04: preserve the behavioural assertion, but require the new honest
    // unknown status. The earlier regression failure compared new behaviour
    // against a stale literal rather than finding a broken implementation.
    required(&out, meter,
        "assert_eq!(consumable_hover_text(\"Serum\",None,now),\"Serum: not detected\");",
        "assert_eq!(consumable_hover_text(\"Serum\",None,now),\"Serum: status unknown (not detected)\");\n        assert_eq!(consumable_hover_text(\"Food\",None,now),\"Food: status unknown (not detected)\");",
        1, "M04 keep and strengthen unknown-status regression");

    // S03/M07: build_v1375 predates the responsive column planner and emits
    // two obsolete-anchor warnings. Its replacements are superseded by the
    // checked final planner in v1377; never regard the warnings as success
    // without proving the actual downstream compiled implementation exists.
    let source = fs::read_to_string(out.join(meter)).expect("read final generated meter");
    for (id, token) in [
        ("S03 identity-first planner", "show_imagines && has_class && spare_after_class >="),
        ("S03 name-space reserve", "badge_outer_gap + dps_adaptive_logical_px(48, scale)"),
        ("M07 corresponding badge heading", "layout.badge_left > layout.spec_right && badge_header_span >= dps_badge_w(scale)"),
        ("S02 complete-row height", "audit_height_preserving_rows(overlay,source,old,new)"),
    ] {
        assert!(source.contains(token), "UI audit {id}: expected implementation missing from compiled source");
        println!("cargo:warning=UI audit {id}: confirmed in final generated source");
    }

    // Extend both geometry tests and the real GDI renderer. The previous
    // diagnostic upload had 140 BMPs but omitted 70 and 90 percent entirely.
    let qa = "ui_meter_qa_tests_raid_v1345.rs";
    required(&out, qa,
        "for scale in [60, 80, 100, 125, 150, 175, 200] {",
        "for scale in [60, 70, 80, 90, 100, 125, 150, 175, 200] {",
        2, "S01/S03 five-step scale matrix and GDI render");
    let mut tests = fs::read_to_string(out.join(qa)).expect("generated native meter tests");
    tests.push_str(r#"
#[test]
fn september_scale_only_preserves_five_complete_rows() {
    for compact in [false, true] {
        let state = state_stub(compact);
        for old in [60, 70, 80, 90, 100] {
            for new in [60, 70, 80, 90, 100] {
                let pitch_old = dps_row_h_for(&state, old);
                let required_logical = dps_rows_top_for(&state) + 5 * pitch_old + 8;
                let original = RECT { left: 0, top: 0, right: scale_px(600, old), bottom: scale_px(required_logical, old) };
                let physical = audit_height_preserving_rows(&state, original, old, new);
                let actual_logical = physical_extent_to_logical(physical, new);
                let visible = (actual_logical - dps_rows_top_for(&state)) / dps_row_h_for(&state, new);
                assert!(visible >= 5, "compact={compact} old={old} new={new} visible={visible}");
            }
        }
    }
}
"#);
    fs::write(out.join(qa), tests).expect("write extended native geometry regression");
    println!("cargo:rerun-if-changed=build/legacy/build_v1380_ui_audit_regressions.rs");
}
