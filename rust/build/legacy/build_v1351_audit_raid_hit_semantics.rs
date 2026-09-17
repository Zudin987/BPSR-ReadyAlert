use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1350_audit_small_scale_matrix.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, reason: &str) {
    let n = source.matches(from).count();
    assert_eq!(n, 1, "raid audit {reason}: expected one source anchor, got {n}");
    *source = source.replacen(from, to, 1);
}
fn replace_within(source: &mut String, start: &str, end: &str, from: &str, to: &str, reason: &str) {
    let a = source.find(start).expect("raid audit section start missing");
    let b = a + source[a..].find(end).expect("raid audit section end missing");
    let mut section = source[a..b].to_owned();
    replace_once(&mut section, from, to, reason);
    source.replace_range(a..b, &section);
}

fn main() {
    previous::run();
    let path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"))
        .join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("generated overlay source");

    // Match painter edges, not the removed, much wider central status gutter.
    let helper = r#"fn raid_painted_player_column(width: i32, compact: bool, scale: i32, x: i32) -> Option<usize> {
    let mid = width / 2;
    let (left_end, right_start) = if compact {
        let gap = dps_adaptive_logical_px(9, scale).max(7);
        ((mid - gap).max(145), (mid + gap).min(width - 145))
    } else {
        ((mid - 6).max(160), (mid + 6).min(width - 160))
    };
    if x >= 6 && x < left_end {
        Some(0)
    } else if x >= right_start && x < width - 8 {
        Some(1)
    } else {
        None
    }
}

"#;
    replace_once(&mut source, "unsafe fn raid_row_at(", &format!("{helper}unsafe fn raid_row_at("), "shared column helper");
    replace_within(&mut source, "unsafe fn raid_row_at(", "unsafe fn hover_badge_at(",
        r#"    let settings = state.features.read().map(|v| v.clone()).unwrap_or_default();
    let scale = dps_layout_scale(state);
    let row_h = dps_row_h_for(state, scale).max(1);"#,
        r#"    let scale = dps_layout_scale(state);
    let row_h = dps_row_h_for(state, scale).max(1);"#, "unused old F/S settings");
    replace_within(&mut source, "unsafe fn raid_row_at(", "unsafe fn hover_badge_at(",
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
        r#"    let column = raid_painted_player_column(rc.right, state.compact_mode, scale, x)?;
    let index = slot + column * RAID_ROWS_PER_COLUMN;"#, "row click matches painted columns");

    replace_within(&mut source, "unsafe fn consumable_hover_at(", "const REFERENCE_METER_BG:",
        r#"        let mid = rc.right / 2;
        let (index, column_right) = if x < mid - 6 {
            (slot, mid - 6)
        } else if x >= mid + 6 {
            (slot + RAID_ROWS_PER_COLUMN, rc.right - 8)
        } else {
            return None;
        };"#,
        r#"        let column = raid_painted_player_column(rc.right, false, scale, x)?;
        let column_right = if column == 0 { (rc.right / 2 - 6).max(160) } else { rc.right - 8 };
        let index = slot + column * RAID_ROWS_PER_COLUMN;"#, "consumables use same column owner");
    replace_within(&mut source, "unsafe fn consumable_hover_at(", "const REFERENCE_METER_BG:",
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
        None // The four-unit visual separation is not a status target.
    }"#, "normal status gap hover");

    let short_label = r#"fn reference_meter_rate_label_short(mode: SortMode) -> &'static str {
    match mode {
        SortMode::Damage => "Act. DPS",
        SortMode::Heal => "Act. HPS",
        SortMode::Tank => "Act. DTPS",
    }
}

"#;
    replace_once(&mut source, "unsafe fn paint_reference_raid_headers(",
        &format!("{short_label}unsafe fn paint_reference_raid_headers("), "active rate abbreviation");
    replace_within(&mut source, "unsafe fn paint_reference_raid_headers(", "#[cfg(test)]\nmod reference_meter_audit_tests",
        r#"                match state.sort_mode {
                    SortMode::Damage => "DPS",
                    SortMode::Heal => "HPS",
                    SortMode::Tank => "DTPS",
                },"#,
        "                reference_meter_rate_label_short(state.sort_mode),", "accurate active-rate heading");
    replace_within(&mut source, "unsafe fn overlay_help(", "fn reference_meter_rate_label(",
        r#"    let row = if reference_raid_columns(state, rc.right) {"#,
        r#"    if y < dps_rows_top_for(state) {
        return Some(format!("Rank: {} descending • {} uses active combat time",
            reference_meter_total_label(state.sort_mode), reference_meter_rate_label(state.sort_mode)));
    }
    let row = if reference_raid_columns(state, rc.right) {"#, "header metric explanation");

    source.push_str(r#"
#[cfg(test)]
mod audit_raid_painted_hit_tests {
    use super::*;
    #[test]
    fn row_clicks_follow_twelve_unit_raid_gutter_with_and_without_consumables() {
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
                let compact_gap = dps_adaptive_logical_px(9, scale).max(7);
                assert_eq!(raid_painted_player_column(width, true, scale, mid - compact_gap - 1), Some(0));
                assert_eq!(raid_painted_player_column(width, true, scale, mid - compact_gap), None);
                assert_eq!(raid_painted_player_column(width, true, scale, mid + compact_gap), Some(1));
            }
        }
    }
    #[test]
    fn raid_heading_names_active_time_metric() {
        assert_eq!(reference_meter_rate_label_short(SortMode::Damage), "Act. DPS");
        assert_eq!(reference_meter_rate_label_short(SortMode::Heal), "Act. HPS");
        assert_eq!(reference_meter_rate_label_short(SortMode::Tank), "Act. DTPS");
    }
}
"#);
    fs::write(path, source).expect("write audit raid hit and semantic fixes");
    println!("cargo:rerun-if-changed=build/legacy/build_v1351_audit_raid_hit_semantics.rs");
}
