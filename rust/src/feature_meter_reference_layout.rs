// Native DPS presentation. Bound by the existing final reference audit stage.
// Telemetry, sorting, history and settings remain in the original implementation.

unsafe fn paint_reference_normal_rows(hdc: HDC, rc: RECT, state: &State) {
    let settings = state.features.read().map(|x| x.clone()).unwrap_or_default();
    let scale = dps_layout_scale(state);
    let primary_font = dps_primary_font(state);
    let secondary_font = dps_secondary_font(state);
    let old_font = SelectObject(hdc, primary_font);
    let show_share = dps_mode_share_enabled(&settings, state.sort_mode);
    let now = now_ms();
    let rows = meter_rows(state);
    let top = dps_rows_top();
    let row_h = dps_row_h(scale);
    let visible = visible_dps_rows_scaled(rc.bottom, scale);
    if rows.is_empty() {
        crate::ui_modern::qa_draw_empty_state(
            hdc,
            RECT {
                left: 6,
                top,
                right: rc.right - 6,
                bottom: rc.bottom,
            },
            if state.history_index.is_some() {
                "No matching players"
            } else {
                "Waiting for combat data"
            },
            if state.history_index.is_some() {
                "Check the current meter filters for this encounter."
            } else {
                "Players and combat totals appear here when data is received."
            },
        );
        SelectObject(hdc, old_font);
        return;
    }
    let leader = rows
        .first()
        .map(|row| mode_metric(row, state.sort_mode))
        .unwrap_or(0)
        .max(1);
    let regular_page = meter_regular_page_size(state, visible);
    let shown = meter_page_rows(state, visible);
    for (screen_i, (rank, row)) in shown.iter().enumerate() {
        let rank = *rank;
        let row = *row;
        let y = top + screen_i as i32 * row_h;
        let r = RECT {
            left: 6,
            top: y,
            right: rc.right - 8,
            bottom: y + row_h - 2,
        };
        let class_bg = reference_meter_row_background(row);
        let bg = class_bg;
        crate::ui_modern::fill_round_rect(hdc, r, bg, crate::ui_modern::RADIUS_SMALL);
        let row_layout_rect =
            normal_meter_row_layout_rect(r, scale, settings.meter.show_consumables);
        let layout = dps_row_layout_responsive(
            hdc,
            row_layout_rect,
            scale,
            settings.meter.show_imagines,
            settings.meter.show_active_rates,
            show_share,
            settings.meter.show_deaths,
            row,
            primary_font,
            secondary_font,
        );
        if settings.meter.show_consumables {
            paint_normal_meter_fs(hdc, state, row, r);
        }
        let bar_bottom = r.bottom;
        let bar_top = (bar_bottom - 3).max(r.top + 3);
        let content_bottom = bar_top - 1;
        fill(
            hdc,
            &RECT {
                left: r.left,
                top: r.top,
                right: r.left + 2,
                bottom: bar_top,
            },
            if row.is_dead {
                crate::ui_modern::BPSR_DANGER
            } else {
                spec_accent_color(row)
            },
        );
        SelectObject(
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
        );
        let base = text_on(bg);
        SelectObject(hdc, primary_font);
        SetTextColor(hdc, base);
        draw(
            hdc,
            &row.name,
            RECT {
                left: layout.name_left,
                top: r.top,
                right: layout.name_right,
                bottom: content_bottom,
            },
            DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS,
        );
        SelectObject(hdc, secondary_font);
        let secondary = if row.is_dead {
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
        );
        if settings.meter.show_imagines && layout.badge_count > 0 {
            let badge_w = dps_badge_w(scale);
            let badge_h = dps_badge_h(scale);
            let badge_gap = dps_badge_gap(scale);
            let by = r.top + ((row_h - 2 - badge_h) / 2).max(0);
            let mut bx = layout.badge_left;
            for badge in row.imagines.iter().take(layout.badge_count) {
                paint_badge_sized(hdc, bx, by, badge_w, badge_h, badge);
                bx += badge_w + badge_gap;
            }
        }
        SelectObject(hdc, primary_font);
        let total = mode_metric(row, state.sort_mode);
        SetTextColor(
            hdc,
            if total == 0 {
                crate::ui_modern::BPSR_MUTED
            } else {
                base
            },
        );
        draw(
            hdc,
            &compact(total as f64),
            RECT {
                left: layout.total_left,
                top: r.top,
                right: layout.total_right,
                bottom: content_bottom,
            },
            DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
        if layout.active_right > layout.active_left {
            let active = active_rate(row, state.sort_mode);
            SetTextColor(
                hdc,
                if active <= 0.0 {
                    crate::ui_modern::BPSR_MUTED
                } else {
                    base
                },
            );
            draw(
                hdc,
                &compact(active),
                RECT {
                    left: layout.active_left,
                    top: r.top,
                    right: layout.active_right,
                    bottom: content_bottom,
                },
                DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
            );
        }
        if layout.share_right > layout.share_left {
            if let Some((share, color)) = active_share(row, state.sort_mode, &settings) {
                SetTextColor(
                    hdc,
                    if share <= 0.0 {
                        crate::ui_modern::BPSR_MUTED
                    } else {
                        color
                    },
                );
                draw(
                    hdc,
                    &format!("{share:.1}%"),
                    RECT {
                        left: layout.share_left,
                        top: r.top,
                        right: layout.share_right,
                        bottom: content_bottom,
                    },
                    DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
                );
            }
        }
        if layout.death_right > layout.death_left && settings.meter.show_deaths && row.deaths > 0 {
            SelectObject(hdc, secondary_font);
            SetTextColor(
                hdc,
                if row.is_dead {
                    crate::ui_modern::BPSR_DANGER
                } else {
                    crate::ui_modern::BPSR_MUTED
                },
            );
            draw(
                hdc,
                &format!("D{}", row.deaths),
                RECT {
                    left: layout.death_left,
                    top: r.top,
                    right: layout.death_right,
                    bottom: content_bottom,
                },
                DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
            );
        }
        fill(
            hdc,
            &RECT {
                left: r.left,
                top: bar_top,
                right: r.right,
                bottom: bar_bottom,
            },
            crate::ui_modern::DARK_BG,
        );
        if total > 0 {
            let width = (r.right - r.left).max(0) as i64;
            let filled = (width.saturating_mul(total.min(leader)) / leader).clamp(0, width) as i32;
            fill(
                hdc,
                &RECT {
                    left: r.left,
                    top: bar_top,
                    right: r.left + filled,
                    bottom: bar_bottom,
                },
                reference_meter_progress_color(state.sort_mode),
            );
        }
        if row.is_local {
            paint_pinned_self_frame(hdc, &r);
        }
    }
    SelectObject(hdc, old_font);
    paint_scrollbar(hdc, rc, rows.len(), regular_page, state.scroll, top);
}

unsafe fn paint_compact_player(
    hdc: HDC,
    state: &State,
    row: &DpsRow,
    rank: usize,
    r: RECT,
    show_total: bool,
) {
    let class_bg = reference_meter_row_background(row);
    let bg = class_bg;
    crate::ui_modern::fill_round_rect(hdc, r, bg, crate::ui_modern::RADIUS_SMALL);
    fill(
        hdc,
        &RECT {
            left: r.left,
            top: r.top,
            right: r.left + 2,
            bottom: r.bottom,
        },
        if row.is_dead {
            crate::ui_modern::BPSR_DANGER
        } else {
            spec_accent_color(row)
        },
    );
    if row.is_local {
        crate::ui_modern::stroke_round_rect(
            hdc,
            r,
            rgb(104, 224, 183),
            crate::ui_modern::RADIUS_SMALL,
            1,
        );
    }
    let base = text_on(bg);
    let scale = dps_layout_scale(state);
    let rank_w = dps_adaptive_logical_px(24, scale);
    let rate_w = dps_adaptive_logical_px(64, scale);
    let total_w = if show_total {
        dps_adaptive_logical_px(70, scale)
    } else {
        0
    };
    let status_w = if row.is_dead && dps_effective_width(r.right - r.left, scale) >= 280 {
        dps_adaptive_logical_px(94, scale)
    } else {
        0
    };
    let gap = dps_adaptive_logical_px(3, scale);
    let right = r.right - 4;
    let rate_right = right;
    let rate_left = rate_right - rate_w;
    let total_right = rate_left - gap;
    let total_left = total_right - total_w;
    let identity_right = (if total_w > 0 { total_left } else { rate_left }) - gap;
    let status_left = (identity_right - status_w).max(r.left + rank_w + 36);
    let name_right = if status_w > 0 {
        status_left - gap
    } else {
        identity_right
    };
    SelectObject(
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
    );
    SelectObject(hdc, dps_primary_font(state));
    SetTextColor(
        hdc,
        if row.is_dead {
            rgb(255, 135, 135)
        } else {
            base
        },
    );
    draw(
        hdc,
        &row.name,
        RECT {
            left: r.left + rank_w,
            top: r.top,
            right: name_right,
            bottom: r.bottom,
        },
        DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS,
    );
    let rate = active_rate(row, state.sort_mode);
    SetTextColor(
        hdc,
        if rate <= 0.0 {
            crate::ui_modern::BPSR_MUTED
        } else {
            base
        },
    );
    draw(
        hdc,
        &compact(rate),
        RECT {
            left: rate_left,
            top: r.top,
            right: rate_right,
            bottom: r.bottom,
        },
        DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
    );
    if total_w > 0 {
        let total = mode_metric(row, state.sort_mode);
        SetTextColor(
            hdc,
            if total == 0 {
                crate::ui_modern::BPSR_MUTED
            } else {
                base
            },
        );
        draw(
            hdc,
            &compact(total as f64),
            RECT {
                left: total_left,
                top: r.top,
                right: total_right,
                bottom: r.bottom,
            },
            DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
    }
    if status_w > 0 {
        let (status, color) = compact_dead_status(row, now_ms());
        SetTextColor(hdc, color);
        draw(
            hdc,
            &status,
            RECT {
                left: status_left,
                top: r.top,
                right: identity_right,
                bottom: r.bottom,
            },
            DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS,
        );
    }
    let leader = meter_rows(state)
        .first()
        .map(|row| mode_metric(row, state.sort_mode))
        .unwrap_or(1)
        .max(1);
    let total = mode_metric(row, state.sort_mode).clamp(0, leader);
    let width = (r.right - r.left - 2).max(0) as i64;
    fill(
        hdc,
        &RECT {
            left: r.left + 1,
            top: r.bottom - 3,
            right: r.left + 1 + (width * total / leader) as i32,
            bottom: r.bottom - 1,
        },
        reference_meter_progress_color(state.sort_mode),
    );
    if row.is_local {
        paint_pinned_self_frame(hdc, &r);
    }
}

unsafe fn paint_compact_raid_rows(hdc: HDC, rc: RECT, state: &State) {
    if !reference_raid_columns(state, rc.right) {
        paint_compact_rows(hdc, rc, state);
        return;
    }
    let rows = meter_rows(state);
    let top = dps_rows_top_for(state);
    let scale = dps_layout_scale(state);
    let row_h = dps_compact_row_h(scale);
    if rows.is_empty() {
        crate::ui_modern::qa_draw_empty_state(
            hdc,
            RECT {
                left: 6,
                top,
                right: rc.right - 6,
                bottom: rc.bottom,
            },
            "Waiting for raid data",
            "Raid players appear here when their combat data is received.",
        );
        return;
    }
    let mid = rc.right / 2;
    let gap = dps_adaptive_logical_px(9, scale).max(7);
    let left = RECT {
        left: 6,
        top,
        right: (mid - gap).max(145),
        bottom: rc.bottom,
    };
    let right = RECT {
        left: (mid + gap).min(rc.right - 145),
        top,
        right: rc.right - 8,
        bottom: rc.bottom,
    };
    let gutter_bottom = (top + RAID_ROWS_PER_COLUMN as i32 * row_h).min(rc.bottom);
    crate::ui_modern::fill_round_rect(
        hdc,
        RECT {
            left: mid - gap / 2,
            top: top + 2,
            right: mid + gap / 2,
            bottom: gutter_bottom,
        },
        crate::ui_modern::DARK_SURFACE,
        crate::ui_modern::RADIUS_SMALL,
    );
    fill(
        hdc,
        &RECT {
            left: mid,
            top: top + 4,
            right: mid + 1,
            bottom: gutter_bottom - 2,
        },
        crate::ui_modern::DARK_BORDER,
    );
    let show_total = dps_effective_width((left.right - left.left).max(1), scale) >= 420;
    for slot in 0..RAID_ROWS_PER_COLUMN {
        let y = top + slot as i32 * row_h;
        if y >= rc.bottom {
            break;
        }
        let bottom = (y + row_h - 1).min(rc.bottom);
        if let Some(row) = rows.get(slot) {
            paint_compact_player(
                hdc,
                state,
                row,
                slot + 1,
                RECT {
                    left: left.left,
                    top: y,
                    right: left.right,
                    bottom,
                },
                show_total,
            );
        }
        let ri = slot + RAID_ROWS_PER_COLUMN;
        if let Some(row) = rows.get(ri) {
            paint_compact_player(
                hdc,
                state,
                row,
                ri + 1,
                RECT {
                    left: right.left,
                    top: y,
                    right: right.right,
                    bottom,
                },
                show_total,
            );
        }
    }
}

unsafe fn raid_row_at(hwnd: HWND, state: &State, x: i32, y: i32) -> Option<DpsRow> {
    let rc = logical_client_rect(hwnd, state.scale_percent);
    if !reference_raid_columns(state, rc.right) {
        return dps_row_at(hwnd, state, y);
    }
    let top = dps_rows_top_for(state);
    if y < top || y >= rc.bottom {
        return None;
    }
    let settings = state.features.read().map(|v| v.clone()).unwrap_or_default();
    let scale = dps_layout_scale(state);
    let row_h = dps_row_h_for(state, scale).max(1);
    let slot = ((y - top) / row_h) as usize;
    if slot >= RAID_ROWS_PER_COLUMN {
        return None;
    }
    let gap = if state.compact_mode {
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
    };
    let rows = meter_rows(state);
    rows.get(index).map(|row| (*row).clone())
}

unsafe fn hover_badge_at(hwnd: HWND, state: &State, x: i32, y: i32) -> Option<String> {
    if state.compact_mode {
        return None;
    }
    if state.kind != Kind::Dps || state.collapsed {
        return None;
    }
    let settings = state.features.read().map(|f| f.clone()).unwrap_or_default();
    if !settings.meter.show_imagines {
        return None;
    }
    let rc = logical_client_rect(hwnd, state.scale_percent);
    if reference_raid_columns(state, rc.right) {
        return None;
    }
    let scale = dps_layout_scale(state);
    let row_h = dps_row_h(scale);
    let top = dps_rows_top();
    if y < top || y >= rc.bottom {
        return None;
    }
    let screen_i = ((y - top) / row_h) as usize;
    let physical = visible_dps_rows_scaled(rc.bottom, scale);
    if screen_i >= physical {
        return None;
    }
    let shown = meter_page_rows(state, physical);
    let row = shown.get(screen_i).map(|(_, row)| *row)?;
    let r = RECT {
        left: 6,
        top: top + screen_i as i32 * row_h,
        right: rc.right - 8,
        bottom: top + screen_i as i32 * row_h + row_h - 2,
    };
    let show_share = dps_mode_share_enabled(&settings, state.sort_mode);
    let name_font = dps_primary_font(state);
    let secondary_font = dps_secondary_font(state);
    let hdc = GetDC(hwnd);
    if hdc.is_null() {
        return None;
    }
    configure_overlay_dc(hdc, state.scale_percent);
    let hover_layout_rect = normal_meter_row_layout_rect(r, scale, settings.meter.show_consumables);
    let layout = dps_row_layout_responsive(
        hdc,
        hover_layout_rect,
        scale,
        true,
        settings.meter.show_active_rates,
        show_share,
        settings.meter.show_deaths,
        row,
        name_font,
        secondary_font,
    );
    reset_overlay_dc(hdc);
    ReleaseDC(hwnd, hdc);
    if layout.badge_count == 0 {
        return None;
    }
    let badge_w = dps_badge_w(scale);
    let badge_h = dps_badge_h(scale);
    let badge_gap = dps_badge_gap(scale);
    let by = r.top + ((row_h - 2 - badge_h) / 2).max(0);
    let mut bx = layout.badge_left;
    for badge in row.imagines.iter().take(layout.badge_count) {
        if x >= bx && x < bx + badge_w && y >= by && y < by + badge_h {
            return Some(format!(
                "{} · {}",
                badge.name,
                crate::model::imagine_tier_label(badge.tier)
            ));
        }
        bx += badge_w + badge_gap;
    }
    None
}

unsafe fn consumable_hover_at(hwnd: HWND, state: &State, x: i32, y: i32) -> Option<String> {
    if state.kind != Kind::Dps || state.collapsed || state.compact_mode {
        return None;
    }
    let settings = state.features.read().map(|v| v.clone()).unwrap_or_default();
    if !settings.meter.show_consumables {
        return None;
    }
    let rc = logical_client_rect(hwnd, state.scale_percent);
    let scale = dps_layout_scale(state);
    let now = now_ms();
    if reference_raid_columns(state, rc.right) {
        let top = dps_rows_top_for(state);
        let row_h = dps_row_h_for(state, scale).max(1);
        if y < top || y >= rc.bottom {
            return None;
        }
        let slot = ((y - top) / row_h) as usize;
        if slot >= RAID_ROWS_PER_COLUMN {
            return None;
        }
        let rows = meter_rows(state);
        let gap = dps_adaptive_logical_px(RAID_CENTER_GAP, scale).max(56);
        let mid = rc.right / 2;
        let pair_w = ((gap / 2) - 8).max(24);
        let (left_x, index) = if x >= mid - gap / 2 + 2 && x < mid - gap / 2 + 2 + pair_w {
            (mid - gap / 2 + 2, slot)
        } else if x >= mid + 6 && x < mid + 6 + pair_w {
            (mid + 6, slot + RAID_ROWS_PER_COLUMN)
        } else {
            return None;
        };
        let row = *rows.get(index)?;
        let half = (pair_w / 2).max(1);
        return Some(if x < left_x + half {
            consumable_hover_text("Food", row.food.as_ref(), now)
        } else {
            consumable_hover_text("Serum", row.serum.as_ref(), now)
        });
    }
    let top = dps_rows_top();
    let row_h = dps_row_h(scale);
    if y < top || y >= rc.bottom {
        return None;
    }
    let screen_i = ((y - top) / row_h) as usize;
    let physical = visible_dps_rows_scaled(rc.bottom, scale);
    if screen_i >= physical {
        return None;
    }
    let shown = meter_page_rows(state, physical);
    let row = *shown.get(screen_i).map(|(_, row)| row)?;
    let r = RECT {
        left: 6,
        top: top + screen_i as i32 * row_h,
        right: rc.right - 8,
        bottom: top + screen_i as i32 * row_h + row_h - 2,
    };
    let fs = normal_meter_fs_rect(r, scale);
    if x < fs.left || x >= fs.right || y < fs.top || y >= fs.bottom {
        return None;
    }
    let half = ((fs.right - fs.left) / 2).max(1);
    Some(if x < fs.left + half {
        consumable_hover_text("Food", row.food.as_ref(), now)
    } else {
        consumable_hover_text("Serum", row.serum.as_ref(), now)
    })
}

const REFERENCE_METER_BG: u32 = 0x001B1715; // Win32 COLORREF: #15171B
const REFERENCE_ENCOUNTER_H: i32 = 34;
const REFERENCE_TABS_H: i32 = 28;
const REFERENCE_HEAD_H: i32 = 20;

fn reference_tabs_top() -> i32 {
    TOOLBAR_H + REFERENCE_ENCOUNTER_H
}
fn dps_rows_top() -> i32 {
    reference_tabs_top() + REFERENCE_TABS_H + REFERENCE_HEAD_H
}
fn dps_rows_top_for(_state: &State) -> i32 {
    dps_rows_top()
}
fn dps_compact_row_h(scale: i32) -> i32 {
    dps_adaptive_logical_px(26, scale)
}
fn toolbar_system_count(_state: &State) -> i32 {
    3
}
fn reference_button_w(width: i32) -> i32 {
    if width < 420 {
        24
    } else {
        28
    }
}
fn reference_system_rect(width: i32, index: i32) -> RECT {
    let w = reference_button_w(width);
    RECT {
        left: width - 4 - w * (3 - index),
        top: 3,
        right: width - 4 - w * (2 - index),
        bottom: TOOLBAR_H - 3,
    }
}
fn reference_contains(r: RECT, x: i32, y: i32) -> bool {
    x >= r.left && x < r.right && y >= r.top && y < r.bottom
}
fn toolbar_items(state: &State, right: i32) -> Vec<ToolbarItem> {
    if state.kind != Kind::Dps {
        return Vec::new();
    }
    let w = reference_button_w(right);
    let selector_right = if right < 340 { 62 } else { 76 };
    let actions_left = reference_system_rect(right, 0).left - 2 * w - 4;
    let live_w = 42;
    let nav_w = 2 * w + live_w;
    let nav_left = (right / 2 - nav_w / 2)
        .min(actions_left - nav_w - 6)
        .max(selector_right + 6);
    let rect = |left, width| RECT {
        left,
        top: 3,
        right: left + width,
        bottom: TOOLBAR_H - 3,
    };
    vec![
        ToolbarItem {
            action: ToolbarAction::More,
            rect: rect(6, selector_right - 6),
            label: "Meter ▾",
        },
        ToolbarItem {
            action: ToolbarAction::Older,
            rect: rect(nav_left, w),
            label: "‹",
        },
        ToolbarItem {
            action: ToolbarAction::Live,
            rect: rect(nav_left + w, live_w),
            label: "Live",
        },
        ToolbarItem {
            action: ToolbarAction::Newer,
            rect: rect(nav_left + w + live_w, w),
            label: "›",
        },
        ToolbarItem {
            action: ToolbarAction::Copy,
            rect: rect(actions_left, w),
            label: "",
        },
        ToolbarItem {
            action: ToolbarAction::Reset,
            rect: rect(actions_left + w, w),
            label: "",
        },
    ]
}
fn reference_action_enabled(state: &State, action: ToolbarAction) -> bool {
    match action {
        ToolbarAction::Older => {
            !state.history.is_empty()
                && state
                    .history_index
                    .map(|i| i + 1 < state.history.len())
                    .unwrap_or(true)
        }
        ToolbarAction::Newer => state.history_index.is_some(),
        _ => true,
    }
}
fn toolbar_action_help(action: ToolbarAction) -> &'static str {
    match action {
        ToolbarAction::More => "Meter layout and actions",
        ToolbarAction::Copy => "Copy results",
        ToolbarAction::Reset => "Reset encounter",
        ToolbarAction::Older => "Older encounter",
        ToolbarAction::Newer => "Newer encounter",
        ToolbarAction::Live => "Return to live encounter",
        ToolbarAction::History => "Encounter history",
        ToolbarAction::Benchmark => "Timed benchmark",
        ToolbarAction::Compact => "Compact layout",
        ToolbarAction::Raid => "Raid layout",
    }
}
unsafe fn reference_line(hdc: HDC, x1: i32, y1: i32, x2: i32, y2: i32, color: u32) {
    use windows_sys::Win32::Graphics::Gdi::{CreatePen, LineTo, MoveToEx, PS_SOLID};
    let pen = CreatePen(PS_SOLID, 1, color);
    let old = SelectObject(hdc, pen);
    MoveToEx(hdc, x1, y1, null_mut());
    LineTo(hdc, x2, y2);
    SelectObject(hdc, old);
    DeleteObject(pen);
}
unsafe fn paint_toolbar(hdc: HDC, rc: RECT, state: &State) {
    if state.kind != Kind::Dps {
        reference_legacy_paint_toolbar(hdc, rc, state);
        return;
    }
    let bg = rgb(27, 29, 35);
    let text = rgb(241, 243, 247);
    fill(
        hdc,
        &RECT {
            left: 0,
            top: 0,
            right: rc.right,
            bottom: TOOLBAR_H,
        },
        bg,
    );
    fill(
        hdc,
        &RECT {
            left: 0,
            top: TOOLBAR_H - 1,
            right: rc.right,
            bottom: TOOLBAR_H,
        },
        rgb(46, 51, 60),
    );
    let old = SelectObject(hdc, dps_cached_font(100, true));
    for item in toolbar_items(state, rc.right) {
        let r = item.rect;
        let enabled = reference_action_enabled(state, item.action);
        let active = item.action == ToolbarAction::Live && state.history_index.is_none();
        let hovered = enabled && reference_contains(r, state.hover_x, state.hover_y);
        let surface = if active {
            rgb(37, 49, 70)
        } else if hovered {
            rgb(44, 50, 59)
        } else {
            bg
        };
        if active || hovered || item.action == ToolbarAction::More {
            crate::ui_modern::fill_round_rect(hdc, r, surface, 5);
            if active {
                crate::ui_modern::stroke_round_rect(hdc, r, rgb(77, 107, 155), 5, 1);
            }
        }
        let color = if !enabled {
            rgb(74, 81, 93)
        } else if active {
            rgb(159, 198, 252)
        } else {
            text
        };
        SetTextColor(hdc, color);
        let cx = (r.left + r.right) / 2;
        let cy = (r.top + r.bottom) / 2;
        match item.action {
            ToolbarAction::Copy => {
                let back = RECT {
                    left: cx - 6,
                    top: cy - 7,
                    right: cx + 3,
                    bottom: cy + 4,
                };
                crate::ui_modern::stroke_round_rect(hdc, back, color, 2, 1);
                let front = RECT {
                    left: cx - 2,
                    top: cy - 3,
                    right: cx + 7,
                    bottom: cy + 8,
                };
                crate::ui_modern::fill_round_rect(hdc, front, surface, 2);
                crate::ui_modern::stroke_round_rect(hdc, front, color, 2, 1);
            }
            ToolbarAction::Reset => {
                use windows_sys::Win32::Graphics::Gdi::{Arc, CreatePen, PS_SOLID};
                let pen = CreatePen(PS_SOLID, 1, color);
                let prev = SelectObject(hdc, pen);
                Arc(
                    hdc,
                    cx - 6,
                    cy - 6,
                    cx + 7,
                    cy + 7,
                    cx + 4,
                    cy - 6,
                    cx + 6,
                    cy - 3,
                );
                SelectObject(hdc, prev);
                DeleteObject(pen);
                reference_line(hdc, cx + 6, cy - 8, cx + 6, cy - 2, color);
                reference_line(hdc, cx + 1, cy - 2, cx + 6, cy - 2, color);
            }
            ToolbarAction::Older | ToolbarAction::Newer => {
                let d = if item.action == ToolbarAction::Older {
                    -1
                } else {
                    1
                };
                reference_line(hdc, cx - 2 * d, cy - 5, cx + 3 * d, cy, color);
                reference_line(hdc, cx + 3 * d, cy, cx - 2 * d, cy + 5, color);
            }
            _ => draw(
                hdc,
                item.label,
                r,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
            ),
        }
    }
    for index in 0..3 {
        let r = reference_system_rect(rc.right, index);
        if reference_contains(r, state.hover_x, state.hover_y) {
            crate::ui_modern::fill_round_rect(hdc, r, rgb(44, 50, 59), 5);
        }
        SetTextColor(hdc, text);
        let cx = (r.left + r.right) / 2;
        let cy = (r.top + r.bottom) / 2;
        match index {
            0 => draw_toolbar_symbol(hdc, "⚙", r, 16),
            1 => {
                reference_line(hdc, cx - 5, cy + 2, cx, cy - 3, text);
                reference_line(hdc, cx, cy - 3, cx + 5, cy + 2, text);
            }
            _ => {
                reference_line(hdc, cx - 4, cy - 4, cx + 5, cy + 5, text);
                reference_line(hdc, cx + 4, cy - 4, cx - 5, cy + 5, text);
            }
        }
    }
    SelectObject(hdc, old);
}
unsafe fn reference_set_layout(hwnd: HWND, state: &mut State, compact: bool, raid: bool) {
    let encounter = state.history_index;
    if raid_active(state) != raid {
        toggle_raid_mode(hwnd, state);
    }
    if state.compact_mode != compact {
        toggle_compact_mode(hwnd, state);
    }
    state.history_index = encounter;
}
unsafe fn show_meter_more_menu(hwnd: HWND, state: &mut State) {
    let menu = CreatePopupMenu();
    if menu.is_null() {
        return;
    }
    for (id, label, compact, raid) in [
        (1, "Normal", false, false),
        (2, "Compact", true, false),
        (3, "Raid", false, true),
        (4, "Compact Raid", true, true),
    ] {
        let selected = state.compact_mode == compact && raid_active(state) == raid;
        AppendMenuW(
            menu,
            MF_STRING | if selected { 0x8 } else { 0 },
            id,
            wide(label).as_ptr(),
        );
    }
    AppendMenuW(menu, MF_SEPARATOR, 0, null());
    AppendMenuW(menu, MF_STRING, 10, wide("Encounter history").as_ptr());
    AppendMenuW(menu, MF_STRING, 11, wide("Benchmark…").as_ptr());
    AppendMenuW(
        menu,
        MF_STRING,
        12,
        wide("Player count and visibility…").as_ptr(),
    );
    let mut wr: RECT = std::mem::zeroed();
    GetWindowRect(hwnd, &mut wr);
    let cmd = TrackPopupMenu(
        menu,
        TPM_RETURNCMD | TPM_RIGHTBUTTON,
        wr.left + scale_px(6, state.scale_percent),
        wr.top + scale_px(TOOLBAR_H, state.scale_percent),
        0,
        hwnd,
        null(),
    );
    DestroyMenu(menu);
    match cmd {
        1..=4 => reference_set_layout(hwnd, state, cmd == 2 || cmd == 4, cmd >= 3),
        10 => crate::encounter_archive::open_local(hwnd),
        11 => crate::telemetry::benchmark_ui::show(hwnd),
        12 => open_feature_settings(hwnd, state),
        _ => {}
    }
}
unsafe fn dispatch_toolbar_action(hwnd: HWND, state: &mut State, action: ToolbarAction) {
    if !reference_action_enabled(state, action) {
        return;
    }
    match action {
        ToolbarAction::More => show_meter_more_menu(hwnd, state),
        ToolbarAction::History => crate::encounter_archive::open_local(hwnd),
        ToolbarAction::Compact => toggle_compact_mode(hwnd, state),
        ToolbarAction::Raid => toggle_raid_mode(hwnd, state),
        ToolbarAction::Older => history_older(state),
        ToolbarAction::Newer => history_newer(state),
        ToolbarAction::Live => {
            state.history_index = None;
            state.scroll = 0;
        }
        ToolbarAction::Copy => copy_view_image(hwnd, state),
        ToolbarAction::Benchmark => crate::telemetry::benchmark_ui::show(hwnd),
        ToolbarAction::Reset => {
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                MessageBoxW, IDYES, MB_DEFBUTTON2, MB_ICONQUESTION, MB_YESNO,
            };
            if MessageBoxW(hwnd,wide("Reset the live encounter? Its current results will remain in encounter history.").as_ptr(),wide("Reset encounter").as_ptr(),MB_YESNO|MB_ICONQUESTION|MB_DEFBUTTON2)==IDYES {
                archive_live_snapshot(state);crate::telemetry::request_manual_reset();
                state.dps=DpsSnapshot::default();state.history_index=None;state.scroll=0;
            }
        }
    }
}
fn reference_tab_rect(index: i32) -> RECT {
    RECT {
        left: 6 + index * 72,
        top: reference_tabs_top() + 2,
        right: 6 + index * 72 + 70,
        bottom: reference_tabs_top() + 26,
    }
}
unsafe fn on_click(hwnd: HWND, state: &mut State, lparam: LPARAM) {
    if state.kind != Kind::Dps || state.collapsed {
        reference_legacy_on_click(hwnd, state, lparam);
        return;
    }
    crate::ui::SetFocus(hwnd);
    let x = physical_to_logical(lo_signed(lparam), state.scale_percent);
    let y = physical_to_logical(hi_signed(lparam), state.scale_percent);
    let rc = logical_client_rect(hwnd, state.scale_percent);
    if y < TOOLBAR_H {
        for index in 0..3 {
            if reference_contains(reference_system_rect(rc.right, index), x, y) {
                match index {
                    0 => open_feature_settings(hwnd, state),
                    1 => collapse(hwnd, state),
                    _ => {
                        PostMessageW(state.main_hwnd, 0x0111, CMD_HIDE_DPS as usize, 0);
                    }
                }
                return;
            }
        }
        for item in toolbar_items(state, rc.right) {
            if reference_contains(item.rect, x, y) {
                dispatch_toolbar_action(hwnd, state, item.action);
                refresh_view_detail(state);
                clamp_scroll(hwnd, state);
                InvalidateRect(hwnd, null(), 0);
                return;
            }
        }
        drag_window(hwnd);
        return;
    }
    for (index, mode) in [SortMode::Damage, SortMode::Heal, SortMode::Tank]
        .into_iter()
        .enumerate()
    {
        if reference_contains(reference_tab_rect(index as i32), x, y) {
            state.sort_mode = mode;
            state.scroll = 0;
            refresh_view_detail(state);
            clamp_scroll(hwnd, state);
            InvalidateRect(hwnd, null(), 0);
            return;
        }
    }
    if y >= dps_rows_top_for(state) {
        if let Some(row) = if reference_raid_columns(state, rc.right) {
            raid_row_at(hwnd, state, x, y)
        } else {
            dps_row_at(hwnd, state, y)
        } {
            open_detail(hwnd, state, row);
        }
    }
}
unsafe fn overlay_help(hwnd: HWND, state: &State, x: i32, y: i32) -> Option<String> {
    if state.kind != Kind::Dps || state.collapsed {
        return reference_legacy_overlay_help(hwnd, state, x, y);
    }
    let rc = logical_client_rect(hwnd, state.scale_percent);
    if y < TOOLBAR_H {
        for index in 0..3 {
            if reference_contains(reference_system_rect(rc.right, index), x, y) {
                return Some(
                    ["Open overlay settings", "Collapse overlay", "Close overlay"][index as usize]
                        .into(),
                );
            }
        }
        for item in toolbar_items(state, rc.right) {
            if reference_contains(item.rect, x, y) {
                return Some(toolbar_action_help(item.action).into());
            }
        }
        return Some("Drag to move meter".into());
    }
    if y < reference_tabs_top() {
        let snap = view_snapshot(state);
        let settings = state.features.read().ok()?;
        let mut text = if settings.meter.show_target {
            format!(
                "{} • HP {} • {}",
                target_title(snap),
                target_hp_value(snap),
                format_time(snap.encounter_ms)
            )
        } else {
            format!("Encounter • {}", format_time(snap.encounter_ms))
        };
        if settings.meter.show_target {
            if let Some(ms) = live_enrage_remaining_ms(state, snap, now_ms()) {
                text.push_str(&format!(" • Enrage {}", format_enrage_countdown(ms)));
            }
        }
        return Some(text);
    }
    if y < reference_tabs_top() + REFERENCE_TABS_H {
        return Some("Damage / Heal / Tank • keyboard 1 / 2 / 3".into());
    }
    if let Some(text) = consumable_hover_at(hwnd, state, x, y) {
        return Some(text);
    }
    let row = if reference_raid_columns(state, rc.right) {
        raid_row_at(hwnd, state, x, y)
    } else {
        dps_row_at(hwnd, state, y)
    }?;
    Some(format!("{} • click to inspect", dps_identity(&row)))
}

fn reference_meter_rate_label(mode: SortMode) -> &'static str {
    match mode {
        SortMode::Damage => "Active DPS",
        SortMode::Heal => "Active HPS",
        SortMode::Tank => "Active DTPS",
    }
}
fn reference_meter_total_label(mode: SortMode) -> &'static str {
    match mode {
        SortMode::Damage => "Total Damage",
        SortMode::Heal => "Total Healing",
        SortMode::Tank => "Total Taken",
    }
}
unsafe fn dps_primary_font(state: &State) -> HFONT {
    dps_cached_font(dps_layout_scale(state), true)
}
unsafe fn dps_secondary_font(state: &State) -> HFONT {
    dps_cached_font(dps_layout_scale(state), false)
}

unsafe fn paint_reference_encounter(hdc: HDC, rc: RECT, state: &State, settings: &FeatureSettings) {
    let snapshot = view_snapshot(state);
    let top = TOOLBAR_H;
    let bottom = reference_tabs_top();
    let mut cursor = rc.right - 8;
    let value = compact(snapshot_mode_total(snapshot, state.sort_mode) as f64);
    let total = if rc.right >= 600 {
        format!(
            "{}  {}",
            reference_meter_total_label(state.sort_mode),
            value
        )
    } else if rc.right >= 420 {
        format!("Total  {value}")
    } else {
        value
    };
    let time = format_time(snapshot.encounter_ms);
    let hp = if settings.meter.show_target {
        format!("HP {}", target_hp_value(snapshot))
    } else {
        String::new()
    };
    for (text, color) in [
        (&total, rgb(241, 243, 247)),
        (&time, rgb(241, 243, 247)),
        (&hp, rgb(207, 91, 109)),
    ] {
        if text.is_empty() {
            continue;
        }
        let width = dps_text_width(hdc, text, identity_text_px(text, true)) + 4;
        let left = cursor - width;
        SetTextColor(hdc, color);
        draw(
            hdc,
            text,
            RECT {
                left,
                top,
                right: cursor,
                bottom,
            },
            DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
        cursor = left - 8;
        fill(
            hdc,
            &RECT {
                left: cursor,
                top: top + 11,
                right: cursor + 1,
                bottom: bottom - 11,
            },
            rgb(57, 63, 74),
        );
        cursor -= 8;
    }
    if settings.meter.show_target && rc.right >= 900 {
        if let Some(ms) = live_enrage_remaining_ms(state, snapshot, now_ms()) {
            let text = format!("Enrage {}", format_enrage_countdown(ms));
            let width = dps_text_width(hdc, &text, 100) + 8;
            if cursor - width > 140 {
                SetTextColor(hdc, rgb(207, 91, 109));
                draw(
                    hdc,
                    &text,
                    RECT {
                        left: cursor - width,
                        top,
                        right: cursor,
                        bottom,
                    },
                    DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
                );
                cursor -= width + 8;
            }
        }
    }
    let title = if settings.meter.show_target {
        target_title(snapshot)
    } else {
        "Encounter".into()
    };
    SetTextColor(hdc, rgb(241, 243, 247));
    draw(
        hdc,
        &title,
        RECT {
            left: 8,
            top,
            right: cursor.max(8),
            bottom,
        },
        DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS,
    );
}
unsafe fn paint_mode_tab(
    hdc: HDC,
    x: i32,
    y: i32,
    w: i32,
    label: &str,
    mode: SortMode,
    active: bool,
) {
    let r = RECT {
        left: x,
        top: y,
        right: x + w,
        bottom: y + 24,
    };
    let color = reference_meter_progress_color(mode);
    let bg = if active {
        crate::ui_modern::mix_color(rgb(34, 38, 45), color, 20)
    } else {
        rgb(27, 30, 36)
    };
    crate::ui_modern::fill_round_rect(hdc, r, bg, 4);
    if active {
        crate::ui_modern::stroke_round_rect(hdc, r, color, 4, 1);
    }
    SetTextColor(
        hdc,
        if active {
            rgb(241, 243, 247)
        } else {
            rgb(166, 177, 195)
        },
    );
    draw(
        hdc,
        label,
        r,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
    );
}
unsafe fn dps_row_layout_responsive(
    hdc: HDC,
    r: RECT,
    scale: i32,
    show_imagines: bool,
    show_active: bool,
    show_share: bool,
    show_deaths: bool,
    row: &DpsRow,
    name_font: HFONT,
    _secondary_font: HFONT,
) -> DpsRowLayout {
    let width = (r.right - r.left).max(1);
    let effective = dps_effective_width(width, scale);
    let old = SelectObject(hdc, name_font);
    let metric_w = dps_text_width(hdc, "888.88 M", 70).max(dps_adaptive_logical_px(70, scale)) + 4;
    SelectObject(hdc, old);
    let gap = if effective >= 600 { 8 } else { 4 };
    let mut cursor = r.right - 4;
    let (death_left, death_right) = dps_take_column(
        &mut cursor,
        if show_deaths && effective >= 600 {
            24
        } else {
            0
        },
        gap,
    );
    let (share_left, share_right) = dps_take_column(
        &mut cursor,
        if show_share && effective >= 480 {
            48
        } else {
            0
        },
        gap,
    );
    let (active_left, active_right) = dps_take_column(
        &mut cursor,
        if show_active {
            metric_w.max(if effective >= 600 { 88 } else { 0 })
        } else {
            0
        },
        gap,
    );
    let (total_left, total_right) = dps_take_column(
        &mut cursor,
        if !show_active || effective >= 340 {
            metric_w
        } else {
            0
        },
        gap,
    );
    let name_left = r.left + 25;
    let badge_w = dps_badge_w(scale);
    let badge_gap = dps_badge_gap(scale);
    let reserve_badges = show_imagines && effective >= 480;
    let badge_span = if reserve_badges {
        2 * badge_w + badge_gap
    } else {
        0
    };
    let badge_left = cursor - badge_span;
    let identity_right = if reserve_badges {
        badge_left - 8
    } else {
        cursor
    };
    let identity_width = (identity_right - name_left).max(0);
    let class_width = if effective >= 480 {
        (identity_width * 44 / 100).min(dps_adaptive_logical_px(150, scale))
    } else {
        0
    };
    let spec_right = identity_right;
    let spec_left = spec_right - class_width;
    let name_right = if class_width > 0 {
        spec_left - 8
    } else {
        identity_right
    };
    DpsRowLayout {
        name_left,
        name_right: name_right.max(name_left),
        spec_left,
        spec_right,
        badge_left,
        badge_count: if reserve_badges {
            row.imagines.len().min(2)
        } else {
            0
        },
        total_left,
        total_right,
        active_left,
        active_right,
        share_left,
        share_right,
        death_left,
        death_right,
    }
}
unsafe fn paint_dps_secondary_colored(hdc: HDC, row: &DpsRow, text: &str, r: RECT) {
    if r.right <= r.left {
        return;
    }
    let spec = dps_spec(row);
    let n = dps_text_width(hdc, &spec, identity_text_px(&spec, false)).min(r.right - r.left);
    SetTextColor(hdc, spec_accent_color(row));
    draw(
        hdc,
        &spec,
        RECT {
            right: r.left + n,
            ..r
        },
        DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS,
    );
    if text.starts_with(&spec) && r.left + n < r.right {
        SetTextColor(hdc, rgb(166, 177, 195));
        draw(
            hdc,
            &text[spec.len()..],
            RECT {
                left: r.left + n,
                ..r
            },
            DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS,
        );
    }
}
fn reference_raid_columns(state: &State, width: i32) -> bool {
    raid_active(state)
        && dps_effective_width(width, dps_layout_scale(state))
            >= if state.compact_mode { 680 } else { 900 }
}
unsafe fn paint_reference_headers(hdc: HDC, rc: RECT, state: &State, settings: &FeatureSettings) {
    let top = reference_tabs_top() + REFERENCE_TABS_H;
    let hr = RECT {
        left: 6,
        top,
        right: rc.right - 8,
        bottom: top + REFERENCE_HEAD_H,
    };
    let scale = dps_layout_scale(state);
    SetTextColor(hdc, rgb(166, 177, 195));
    if reference_raid_columns(state, rc.right) && !state.compact_mode {
        paint_reference_raid_headers(hdc, hr, state, settings, scale);
        return;
    }
    if state.compact_mode {
        let mut columns = vec![hr];
        if reference_raid_columns(state, rc.right) {
            let mid = rc.right / 2;
            let gap = dps_adaptive_logical_px(9, scale).max(7);
            columns = vec![
                RECT {
                    right: mid - gap,
                    ..hr
                },
                RECT {
                    left: mid + gap,
                    ..hr
                },
            ];
        }
        for r in columns {
            let rate_right = r.right - 4;
            let rate_left = rate_right - dps_adaptive_logical_px(64, scale);
            let show_total = dps_effective_width(r.right - r.left, scale)
                >= if reference_raid_columns(state, rc.right) {
                    420
                } else {
                    460
                };
            let total_right = rate_left - dps_adaptive_logical_px(3, scale);
            let total_left = total_right - dps_adaptive_logical_px(70, scale);
            let name_right = if show_total { total_left } else { rate_left } - 4;
            for (label, left, right, flags) in [
                ("#", r.left + 3, r.left + 24, 0),
                ("Player", r.left + 24, name_right, 0),
                (
                    reference_meter_rate_label(state.sort_mode),
                    rate_left,
                    rate_right,
                    DT_RIGHT,
                ),
            ] {
                draw(
                    hdc,
                    label,
                    RECT { left, right, ..r },
                    flags | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
                );
            }
            if show_total {
                draw(
                    hdc,
                    "Total",
                    RECT {
                        left: total_left,
                        right: total_right,
                        ..r
                    },
                    DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
                );
            }
        }
        return;
    }
    let content = normal_meter_row_layout_rect(hr, scale, settings.meter.show_consumables);
    let layout = dps_row_layout_responsive(
        hdc,
        content,
        scale,
        settings.meter.show_imagines,
        settings.meter.show_active_rates,
        dps_mode_share_enabled(settings, state.sort_mode),
        settings.meter.show_deaths,
        &DpsRow::default(),
        dps_primary_font(state),
        dps_secondary_font(state),
    );
    for (label, left, right, flags) in [
        ("#", hr.left + 3, hr.left + 21, 0),
        ("Player", layout.name_left, layout.name_right, 0),
        ("Class · HR", layout.spec_left, layout.spec_right, 0),
        ("Total", layout.total_left, layout.total_right, DT_RIGHT),
        (
            reference_meter_rate_label(state.sort_mode),
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
                RECT { left, right, ..hr },
                flags | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS,
            );
        }
    }
    if settings.meter.show_imagines
        && dps_effective_width(content.right - content.left, scale) >= 480
    {
        draw(
            hdc,
            "Imagines",
            RECT {
                left: layout.badge_left,
                right: layout.badge_left + 2 * dps_badge_w(scale) + dps_badge_gap(scale),
                ..hr
            },
            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
    }
    if settings.meter.show_consumables {
        let fs = normal_meter_fs_rect(hr, scale);
        let half = (fs.right - fs.left) / 2;
        for (label, left, right) in [
            ("F", fs.left, fs.left + half),
            ("S", fs.left + half, fs.right),
        ] {
            draw(
                hdc,
                label,
                RECT { left, right, ..hr },
                DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
            );
        }
    }
}
unsafe fn paint_dps(hdc: HDC, rc: RECT, state: &State) {
    let settings = state.features.read().map(|v| v.clone()).unwrap_or_default();
    let old = SelectObject(hdc, dps_primary_font(state));
    fill(
        hdc,
        &RECT {
            top: TOOLBAR_H,
            ..rc
        },
        REFERENCE_METER_BG,
    );
    paint_reference_encounter(hdc, rc, state, &settings);
    for (index, mode) in [SortMode::Damage, SortMode::Heal, SortMode::Tank]
        .into_iter()
        .enumerate()
    {
        let r = reference_tab_rect(index as i32);
        paint_mode_tab(
            hdc,
            r.left,
            r.top,
            r.right - r.left,
            compact_mode_label(mode),
            mode,
            state.sort_mode == mode,
        );
    }
    SelectObject(hdc, dps_secondary_font(state));
    paint_reference_headers(hdc, rc, state, &settings);
    if state.compact_mode {
        if reference_raid_columns(state, rc.right) {
            paint_compact_raid_rows(hdc, rc, state);
        } else {
            paint_compact_rows(hdc, rc, state);
        }
    } else if reference_raid_columns(state, rc.right) {
        paint_raid_rows(hdc, rc, state);
    } else {
        paint_reference_normal_rows(hdc, rc, state);
    }
    SelectObject(hdc, old);
}
unsafe fn paint_dps_with_raid(hdc: HDC, rc: RECT, state: &State) {
    paint_dps(hdc, rc, state);
}

fn overlay_min_height_mode(kind: Kind, scale: i32, compact: bool) -> i32 {
    if kind == Kind::Dps && compact {
        scale_px(
            (dps_rows_top() + 2 * dps_compact_row_h(scale) + 4).max(180),
            clamp_kind_scale(kind, scale),
        )
    } else {
        overlay_min_height(kind, scale)
    }
}
fn dps_image_dimensions_for(state: &State, client_width: i32, row_count: usize) -> (i32, i32) {
    let raid = raid_active(state);
    let rows = if raid {
        row_count.min(RAID_ROWS_PER_COLUMN)
    } else {
        row_count
    }
    .max(1);
    let width = if raid {
        client_width.max(if state.compact_mode {
            760
        } else {
            RAID_BASE_WIDTH
        })
    } else {
        client_width.max(if state.compact_mode { 360 } else { 620 })
    };
    let height = dps_rows_top_for(state) + (rows as i32) * dps_row_h_for(state, 100) + 6;
    (width, height)
}

unsafe fn resize_hit_test(hwnd: HWND, lparam: LPARAM) -> LRESULT {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const State;
    if !ptr.is_null() && (*ptr).kind == Kind::Dps && !(*ptr).collapsed {
        let state = &*ptr;
        let mut wr: RECT = std::mem::zeroed();
        GetWindowRect(hwnd, &mut wr);
        let x = physical_to_logical(lo_signed(lparam) - wr.left, state.scale_percent);
        let y = physical_to_logical(hi_signed(lparam) - wr.top, state.scale_percent);
        let rc = logical_client_rect(hwnd, state.scale_percent);
        if toolbar_items(state, rc.right)
            .iter()
            .any(|item| reference_contains(item.rect, x, y))
            || (0..3).any(|i| reference_contains(reference_system_rect(rc.right, i), x, y))
        {
            return HTCLIENT as LRESULT;
        }
    }
    reference_legacy_resize_hit_test(hwnd, lparam)
}

unsafe fn dps_cached_font(scale: i32, bold: bool) -> HFONT {
    static FONTS: OnceLock<std::sync::Mutex<std::collections::HashMap<(i32, bool), usize>>> =
        OnceLock::new();
    let key = (clamp_kind_scale(Kind::Dps, scale), bold);
    let fonts = FONTS.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()));
    let mut fonts = match fonts.lock() {
        Ok(value) => value,
        Err(poisoned) => poisoned.into_inner(),
    };
    if let Some(font) = fonts.get(&key) {
        return *font as HFONT;
    }
    let base = if bold { 14 } else { 12 };
    let height = dps_adaptive_logical_px(base, scale);
    let face = wide("Segoe UI");
    let font = CreateFontW(
        -height,
        0,
        0,
        0,
        if bold { 600 } else { 400 },
        0,
        0,
        0,
        1,
        0,
        0,
        5,
        0,
        face.as_ptr(),
    );
    if !font.is_null() {
        fonts.insert(key, font as usize);
    }
    font
}

unsafe fn paint_raid_rows(hdc: HDC, rc: RECT, state: &State) {
    let top = dps_rows_top();
    if rc.bottom <= top {
        return;
    }
    fill(
        hdc,
        &RECT {
            left: 0,
            top,
            right: rc.right,
            bottom: rc.bottom,
        },
        REFERENCE_METER_BG,
    );
    let rows = meter_rows(state);
    if rows.is_empty() {
        crate::ui_modern::qa_draw_empty_state(
            hdc,
            RECT {
                left: 6,
                top,
                right: rc.right - 6,
                bottom: rc.bottom,
            },
            "Waiting for raid data",
            "Raid players appear here when their combat data is received.",
        );
        return;
    }
    let settings = state.features.read().map(|v| v.clone()).unwrap_or_default();
    let snapshot = view_snapshot(state);
    let scale = dps_layout_scale(state);
    let row_h = dps_row_h(scale);
    let gap = if settings.meter.show_consumables {
        dps_adaptive_logical_px(RAID_CENTER_GAP, scale).max(56)
    } else {
        0
    };
    let mid = rc.right / 2;
    let left = RECT {
        left: 6,
        top,
        right: (mid - gap / 2 - 4).max(160),
        bottom: rc.bottom,
    };
    let right = RECT {
        left: (mid + gap / 2 + 4).min(rc.right - 160),
        top,
        right: rc.right - 8,
        bottom: rc.bottom,
    };
    let leader = rows
        .iter()
        .take(RAID_MAX_ROWS)
        .map(|r| mode_metric(r, state.sort_mode))
        .max()
        .unwrap_or(1)
        .max(1);
    let gutter_bottom = (top + RAID_ROWS_PER_COLUMN as i32 * row_h).min(rc.bottom);
    if settings.meter.show_consumables {
        crate::ui_modern::fill_round_rect(
            hdc,
            RECT {
                left: mid - gap / 2,
                top: top + 2,
                right: mid + gap / 2,
                bottom: gutter_bottom,
            },
            crate::ui_modern::DARK_SURFACE,
            crate::ui_modern::RADIUS_SMALL,
        );
    }
    fill(
        hdc,
        &RECT {
            left: mid,
            top: top + 4,
            right: mid + 1,
            bottom: gutter_bottom - 2,
        },
        crate::ui_modern::DARK_BORDER,
    );
    for slot in 0..RAID_ROWS_PER_COLUMN {
        let y = top + slot as i32 * row_h;
        if y >= rc.bottom {
            break;
        }
        let bottom = (y + row_h - 2).min(rc.bottom);
        let li = slot;
        let ri = slot + RAID_ROWS_PER_COLUMN;
        if let Some(row) = rows.get(li) {
            paint_raid_player(
                hdc,
                state,
                row,
                li + 1,
                RECT {
                    left: left.left,
                    top: y,
                    right: left.right,
                    bottom,
                },
                leader,
                snapshot,
                &settings,
            );
        }
        if let Some(row) = rows.get(ri) {
            paint_raid_player(
                hdc,
                state,
                row,
                ri + 1,
                RECT {
                    left: right.left,
                    top: y,
                    right: right.right,
                    bottom,
                },
                leader,
                snapshot,
                &settings,
            );
        }
        if settings.meter.show_consumables {
            let pair_w = ((gap / 2) - 8).max(24);
            paint_raid_fs(
                hdc,
                state,
                rows.get(li),
                mid - gap / 2 + 2,
                y,
                pair_w,
                bottom - y,
            );
            paint_raid_fs(hdc, state, rows.get(ri), mid + 6, y, pair_w, bottom - y);
        }
    }
}

unsafe fn paint_raid_player(
    hdc: HDC,
    state: &State,
    row: &DpsRow,
    rank: usize,
    r: RECT,
    leader: i64,
    _snapshot: &DpsSnapshot,
    settings: &FeatureSettings,
) {
    let scale = dps_layout_scale(state);
    let primary = dps_primary_font(state);
    let secondary = dps_secondary_font(state);
    let layout = dps_row_layout_responsive(
        hdc,
        r,
        scale,
        false,
        settings.meter.show_active_rates,
        dps_mode_share_enabled(settings, state.sort_mode),
        settings.meter.show_deaths,
        row,
        primary,
        secondary,
    );
    crate::ui_modern::fill_round_rect(hdc, r, reference_meter_row_background(row), 5);
    fill(
        hdc,
        &RECT {
            right: r.left + 3,
            bottom: r.bottom - 3,
            ..r
        },
        spec_accent_color(row),
    );
    SelectObject(hdc, secondary);
    SetTextColor(hdc, spec_accent_color(row));
    draw(
        hdc,
        &rank.to_string(),
        RECT {
            left: r.left + 4,
            right: r.left + 24,
            bottom: r.bottom - 3,
            ..r
        },
        DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
    );
    SelectObject(hdc, primary);
    SetTextColor(hdc, rgb(241, 243, 247));
    let middle = r.top + (r.bottom - r.top - 3) / 2;
    let identity = RECT {
        left: layout.name_left,
        top: r.top + 1,
        right: layout.spec_right,
        bottom: middle + 2,
    };
    draw(
        hdc,
        &row.name,
        identity,
        DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS,
    );
    SelectObject(hdc, secondary);
    let detail = RECT {
        top: middle,
        bottom: r.bottom - 4,
        ..identity
    };
    if row.is_dead {
        let (text, color) = compact_dead_status(row, now_ms());
        SetTextColor(hdc, color);
        draw(
            hdc,
            &text,
            detail,
            DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS,
        );
    } else {
        let text = dps_secondary_compact_measured(hdc, row, detail.right - detail.left);
        paint_dps_secondary_colored(hdc, row, &text, detail);
    }
    SelectObject(hdc, primary);
    SetTextColor(hdc, rgb(241, 243, 247));
    for (text, left, right) in [
        (
            compact(mode_metric(row, state.sort_mode) as f64),
            layout.total_left,
            layout.total_right,
        ),
        (
            compact(active_rate(row, state.sort_mode)),
            layout.active_left,
            layout.active_right,
        ),
    ] {
        if right > left {
            draw(
                hdc,
                &text,
                RECT {
                    left,
                    right,
                    bottom: r.bottom - 3,
                    ..r
                },
                DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
            );
        }
    }
    if layout.share_right > layout.share_left {
        if let Some((share, color)) = active_share(row, state.sort_mode, settings) {
            SetTextColor(hdc, color);
            draw(
                hdc,
                &format!("{share:.1}%"),
                RECT {
                    left: layout.share_left,
                    right: layout.share_right,
                    bottom: r.bottom - 3,
                    ..r
                },
                DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
            );
        }
    }
    if layout.death_right > layout.death_left && row.deaths > 0 {
        SetTextColor(hdc, crate::ui_modern::BPSR_DANGER);
        draw(
            hdc,
            &row.deaths.to_string(),
            RECT {
                left: layout.death_left,
                right: layout.death_right,
                bottom: r.bottom - 3,
                ..r
            },
            DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
    }
    let width = (r.right - r.left - 2).max(0) as i64;
    let total = mode_metric(row, state.sort_mode).clamp(0, leader.max(1));
    fill(
        hdc,
        &RECT {
            left: r.left + 1,
            top: r.bottom - 3,
            right: r.left + 1 + (width * total / leader.max(1)) as i32,
            bottom: r.bottom - 1,
        },
        reference_meter_progress_color(state.sort_mode),
    );
    if row.is_local {
        paint_pinned_self_frame(hdc, &r);
    }
}
