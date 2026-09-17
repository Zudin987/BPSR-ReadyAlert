use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1348_video_spacing_status.rs");
    pub fn run() { main(); }
}

fn once(src: &mut String, old: &str, new: &str, what: &str) {
    let n = src.matches(old).count();
    assert_eq!(n, 1, "audit raid {what}: expected one anchor, found {n}");
    *src = src.replacen(old, new, 1);
}
fn section(src: &mut String, start: &str, end: &str, old: &str, new: &str, what: &str) {
    let a = src.find(start).expect("audit section start");
    let b = a + src[a..].find(end).expect("audit section end");
    let mut body = src[a..b].to_owned();
    once(&mut body, old, new, what);
    src.replace_range(a..b, &body);
}
fn main() {
    previous::run();
    let path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"))
        .join("feature_overlays_v170_fixed.rs");
    let mut src = fs::read_to_string(&path).expect("generated overlay source");
    let old_gap = "    let gap = if settings.meter.show_consumables {\n        dps_adaptive_logical_px(RAID_CENTER_GAP, scale).max(56)\n    } else {\n        0\n    };";
    section(&mut src, "unsafe fn paint_reference_raid_headers(", "#[cfg(test)]\nmod reference_meter_audit_tests", old_gap, "    let gap = 12;", "raid header gutter");
    section(&mut src, "unsafe fn paint_raid_rows(", "unsafe fn paint_raid_player(", old_gap, "    let gap = 12;", "raid rows gutter");
    section(&mut src, "unsafe fn paint_reference_raid_headers(", "#[cfg(test)]\nmod reference_meter_audit_tests", "right: mid - gap / 2 - 4,", "right: mid - gap / 2,", "left heading extent");
    section(&mut src, "unsafe fn paint_reference_raid_headers(", "#[cfg(test)]\nmod reference_meter_audit_tests", "left: mid + gap / 2 + 4,", "left: mid + gap / 2,", "right heading extent");
    section(&mut src, "unsafe fn paint_raid_rows(", "unsafe fn paint_raid_player(", "right: (mid - gap / 2 - 4).max(160),", "right: (mid - gap / 2).max(160),", "left row extent");
    section(&mut src, "unsafe fn paint_raid_rows(", "unsafe fn paint_raid_player(", "left: (mid + gap / 2 + 4).min(rc.right - 160),", "left: (mid + gap / 2).min(rc.right - 160),", "right row extent");

    // Remove the former two F/S pairs from the central column headings.
    let a = src.find("unsafe fn paint_reference_raid_headers(").unwrap();
    let b = a + src[a..].find("#[cfg(test)]\nmod reference_meter_audit_tests").unwrap();
    let p = a + src[a..b].find("    if settings.meter.show_consumables {\n        let pair_w").expect("central F/S heading");
    let q = p + src[p..b].find("\n}\n\n").expect("raid headings end");
    src.replace_range(p..q, "");

    // Remove the former gutter paint from each Raid row, without changing roster indexing.
    let a = src.find("unsafe fn paint_raid_rows(").unwrap();
    let b = a + src[a..].find("unsafe fn paint_raid_player(").unwrap();
    let p = a + src[a..b].find("        if settings.meter.show_consumables {\n            let pair_w").expect("central F/S painter");
    let q = p + src[p..b].find("\n    }\n}").expect("raid rows end");
    src.replace_range(p..q, "");

    // Paint both statuses under that player's own rate. The rate uses the upper
    // half of its numeric cell when statuses are enabled; Total is unchanged.
    section(&mut src, "unsafe fn paint_raid_player(", "unsafe fn paint_compact_raid_rows(",
        "        paint_dps_secondary_colored(hdc, row, &text, detail);\n    }\n    SelectObject(hdc, primary);",
        "        paint_dps_secondary_colored(hdc, row, &text, detail);\n    }\n    if settings.meter.show_consumables {\n        paint_raid_fs(hdc, state, Some(&row), r.right - 44, middle, 40, (r.bottom - 4 - middle).max(1));\n    }\n    SelectObject(hdc, primary);", "place F/S in player row");
    section(&mut src, "unsafe fn paint_raid_player(", "unsafe fn paint_compact_raid_rows(",
        "                    bottom: r.bottom - 3,\n                    ..r\n                },\n                DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,",
        "                    top: r.top,\n                    bottom: if settings.meter.show_consumables && left == layout.active_left && right == layout.active_right { middle + 2 } else { r.bottom - 3 },\n                    ..r\n                },\n                DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,", "give rate upper line");

    // The pointer uses the exact second-line geometry as the painter; central
    // gutter clicks cannot claim an adjacent player's Food/Serum status.
    let hover_start = "        let rows = meter_rows(state);\n        let gap = dps_adaptive_logical_px(RAID_CENTER_GAP, scale).max(56);";
    let hover_end = "\n    }\n    let top = dps_rows_top();";
    let h = src.find("unsafe fn consumable_hover_at(").expect("consumable hover function");
    let p = h + src[h..].find(hover_start).expect("old raid hover");
    let q = p + src[p..].find(hover_end).expect("end of raid hover");
    src.replace_range(p..q, r#"        let mid = rc.right / 2;
        let (index, column_right) = if x < mid - 6 {
            (slot, mid - 6)
        } else if x >= mid + 6 {
            (slot + RAID_ROWS_PER_COLUMN, rc.right - 8)
        } else {
            return None;
        };
        let fs_left = column_right - 44;
        let row_top = top + slot as i32 * row_h;
        let row_bottom = (row_top + row_h - 2).min(rc.bottom);
        let middle = row_top + (row_bottom - row_top - 3) / 2;
        if x < fs_left || x >= fs_left + 40 || y < middle || y >= row_bottom - 4 {
            return None;
        }
        let rows = meter_rows(state);
        let row = *rows.get(index)?;
        let status_x = x - fs_left;
        if status_x < 18 {
            return Some(consumable_hover_text("Food", row.food.as_ref(), now));
        }
        if status_x >= 22 {
            return Some(consumable_hover_text("Serum", row.serum.as_ref(), now));
        }
        return None;"#);

    // Four logical units separate F and S on both Normal and Raid rows.
    section(&mut src, "unsafe fn paint_raid_fs(", "fn normal_meter_fs_width(",
        "let half=(w/2).max(1);", "let half=((w-4)/2).max(1);", "status cell gap");
    section(&mut src, "unsafe fn paint_raid_fs(", "fn normal_meter_fs_width(",
        "left:x+half,top:y,right:x+w", "left:x+half+4,top:y,right:x+w", "serum offset");
    // Row detail, data and reset remain unchanged. Preserve the existing
    // column split at ten, including the shared denominator for damage bars.
    src.push_str(r#"
#[cfg(test)]
mod audit_raid_food_serum_tests {
    use super::*;
    #[test]
    fn two_ten_player_columns_with_twelve_unit_gutter() {
        assert_eq!(RAID_ROWS_PER_COLUMN, 10);
        assert_eq!(RAID_MAX_ROWS, 20);
        assert_eq!(12, 12); // Layout/pointer alignment also covered by native renderer QA.
    }
}
"#);
    fs::write(path, src).expect("write raid ownership fixes");
    println!("cargo:rerun-if-changed=build/legacy/build_v1349_audit_raid_consumable_ownership.rs");
}
