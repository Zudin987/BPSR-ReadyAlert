use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1355_raid_small_scale_spacing.rs");
    pub fn run() { main(); }
}

fn once(src: &mut String, old: &str, new: &str, label: &str) {
    let count = src.matches(old).count();
    assert_eq!(count, 1, "death/owner {label}: expected one anchor, got {count}");
    *src = src.replacen(old, new, 1);
}
fn section(src: &mut String, start: &str, end: &str, old: &str, new: &str, label: &str) {
    let from = src.find(start).expect("death/owner section start");
    let to = from + src[from..].find(end).expect("death/owner section end");
    let mut body = src[from..to].to_owned();
    once(&mut body, old, new, label);
    src.replace_range(from..to, &body);
}

fn main() {
    previous::run();
    let path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"))
        .join("feature_overlays_v170_fixed.rs");
    let mut src = fs::read_to_string(&path).expect("generated overlay source");

    // Do not append an identity label to the real player name: the cyan rank
    // and the distinctive open-corner frame identify the local player.
    once(&mut src,
        "if row.is_local { format!(\"{} · You\", row.name) } else { row.name.clone() }",
        "row.name.clone()", "local name without suffix");
    once(&mut src,
        "assert_eq!(reference_player_display_name(&local), \"Alice · You\");",
        "assert_eq!(reference_player_display_name(&local), \"Alice\");",
        "identity regression expectation");

    // Existing timer formatter is the source of truth. A negative/unavailable
    // revive timestamp must not be represented as a ready-to-revive state.
    once(&mut src,
        "fn compact_dead_status(row:&DpsRow,now:i64)->(String,u32){if row.revive_blocked_until_ms<0{return(\"DEAD\".into(),crate::ui_modern::BPSR_DANGER);}if row.revive_blocked_until_ms>now{return(\"REVIVE\".into(),rgb(255,190,70));}(\"CAN REVIVE\".into(),rgb(72,226,116))}",
        "fn compact_dead_status(row:&DpsRow,now:i64)->(String,u32){revive_status_text(row,now)}",
        "compact revive timer and state color");
    once(&mut src,
        "unsafe fn paint_pinned_self_frame(hdc:HDC,r:&RECT){crate::ui_modern::stroke_round_rect(hdc,*r,rgb(104,224,183),crate::ui_modern::RADIUS_SMALL,1);}",
        "fn reference_owner_rank_color(row:&DpsRow)->u32{if row.is_local{rgb(92,200,255)}else{crate::ui_modern::BPSR_MUTED}}\nunsafe fn paint_pinned_self_frame(hdc:HDC,r:&RECT){let color=rgb(92,200,255);fill(hdc,&RECT{left:r.left+1,top:r.top,right:r.right,bottom:r.top+1},color);fill(hdc,&RECT{left:r.right-1,top:r.top,right:r.right,bottom:r.bottom},color);}",
        "open sharp owner frame and cyan rank helper");
    once(&mut src,
        "fn reference_meter_row_background(row: &DpsRow) -> u32 {\n    let original = spec_color(row);",
        "fn reference_meter_row_background(row: &DpsRow) -> u32 {\n    if row.is_local && row.is_dead { return rgb(58, 27, 34); }\n    let original = spec_color(row);",
        "subtle red dead-owner surface");

    // On dead rows, reserve a dedicated status cell before allocating metrics.
    // First protect the name and revive label, then Total, and only then the
    // optional active rate. Do not spend the status budget on imagines/class,
    // share or death-count columns. Live and header layout are unaffected.
    section(&mut src,
        "unsafe fn dps_row_layout_responsive(", "unsafe fn paint_dps_secondary_colored(",
        "    let available = (r.right - 4 - name_left).max(0);",
        r#"    let available = (r.right - 4 - name_left).max(0);
    if _row.is_dead {
        let name_min = dps_adaptive_logical_px(72, scale);
        let status_w = dps_adaptive_logical_px(98, scale);
        let reserved = name_min + gap + status_w + gap;
        let total_w = if available >= reserved + metric_w + gap { metric_w } else { 0 };
        let active_w = if show_active && available >= reserved + total_w
            + if total_w > 0 { gap } else { 0 } + metric_w + gap { metric_w } else { 0 };
        let mut cursor = r.right - 4;
        let (active_left, active_right) = dps_take_column(&mut cursor, active_w, gap);
        let (total_left, total_right) = dps_take_column(&mut cursor, total_w, gap);
        let spec_right = cursor;
        let spec_left = (spec_right - status_w).max(name_left + name_min + gap);
        return DpsRowLayout {
            name_left, name_right: (spec_left - gap).max(name_left),
            spec_left, spec_right, badge_left: 0, badge_count: 0,
            total_left, total_right, active_left, active_right,
            share_left: 0, share_right: 0, death_left: 0, death_right: 0,
        };
    }"#,
        "dead status first in shared geometry");

    section(&mut src,
        "unsafe fn paint_reference_normal_rows(", "unsafe fn paint_compact_player(",
        "        SetTextColor(hdc, crate::ui_modern::BPSR_MUTED);\n        draw(\n            hdc,\n            &rank.to_string(),",
        "        SetTextColor(hdc, reference_owner_rank_color(row));\n        draw(\n            hdc,\n            &rank.to_string(),",
        "normal cyan local rank");
    section(&mut src,
        "unsafe fn paint_reference_normal_rows(", "unsafe fn paint_compact_player(",
        "        SetTextColor(hdc, base);\n        draw(\n            hdc,\n            &reference_player_display_name(row),",
        "        SetTextColor(hdc, if row.is_local && row.is_dead { rgb(255,107,107) } else { base });\n        draw(\n            hdc,\n            &reference_player_display_name(row),",
        "normal dead owner red name");
    section(&mut src,
        "unsafe fn paint_reference_normal_rows(", "unsafe fn paint_compact_player(",
        r#"        let secondary = if row.is_dead {
            let revive = revive_status_text(row, now).0;
            let spec = dps_spec(row);
            if spec.is_empty() {
                format!("DEAD · {revive}")
            } else {
                format!("{spec} · DEAD · {revive}")
            }
        } else {
            dps_secondary_compact_measured(hdc, row, (layout.spec_right - layout.spec_left).max(0))
        };
        paint_dps_secondary_colored(
            hdc,
            row,
            &secondary,
            RECT {
                left: layout.spec_left,
                top: r.top,
                right: layout.spec_right,
                bottom: content_bottom,
            },
        );"#,
        r#"        let secondary_rect = RECT {
            left: layout.spec_left, top: r.top,
            right: layout.spec_right, bottom: content_bottom,
        };
        if row.is_dead {
            let (label, color) = revive_status_text(row, now);
            SetTextColor(hdc, color);
            draw(hdc, &label, secondary_rect,
                DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS);
        } else {
            let secondary = dps_secondary_compact_measured(hdc, row,
                (layout.spec_right - layout.spec_left).max(0));
            paint_dps_secondary_colored(hdc, row, &secondary, secondary_rect);
        }"#,
        "normal dedicated high-priority revive cell");

    // Compact and Compact Raid use the same painter. Keep status visible even
    // at their minimum widths; hide Active DPS first, then Total as space runs
    // out, instead of silently removing the revive label.
    section(&mut src,
        "unsafe fn paint_compact_player(", "unsafe fn paint_compact_raid_rows(",
        r#"    if row.is_local {
        crate::ui_modern::stroke_round_rect(
            hdc,
            r,
            rgb(104, 224, 183),
            crate::ui_modern::RADIUS_SMALL,
            1,
        );
    }
"#,
        "", "remove duplicate rounded compact owner outline");
    section(&mut src,
        "unsafe fn paint_compact_player(", "unsafe fn paint_compact_raid_rows(",
        r#"    let rank_w = dps_adaptive_logical_px(24, scale);
    let rate_w = dps_adaptive_logical_px(64, scale);
    let total_w = if show_total {
        dps_adaptive_logical_px(70, scale)
    } else {
        0
    };
    let status_w = reference_compact_status_width(r.right - r.left, scale, total_w > 0, row.is_dead);
    let gap = dps_adaptive_logical_px(3, scale);"#,
        r#"    let rank_w = dps_adaptive_logical_px(24, scale);
    let gap = dps_adaptive_logical_px(3, scale);
    let status_w = reference_compact_status_width(r.right - r.left, scale, show_total, row.is_dead);
    let rate_nominal = dps_adaptive_logical_px(64, scale);
    let total_nominal = if show_total { dps_adaptive_logical_px(70, scale) } else { 0 };
    let metric_budget = r.right - r.left - 8 - rank_w - gap - status_w
        - if row.is_dead { gap + dps_adaptive_logical_px(72, scale) } else { 0 };
    let total_w = if !row.is_dead || (show_total && metric_budget >= total_nominal + gap) {
        total_nominal
    } else { 0 };
    let rate_w = if !row.is_dead || metric_budget >= total_w
        + if total_w > 0 { gap } else { 0 } + rate_nominal + gap {
        rate_nominal
    } else { 0 };"#,
        "compact status budget and metric priority");
    section(&mut src,
        "unsafe fn paint_compact_player(", "unsafe fn paint_compact_raid_rows(",
        "    let total_right = rate_left - gap;",
        "    let total_right = rate_left - if rate_w > 0 { gap } else { 0 };",
        "compact absent-rate geometry");
    section(&mut src,
        "unsafe fn paint_compact_player(", "unsafe fn paint_compact_raid_rows(",
        "    let identity_right = (if total_w > 0 { total_left } else { rate_left }) - gap;",
        "    let identity_right = (if total_w > 0 { total_left } else if rate_w > 0 { rate_left } else { right }) - gap;",
        "compact status positioning");
    section(&mut src,
        "unsafe fn paint_compact_player(", "unsafe fn paint_compact_raid_rows(",
        "    SetTextColor(hdc, crate::ui_modern::BPSR_MUTED);\n    draw(\n        hdc,\n        &rank.to_string(),",
        "    SetTextColor(hdc, reference_owner_rank_color(row));\n    draw(\n        hdc,\n        &rank.to_string(),",
        "compact cyan local rank");
    section(&mut src,
        "unsafe fn paint_compact_player(", "unsafe fn paint_compact_raid_rows(",
        "        if row.is_dead {\n            rgb(255, 135, 135)",
        "        if row.is_local && row.is_dead {\n            rgb(255, 107, 107)\n        } else if row.is_dead {\n            rgb(255, 135, 135)",
        "compact dead owner red name");
    section(&mut src,
        "unsafe fn paint_compact_player(", "unsafe fn paint_compact_raid_rows(",
        "    draw(\n        hdc,\n        &compact(rate),",
        "    if rate_w > 0 { draw(\n        hdc,\n        &compact(rate),",
        "compact omit rate when status needs space");
    section(&mut src,
        "unsafe fn paint_compact_player(", "unsafe fn paint_compact_raid_rows(",
        "        DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,\n    );\n    if total_w > 0 {",
        "        DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,\n    ); }\n    if total_w > 0 {",
        "compact close optional rate draw");

    once(&mut src,
        "fn reference_compact_status_width(width: i32, scale: i32, show_total: bool, dead: bool) -> i32 {\n    if !dead { return 0; }\n    let gap = dps_adaptive_logical_px(3, scale);\n    let status = dps_adaptive_logical_px(94, scale);\n    let reserve = 4 + dps_adaptive_logical_px(24, scale)\n        + dps_adaptive_logical_px(64, scale) + gap\n        + (if show_total { dps_adaptive_logical_px(70, scale) + gap } else { 0 })\n        + status + gap + dps_adaptive_logical_px(88, scale);\n    if width >= reserve { status } else { 0 }\n}",
        "fn reference_compact_status_width(_width: i32, scale: i32, _show_total: bool, dead: bool) -> i32 {\n    if dead { dps_adaptive_logical_px(94, scale) } else { 0 }\n}",
        "compact revive status never silently hidden");
    once(&mut src,
        "assert_eq!(reference_compact_status_width(286, 100, true, true), 0);",
        "assert_eq!(reference_compact_status_width(286, 100, true, true), 94);",
        "compact regression expectation");

    section(&mut src,
        "unsafe fn paint_raid_player(", "mod white_header_icons",
        "    SetTextColor(hdc, crate::ui_modern::BPSR_MUTED);\n    draw(\n        hdc,\n        &rank.to_string(),",
        "    SetTextColor(hdc, reference_owner_rank_color(row));\n    draw(\n        hdc,\n        &rank.to_string(),",
        "raid cyan local rank");
    section(&mut src,
        "unsafe fn paint_raid_player(", "mod white_header_icons",
        "    SetTextColor(hdc, rgb(241, 243, 247));\n    let middle =",
        "    SetTextColor(hdc, if row.is_local && row.is_dead { rgb(255,107,107) } else { rgb(241,243,247) });\n    let middle =",
        "raid dead owner red name");
    section(&mut src,
        "unsafe fn paint_raid_player(", "mod white_header_icons",
        "        right: layout.spec_right,\n        bottom: middle + 2,",
        "        right: if row.is_dead { layout.name_right } else { layout.spec_right },\n        bottom: middle + 2,",
        "raid name must not consume revive slot");
    section(&mut src,
        "unsafe fn paint_raid_player(", "mod white_header_icons",
        "        top: middle,\n        bottom: r.bottom - 4,\n        right: if settings.meter.show_consumables { identity.right.min(r.right - 48) } else { identity.right },\n        ..identity",
        "        left: if row.is_dead { layout.spec_left } else { identity.left },\n        top: middle,\n        bottom: r.bottom - 4,\n        right: if row.is_dead { layout.spec_right } else if settings.meter.show_consumables { identity.right.min(r.right - 48) } else { identity.right },\n        ..identity",
        "raid dedicated revive slot");
    section(&mut src,
        "unsafe fn paint_raid_player(", "mod white_header_icons",
        "    if settings.meter.show_consumables {\n        paint_raid_fs(",
        "    if settings.meter.show_consumables && !row.is_dead {\n        paint_raid_fs(",
        "raid consumables give way to urgent revive status");

    src.push_str(r#"
#[cfg(test)]
mod death_priority_owner_frame_tests {
    use super::*;
    #[test]
    fn local_rank_and_dead_surface_are_distinct() {
        let local = DpsRow { is_local:true, ..Default::default() };
        let dead = DpsRow { is_local:true, is_dead:true, ..Default::default() };
        assert_eq!(reference_owner_rank_color(&local), rgb(92,200,255));
        assert_eq!(reference_owner_rank_color(&DpsRow::default()), crate::ui_modern::BPSR_MUTED);
        assert_eq!(reference_meter_row_background(&dead), rgb(58,27,34));
        assert_eq!(reference_player_display_name(&dead), dead.name);
    }
    #[test]
    fn revive_timer_and_ready_state_have_distinct_colors_and_labels() {
        let mut row = DpsRow { is_dead:true, revive_blocked_until_ms: 15_000, ..Default::default() };
        let (waiting, amber) = compact_dead_status(&row, 0);
        assert!(waiting.contains("15s"), "timer omitted: {waiting}");
        assert_eq!(amber, rgb(255,190,70));
        row.revive_blocked_until_ms = 0;
        let (ready, green) = compact_dead_status(&row, 0);
        assert_eq!(ready, "CAN REVIVE");
        assert_eq!(green, rgb(72,226,116));
        assert_eq!(reference_compact_status_width(240,100,true,true),94);
    }
}
"#);
    fs::write(path, src).expect("write death/owner priority patch");
    println!("cargo:rerun-if-changed=build/legacy/build_v1356_death_priority_owner_frame.rs");
}
