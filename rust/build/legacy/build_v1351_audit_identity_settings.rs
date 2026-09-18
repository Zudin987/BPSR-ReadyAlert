use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1350_audit_small_scale_matrix.rs");
    pub fn run() { main(); }
}

fn once(src: &mut String, old: &str, new: &str, what: &str) {
    let n = src.matches(old).count();
    assert_eq!(n, 1, "audit identity/settings {what}: expected one anchor, found {n}");
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

    // R03: use the requested native family for the DPS role fonts. Windows
    // continues to provide normal platform fallback for missing script glyphs.
    section(
        &mut src,
        "unsafe fn dps_cached_font(",
        "unsafe fn paint_raid_rows(",
        "let face = wide(\"Segoe UI\");",
        "let face = wide(\"Segoe UI Variable Text\");",
        "DPS font family",
    );

    // R06: local identity must not depend on colour alone. Do not call the
    // sticky/self visibility behaviour 'Pinned' because it is not a user pin.
    let normal_anchor = "unsafe fn paint_reference_normal_rows(hdc: HDC, rc: RECT, state: &State) {";
    let helper = r#"fn reference_player_display_name(row: &DpsRow) -> String {
    if row.is_local { format!("{} · You", row.name) } else { row.name.clone() }
}

unsafe fn paint_reference_normal_rows(hdc: HDC, rc: RECT, state: &State) {"#;
    once(&mut src, normal_anchor, helper, "local You helper");

    // R05: ranks are a neutral, right-aligned fixed cell in all layouts.
    section(
        &mut src,
        "unsafe fn paint_reference_normal_rows(",
        "unsafe fn paint_compact_player(",
        r#"        SelectObject(
            hdc,
            if row.is_local {
                primary_font
            } else {
                secondary_font
            },
        );
        SetTextColor(
            hdc,
            if row.is_local {
                rgb(225, 169, 129)
            } else {
                crate::ui_modern::BPSR_TEXT_SECONDARY
            },
        );
        draw(
            hdc,
            &rank.to_string(),
            RECT {
                left: r.left + 3,
                top: r.top,
                right: r.left + 21,
                bottom: content_bottom,
            },
            DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );"#,
        r#"        SelectObject(hdc, secondary_font);
        SetTextColor(hdc, crate::ui_modern::BPSR_MUTED);
        draw(
            hdc,
            &rank.to_string(),
            RECT {
                left: r.left + 3,
                top: r.top,
                right: r.left + 25,
                bottom: content_bottom,
            },
            DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );"#,
        "normal neutral rank",
    );
    section(
        &mut src,
        "unsafe fn paint_reference_normal_rows(",
        "unsafe fn paint_compact_player(",
        "            &row.name,",
        "            &reference_player_display_name(row),",
        "normal You label",
    );

    section(
        &mut src,
        "unsafe fn paint_compact_player(",
        "unsafe fn paint_compact_raid_rows(",
        r#"    SelectObject(
        hdc,
        if row.is_local {
            dps_primary_font(state)
        } else {
            dps_secondary_font(state)
        },
    );
    SetTextColor(
        hdc,
        if row.is_local {
            rgb(225, 169, 129)
        } else {
            crate::ui_modern::BPSR_TEXT_SECONDARY
        },
    );
    draw(
        hdc,
        &rank.to_string(),
        RECT {
            left: r.left + 3,
            top: r.top,
            right: r.left + rank_w,
            bottom: r.bottom,
        },
        DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
    );"#,
        r#"    SelectObject(hdc, dps_secondary_font(state));
    SetTextColor(hdc, crate::ui_modern::BPSR_MUTED);
    draw(
        hdc,
        &rank.to_string(),
        RECT {
            left: r.left + 3,
            top: r.top,
            right: r.left + rank_w,
            bottom: r.bottom,
        },
        DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
    );"#,
        "compact neutral rank",
    );
    section(
        &mut src,
        "unsafe fn paint_compact_player(",
        "unsafe fn paint_compact_raid_rows(",
        "        &row.name,",
        "        &reference_player_display_name(row),",
        "compact You label",
    );

    section(
        &mut src,
        "unsafe fn paint_raid_player(",
        "mod white_header_icons",
        "    SetTextColor(hdc, spec_accent_color(row));",
        "    SetTextColor(hdc, crate::ui_modern::BPSR_MUTED);",
        "raid neutral rank color",
    );
    section(
        &mut src,
        "unsafe fn paint_raid_player(",
        "mod white_header_icons",
        "        DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,",
        "        DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,",
        "raid rank alignment",
    );
    section(
        &mut src,
        "unsafe fn paint_raid_player(",
        "mod white_header_icons",
        "        &row.name,",
        "        &reference_player_display_name(row),",
        "raid You label",
    );

    // R07: the right-hand raid rate is active-time, so label it truthfully.
    section(
        &mut src,
        "unsafe fn paint_reference_raid_headers(",
        "#[cfg(test)]\nmod reference_meter_audit_tests",
        r#"                match state.sort_mode {
                    SortMode::Damage => "DPS",
                    SortMode::Heal => "HPS",
                    SortMode::Tank => "DTPS",
                },"#,
        r#"                match state.sort_mode {
                    SortMode::Damage => "Act. DPS",
                    SortMode::Heal => "Act. HPS",
                    SortMode::Tank => "Act. DTPS",
                },"#,
        "raid active-rate heading",
    );

    // R15: give the first settings group three aligned rows instead of one
    // crowded strip, and state the readability-floor behaviour explicitly.
    let old_form = r#"    let title=if state.kind==Kind::Dps{"DPS Meter"}else{"Dungeon Mechanics"};let title_hwnd=feature_label(hwnd,0,title,18,14,320,24);crate::ui_theme::set_font(title_hwnd,crate::ui_theme::FontRole::Heading);
    feature_label(hwnd,7000,"",18,50,84,22);feature_button(hwnd,7001,"−",108,44,30);feature_button(hwnd,7002,"+",142,44,30);
    feature_label(hwnd,7004,"",190,50,90,22);feature_button(hwnd,7005,"−",286,44,30);feature_button(hwnd,7006,"+",320,44,30);feature_button(hwnd,7008,"Reset 100%",354,44,88);
    feature_label(hwnd,0,"Collapse",458,50,60,22);feature_combo_sized(hwnd,7003,520,44,84,&COLLAPSE_EDGE_ITEMS);
    feature_scale_slider(hwnd,7007,190,74,252);SendMessageW(fc(hwnd,7007),0x0407,1,overlay_scale_min(state.kind)as isize);feature_label(hwnd,7009,"",458,74,146,22);
    if state.kind==Kind::Dps{
        let section_hwnd=feature_label(hwnd,0,"DISPLAY",18,100,180,18);crate::ui_theme::set_font(section_hwnd,crate::ui_theme::FontRole::Secondary);"#;
    let new_form = r#"    let title=if state.kind==Kind::Dps{"DPS Meter"}else{"Dungeon Mechanics"};let title_hwnd=feature_label(hwnd,0,title,20,16,320,24);crate::ui_theme::set_font(title_hwnd,crate::ui_theme::FontRole::Heading);
    feature_label(hwnd,7000,"",20,52,116,22);feature_button(hwnd,7001,"−",146,46,34);feature_button(hwnd,7002,"+",188,46,34);
    feature_label(hwnd,7004,"",20,94,116,22);feature_button(hwnd,7005,"−",146,88,34);feature_button(hwnd,7006,"+",188,88,34);feature_button(hwnd,7008,"Reset 100%",230,88,96);
    feature_scale_slider(hwnd,7007,338,94,214);SendMessageW(fc(hwnd,7007),0x0407,1,overlay_scale_min(state.kind)as isize);feature_label(hwnd,7009,"",560,94,126,22);
    feature_label(hwnd,0,"Collapse",20,136,116,22);feature_combo_sized(hwnd,7003,146,130,132,&COLLAPSE_EDGE_ITEMS);
    feature_label(hwnd,0,"Text and controls keep minimum readable sizes.",296,136,390,22);
    if state.kind==Kind::Dps{
        let section_hwnd=feature_label(hwnd,0,"DISPLAY",20,174,180,18);crate::ui_theme::set_font(section_hwnd,crate::ui_theme::FontRole::Secondary);"#;
    once(&mut src, old_form, new_form, "settings first group reflow");

    // Shift the remaining DPS controls down with the new three-row header.
    for (old, new) in [
        ("18+col*286,120+row*27", "20+col*286,196+row*27"),
        ("304,201,270", "306,277,270"),
        ("18,236,220,18", "20,312,220,18"),
        ("18+col*286,258+row*27", "20+col*286,334+row*27"),
        ("18,322,90,22", "20,398,90,22"),
        ("108,316,96", "110,392,96"),
        ("286,322,116,22", "288,398,116,22"),
        ("404,316,142", "406,392,142"),
        ("18,362,220,20", "20,438,250,20"),
        ("494,356,92", "594,432,92"),
    ] {
        once(&mut src, old, new, "settings DPS control shift");
    }
    once(
        &mut src,
        "(crate::ui_theme::DPS_SETTINGS_W,crate::ui_theme::DPS_SETTINGS_H+30)",
        "(crate::ui_theme::DPS_SETTINGS_W,crate::ui_theme::DPS_SETTINGS_H+110)",
        "settings window height",
    );

    // Regression checks for identity and terminology that do not depend on
    // screenshot OCR.
    src.push_str(r#"
#[cfg(test)]
mod audit_identity_typography_tests {
    use super::*;
    #[test]
    fn local_identity_has_text_marker_without_claiming_pin() {
        let local = DpsRow { name: "Alice".into(), is_local: true, ..Default::default() };
        let remote = DpsRow { name: "Bob".into(), ..Default::default() };
        assert_eq!(reference_player_display_name(&local), "Alice · You");
        assert_eq!(reference_player_display_name(&remote), "Bob");
        assert!(!reference_player_display_name(&local).contains("Pinned"));
    }
    #[test]
    fn active_rate_labels_are_explicit() {
        assert_eq!(reference_meter_rate_label(SortMode::Damage), "Active DPS");
        assert_eq!(reference_meter_rate_label(SortMode::Heal), "Active HPS");
        assert_eq!(reference_meter_rate_label(SortMode::Tank), "Active DTPS");
    }
}
"#);

    fs::write(path, src).expect("write audit identity/settings refinements");
    println!("cargo:rerun-if-changed=build/legacy/build_v1351_audit_identity_settings.rs");
}
