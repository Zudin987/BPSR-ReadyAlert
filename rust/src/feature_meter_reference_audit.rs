// Follow-up visual audit. Rendering only: captured values and sorting stay intact.
fn reference_meter_progress_color(mode: SortMode) -> u32 {
    match mode {
        SortMode::Damage => rgb(207, 91, 109),
        SortMode::Heal | SortMode::Tank => {
            crate::ui_modern::mix_color(crate::ui_modern::DARK_RAISED, mode_color(mode), 55)
        }
    }
}

// Raid uses two independent player columns and a central consumables gutter.
// Never reuse the full-width normal-meter headings for these player rows.
unsafe fn paint_reference_raid_headers(
    hdc: HDC,
    header: RECT,
    state: &State,
    settings: &FeatureSettings,
    scale: i32,
) {
    let width = header.right + 8;
    let mid = width / 2;
    let gap = if settings.meter.show_consumables {
        dps_adaptive_logical_px(RAID_CENTER_GAP, scale).max(56)
    } else {
        0
    };
    let columns = [
        RECT {
            left: 6,
            right: mid - gap / 2 - 4,
            ..header
        },
        RECT {
            left: mid + gap / 2 + 4,
            right: width - 8,
            ..header
        },
    ];
    for col in columns {
        let layout = dps_row_layout_responsive(
            hdc,
            col,
            scale,
            false,
            settings.meter.show_active_rates,
            dps_mode_share_enabled(settings, state.sort_mode),
            settings.meter.show_deaths,
            &DpsRow::default(),
            dps_primary_font(state),
            dps_secondary_font(state),
        );
        SetTextColor(hdc, crate::ui_modern::BPSR_MUTED);
        for (label, left, right, flags) in [
            ("#", col.left + 4, col.left + 24, 0),
            ("Player", layout.name_left, layout.spec_right, 0),
            ("Total", layout.total_left, layout.total_right, DT_RIGHT),
            (
                match state.sort_mode {
                    SortMode::Damage => "DPS",
                    SortMode::Heal => "HPS",
                    SortMode::Tank => "DTPS",
                },
                layout.active_left,
                layout.active_right,
                DT_RIGHT,
            ),
            ("%", layout.share_left, layout.share_right, DT_RIGHT),
            ("D", layout.death_left, layout.death_right, DT_RIGHT),
        ] {
            if right > left {
                draw(
                    hdc,
                    label,
                    RECT {
                        left,
                        right,
                        ..header
                    },
                    flags | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
                );
            }
        }
    }
    if settings.meter.show_consumables {
        let pair_w = ((gap / 2) - 8).max(24);
        for x in [mid - gap / 2 + 2, mid + 6] {
            let half = (pair_w / 2).max(1);
            draw(
                hdc,
                "F",
                RECT {
                    left: x,
                    right: x + half,
                    ..header
                },
                DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
            );
            draw(
                hdc,
                "S",
                RECT {
                    left: x + half,
                    right: x + pair_w,
                    ..header
                },
                DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
            );
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
            assert!(
                logical >= dps_rows_top() + 2 * dps_row_h(scale),
                "{scale}% should fit two complete player rows"
            );
        }
    }

    #[test]
    fn progress_bar_retains_distinct_damage_heal_tank_colours() {
        assert_eq!(
            reference_meter_progress_color(SortMode::Damage),
            rgb(207, 91, 109)
        );
        assert_ne!(
            reference_meter_progress_color(SortMode::Damage),
            reference_meter_progress_color(SortMode::Heal)
        );
        assert_ne!(
            reference_meter_progress_color(SortMode::Heal),
            reference_meter_progress_color(SortMode::Tank)
        );
    }

    #[test]
    fn fs_hit_rects_align_with_normal_meter_painter() {
        for scale in [50, 75, 100, 150, 200] {
            let row = RECT {
                left: 6,
                top: dps_rows_top(),
                right: 620,
                bottom: dps_rows_top() + dps_row_h(scale) - 2,
            };
            let fs = normal_meter_fs_rect(row, scale);
            let half = ((fs.right - fs.left) / 2).max(1);
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
            let badge_y = ((row_h - 2 - badge_h) / 2).max(0);
            assert!(badge_h > 0 && badge_y + badge_h <= row_h - 2);
        }
    }
}
