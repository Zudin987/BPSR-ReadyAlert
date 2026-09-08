use crate::{logging, paths::AppPaths};
use std::{fs, io};

mod legacy {
    include!("settings.rs");
}

pub use legacy::{
    AppSettings, ChatBlockedUser, ChatOverlaySettings, ChatSoundRule, ChatTabSettings,
    SpeechSettings,
};

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
                settings.normalize();
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
    settings.normalize();
    if let Err(err) = save(paths, &settings) {
        logging::write(format!("settings: default save failed: {err}"));
    }
    settings
}

pub fn save(paths: &AppPaths, settings: &AppSettings) -> io::Result<()> {
    let mut normalized = settings.clone();
    normalized.normalize();

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

    #[test]
    fn defaults_remain_compatible() {
        let mut settings = AppSettings::default();
        settings.normalize();
        assert!(settings.chat_overlay_enabled);
        assert_eq!(settings.chat.tabs.len(), 4);
        assert_eq!(settings.chat.local_chat_log_retention_hours, 168);
    }
}
