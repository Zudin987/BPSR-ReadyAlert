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
fn qa_rows() -> Vec<DpsRow> {
    (0..20).map(|i| DpsRow {
        actor_uuid: i + 1,
        uid: i + 1,
        name: if i == 0 {
            "A Very Long Local Player Name Used For DPI Clipping QA".into()
        } else {
            format!("Player {:02}", i + 1)
        },
        profession_id: 1 + (i % 8) as i32,
        subprofession_name: if i == 0 {
            "Specialization With Long Localized Text".into()
        } else {
            "Specialization".into()
        },
        is_party: true,
        is_local: i == 0,
        is_dead: i == 2,
        revive_blocked_until_ms: if i == 2 { 0 } else { -1 },
        damage: (20-i)*100_000,
        dps: (20-i) as f64*10_000.0,
        ability_score: 65000,
        illusion_break: 15000,
        ..DpsRow::default()
    }).collect()
}

#[cfg(windows)]
fn write_qa_bmp(path: &std::path::Path, pixels: &[u8], width: i32, height: i32) {
    let mut bytes = Vec::with_capacity(54 + pixels.len());
    bytes.extend_from_slice(b"BM");
    bytes.extend_from_slice(&(54u32 + pixels.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&[0;4]);
    bytes.extend_from_slice(&54u32.to_le_bytes());
    bytes.extend_from_slice(&40u32.to_le_bytes());
    bytes.extend_from_slice(&width.to_le_bytes());
    bytes.extend_from_slice(&(-height).to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&32u16.to_le_bytes());
    bytes.extend_from_slice(&[0;24]);
    bytes.extend_from_slice(pixels);
    std::fs::write(path, bytes).unwrap();
}

#[cfg(windows)]
unsafe fn render_meter_case(
    output: &std::path::Path,
    scale: i32,
    compact: bool,
    raid: bool,
    logical_width: i32,
) {
    use windows_sys::Win32::Graphics::Gdi::{
        CreateDIBSection, BITMAPINFO, BI_RGB, DIB_RGB_COLORS, GdiFlush,
    };

    let mut s = state_stub(compact);
    s.scale_percent = scale;
    s.image_render_all = false;
    s.dps.rows = qa_rows();
    RAID_WINDOWS.with(|map| {
        map.borrow_mut().insert(raid_key(&s), RaidWindowState {
            active: raid,
            normal: s.expanded,
            raid: s.expanded,
            has_raid: raid,
            entry_compact: compact,
        });
    });

    let row_h = dps_row_h_for(&s, scale);
    let logical_rows = if raid { RAID_ROWS_PER_COLUMN as i32 } else { 10 };
    let logical_height = (dps_rows_top_for(&s) + logical_rows * row_h + 8)
        .max(if compact { 140 } else { 180 });
    let physical_width = scale_px(logical_width, scale);
    let physical_height = scale_px(logical_height, scale);

    let dc = CreateCompatibleDC(null_mut());
    assert!(!dc.is_null());
    let mut info: BITMAPINFO = std::mem::zeroed();
    info.bmiHeader.biSize = std::mem::size_of_val(&info.bmiHeader) as u32;
    info.bmiHeader.biWidth = physical_width;
    info.bmiHeader.biHeight = -physical_height;
    info.bmiHeader.biPlanes = 1;
    info.bmiHeader.biBitCount = 32;
    info.bmiHeader.biCompression = BI_RGB;
    let mut bits: *mut c_void = null_mut();
    let bitmap = CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
    assert!(!bitmap.is_null());
    let old = SelectObject(dc, bitmap);

    // Mirror the production WM_PAINT path. In particular, transparent text mode
    // matters for a useful visual artifact: a virgin memory DC defaults to an
    // opaque white text background even though the live overlay configures it.
    SelectObject(dc, GetStockObject(DEFAULT_GUI_FONT));
    SetBkMode(dc, TRANSPARENT as i32);
    configure_overlay_dc(dc, scale);
    let rc = RECT { left: 0, top: 0, right: logical_width, bottom: logical_height };
    fill(dc, &rc, crate::ui_modern::DARK_BG);
    paint_toolbar(dc, rc, &s);
    paint_dps_with_raid(dc, rc, &s);
    reset_overlay_dc(dc);
    GdiFlush();

    let pixels = std::slice::from_raw_parts(
        bits as *const u8,
        (physical_width * physical_height * 4) as usize,
    );
    assert!(pixels.iter().any(|byte| *byte != 0));

    // At every DPI, the final raid slot in the second column must contain paint.
    // This catches regressions where Raid Copy/live rendering silently falls back
    // to the normal one-column renderer.
    if raid {
        let logical_x = logical_width - 12;
        let logical_y = dps_rows_top_for(&s) + 9 * row_h + row_h / 2;
        let x = scale_px(logical_x, scale).clamp(0, physical_width - 1);
        let y = scale_px(logical_y, scale).clamp(0, physical_height - 1);
        let offset = ((y * physical_width + x) * 4) as usize;
        let bg = crate::ui_modern::DARK_BG;
        assert_ne!(
            &pixels[offset..offset+3],
            &[(bg>>16) as u8, (bg>>8) as u8, bg as u8],
            "right raid column disappeared at {scale}% compact={compact} width={logical_width}",
        );
    }

    write_qa_bmp(
        &output.join(format!(
            "meter-scale-{scale}-compact-{compact}-raid-{raid}-width-{logical_width}.bmp"
        )),
        pixels,
        physical_width,
        physical_height,
    );

    SelectObject(dc, old);
    DeleteObject(bitmap);
    DeleteDC(dc);
    RAID_WINDOWS.with(|map| { map.borrow_mut().remove(&raid_key(&s)); });
}

#[cfg(windows)]
#[test]
fn native_meter_rendered_dpi_matrix_covers_narrow_and_wide_states() {
    unsafe {
        let output = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/ui-qa-render");
        std::fs::create_dir_all(&output).unwrap();
        for scale in [100, 125, 150, 175, 200] {
            for compact in [false, true] {
                for raid in [false, true] {
                    let widths = match (compact, raid) {
                        (false, false) => [620, 900],
                        (true, false) => [360, 600],
                        (false, true) => [1080, 1320],
                        (true, true) => [760, 1000],
                    };
                    for logical_width in widths {
                        render_meter_case(&output, scale, compact, raid, logical_width);
                    }
                }
            }
        }
    }
}

#[cfg(windows)]
#[test]
fn raid_image_export_keeps_both_columns() {
    use windows_sys::Win32::Graphics::Gdi::{CreateDIBSection, BITMAPINFO, BI_RGB, DIB_RGB_COLORS, GdiFlush};
    unsafe {
        for compact in [false, true] {
            let mut s = state_stub(compact);
            s.image_render_all = true;
            s.dps.rows = qa_rows();
            RAID_WINDOWS.with(|map| { map.borrow_mut().insert(raid_key(&s), RaidWindowState {
                active: true, normal: s.expanded, raid: s.expanded, has_raid: true, entry_compact: compact,
            }); });
            let (w,h) = dps_image_dimensions_for(&s, if compact { 760 } else { 1080 }, 20);
            assert_eq!(dps_image_dimensions_for(&s, w, 10).1, h);
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
            SelectObject(dc, GetStockObject(DEFAULT_GUI_FONT));
            SetBkMode(dc, TRANSPARENT as i32);
            paint_toolbar(dc, rc, &s);
            paint_dps_with_raid(dc, rc, &s);
            GdiFlush();
            let pixels = std::slice::from_raw_parts(bits as *const u8, (w*h*4) as usize);
            let top = dps_rows_top_for(&s);
            let row_h = dps_row_h_for(&s, 100);
            let x = w-12;
            let y = top + 9*row_h + row_h/2;
            let offset = ((y*w+x)*4) as usize;
            let bg = crate::ui_modern::DARK_BG;
            assert_ne!(&pixels[offset..offset+3], &[(bg>>16) as u8, (bg>>8) as u8, bg as u8]);

            let tail = s.dps.rows.split_off(10);
            fill(dc, &rc, crate::ui_modern::DARK_BG);
            paint_dps_with_raid(dc, rc, &s);
            GdiFlush();
            let pixels = std::slice::from_raw_parts(bits as *const u8, (w*h*4) as usize);
            let offset = ((y*w+x)*4) as usize;
            assert_eq!(&pixels[offset..offset+3], &[(bg>>16) as u8, (bg>>8) as u8, bg as u8]);
            s.dps.rows.extend(tail);

            SelectObject(dc, old);
            DeleteObject(bitmap);
            DeleteDC(dc);
            RAID_WINDOWS.with(|map| { map.borrow_mut().remove(&raid_key(&s)); });
        }
    }
}
