// Presentation helpers appended to the generated overlay after the v1.33.9
// layout transform. Values derive from captured encounter snapshots only.
fn reference_meter_rate_label(mode: SortMode) -> &'static str {
    match mode {
        SortMode::Damage => "ACTIVE DPS",
        SortMode::Heal => "ACTIVE HPS",
        SortMode::Tank => "ACTIVE DTPS",
    }
}

unsafe fn paint_reference_target_total(hdc: HDC, target: RECT, snapshot: &DpsSnapshot, mode: SortMode) {
    let label = match mode {
        SortMode::Damage => "Total Damage",
        SortMode::Heal => "Total Healing",
        SortMode::Tank => "Total Taken",
    };
    let bottom = RECT { left: (target.right - 250).max(target.left + 8), top: target.top + 22,
        right: target.right - 8, bottom: target.bottom - 2 };
    let value_left = bottom.right - 95;
    SetTextColor(hdc, crate::ui_modern::BPSR_MUTED);
    draw(hdc, label, RECT { right: value_left - 6, ..bottom }, DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
    SetTextColor(hdc, crate::ui_modern::BPSR_TEXT);
    draw(hdc, &compact(snapshot_mode_total(snapshot, mode) as f64),
        RECT { left: value_left, ..bottom }, DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
}

unsafe fn paint_reference_column_headers(hdc: HDC, header: RECT, layout: &DpsRowLayout,
    tier: DpsLayoutTier, scale: i32, show_imagines: bool) {
    if !matches!(tier, DpsLayoutTier::Comfortable) || header.right - header.left < 760 { return; }
    let badge_w = if show_imagines { 2 * dps_badge_w(scale) + dps_badge_gap(scale) } else { 0 };
    let imagine_left = layout.total_left - badge_w - 7;
    let class_left = header.left + ((imagine_left - header.left).max(0) * 48 / 100);
    SetTextColor(hdc, crate::ui_modern::BPSR_MUTED);
    draw(hdc, "CLASS · HR", RECT { left: class_left, top: header.top,
        right: imagine_left - 8, bottom: header.bottom }, DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS);
    if show_imagines {
        draw(hdc, "IMAGINES", RECT { left: imagine_left, top: header.top,
            right: layout.total_left - 4, bottom: header.bottom }, DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS);
    }
}

#[cfg(test)]
mod reference_meter_layout_tests {
    use super::*;

    #[test]
    fn every_mode_has_an_honest_rate_heading() {
        assert_eq!(reference_meter_rate_label(SortMode::Damage), "ACTIVE DPS");
        assert_eq!(reference_meter_rate_label(SortMode::Heal), "ACTIVE HPS");
        assert_eq!(reference_meter_rate_label(SortMode::Tank), "ACTIVE DTPS");
    }

    #[test]
    fn spacious_normal_rows_preserve_compact_mode() {
        assert_eq!(DPS_ROW_H, 40);
        assert!(dps_compact_row_h(100) < DPS_ROW_H);
        assert_eq!(dps_rows_top(), TOOLBAR_H + DPS_CONTROL_H + 3);
    }
}
