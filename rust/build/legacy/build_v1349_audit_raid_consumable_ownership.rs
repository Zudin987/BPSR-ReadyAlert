use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1348_video_spacing_status.rs");
    pub fn run() { main(); }
}

fn once(src: &mut String, old: &str, new: &str, what: &str) {
    let n = src.matches(old).count();
    let expected = if what == "keep numeric metrics above F/S" { 2 } else { 1 };
    assert_eq!(n, expected, "audit raid {what}: expected {expected} anchor(s), found {n}");
    *src = src.replace(old, new);
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

    // Remove the two F/S pairs from the central column headings.
    let a = src.find("unsafe fn paint_reference_raid_headers(").unwrap();
    let b = a + src[a..].find("#[cfg(test)]\nmod reference_meter_audit_tests").unwrap();
    let p = a + src[a..b].find("    if settings.meter.show_consumables {\n        let pair_w").expect("central F/S heading");
    let q = p + src[p..b].find("\n}\n\n").expect("raid headings end");
    src.replace_range(p..q, "");

    // Remove gutter paint, keeping roster indexing and denominator unchanged.
    let a = src.find("unsafe fn paint_raid_rows(").unwrap();
    let b = a + src[a..].find("unsafe fn paint_raid_player(").unwrap();
    let p = a + src[a..b].find("        if settings.meter.show_consumables {\n            let pair_w").expect("central F/S painter");
    let q = p + src[p..b].find("\n    }\n}").expect("raid rows end");
    src.replace_range(p..q, "");

    // Give rates/totals their original numeric widths but upper-line height;
    // statuses reside on the second line inside the corresponding player row.
    section(&mut src, "unsafe fn paint_raid_player(", "mod white_header_icons {",
        "        paint_dps_secondary_colored(hdc, row, &text, detail);\n    }\n    SelectObject(hdc, primary);",
        "        paint_dps_secondary_colored(hdc, row, &text, detail);\n    }\n    if settings.meter.show_consumables {\n        paint_raid_fs(hdc, state, Some(&row), r.right - 44, middle, 40, (r.bottom - 4 - middle).max(1));\n    }\n    SelectObject(hdc, primary);", "place F/S in player row");
    section(&mut src, "unsafe fn paint_raid_player(", "mod white_header_icons {",
        "                    bottom: r.bottom - 3,\n                    ..r\n                },\n                DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,",
        "                    top: r.top,\n                    bottom: if settings.meter.show_consumables { middle + 2 } else { r.bottom - 3 },\n                    ..r\n                },\n                DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,", "keep numeric metrics above F/S");

    // Hit regions use same second-line coordinates as painting and exclude gutter.
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
    section(&mut src, "unsafe fn paint_raid_fs(", "fn normal_meter_fs_width(",
        "let half=(w/2).max(1);", "let half=((w-4)/2).max(1);", "status cell gap");
    section(&mut src, "unsafe fn paint_raid_fs(", "fn normal_meter_fs_width(",
        "left:x+half,top:y,right:x+w", "left:x+half+4,top:y,right:x+w", "serum offset");
    fs::write(path, src).expect("write raid ownership fixes");
    println!("cargo:rerun-if-changed=build/legacy/build_v1349_audit_raid_consumable_ownership.rs");
}
