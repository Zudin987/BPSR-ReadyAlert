use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1352_audit_rank_gap.rs");
    pub fn run() { main(); }
}

fn once(src: &mut String, old: &str, new: &str, what: &str) {
    let n = src.matches(old).count();
    assert_eq!(n, 1, "raid hit {what}: expected one anchor, found {n}");
    *src = src.replacen(old, new, 1);
}
fn within(src: &mut String, start: &str, end: &str, old: &str, new: &str, what: &str) {
    let a = src.find(start).expect("raid hit section start missing");
    let b = a + src[a..].find(end).expect("raid hit section end missing");
    let mut section = src[a..b].to_owned();
    once(&mut section, old, new, what);
    src.replace_range(a..b, &section);
}
fn main() {
    previous::run();
    let path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("feature_overlays_v170_fixed.rs");
    let mut src = fs::read_to_string(&path).expect("generated overlay source");

    // Both the native painter and pointer paths must use the same physical
    // left/right column boundaries. The old pointer still reserved the
    // removed 56-unit Food/Serum gutter and missed real player rows.
    let helper = r#"fn raid_painted_player_column(width: i32, compact: bool, scale: i32, x: i32) -> Option<usize> {
    let mid = width / 2;
    let (left_end, right_start) = if compact {
        let gap = dps_adaptive_logical_px(9, scale).max(7);
        ((mid - gap).max(145), (mid + gap).min(width - 145))
    } else {
        ((mid - 6).max(160), (mid + 6).min(width - 160))
    };
    if x >= 6 && x < left_end { Some(0) }
    else if x >= right_start && x < width - 8 { Some(1) }
    else { None }
}

"#;
    once(&mut src, "unsafe fn raid_row_at(", &format!("{helper}unsafe fn raid_row_at("), "shared column bounds");
    within(&mut src, "unsafe fn raid_row_at(", "unsafe fn hover_badge_at(",
        "    let settings = state.features.read().map(|v| v.clone()).unwrap_or_default();\n    let scale = dps_layout_scale(state);",
        "    let scale = dps_layout_scale(state);", "remove obsolete gutter-setting read");
    within(&mut src, "unsafe fn raid_row_at(", "unsafe fn hover_badge_at(",
        r#"    let gap = if state.compact_mode {
        dps_adaptive_logical_px(18, scale).max(14)
    } else if settings.meter.show_consumables {
        dps_adaptive_logical_px(RAID_CENTER_GAP, scale).max(56)
    } else {
        0
    };
    let mid = rc.right / 2;
    let index = if x < mid - gap / 2 {
        slot
    } else if x > mid + gap / 2 {
        slot + RAID_ROWS_PER_COLUMN
    } else {
        return None;
    };"#,
        "    let column = raid_painted_player_column(rc.right, state.compact_mode, scale, x)?;\n    let index = slot + column * RAID_ROWS_PER_COLUMN;", "player row click ownership");
    within(&mut src, "unsafe fn consumable_hover_at(", "const REFERENCE_METER_BG:",
        r#"        let mid = rc.right / 2;
        let (index, column_right) = if x < mid - 6 {
            (slot, mid - 6)
        } else if x >= mid + 6 {
            (slot + RAID_ROWS_PER_COLUMN, rc.right - 8)
        } else {
            return None;
        };"#,
        "        let column = raid_painted_player_column(rc.right, false, scale, x)?;\n        let column_right = if column == 0 { (rc.right / 2 - 6).max(160) } else { rc.right - 8 };\n        let index = slot + column * RAID_ROWS_PER_COLUMN;", "Food/Serum ownership");
    within(&mut src, "unsafe fn consumable_hover_at(", "const REFERENCE_METER_BG:",
        r#"    let half = ((fs.right - fs.left) / 2).max(1);
    Some(if x < fs.left + half {
        consumable_hover_text("Food", row.food.as_ref(), now)
    } else {
        consumable_hover_text("Serum", row.serum.as_ref(), now)
    })"#,
        r#"    let half = (((fs.right - fs.left) - 4) / 2).max(1);
    if x < fs.left + half {
        Some(consumable_hover_text("Food", row.food.as_ref(), now))
    } else if x >= fs.left + half + 4 {
        Some(consumable_hover_text("Serum", row.serum.as_ref(), now))
    } else {
        None // The four-unit gap is not a status target.
    }"#, "normal Food/Serum gap");
    // Ranking uses encounter total; the rate heading uses active time.
    within(&mut src, "unsafe fn overlay_help(", "fn reference_meter_rate_label(",
        "    let row = if reference_raid_columns(state, rc.right) {",
        "    if y < dps_rows_top_for(state) {\n        return Some(format!(\"Rank: {} descending • {} uses active combat time\",\n            reference_meter_total_label(state.sort_mode), reference_meter_rate_label(state.sort_mode)));\n    }\n    let row = if reference_raid_columns(state, rc.right) {", "accurate heading tooltip");
    src.push_str(r#"
#[cfg(test)]
mod audit_raid_hit_alignment_tests {
    use super::*;
    #[test]
    fn rendered_raid_column_edges_are_clickable_and_gutter_is_not() {
        for width in [460, 600, 620, 720, 900] {
            for scale in [60, 80, 100, 125, 150, 200] {
                let mid = width / 2;
                assert_eq!(raid_painted_player_column(width, false, scale, 6), Some(0));
                assert_eq!(raid_painted_player_column(width, false, scale, mid - 7), Some(0));
                assert_eq!(raid_painted_player_column(width, false, scale, mid - 6), None);
                assert_eq!(raid_painted_player_column(width, false, scale, mid + 5), None);
                assert_eq!(raid_painted_player_column(width, false, scale, mid + 6), Some(1));
                assert_eq!(raid_painted_player_column(width, false, scale, width - 9), Some(1));
                assert_eq!(raid_painted_player_column(width, false, scale, width - 8), None);
                let gap = dps_adaptive_logical_px(9, scale).max(7);
                assert_eq!(raid_painted_player_column(width, true, scale, mid - gap - 1), Some(0));
                assert_eq!(raid_painted_player_column(width, true, scale, mid - gap), None);
                assert_eq!(raid_painted_player_column(width, true, scale, mid + gap), Some(1));
            }
        }
    }
}
"#);
    fs::write(path, src).expect("write raid hit alignment");
    println!("cargo:rerun-if-changed=build/legacy/build_v1353_audit_raid_hit_alignment.rs");
}
