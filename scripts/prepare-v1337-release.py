#!/usr/bin/env python3
"""One-time, assertion-guarded v1.33.7 release preparation.

Run in an authenticated GitHub Actions checkout of the release PR branch.
Every substitution fails if upstream source has changed; do not silently publish
an incomplete migration.
"""
from pathlib import Path


def replace(path, old, new):
    p = Path(path)
    data = p.read_text(encoding='utf-8')
    n = data.count(old)
    if n != 1:
        raise RuntimeError(f'{path}: expected exactly one anchor, found {n}: {old[:90]!r}')
    p.write_text(data.replace(old, new, 1), encoding='utf-8')


replace('rust/Cargo.toml', 'version = "1.33.6"\n# Patch release:', 'version = "1.33.7"\n# Patch release:')
replace('rust/Cargo.lock', 'name = "bpsr-readyalert"\nversion = "1.33.6"', 'name = "bpsr-readyalert"\nversion = "1.33.7"')

# Versioned consent metadata is intentionally outside AppSettings: legacy JSON
# cannot establish consent even when its translation/TTS booleans were true.
p = 'rust/src/settings_v181.rs'
replace(p,
'''                normalize_v1140(&mut settings);
                if label != "primary" {''',
'''                let consent = serde_json::from_str::<serde_json::Value>(&text)
                    .ok()
                    .and_then(|value| value.get("privacyConsentV1337")
                        .and_then(serde_json::Value::as_bool))
                    .unwrap_or(false);
                if !consent {
                    // Earlier releases enabled cloud speech by default; do not
                    // treat a stored `true` as informed consent to send chat to Google.
                    settings.speech_translation.translation_enabled = false;
                    settings.speech_translation.tts_enabled = false;
                    logging::write("settings: cloud speech disabled pending explicit v1.33.7 consent");
                }
                normalize_v1140(&mut settings);
                if label != "primary" || !consent {''')
replace(p,
'''    let json = serde_json::to_string_pretty(&normalized).map_err(io::Error::other)?;''',
'''    let mut document = serde_json::to_value(&normalized).map_err(io::Error::other)?;
    // Only post-migration settings writes contain this versioned marker.
    // The UI asks for confirmation before enabling either cloud feature.
    document["privacyConsentV1337"] = serde_json::Value::Bool(true);
    let json = serde_json::to_string_pretty(&document).map_err(io::Error::other)?;''')
replace(p,
'''    #[test]
    fn defaults_remain_compatible() {''',
'''    #[test]
    fn legacy_cloud_speech_is_disabled_until_user_opts_in_again() {
        let paths = temp_paths("privacy-consent");
        let mut legacy = AppSettings::default();
        legacy.speech_translation.translation_enabled = true;
        legacy.speech_translation.tts_enabled = true;
        fs::write(&paths.settings, serde_json::to_vec_pretty(&legacy).unwrap()).unwrap();
        let mut migrated = load(&paths);
        assert!(!migrated.speech_translation.translation_enabled);
        assert!(!migrated.speech_translation.tts_enabled);
        let on_disk: serde_json::Value =
            serde_json::from_slice(&fs::read(&paths.settings).unwrap()).unwrap();
        assert_eq!(on_disk["privacyConsentV1337"], true);
        migrated.speech_translation.translation_enabled = true;
        save(&paths, &migrated).unwrap();
        assert!(load(&paths).speech_translation.translation_enabled);
        fs::remove_dir_all(&paths.root).unwrap();
    }

    #[test]
    fn defaults_remain_compatible() {''')

# Runtime history has different storage than encounter_store and previously
# omitted scene context. Missing legacy context remains readable but cannot PB.
p = 'rust/src/history.rs'
replace(p,
'''    pub target_name: String,
    pub snapshot: DpsSnapshot,
}''',
'''    pub target_name: String,
    #[serde(default)]
    pub context: crate::encounter_context::EncounterContextSnapshot,
    pub snapshot: DpsSnapshot,
}''')
replace(p,
'''        target_name,
        snapshot: snapshot.clone(),
    };''',
'''        target_name,
        context: crate::encounter_context::snapshot(),
        snapshot: snapshot.clone(),
    };''')

p = 'rust/src/sharing.rs'
replace(p,
'''    let target = normalized_target(current)?;
    let current_row = current.rows.iter().find(|row| row.is_local)?;''',
'''    // The selected archived encounter supplies its own context; a live
    // encounter reads the current scene. Unknown/legacy context is not a PB.
    let live_context = crate::encounter_context::snapshot();
    let current_context = match exclude_history_id {
        Some(id) => &records.iter().find(|record| record.id == id)?.context,
        None => &live_context,
    };
    if current_context.scene_id == 0 || current_context.difficulty.trim().is_empty() {
        return None;
    }
    let target = normalized_target(current)?;
    let current_row = current.rows.iter().find(|row| row.is_local)?;''')
replace(p,
'''            || record.snapshot.encounter_ms < MIN_PB_ENCOUNTER_MS
            || normalized_target(&record.snapshot).as_deref() != Some(target.as_str())''',
'''            || record.snapshot.encounter_ms < MIN_PB_ENCOUNTER_MS
            || record.context.scene_id != current_context.scene_id
            || !record.context.difficulty.trim().eq_ignore_ascii_case(current_context.difficulty.trim())
            || normalized_target(&record.snapshot).as_deref() != Some(target.as_str())''')
replace(p,
'''            target_name: "Dummy".into(),
            snapshot: snap,
        }''',
'''            target_name: "Dummy".into(),
            context: crate::encounter_context::EncounterContextSnapshot {
                scene_id: 6593,
                difficulty: "Hard".into(),
                ..Default::default()
            },
            snapshot: snap,
        }''')
replace(p,
'''    #[test]
    fn pb_rejects_short_or_tank_runs() {''',
'''    #[test]
    fn pb_never_compares_different_scene_difficulty_or_legacy_context() {
        let mut records = vec![
            record(1, snapshot("Boss", 20_000, 2_000_000, 4, 41)),
            record(2, snapshot("Boss", 20_000, 3_000_000, 4, 41)),
        ];
        records[0].context.scene_id = 6594;
        assert!(personal_best(&records, &records[1].snapshot, Some(2), ViewMode::Damage).is_none());
        records[0].context.scene_id = 6593;
        records[0].context.difficulty = "Master".into();
        assert!(personal_best(&records, &records[1].snapshot, Some(2), ViewMode::Damage).is_none());
        records[0].context.difficulty = "Hard".into();
        assert!(personal_best(&records, &records[1].snapshot, Some(2), ViewMode::Damage).is_some());
        records[0].context = Default::default();
        assert!(personal_best(&records, &records[1].snapshot, Some(2), ViewMode::Damage).is_none());
    }

    #[test]
    fn pb_rejects_short_or_tank_runs() {''')

# Modify only the final generated sources. Earlier build stages can rewrite the
# original source; assert anchors here and in the build to prevent silent drift.
p = 'rust/build/legacy/build_v1337_audit_hardening.rs'
replace(p,
'''    println!("cargo:rerun-if-changed=build/legacy/build_v1337_audit_hardening.rs");''',
'''    let ui_path = out.join("settings_ui_v1160_fixed.rs");
    let mut ui = fs::read_to_string(&ui_path).expect("read generated speech settings UI");
    let from = "ID_TTS | ID_TRANSLATE => refresh_speech_enabled(hwnd),";
    assert_eq!(ui.matches(from).count(), 1, "expected speech toggle handler in generated UI");
    ui = ui.replacen(from, r#"ID_TTS | ID_TRANSLATE => {
            if get_check(hwnd, id) {
                let service = if id == ID_TRANSLATE { "Translation" } else { "Text-to-speech" };
                let notice = format!("{service} uses Google's online service and sends chat message text to Google. Text-to-speech may also include sender names if enabled.\\r\\n\\r\\nEnable this service and consent to sending this chat text?");
                let choice = MessageBoxW(hwnd, wide(&notice).as_ptr(), wide("Cloud chat privacy consent").as_ptr(), windows_sys::Win32::UI::WindowsAndMessaging::MB_YESNO | MB_ICONINFORMATION);
                if choice != windows_sys::Win32::UI::WindowsAndMessaging::IDYES {
                    set_check(hwnd, id, false);
                }
            }
            refresh_speech_enabled(hwnd);
        },"#, 1);
    let from = "TTS is intentionally limited to Guild and Party / Team. World chat is never spoken. Translation/TTS run off the capture thread.";
    assert_eq!(ui.matches(from).count(), 1, "expected speech disclosure in generated UI");
    ui = ui.replacen(from, "Translation and TTS send enabled chat text to Google. Both require consent. TTS speaks only Guild / Party / Team chat, never World chat.", 1);
    fs::write(ui_path, ui).expect("write affirmative cloud chat consent UI");

    // Earlier updater versions saved autoDownload=true as a default. An old
    // true value is not proof of opt-in. New settings saves stamp consent.
    let updater_path = out.join("updater_v1241.rs");
    let mut updater = fs::read_to_string(&updater_path).expect("read generated updater migration");
    let from = ".and_then(|text| serde_json::from_str::<UpdatePreferences>(&text).ok())";
    assert_eq!(updater.matches(from).count(), 1, "expected updater preferences load");
    updater = updater.replacen(from, r#".and_then(|text| {
            let mut prefs = serde_json::from_str::<UpdatePreferences>(&text).ok()?;
            let consent = serde_json::from_str::<serde_json::Value>(&text).ok()
                .and_then(|value| value.get("autoDownloadConsentV1337")
                    .and_then(serde_json::Value::as_bool))
                .unwrap_or(false);
            if !consent { prefs.auto_download = false; }
            Some(prefs)
        })"#, 1);
    let from = "let data = serde_json::to_vec_pretty(prefs).map_err(|e| e.to_string())?;";
    assert_eq!(updater.matches(from).count(), 1, "expected updater preferences serialization");
    updater = updater.replacen(from, r#"let mut document = serde_json::to_value(prefs).map_err(|e| e.to_string())?;
    document["autoDownloadConsentV1337"] = serde_json::Value::Bool(true);
    let data = serde_json::to_vec_pretty(&document).map_err(|e| e.to_string())?;"#, 1);
    updater.push_str(r#"

#[cfg(test)]
mod audit_updater_migration_tests {
    use super::*;
    #[test]
    fn legacy_automatic_download_requires_new_opt_in() {
        let root = std::env::temp_dir().join(format!("readyalert-updater-consent-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(preferences_path(&root), br#"{"autoCheck":true,"autoDownload":true}"#).unwrap();
        let mut migrated = load_preferences(&root);
        assert!(migrated.auto_check);
        assert!(!migrated.auto_download);
        migrated.auto_download = true;
        save_preferences(&root, &migrated).unwrap();
        assert!(load_preferences(&root).auto_download);
        std::fs::remove_dir_all(root).unwrap();
    }
}
"#);
    fs::write(updater_path, updater).expect("write updater consent migration");
    println!("cargo:rerun-if-changed=build/legacy/build_v1337_audit_hardening.rs");''')

notes = Path('docs/releases/v1.33.7.md')
notes.parent.mkdir(parents=True, exist_ok=True)
if notes.exists():
    raise RuntimeError('release notes already exist; inspect before replacing')
notes.write_text('''# BPSR ReadyAlert v1.33.7

- Privacy: cloud chat translation and text-to-speech are disabled on fresh and legacy installations until explicitly re-enabled after a Google text-transmission notice. The speech settings UI asks for confirmation.
- Updates: automatic binary downloads default off; a legacy `autoDownload: true` setting is no longer treated as opt-in. Automatic update checks remain available.
- Encounter history: recover older valid encounters after corrupt newest files and bound encoded/decoded payloads.
- Personal best: compare only encounters with matching known scene, difficulty, boss and profession. Legacy records lacking context remain readable but cannot establish a personal best.
- Capture: enforce bounded decompression and add regression coverage.
- Chat: avoid unnecessary translation requests when only TTS is enabled; expire local logs while idle.
- Export and mechanics: protect CSV text against spreadsheet formula injection and suppress unresolved mechanic placeholders.
- Tests: Windows unit/regression, Clippy, native release smoke, binary budget and UI render diagnostics required. Live in-game/Npcap behavior requires separate manual validation.
''', encoding='utf-8')
print('Prepared v1.33.7 release changes and migration tests')
