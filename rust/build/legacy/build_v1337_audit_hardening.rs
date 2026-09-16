use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1335_stronger_wipe_detection.rs");
    pub fn run() { main(); }
}

fn main() {
    previous::run();

    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let path = out.join("capture_v185.rs");
    let mut source = fs::read_to_string(&path)
        .expect("read generated capture for decompression safety regression")
        .replace("\r\n", "\n");

    // Generated capture already has a bounded decoder, used by both compressed
    // Notify bodies and nested bundles. Fail closed if a future upstream build
    // step removes that protection rather than adding a duplicate decoder.
    assert!(source.contains("const MAX_DECODED_PACKET: usize = 32 * 1024 * 1024;"),
        "generated capture decoded packet limit changed: review decompression safety");
    assert_eq!(source.matches("fn decode_zstd_limited(").count(), 1,
        "expected exactly one bounded capture decoder");
    assert!(source.contains("decode_zstd_limited(&payload[4..])")
        && source.contains("decode_zstd_limited(raw)"),
        "both compressed message types must use the bounded decoder");
    assert!(!source.contains("zstd::stream::decode_all("),
        "generated capture introduced unbounded zstd decompression");

    source.push_str(r#"

#[cfg(test)]
mod audit_capture_decoder_tests {
    use super::*;

    #[test]
    fn rejects_compressed_output_past_limit() {
        let payload = vec![0u8; MAX_DECODED_PACKET + 1];
        let compressed = zstd::stream::encode_all(Cursor::new(&payload), 1).unwrap();
        assert!(decode_zstd_limited(&compressed).is_err());
    }
}
"#);
    fs::write(path, source).expect("write capture decompression regression test");

    // Keep the original updater source untouched, as older build stages
    // generate and transform the active implementation in OUT_DIR.
    let updater_path = out.join("updater_v1241.rs");
    let mut updater = fs::read_to_string(&updater_path)
        .expect("read generated updater for automatic download consent")
        .replace("\r\n", "\n");
    let from = "auto_download: true,";
    assert_eq!(updater.matches(from).count(), 1,
        "expected one updater auto-download default; review consent behavior");
    updater = updater.replacen(from, "auto_download: false,", 1);
    updater.push_str(r#"

#[cfg(test)]
mod audit_updater_privacy_tests {
    use super::*;

    #[test]
    fn new_installations_do_not_download_updates_without_consent() {
        let prefs = UpdatePreferences::default();
        assert!(prefs.auto_check);
        assert!(!prefs.auto_download);
    }
}
"#);
    fs::write(updater_path, updater).expect("write safer updater default and regression test");

    let ui_path = out.join("settings_ui_v1160_fixed.rs");
    let mut ui = fs::read_to_string(&ui_path).expect("read generated speech settings UI");
    let from = "ID_TTS | ID_TRANSLATE => refresh_speech_enabled(hwnd),";
    assert_eq!(ui.matches(from).count(), 1, "expected speech toggle handler in generated UI");
    ui = ui.replacen(from, r#"ID_TTS | ID_TRANSLATE => {
            if get_check(hwnd, id) {
                let service = if id == ID_TRANSLATE { "Translation" } else { "Text-to-speech" };
                let notice = format!("{service} uses Google's online service and sends chat message text to Google. Text-to-speech may also include sender names if enabled.\r\n\r\nEnable this service and consent to sending this chat text?");
                let choice = windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(hwnd, wide(&notice).as_ptr(), wide("Cloud chat privacy consent").as_ptr(), windows_sys::Win32::UI::WindowsAndMessaging::MB_YESNO | MB_ICONINFORMATION);
                if choice != windows_sys::Win32::UI::WindowsAndMessaging::IDYES {
                    set_check(hwnd, id, false);
                }
            }
            refresh_speech_enabled(hwnd);
        },"#, 1);
    let from = "TTS is limited to Guild and Party / Team; World chat is never spoken. Eligible Translation/TTS chat text is sent to Google services.";
    assert_eq!(ui.matches(from).count(), 1, "expected speech disclosure in generated UI");
    ui = ui.replacen(from, "Translation/TTS send chat text to Google. Enable either only with consent. TTS: Guild / Party only; never World.", 1);
    fs::write(ui_path, ui).expect("write affirmative cloud chat consent UI");

    // Older updater versions saved autoDownload=true by default. Reset legacy
    // values before recovering from backup, then mark new explicit saves.
    let updater_path = out.join("updater_v1241.rs");
    let mut updater = fs::read_to_string(&updater_path).expect("read generated updater migration");
    let from = "            Ok(prefs) => {\n                if label != \"primary\" {";
    assert_eq!(updater.matches(from).count(), 1, "expected updater preferences recovery branch");
    updater = updater.replacen(from, r#"            Ok(mut prefs) => {
                let consent = serde_json::from_str::<serde_json::Value>(&text).ok()
                    .and_then(|value| value.get("autoDownloadConsentV1337")
                        .and_then(serde_json::Value::as_bool))
                    .unwrap_or(false);
                if !consent { prefs.auto_download = false; }
                if label != "primary" {"#, 1);
    let from = "let data = serde_json::to_vec_pretty(prefs).map_err(|e| e.to_string())?;";
    assert_eq!(updater.matches(from).count(), 1, "expected updater preferences serialization");
    updater = updater.replacen(from, r#"let mut document = serde_json::to_value(prefs).map_err(|e| e.to_string())?;
    document["autoDownloadConsentV1337"] = serde_json::Value::Bool(true);
    let data = serde_json::to_vec_pretty(&document).map_err(|e| e.to_string())?;"#, 1);
    updater.push_str(r##"

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
"##);
    fs::write(updater_path, updater).expect("write updater consent migration");
    println!("cargo:rerun-if-changed=build/legacy/build_v1337_audit_hardening.rs");
}
