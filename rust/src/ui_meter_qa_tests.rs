// Included inside the existing compact-meter regression module. Exercise the
// production geometry and renderer with ordinary encounter rows.
#[test]
fn toolbar_bounds_and_semantic_states_survive_the_scale_matrix() {
    for scale in [100, 125, 150, 175, 200] {
        for compact in [false, true] {
            let mut s = state_stub(compact);
            s.scale_percent = scale;
            for physical_width in [300, 340, 400, 480, 600, 900].map(|w| scale_px(w, scale)) {
                let width = physical_extent_to_logical(physical_width, scale);
                let items = toolbar_items(&s, width);
                let system_left = width - dps_toolbar_button_w(width, scale) * toolbar_system_count(&s);
                let first = items.first().unwrap();
                assert!(first.rect.left >= if compact { compact_mode_rect().right } else { 0 }, "scale={scale}, compact={compact}, width={width}, left={}", first.rect.left);
                assert!(items.last().unwrap().rect.right <= system_left);
                for pair in items.windows(2) { assert!(pair[0].rect.right <= pair[1].rect.left); }
                assert!(!toolbar_action_active(&s, ToolbarAction::Older));
                assert!(!toolbar_action_active(&s, ToolbarAction::History));
                assert!(toolbar_action_active(&s, ToolbarAction::Live));
                s.history_index = Some(0);
                assert!(!toolbar_action_active(&s, ToolbarAction::Live));
                s.history_index = None;
            }
        }
    }
}

#[cfg(windows)]
#[test]
fn native_meter_render_and_raid_export_keep_both_columns() {
    use windows_sys::Win32::Graphics::Gdi::{CreateDIBSection, BITMAPINFO, BI_RGB, DIB_RGB_COLORS, GdiFlush};
    unsafe {
        let output = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/ui-qa-render");
        std::fs::create_dir_all(&output).unwrap();
        for compact in [false, true] {
            let mut s = state_stub(compact);
            s.image_render_all = true;
            s.dps.rows = (0..20).map(|i| DpsRow {
                actor_uuid: i + 1, uid: i + 1, name: format!("Player {:02}", i + 1),
                profession_id: 1 + (i % 8) as i32, subprofession_name: "Specialization".into(),
                is_party: true, damage: (20-i)*100_000, dps: (20-i) as f64*10_000.0,
                ability_score: 65000, illusion_break: 15000, ..DpsRow::default()
            }).collect();
            for raid in [false, true] {
                RAID_WINDOWS.with(|map| { map.borrow_mut().insert(raid_key(&s), RaidWindowState {
                    active: raid, normal: s.expanded, raid: s.expanded, has_raid: raid, entry_compact: compact,
                }); });
                let (w,h) = dps_image_dimensions_for(&s, if raid { 1040 } else { 700 }, 20);
                if raid { assert_eq!(dps_image_dimensions_for(&s, w, 10).1, h); }
                else { assert!(dps_image_dimensions_for(&s, w, 10).1 < h); }
                let dc = CreateCompatibleDC(null_mut());
                assert!(!dc.is_null());
                let mut info: BITMAPINFO = std::mem::zeroed();
                info.bmiHeader.biSize = std::mem::size_of_val(&info.bmiHeader) as u32;
                info.bmiHeader.biWidth = w;
                info.bmiHeader.biHeight = -h;
                info.bmiHeader.biPlanes = 1;
                info.bmiHeader.biBitCount = 32;
                info.bmiHeader.biCompression = BI_RGB;
                let mut bits: *mut c_void = null_mut();
                let bitmap = CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
                assert!(!bitmap.is_null());
                let old = SelectObject(dc, bitmap);
                let rc = RECT { left:0, top:0, right:w, bottom:h };
                fill(dc, &rc, crate::ui_modern::DARK_BG);
                paint_toolbar(dc, rc, &s);
                paint_dps_with_raid(dc, rc, &s);
                GdiFlush();
                let pixels = std::slice::from_raw_parts(bits as *const u8, (w*h*4) as usize);
                let top = dps_rows_top_for(&s);
                let row_h = dps_row_h_for(&s, 100);
                // Check both the populated and empty right column. A renderer
                // accidentally using normal rows cannot satisfy both states.
                if raid {
                    let x = w-12;
                    let y = top + 9*row_h + 8;
                    let offset = ((y*w+x)*4) as usize;
                    let bg = crate::ui_modern::DARK_BG;
                    assert_ne!(&pixels[offset..offset+3], &[(bg>>16) as u8, (bg>>8) as u8, bg as u8]);
                }
                let mut bytes = Vec::new();
                bytes.extend_from_slice(b"BM");
                bytes.extend_from_slice(&(54u32 + pixels.len() as u32).to_le_bytes());
                bytes.extend_from_slice(&[0;4]);
                bytes.extend_from_slice(&54u32.to_le_bytes());
                bytes.extend_from_slice(&40u32.to_le_bytes());
                bytes.extend_from_slice(&w.to_le_bytes());
                bytes.extend_from_slice(&(-h).to_le_bytes());
                bytes.extend_from_slice(&1u16.to_le_bytes());
                bytes.extend_from_slice(&32u16.to_le_bytes());
                bytes.extend_from_slice(&[0;24]);
                bytes.extend_from_slice(pixels);
                std::fs::write(output.join(format!("meter-compact-{compact}-raid-{raid}.bmp")), bytes).unwrap();
                if raid {
                    let tail = s.dps.rows.split_off(10);
                    fill(dc, &rc, crate::ui_modern::DARK_BG);
                    paint_dps_with_raid(dc, rc, &s);
                    GdiFlush();
                    let pixels = std::slice::from_raw_parts(bits as *const u8, (w*h*4) as usize);
                    let offset = (((top + 9*row_h + 8)*w + w-12)*4) as usize;
                    let bg = crate::ui_modern::DARK_BG;
                    assert_eq!(&pixels[offset..offset+3], &[(bg>>16) as u8, (bg>>8) as u8, bg as u8]);
                    s.dps.rows.extend(tail);
                }
                SelectObject(dc, old); DeleteObject(bitmap); DeleteDC(dc);
                RAID_WINDOWS.with(|map| { map.borrow_mut().remove(&raid_key(&s)); });
            }
        }
    }
}
