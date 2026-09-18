use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1345_narrow_header_fit.rs");
    pub fn run() { main(); }
}

fn once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "video spacing {label}: expected one anchor, got {count}");
    *source = source.replacen(from, to, 1);
}
fn between(source: &mut String, from: &str, until: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "video spacing {label}: expected one start, got {count}");
    let start = source.find(from).expect("start");
    let end = start + source[start..].find(until).expect("end");
    source.replace_range(start..end, to);
}
fn main() {
    previous::run();
    let path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"))
        .join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("generated meter source");

    // Use actual row space rather than the previous 340/480/600px breakpoints.
    between(&mut source,
        "unsafe fn dps_row_layout_responsive(",
        "unsafe fn paint_dps_secondary_colored(",
        r#"unsafe fn dps_row_layout_responsive(
    hdc: HDC,
    r: RECT,
    scale: i32,
    show_imagines: bool,
    show_active: bool,
    show_share: bool,
    show_deaths: bool,
    _row: &DpsRow,
    name_font: HFONT,
    _secondary_font: HFONT,
) -> DpsRowLayout {
    let old = SelectObject(hdc, name_font);
    let metric_w = dps_text_width(hdc, "888.88 M", 70)
        .max(dps_adaptive_logical_px(66, scale)) + 3;
    SelectObject(hdc, old);
    let gap = dps_adaptive_logical_px(4, scale);
    let name_left = r.left + 25;
    let name_min = dps_adaptive_logical_px(94, scale);
    let class_min = dps_adaptive_logical_px(62, scale);
    let available = (r.right - 4 - name_left).max(0);
    let active_w = if show_active { metric_w } else { 0 };
    let active_span = if show_active { active_w + gap } else { 0 };
    // Keep the player readable when the two numeric metrics cannot both fit.
    let total_w = if !show_active
        || available >= active_span + metric_w + gap + name_min {
        metric_w
    } else { 0 };
    let total_span = if total_w > 0 { total_w + gap } else { 0 };
    // Optional columns only consume true surplus after name, class and rates.
    let mut extra = available - active_span - total_span - name_min - class_min - gap;
    let share_w = dps_adaptive_logical_px(42, scale);
    let share_w = if show_share && extra >= share_w + gap {
        extra -= share_w + gap;
        share_w
    } else { 0 };
    let death_w = dps_adaptive_logical_px(22, scale);
    let death_w = if show_deaths && extra >= death_w + gap { death_w } else { 0 };
    let mut cursor = r.right - 4;
    let (death_left, death_right) = dps_take_column(&mut cursor, death_w, gap);
    let (share_left, share_right) = dps_take_column(&mut cursor, share_w, gap);
    let (active_left, active_right) = dps_take_column(&mut cursor, active_w, gap);
    let (total_left, total_right) = dps_take_column(&mut cursor, total_w, gap);

    let identity_w = (cursor - name_left).max(0);
    let has_class = identity_w >= name_min + gap + class_min;
    let badge_w = dps_badge_w(scale);
    let badge_gap = dps_badge_gap(scale);
    let badge_outer_gap = gap + 2;
    let spare_after_class = identity_w - name_min
        - if has_class { class_min + gap } else { 0 };
    let badge_count = if show_imagines && spare_after_class >=
        2 * badge_w + badge_gap + badge_outer_gap { 2 }
        else if show_imagines && spare_after_class >= badge_w + badge_outer_gap { 1 }
        else { 0 };
    let badge_span = if badge_count == 0 { 0 } else {
        badge_count * badge_w + (badge_count - 1) * badge_gap
    };
    let badge_left = cursor - badge_span;
    let identity_right = if badge_count > 0 { badge_left - badge_outer_gap } else { cursor };
    let identity_space = (identity_right - name_left).max(0);
    let class_w = if has_class {
        let spare = (identity_space - name_min - class_min - gap).max(0);
        class_min + (spare / 2).min(dps_adaptive_logical_px(150, scale) - class_min)
    } else { 0 };
    let spec_right = identity_right;
    let spec_left = spec_right - class_w;
    let name_right = if class_w > 0 { spec_left - gap } else { identity_right };
    DpsRowLayout {
        name_left,
        name_right: name_right.max(name_left),
        spec_left,
        spec_right,
        badge_left,
        badge_count: badge_count as usize,
        total_left,
        total_right,
        active_left,
        active_right,
        share_left,
        share_right,
        death_left,
        death_right,
    }
}
"#,
        "real-space-aware player columns");

    // Header and rows must make the same Compact Total visibility decision.
    once(&mut source,
        "let show_total=dps_effective_width(rc.right,scale)>=460;",
        "let show_total=reference_compact_show_total((rc.right-14).max(1),scale);",
        "compact row total");
    once(&mut source,
        "let show_total = dps_effective_width((left.right - left.left).max(1), scale) >= 420;",
        "let show_total = reference_compact_show_total((left.right - left.left).max(1), scale);",
        "compact raid row total");
    between(&mut source,
        "            let show_total = dps_effective_width(r.right - r.left, scale)\n",
        "            let total_right = rate_left - dps_adaptive_logical_px(3, scale);",
        "            let show_total = reference_compact_show_total(r.right - r.left, scale);\n",
        "compact header total matches row");
    // Show an Imagines heading when even one image fits; use the same reserved
    // span as the row, instead of a hard 480px width cutoff.
    once(&mut source,
        "    if settings.meter.show_imagines\n        && dps_effective_width(content.right - content.left, scale) >= 480\n    {",
        "    let badge_header_span = (layout.total_left - layout.badge_left).max(0);\n    if settings.meter.show_imagines && badge_header_span >= dps_badge_w(scale) {",
        "badges header visibility");
    once(&mut source,
        "                right: layout.badge_left + 2 * dps_badge_w(scale) + dps_badge_gap(scale),",
        "                right: layout.badge_left + badge_header_span,",
        "badge header width");

    // Calculate encounter label space from the visible HP/time/title, rather
    // than shortening 'Total Damage' immediately at 600 or 420px.
    between(&mut source,
        "    let total = if rc.right >= 600 {",
        "    let time = format_time(snapshot.encounter_ms);",
        r#"    let full = format!("{}  {}", reference_meter_total_label(state.sort_mode), value);
    let short = format!("Total  {value}");
    let time_width = dps_text_width(hdc, &format_time(snapshot.encounter_ms), 44) + 4;
    let hp_width = if settings.meter.show_target {
        dps_text_width(hdc, &format!("HP {}", target_hp_value(snapshot)), 46) + 4
    } else { 0 };
    let separators = if hp_width > 0 { 32 } else { 16 };
    let title_min = if rc.right < 320 { 56 } else { 88 };
    let total_room = rc.right - 16 - time_width - hp_width - separators - title_min;
    let total = if dps_text_width(hdc, &full, 140) + 4 <= total_room { full }
        else if dps_text_width(hdc, &short, 80) + 4 <= total_room { short }
        else { value };
"#,
        "measure encounter text");

    // Scale and saved-rectangle/edge anchoring are still handled by v1345.
    once(&mut source,
        "scale_px(if compact { 460 } else { 700 }, scale)",
        "scale_px(if compact { 460 } else { 620 }, scale)",
        "narrower normal Raid min");
    once(&mut source,
        "assert_eq!(reference_raid_min_width(100, false), 700);",
        "assert_eq!(reference_raid_min_width(100, false), 620);",
        "raid width regression assertion");

    source.push_str(r#"
// Hide Compact's Total only after the remaining player name becomes too small.
fn reference_compact_show_total(width: i32, scale: i32) -> bool {
    let min_name = dps_adaptive_logical_px(98, scale);
    width >= dps_adaptive_logical_px(24 + 64 + 70 + 6 + 8, scale) + min_name
}
#[cfg(test)]
mod video_spacing_regression_tests {
    use super::*;
    #[test]
    fn compact_total_stays_until_name_space_runs_out() {
        assert!(!reference_compact_show_total(220, 100));
        assert!(reference_compact_show_total(300, 100));
        assert!(reference_compact_show_total(400, 100));
    }
    #[test]
    fn normal_raid_min_is_narrower_but_still_two_columns() {
        assert_eq!(reference_raid_min_width(100, false), 620);
        assert_eq!(reference_raid_min_width(150, false), 930);
        assert_eq!(reference_raid_min_width(100, true), 460);
        assert_eq!(RAID_ROWS_PER_COLUMN, 10);
    }
}
"#);

    fs::write(path, source).expect("write measured spacing and narrower Raid");
    println!("cargo:rerun-if-changed=build/legacy/build_v1346_video_spacing.rs");
}
