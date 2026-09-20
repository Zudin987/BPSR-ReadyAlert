// Follow-up native UI fixes verified against generated-source CI output.
// Fail loudly on stale anchors; a green build must not silently skip a fix.
use std::{env, fs, path::PathBuf};
mod previous {
    include!("build_v1376_ui_audit_copy.rs");
    pub fn run() { main(); }
}
fn change(out: &std::path::Path, name: &str, old: &str, new: &str, count: usize, issue: &str) {
    let path = out.join(name);
    let mut text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{name}: {e}"));
    assert_eq!(text.matches(old).count(), count, "{issue}: missing/ambiguous native source anchor");
    text = text.replace(old, new);
    fs::write(path, text).unwrap_or_else(|e| panic!("{name}: {e}"));
    println!("cargo:warning=UI audit {issue}: verified {count} generated-source edit(s)");
}
fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let meter = "feature_overlays_v170_fixed.rs";
    // S03/M06: badges are OPTIONAL. Preserve measured minimum player AND class
    // width plus a 48-unit breathing zone before allocating image slots.
    // Never change persisted show_imagines or row identity, and use this SAME
    // layout for headers and normal rows.
    change(&out, meter,
        "let badge_count = if show_imagines && spare_after_class >=",
        "let badge_count = if show_imagines && has_class && spare_after_class >=",
        1, "S03 require class before double badge");
    change(&out, meter,
        "2 * badge_w + badge_gap + badge_outer_gap { 2 }",
        "2 * badge_w + badge_gap + badge_outer_gap + dps_adaptive_logical_px(48, scale) { 2 }",
        1, "S03 reserve readable name before double badge");
    change(&out, meter,
        "else if show_imagines && spare_after_class >= badge_w + badge_outer_gap { 1 }",
        "else if show_imagines && has_class && spare_after_class >= badge_w + badge_outer_gap + dps_adaptive_logical_px(48, scale) { 1 }",
        1, "S03 reserve readable name before single badge");
    change(&out, meter,
        "if settings.meter.show_imagines && badge_header_span >= dps_badge_w(scale) {",
        "if settings.meter.show_imagines && layout.badge_left > layout.spec_right && badge_header_span >= dps_badge_w(scale) {",
        1, "M07 hide imaginary badge column heading when there is no column");
    // M01: the history mode intentionally labels its return action "↩ Live".
    // Update the existing behavior regression test to reflect that user-visible
    // differentiation instead of reverting a legitimate improvement.
    change(&out, "ui_meter_qa_tests_raid_v1345.rs",
        "            .label,\n        \"Live\"\n    );\n    history_newer(&mut s);",
        "            .label,\n        \"↩ Live\"\n    );\n    history_newer(&mut s);",
        1, "M01 history return label regression");
    // H01: original always remains. A translated line that differs only in
    // case/spacing/punctuation or is equivalent repeated 'ha' laughter wastes
    // space and wrongly implies useful translation. Preserve all other short,
    // mixed-language, CJK and punctuation-rich messages.
    change(&out, "overlay_v150_v1181.rs",
        "        let source = if source_language.trim().is_empty() { \"AUTO\".to_string() } else { source_language.trim().to_ascii_uppercase() };\n        item.translation = Some((source, text));",
        "        if audit_redundant_translation(&item.message.text, &text) {\n            item.translation = None;\n            InvalidateRect(hwnd, null(), 0);\n            return;\n        }\n        let source = if source_language.trim().is_empty() { \"AUTO\".to_string() } else { source_language.trim().to_ascii_uppercase() };\n        item.translation = Some((source, text));",
        1, "H01 suppress redundant translated chat line");
    let chat = out.join("overlay_v150_v1181.rs");
    let mut text = fs::read_to_string(&chat).expect("chat generated source");
    text.push_str(r#"
// Presentation-only: never suppress or modify the actual received chat message.
fn audit_redundant_translation(original: &str, translated: &str) -> bool {
    let normalized = |s: &str| -> String {
        s.chars().filter(|c| c.is_alphanumeric())
            .flat_map(|c| c.to_lowercase()).collect()
    };
    let a = normalized(original);
    let b = normalized(translated);
    if a.is_empty() || b.is_empty() { return false; }
    if a == b { return true; }
    let ha = |s: &str| s.len() >= 4 && s.len() % 2 == 0
        && s.as_bytes().chunks_exact(2).all(|part| part == b"ha");
    ha(&a) && ha(&b)
}
#[cfg(test)]
mod september_chat_translation_tests {
    use super::*;
    #[test]
    fn suppress_only_redundant_translation_not_the_original() {
        assert!(audit_redundant_translation("HA HA!", "haha"));
        assert!(audit_redundant_translation("hahaha", "hahahaha"));
        assert!(!audit_redundant_translation("ha", "Yes"));
        assert!(!audit_redundant_translation("Apa khabar?", "How are you?"));
        assert!(!audit_redundant_translation("你好", "Hello"));
        assert!(!audit_redundant_translation("Hello?", "Yes!"));
    }
}
"#);
    fs::write(chat, text).expect("write chat translation UI");
    println!("cargo:rerun-if-changed=build/legacy/build_v1377_ui_audit_verified.rs");
}
