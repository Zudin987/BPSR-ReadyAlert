use crate::{logging, paths::AppPaths};
use std::{fs, io};

mod legacy {
    include!("settings.rs");
}

pub use legacy::{
    AppSettings, ChatBlockedUser, ChatOverlaySettings, ChatSoundRule, ChatTabSettings,
    SpeechSettings,
};

const RESERVED_COMBAT_HOTKEY: &str = "Ctrl+Shift+F10";
const CHAT_RECOVERY_HOTKEY_V114: &str = "Ctrl+Shift+F9";

fn normalize_v1140(settings: &mut AppSettings) {
    settings.normalize();
    // v1.13 reserved Ctrl+Shift+F10 for the global DPS + Mechanics toggle.
    // Older Chat defaults used the same chord for click-through recovery, which
    // makes RegisterHotKey nondeterministic. Migrate that exact old/default chord
    // and keep it reserved on future saves.
    if settings.chat.click_through_hotkey.trim().eq_ignore_ascii_case(RESERVED_COMBAT_HOTKEY) {
        settings.chat.click_through_hotkey = CHAT_RECOVERY_HOTKEY_V114.into();
    }
}

pub fn load(paths: &AppPaths) -> AppSettings {
    let primary = &paths.settings;
    let backup = paths.settings.with_extension("json.bak");
    let pending = paths.settings.with_extension("json.new");

    for (candidate, label) in [
        (primary, "primary"),
        (&backup, "backup"),
        (&pending, "pending"),
    ] {
        let Ok(text) = fs::read_to_string(candidate) else { continue; };
        match serde_json::from_str::<AppSettings>(&text) {
            Ok(mut settings) => {
                normalize_v1140(&mut settings);
                if label != "primary" {
                    logging::write(format!("settings: recovered from {label}"));
                    if let Err(err) = save(paths, &settings) {
                        logging::write(format!("settings: recovery save failed: {err}"));
                    }
                }
                return settings;
            }
            Err(err) => logging::write(format!(
                "settings: load failed {}: {err}",
                candidate.display()
            )),
        }
    }

    let mut settings = AppSettings::default();
    normalize_v1140(&mut settings);
    if let Err(err) = save(paths, &settings) {
        logging::write(format!("settings: default save failed: {err}"));
    }
    settings
}

pub fn save(paths: &AppPaths, settings: &AppSettings) -> io::Result<()> {
    let mut normalized = settings.clone();
    normalize_v1140(&mut normalized);

    let primary = &paths.settings;
    let backup = paths.settings.with_extension("json.bak");
    let pending = paths.settings.with_extension("json.new");
    let json = serde_json::to_string_pretty(&normalized).map_err(io::Error::other)?;

    fs::write(&pending, json.as_bytes())?;
    // Validate the exact bytes written before touching the last known-good copy.
    let _: AppSettings = serde_json::from_slice(&fs::read(&pending)?).map_err(io::Error::other)?;

    if primary.exists() {
        // A corrupt primary may be exactly why load() fell back to the backup.
        // Rotate only a parseable primary so recovery can never destroy the
        // last known-good backup with corrupt JSON.
        let primary_is_valid = fs::read_to_string(primary)
            .ok()
            .and_then(|text| serde_json::from_str::<AppSettings>(&text).ok())
            .is_some();
        if primary_is_valid {
            fs::copy(primary, &backup)?;
        }
        fs::remove_file(primary)?;
    }

    fs::rename(&pending, primary)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temp_paths(label: &str) -> AppPaths {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let root: PathBuf = std::env::temp_dir().join(format!(
            "bpsr-readyalert-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(root.join("ChatLogs")).expect("create temp app paths");
        AppPaths {
            settings: root.join("settings.json"),
            log: root.join("readyalert.log"),
            chat_logs: root.join("ChatLogs"),
            root,
        }
    }

    #[test]
    fn defaults_remain_compatible() {
        let mut settings = AppSettings::default();
        normalize_v1140(&mut settings);
        assert!(settings.chat_overlay_enabled);
        assert_eq!(settings.chat.tabs.len(), 4);
        assert_eq!(settings.chat.local_chat_log_retention_hours, 168);
        assert_eq!(settings.chat.click_through_hotkey, CHAT_RECOVERY_HOTKEY_V114);
    }

    #[test]
    fn old_chat_recovery_hotkey_is_migrated_away_from_combat_toggle() {
        let mut settings = AppSettings::default();
        settings.chat.click_through_hotkey = "ctrl+shift+f10".into();
        normalize_v1140(&mut settings);
        assert_eq!(settings.chat.click_through_hotkey, "Ctrl+Shift+F9");
    }

    #[test]
    fn corrupt_primary_never_overwrites_good_backup() {
        let paths = temp_paths("settings-backup");
        let backup = paths.settings.with_extension("json.bak");

        let mut known_good = AppSettings::default();
        known_good.alert_volume = 37;
        normalize_v1140(&mut known_good);
        fs::write(
            &backup,
            serde_json::to_vec_pretty(&known_good).expect("serialize known-good backup"),
        )
        .expect("write known-good backup");
        fs::write(&paths.settings, b"{ definitely-not-valid-json ")
            .expect("write corrupt primary");

        let mut replacement = AppSettings::default();
        replacement.alert_volume = 88;
        save(&paths, &replacement).expect("save replacement settings");

        let preserved: AppSettings = serde_json::from_slice(
            &fs::read(&backup).expect("read preserved backup"),
        )
        .expect("backup remains valid JSON");
        let primary: AppSettings = serde_json::from_slice(
            &fs::read(&paths.settings).expect("read replacement primary"),
        )
        .expect("replacement primary is valid JSON");

        assert_eq!(preserved.alert_volume, 37);
        assert_eq!(primary.alert_volume, 88);
        assert!(!paths.settings.with_extension("json.new").exists());

        fs::remove_dir_all(&paths.root).expect("remove temp app paths");
    }
}
