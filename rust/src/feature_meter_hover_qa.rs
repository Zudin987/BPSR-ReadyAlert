// Real Win32 pointer/tooltip regression: test with a hidden STATIC window so
// hit-testing uses the same client size, font measurement and mouse handler as
// the meter. The synthetic rows deliberately provide observable Imagine and
// food/serum data, unlike the render-only fixture's empty statuses.
#[cfg(all(test, windows))]
mod reference_meter_hover_windows_tests {
    use super::*;
    use windows_sys::Win32::Graphics::Gdi::{GetDC, ReleaseDC};

    fn state_stub() -> State {
        let root = std::env::temp_dir();
        let paths = AppPaths {
            settings: root.join("settings.json"),
            log: root.join("readyalert.log"),
            chat_logs: root.join("ChatLogs"), root,
        };
        State {
            kind: Kind::Dps, main_hwnd: null_mut(), paths,
            features: Arc::new(RwLock::new(FeatureSettings::default())),
            scale_percent: 100, compact_mode: false, dps: DpsSnapshot::default(),
            dps_updated_unix_ms: 0, image_render_all: false,
            mechanics: MechanicSnapshot::default(), collapsed: false,
            expanded: RECT { left: 0, top: 0, right: 900, bottom: 500 },
            scroll: 0, sort_mode: SortMode::Damage,
            detail_hwnd: null_mut(), detail_uid: 0,
            settings_hwnd: null_mut(), consumable_hwnd: null_mut(),
            font: null_mut(), capture_status: String::new(), history: Vec::new(),
            history_index: None, share_notice: None,
            hover_text: None, hover_x: 0, hover_y: 0,
        }
    }

    fn mouse_lparam(x: i32, y: i32) -> LPARAM {
        (((y as u16 as u32) << 16) | x as u16 as u32) as LPARAM
    }

    #[test]
    fn real_mouse_handler_displays_imagine_food_and_serum_details() {
        unsafe {
            let class = wide("STATIC");
            let title = wide("ReadyAlert Hover QA");
            let hwnd = CreateWindowExW(0, class.as_ptr(), title.as_ptr(), WS_POPUP,
                0, 0, 900, 500, null_mut(), null_mut(), GetModuleHandleW(null()), null());
            assert!(!hwnd.is_null(), "hidden test window must exist");

            let mut state = state_stub();
            {
                let mut settings = state.features.write().unwrap();
                settings.meter.show_imagines = true;
                settings.meter.show_consumables = true;
            }
            let mut row = DpsRow {
                actor_uuid: 1, uid: 1, name: "Hover Test Player".into(),
                damage: 2_000_000, dps: 20_000.0, is_local: true,
                ..DpsRow::default()
            };
            row.imagines = vec![ImagineBadge {
                skill_id: 1, name: "QA Battle Imagine".into(),
                tier: 3, icon_key: String::new(),
            }];
            row.food = Some(ConsumableStatus {
                buff_id: 11, name: "QA Feast".into(),
                expires_unix_ms: now_ms() + 180_000, duration_ms: 300_000,
            });
            row.serum = Some(ConsumableStatus {
                buff_id: 12, name: "QA Serum".into(),
                expires_unix_ms: now_ms() + 180_000, duration_ms: 300_000,
            });
            state.dps.rows.push(row);

            let scale = 100;
            let row_h = dps_row_h(scale);
            let row_rect = RECT {
                left: 6, top: dps_rows_top(), right: 892,
                bottom: dps_rows_top() + row_h - 2,
            };
            let settings = state.features.read().unwrap().clone();
            let hdc = GetDC(hwnd);
            assert!(!hdc.is_null(), "font measurement requires a window DC");
            configure_overlay_dc(hdc, scale);
            let content = normal_meter_row_layout_rect(row_rect, scale, true);
            let layout = dps_row_layout_responsive(hdc, content, scale, true,
                settings.meter.show_active_rates,
                dps_mode_share_enabled(&settings, state.sort_mode),
                settings.meter.show_deaths, &state.dps.rows[0],
                dps_primary_font(&state), dps_secondary_font(&state));
            reset_overlay_dc(hdc);
            ReleaseDC(hwnd, hdc);
            assert_eq!(layout.badge_count, 1, "fixture must actually paint an Imagine");

            let imagine_x = layout.badge_left + dps_badge_w(scale) / 2;
            let imagine_y = row_rect.top + (row_h - 2 - dps_badge_h(scale)) / 2
                + dps_badge_h(scale) / 2;
            on_mouse_move(hwnd, &mut state, mouse_lparam(imagine_x, imagine_y));
            let imagine = state.hover_text.as_deref().unwrap_or("");
            assert!(imagine.contains("QA Battle Imagine") && imagine.contains("T3"),
                "Imagine hover should show name and tier: {imagine}");

            let fs = normal_meter_fs_rect(row_rect, scale);
            let half = ((fs.right - fs.left) / 2).max(1);
            let food_x = fs.left + half / 2;
            let serum_x = fs.left + half + (fs.right - fs.left - half) / 2;
            let fs_y = fs.top + (fs.bottom - fs.top) / 2;
            on_mouse_move(hwnd, &mut state, mouse_lparam(food_x, fs_y));
            let food = state.hover_text.as_deref().unwrap_or("");
            assert!(food.contains("Food: QA Feast") && food.contains("left"),
                "F hover should show food name and remaining time: {food}");
            on_mouse_move(hwnd, &mut state, mouse_lparam(serum_x, fs_y));
            let serum = state.hover_text.as_deref().unwrap_or("");
            assert!(serum.contains("Serum: QA Serum") && serum.contains("left"),
                "S hover should show serum name and remaining time: {serum}");
            DestroyWindow(hwnd);
        }
    }
}
