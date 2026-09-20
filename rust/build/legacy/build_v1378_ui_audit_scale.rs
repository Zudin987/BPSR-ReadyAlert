// September UI audit follow-up. The attempted 1:1 row rescale removed the
// two-line font clearance and violated minimum-window and hit-rect contracts.
// Retain the verified geometry until a coordinated header/row redesign has
// native visual coverage. Apply the narrow-layout badge priority against the
// current responsive row implementation rather than a stale anchor.
use std::{env, fs, path::PathBuf};
mod previous {
    include!("build_v1377_ui_audit_verified.rs");
    pub fn run() { main(); }
}
fn verified_replace(src: &mut String, from: &str, to: &str, label: &str) {
    assert_eq!(src.matches(from).count(), 1, "September UI audit {label}: expected exactly one anchor");
    *src = src.replacen(from, to, 1);
    println!("cargo:warning=September UI audit: verified {label}");
}
fn main() {
    previous::run();
    let path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"))
        .join("feature_overlays_v170_fixed.rs");
    let mut src = fs::read_to_string(&path).expect("generated overlay");
    // The actual responsive layout allocates zero/one/two badges based on
    // remaining identity space. The obsolete reserve_badges anchor no longer
    // exists; guard both allocations with the same readable-width threshold.
    verified_replace(&mut src,
        "let badge_count = if show_imagines && spare_after_class >=",
        "let badge_count = if show_imagines && audit_allow_optional_badges(r.right - r.left, scale) && spare_after_class >=",
        "optional badges never displace narrow player names (two badge branch)");
    verified_replace(&mut src,
        "else if show_imagines && spare_after_class >= badge_w + badge_outer_gap { 1 }",
        "else if show_imagines && audit_allow_optional_badges(r.right - r.left, scale) && spare_after_class >= badge_w + badge_outer_gap { 1 }",
        "optional badges never displace narrow player names (one badge branch)");
    src.push_str(r#"
fn audit_allow_optional_badges(row_width: i32, scale: i32) -> bool {
    dps_effective_width(row_width, scale) >= 640
}
#[cfg(test)]
mod september_badge_priority_tests {
    use super::*;
    #[test]
    fn badges_hide_below_the_identity_safe_breakpoint() {
        for scale in [50, 60, 75, 84, 100, 125, 150] {
            for physical_width in [280, 340, 420, 480, 560, 620] {
                let effective = dps_effective_width(physical_width, scale);
                assert_eq!(audit_allow_optional_badges(physical_width, scale), effective >= 640);
            }
        }
        assert!(!audit_allow_optional_badges(639, 100));
        assert!(audit_allow_optional_badges(640, 100));
    }
}
"#);
    fs::write(path, src).expect("write September responsive badge priority");
    println!("cargo:rerun-if-changed=build/legacy/build_v1378_ui_audit_scale.rs");
}
