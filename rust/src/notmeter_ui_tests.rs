// Native review artifacts use the production windows and illustrative encounters.
use super::*;

#[link(name = "user32")]
extern "system" {
    fn PrintWindow(hwnd: HWND, hdc: HDC, flags: u32) -> i32;
}

unsafe fn capture_window(hwnd: HWND, output: &std::path::Path) {
    use windows_sys::Win32::Graphics::Gdi::{
        CreateDIBSection, DeleteDC, GdiFlush, GetDC, ReleaseDC, BITMAPINFO, BI_RGB, DIB_RGB_COLORS,
    };
    let mut bounds: RECT = std::mem::zeroed();
    assert_ne!(GetWindowRect(hwnd, &mut bounds), 0);
    let width = bounds.right - bounds.left;
    let height = bounds.bottom - bounds.top;
    let screen = GetDC(null_mut());
    let dc = CreateCompatibleDC(screen);
    let mut info: BITMAPINFO = std::mem::zeroed();
    info.bmiHeader.biSize = 40;
    info.bmiHeader.biWidth = width;
    info.bmiHeader.biHeight = -height;
    info.bmiHeader.biPlanes = 1;
    info.bmiHeader.biBitCount = 32;
    info.bmiHeader.biCompression = BI_RGB;
    let mut pixels: *mut c_void = null_mut();
    let bitmap = CreateDIBSection(screen, &info, DIB_RGB_COLORS, &mut pixels, null_mut(), 0);
    assert!(!bitmap.is_null() && !pixels.is_null());
    let old = SelectObject(dc, bitmap);
    assert_ne!(PrintWindow(hwnd, dc, 2), 0, "native preview capture failed");
    GdiFlush();
    let bytes = std::slice::from_raw_parts(pixels as *const u8, (width * height * 4) as usize);
    assert!(
        bytes.iter().any(|value| *value != 0),
        "native preview is blank"
    );
    let mut bmp = Vec::with_capacity(54 + bytes.len());
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&(54u32 + bytes.len() as u32).to_le_bytes());
    bmp.extend_from_slice(&[0; 4]);
    bmp.extend_from_slice(&54u32.to_le_bytes());
    bmp.extend_from_slice(&40u32.to_le_bytes());
    bmp.extend_from_slice(&width.to_le_bytes());
    bmp.extend_from_slice(&(-height).to_le_bytes());
    bmp.extend_from_slice(&1u16.to_le_bytes());
    bmp.extend_from_slice(&32u16.to_le_bytes());
    bmp.extend_from_slice(&[0; 24]);
    bmp.extend_from_slice(bytes);
    std::fs::write(output, bmp).unwrap();
    SelectObject(dc, old);
    DeleteObject(bitmap);
    DeleteDC(dc);
    ReleaseDC(null_mut(), screen);
}

fn sample_encounter() -> DpsSnapshot {
    let mut snapshot = DpsSnapshot {
        encounter_ms: 132_000,
        target: Some(crate::model::TargetSnapshot {
            name: "Paradox-Calamity Remnant".into(),
            hp: 114_000_000,
            max_hp: 300_000_000,
            enrage_remaining_ms: Some(468_000),
            ..Default::default()
        }),
        ..Default::default()
    };
    for (index, (name, spec, dps)) in [
        ("Izzuddin", "Moonstrike", 591_900),
        ("Luna", "Smite", 413_400),
        ("Rin", "Frostbeam", 337_500),
        ("Aster", "Falconry", 333_300),
        ("Mira", "Vanguard", 250_000),
        ("Noel", "Wildpack", 166_700),
    ]
    .into_iter()
    .enumerate()
    {
        let damage = i64::from(dps) * 132;
        let skills = [
            ("Lucky Strike", 41),
            ("Stag Charge", 24),
            ("Fantasia Impact", 19),
            ("Flame Spirit", 16),
        ]
        .into_iter()
        .enumerate()
        .map(|(skill, (name, percent))| crate::model::SkillBreakdown {
            skill_id: 1241 + skill as i32,
            name: name.into(),
            damage: damage * percent / 100,
            hits: 60,
            crits: 25,
            lucky_hits: 9,
            min_value: 140_000,
            max_value: 850_000,
            ..Default::default()
        })
        .collect();
        snapshot.rows.push(DpsRow {
            uid: index as i64 + 1,
            actor_uuid: index as i64 + 1,
            name: name.into(),
            subprofession_name: spec.into(),
            is_local: index == 0,
            is_party: true,
            ability_score: 65_000,
            illusion_break: 15_000,
            damage,
            dps: f64::from(dps),
            active_dps: f64::from(dps) * 1.08,
            hits: 240,
            crits: 100,
            lucky_hits: 36,
            hp: if index == 4 { 0 } else { 38_000 },
            max_hp: 50_000,
            is_dead: index == 4,
            deaths: if index == 4 { 1 } else { 0 },
            revive_blocked_until_ms: -1,
            food: (index != 5).then(|| crate::model::ConsumableStatus {
                buff_id: 2_032_061,
                name: "Food".into(),
                expires_unix_ms: now_ms() + 600_000,
                duration_ms: 1_200_000,
            }),
            serum: Some(crate::model::ConsumableStatus {
                buff_id: 2_032_500,
                name: "Serum".into(),
                expires_unix_ms: if index == 2 {
                    now_ms() - 1_000
                } else {
                    now_ms() + 900_000
                },
                duration_ms: 1_200_000,
            }),
            imagines: vec![
                ImagineBadge {
                    skill_id: 3903,
                    name: "Muku Chief".into(),
                    tier: 5,
                    icon_key: String::new(),
                },
                ImagineBadge {
                    skill_id: 3920,
                    name: "Battle Imagine".into(),
                    tier: 4,
                    icon_key: String::new(),
                },
            ],
            skills,
            buff_uptimes: vec![
                crate::model::BuffUptime {
                    buff_id: 1,
                    name: "Food".into(),
                    uptime_ms: 126_720,
                    activations: 1,
                },
                crate::model::BuffUptime {
                    buff_id: 2,
                    name: "Serum".into(),
                    uptime_ms: 121_440,
                    activations: 1,
                },
            ],
            ..Default::default()
        });
        snapshot.total_damage += damage;
    }
    for row in &mut snapshot.rows {
        row.damage_share = row.damage as f64 * 100.0 / snapshot.total_damage as f64;
    }
    snapshot
}

#[test]
fn native_design_suite_renders_real_windows_and_a_readable_local_report() {
    unsafe {
        let output =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/ui-qa-render");
        let root = output.join("design-fixture");
        std::fs::create_dir_all(&root).unwrap();
        let paths = AppPaths {
            settings: root.join("settings.json"),
            log: root.join("readyalert.log"),
            chat_logs: root.join("ChatLogs"),
            root: root.clone(),
        };
        let mut settings = FeatureSettings::default();
        settings.dps.width = 720;
        settings.dps.height = 326;
        let features = Arc::new(RwLock::new(settings));
        let meter = create_dps(GetModuleHandleW(null()), null_mut(), features, paths).unwrap();
        let snapshot = sample_encounter();
        update_dps(meter, snapshot.clone());
        ShowWindow(meter, SW_SHOW);
        let state = GetWindowLongPtrW(meter, GWLP_USERDATA) as *mut State;
        assert!(!state.is_null());
        capture_window(meter, &output.join("notmeter-meter.bmp"));
        open_feature_settings(meter, &mut *state);
        let settings = (*state).settings_hwnd;
        assert!(!settings.is_null());
        assert!(!fc(settings, DESIGN_THEME).is_null());
        assert!(!fc(settings, DESIGN_PREVIEW).is_null());
        capture_window(settings, &output.join("notmeter-settings.bmp"));
        DestroyWindow(settings);
        open_detail(meter, &mut *state, snapshot.rows[0].clone());
        capture_window((*state).detail_hwnd, &output.join("notmeter-details.bmp"));
        DestroyWindow((*state).detail_hwnd);
        crate::encounter_store::archive(
            &root,
            &snapshot,
            &crate::encounter_context::EncounterContextSnapshot::default(),
            10,
        )
        .unwrap();
        let report = crate::encounter_archive::generate(&root).unwrap();
        let html = std::fs::read_to_string(&report).unwrap();
        assert!(html.contains("readyalert-notmeter"));
        assert!(html.contains("player-rate"));
        assert!(html.contains("Izzuddin"));
        std::fs::copy(report, output.join("notmeter-history.html")).unwrap();
        DestroyWindow(meter);
    }
}
