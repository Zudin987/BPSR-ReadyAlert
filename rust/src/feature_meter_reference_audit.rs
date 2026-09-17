// Follow-up visual audit. Rendering only: captured values and sorting stay intact.
fn reference_meter_progress_color(mode: SortMode) -> u32 {
    match mode {
        SortMode::Damage => rgb(207, 91, 109),
        SortMode::Heal | SortMode::Tank => crate::ui_modern::mix_color(
            crate::ui_modern::DARK_RAISED, mode_color(mode), 55),
    }
}

// Raid uses two independent player columns and a central consumables gutter.
// Never reuse the full-width normal-meter headings for these player rows.
unsafe fn paint_reference_raid_headers(
    hdc: HDC, header: RECT, state: &State, settings: &FeatureSettings, scale: i32,
) {
    fill(hdc, &header, crate::ui_modern::DARK_BG);
    let width = header.right + 8;
    let mid = width / 2;
    let gap = if settings.meter.show_consumables {
        dps_adaptive_logical_px(RAID_CENTER_GAP, scale).max(56)
    } else { 0 };
    let columns = [
        RECT { left: 6, top: header.top, right: (mid - gap/2 - 4).max(160), bottom: header.bottom },
        RECT { left: (mid + gap/2 + 4).min(width - 160), top: header.top,
            right: width - 8, bottom: header.bottom },
    ];
    let share_w = if dps_mode_share_enabled(settings, state.sort_mode) {
        dps_adaptive_logical_px(48, scale)
    } else { 0 };
    let death_w = if settings.meter.show_deaths { dps_adaptive_logical_px(24, scale) } else { 0 };
    let total_w = dps_adaptive_logical_px(66, scale);
    let rate_w = dps_adaptive_logical_px(58, scale);
    let metric_gap = dps_adaptive_logical_px(1, scale).max(1);
    SetTextColor(hdc, crate::ui_modern::BPSR_MUTED);
    for col in columns {
        let mut end = col.right - 4;
        let death_left = end - death_w;
        if death_w > 0 { end = death_left - 2; }
        let share_left = end - share_w;
        if share_w > 0 { end = share_left - 2; }
        let rate_left = end - rate_w;
        let total_right = rate_left - metric_gap;
        let total_left = total_right - total_w;
        draw(hdc, "#", RECT { left: col.left + 3, top: col.top,
            right: col.left + 22, bottom: col.bottom }, DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        draw(hdc, "PLAYER", RECT { left: col.left + dps_adaptive_logical_px(24, scale),
            top: col.top, right: (total_left - 4).max(col.left + 24), bottom: col.bottom },
            DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS);
        draw(hdc, "TOTAL", RECT { left: total_left, top: col.top,
            right: total_right, bottom: col.bottom }, DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        if settings.meter.show_active_rates {
            draw(hdc, "ACT/s", RECT { left: rate_left, top: col.top,
                right: rate_left + rate_w, bottom: col.bottom },
                DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        }
        if share_w > 0 {
            draw(hdc, "%", RECT { left: share_left, top: col.top,
                right: share_left + share_w, bottom: col.bottom },
                DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        }
        if death_w > 0 {
            draw(hdc, "D", RECT { left: death_left, top: col.top,
                right: death_left + death_w, bottom: col.bottom },
                DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        }
    }
    if settings.meter.show_consumables {
        let pair_w = ((gap/2) - 8).max(24);
        let left_x = mid - gap/2 + 2;
        let right_x = mid + 6;
        for x in [left_x, right_x] {
            let half = (pair_w/2).max(1);
            draw(hdc, "F", RECT { left: x, top: header.top,
                right: x + half, bottom: header.bottom },
                DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
            draw(hdc, "S", RECT { left: x + half, top: header.top,
                right: x + pair_w, bottom: header.bottom },
                DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        }
    }
}

#[cfg(test)]
mod reference_meter_audit_tests {
    use super::*;

    #[test]
    fn minimum_height_has_two_complete_normal_rows_at_all_supported_scales() {
        for scale in [50, 65, 75, 100, 125, 150, 200, 300] {
            let physical = overlay_min_height(Kind::Dps, scale);
            let logical = physical_extent_to_logical(physical, scale);
            assert!(logical >= dps_rows_top() + 2*dps_row_h(scale),
                "{scale}% should fit two complete player rows");
        }
    }

    #[test]
    fn progress_bar_retains_distinct_damage_heal_tank_colours() {
        assert_eq!(reference_meter_progress_color(SortMode::Damage), rgb(207,91,109));
        assert_ne!(reference_meter_progress_color(SortMode::Damage),
            reference_meter_progress_color(SortMode::Heal));
        assert_ne!(reference_meter_progress_color(SortMode::Heal),
            reference_meter_progress_color(SortMode::Tank));
    }

    #[test]
    fn fs_hit_rects_align_with_normal_meter_painter() {
        for scale in [50, 75, 100, 150, 200] {
            let row = RECT { left: 6, top: dps_rows_top(), right: 620,
                bottom: dps_rows_top() + dps_row_h(scale) - 2 };
            let fs = normal_meter_fs_rect(row, scale);
            let half = ((fs.right-fs.left)/2).max(1);
            assert!(fs.left + half < fs.right);
            assert_eq!(fs.top, row.top);
            assert_eq!(fs.bottom, row.bottom - 4);
            assert!(fs.right <= row.right);
        }
    }

    #[test]
    fn imagine_icons_fit_inside_normal_rows_at_supported_scales() {
        for scale in [50, 75, 100, 150, 200, 300] {
            let row_h = dps_row_h(scale);
            let badge_h = dps_badge_h(scale);
            let badge_y = ((row_h - 2 - badge_h)/2).max(0);
            assert!(badge_h > 0 && badge_y + badge_h <= row_h - 2);
        }
    }
}
