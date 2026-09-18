use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1356_death_priority_owner_frame.rs");
    pub fn run() { main(); }
}
fn once(src: &mut String, old: &str, new: &str, label: &str) {
    let count = src.matches(old).count();
    assert_eq!(count, 1, "narrow revive {label}: expected one anchor, got {count}");
    *src = src.replacen(old, new, 1);
}
fn section(src: &mut String, start: &str, end: &str, old: &str, new: &str, label: &str) {
    let a = src.find(start).expect("narrow revive section start");
    let b = a + src[a..].find(end).expect("narrow revive section end");
    let mut fragment = src[a..b].to_owned();
    once(&mut fragment, old, new, label);
    src.replace_range(a..b, &fragment);
}
fn main() {
    previous::run();
    let path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"))
        .join("feature_overlays_v170_fixed.rs");
    let mut src = fs::read_to_string(&path).expect("generated overlay source");

    // The Raid card has separate first and second lines. The first contains
    // the player name; use the whole second-line identity area for an urgent
    // revive message rather than restricting it to the class/status cell.
    section(&mut src, "unsafe fn paint_raid_player(", "mod white_header_icons",
        "        left: if row.is_dead { layout.spec_left } else { identity.left },\n        top: middle,",
        "        left: identity.left,\n        top: middle,",
        "raid second-line revive width");

    // Compact cards have a single line. Measure the actual text with the
    // selected native font, then use short but unambiguous labels if needed:
    // green READY means revive now; amber countdown means revive blocked.
    once(&mut src,
        "fn compact_dead_status(row:&DpsRow,now:i64)->(String,u32){revive_status_text(row,now)}",
        "fn compact_dead_status(row:&DpsRow,now:i64)->(String,u32){revive_status_text(row,now)}\nfn reference_short_revive_text(row:&DpsRow,now:i64)->String{if row.revive_blocked_until_ms<0{\"WAIT\".into()}else if row.revive_blocked_until_ms>now{format_countdown(row.revive_blocked_until_ms.saturating_sub(now)as u64)}else{\"READY\".into()}}",
        "measured narrow label fallback");
    section(&mut src, "unsafe fn paint_compact_player(", "unsafe fn paint_compact_raid_rows(",
        "        let (status, color) = compact_dead_status(row, now_ms());\n        SetTextColor(hdc, color);",
        "        let now = now_ms();\n        let (mut status, color) = compact_dead_status(row, now);\n        if dps_text_width(hdc, &status, status_w) > status_w {\n            status = reference_short_revive_text(row, now);\n        }\n        SetTextColor(hdc, color);",
        "compact measured revive label");

    src.push_str(r#"
#[cfg(test)]
mod narrow_revive_label_tests {
    use super::*;
    #[test]
    fn compact_fallback_preserves_ready_state_and_blocked_timer() {
        let mut row = DpsRow { is_dead:true, revive_blocked_until_ms: 15_000, ..Default::default() };
        assert_eq!(reference_short_revive_text(&row, 0), "15s");
        row.revive_blocked_until_ms = 0;
        assert_eq!(reference_short_revive_text(&row, 0), "READY");
        row.revive_blocked_until_ms = -1;
        assert_eq!(reference_short_revive_text(&row, 0), "WAIT");
    }
}
"#);
    fs::write(path, src).expect("write narrow revive label fixes");
    println!("cargo:rerun-if-changed=build/legacy/build_v1357_narrow_revive_labels.rs");
}
