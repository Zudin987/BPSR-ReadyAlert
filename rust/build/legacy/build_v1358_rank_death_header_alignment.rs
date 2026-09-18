use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1357_narrow_revive_labels.rs");
    pub fn run() { main(); }
}

fn once(src: &mut String, old: &str, new: &str, label: &str) {
    let count = src.matches(old).count();
    assert_eq!(count, 1, "row alignment {label}: expected one anchor, found {count}");
    *src = src.replacen(old, new, 1);
}

fn section(src: &mut String, start: &str, end: &str, old: &str, new: &str, label: &str) {
    let a = src.find(start).expect("row alignment section start");
    let b = a + src[a..].find(end).expect("row alignment section end");
    let mut fragment = src[a..b].to_owned();
    once(&mut fragment, old, new, label);
    src.replace_range(a..b, &fragment);
}

fn main() {
    previous::run();
    let path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"))
        .join("feature_overlays_v170_fixed.rs");
    let mut src = fs::read_to_string(&path).expect("generated overlay source");

    // The primary row's text and its rank/death count must use the same
    // font metrics; secondary fonts place numerals at a different baseline.
    section(&mut src, "unsafe fn paint_reference_normal_rows(", "unsafe fn paint_compact_player(",
        "SelectObject(hdc, secondary_font);\n        SetTextColor(hdc, reference_owner_rank_color(row));",
        "SelectObject(hdc, primary_font);\n        SetTextColor(hdc, reference_owner_rank_color(row));",
        "Normal rank baseline");
    section(&mut src, "unsafe fn paint_reference_normal_rows(", "unsafe fn paint_compact_player(",
        "            SelectObject(hdc, secondary_font);\n            SetTextColor(",
        "            SelectObject(hdc, primary_font);\n            SetTextColor(",
        "Normal death baseline");
    section(&mut src, "unsafe fn paint_compact_player(", "unsafe fn paint_compact_raid_rows(",
        "SelectObject(hdc, dps_secondary_font(state));\n    SetTextColor(hdc, reference_owner_rank_color(row));",
        "SelectObject(hdc, dps_primary_font(state));\n    SetTextColor(hdc, reference_owner_rank_color(row));",
        "Compact and Compact Raid rank baseline");

    // Raid cards have two lines. Rank and death count belong on the first
    // line with the player's name and damage numbers, not mid-card between
    // the name and revive/class detail. Share the exact first-line rect.
    once(&mut src, "unsafe fn paint_raid_player(",
        "fn reference_raid_first_line(r: RECT) -> RECT {\n    let middle = r.top + (r.bottom - r.top - 3) / 2;\n    RECT { top: r.top + 1, bottom: middle + 2, ..r }\n}\n\nunsafe fn paint_raid_player(",
        "shared Raid first-line geometry");
    section(&mut src, "unsafe fn paint_raid_player(", "mod white_header_icons",
        "SelectObject(hdc, secondary);\n    SetTextColor(hdc, reference_owner_rank_color(row));",
        "SelectObject(hdc, primary);\n    SetTextColor(hdc, reference_owner_rank_color(row));",
        "Raid rank font baseline");
    section(&mut src, "unsafe fn paint_raid_player(", "mod white_header_icons",
        "        RECT {\n            left: r.left + 4,\n            right: r.left + dps_adaptive_logical_px(24, scale),\n            bottom: r.bottom - 3,\n            ..r\n        },",
        "        RECT {\n            left: r.left + 4,\n            right: r.left + dps_adaptive_logical_px(24, scale),\n            ..reference_raid_first_line(r)\n        },",
        "Raid rank first-line alignment");
    section(&mut src, "unsafe fn paint_raid_player(", "mod white_header_icons",
        "            RECT {\n                left: layout.death_left,\n                right: layout.death_right,\n                bottom: r.bottom - 3,\n                ..r\n            },",
        "            RECT {\n                left: layout.death_left,\n                right: layout.death_right,\n                ..reference_raid_first_line(r)\n            },",
        "Raid death first-line alignment");

    // Empty death and Food/Serum header captions added clutter. Preserve
    // actual death counts and food/serum values in the player rows.
    section(&mut src, "unsafe fn paint_reference_raid_headers(", "#[cfg(test)]\nmod reference_meter_audit_tests",
        "            (\"D\", layout.death_left, layout.death_right, DT_RIGHT),\n", "",
        "Raid death heading");
    section(&mut src, "unsafe fn paint_reference_headers(", "unsafe fn paint_dps(",
        "        (\"D\", layout.death_left, layout.death_right, DT_RIGHT),\n", "",
        "Normal death heading");
    section(&mut src, "unsafe fn paint_reference_headers(", "unsafe fn paint_dps(",
        "(\"Class · HR\", layout.spec_left, layout.spec_right, 0),",
        "(\"Class\", layout.spec_left, layout.spec_right, 0),",
        "Normal class heading");
    section(&mut src, "unsafe fn paint_reference_headers(", "unsafe fn paint_dps(",
        "    if settings.meter.show_consumables {\n        let fs = normal_meter_fs_rect(hr, scale);\n        let half = (fs.right - fs.left) / 2;\n        for (label, left, right) in [\n            (\"F\", fs.left, fs.left + half),\n            (\"S\", fs.left + half, fs.right),\n        ] {\n            draw(\n                hdc,\n                label,\n                RECT { left, right, ..hr },\n                DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,\n            );\n        }\n    }\n", "",
        "Food/Serum headings only");

    src.push_str(r#"
#[cfg(test)]
mod audit_rank_death_alignment_tests {
    use super::*;
    #[test]
    fn raid_rank_and_death_share_name_line_in_two_line_rows() {
        for height in [30, 44, 48, 60] {
            let row = RECT { left: 6, top: 20, right: 310, bottom: 20 + height };
            let line = reference_raid_first_line(row);
            let name_middle = row.top + (row.bottom - row.top - 3) / 2;
            assert_eq!(line.top, row.top + 1);
            assert_eq!(line.bottom, name_middle + 2);
            assert!(line.bottom < row.bottom);
        }
    }
}
"#);
    fs::write(path, src).expect("write row alignment and simplified headings");
    println!("cargo:rerun-if-changed=build/legacy/build_v1358_rank_death_header_alignment.rs");
}
