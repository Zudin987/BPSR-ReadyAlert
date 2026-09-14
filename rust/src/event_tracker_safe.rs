use std::{fs, io, path::{Path, PathBuf}, sync::OnceLock};

mod legacy {
    include!("event_tracker.rs");
}

pub use legacy::{
    TrackerDisplayRow, TrackerKind, TrackerRule, TrackerScope, TrackerSettings,
    clear_scene, clear_session, current_settings, observe_buff, observe_skill,
    remove_entity, reset_counts, rows, rule_label, set_local_uid, set_party,
    sync_buff_instances,
};

static SETTINGS_PATH: OnceLock<PathBuf> = OnceLock::new();

pub fn init(root: &Path) {
    let _ = SETTINGS_PATH.set(root.join("event_tracker.json"));
    legacy::init(root);
}

/// `event_tracker.rs` already validates a pending JSON file and maintains a
/// `.bak`, but its final replacement still removes the canonical file before
/// renaming the pending file. Keep one outer last-known-good copy so an AV/disk/
/// permission failure in that final rename cannot leave event_tracker.json gone.
pub fn replace_settings(settings: TrackerSettings) -> io::Result<()> {
    let path = SETTINGS_PATH
        .get()
        .ok_or_else(|| io::Error::other("event tracker is not initialized"))?;
    let guard = path.with_extension("json.last-good");
    let had_primary = path.is_file();

    if had_primary {
        fs::copy(path, &guard)?;
    } else {
        let _ = fs::remove_file(&guard);
    }

    match legacy::replace_settings(settings) {
        Ok(()) => {
            let _ = fs::remove_file(&guard);
            Ok(())
        }
        Err(err) => {
            let restore = if had_primary {
                fs::copy(&guard, path).map(|_| ())
            } else {
                Ok(())
            };
            let _ = fs::remove_file(&guard);
            match restore {
                Ok(()) => Err(err),
                Err(restore_err) => Err(io::Error::other(format!(
                    "{err}; restoring previous event tracker settings also failed: {restore_err}"
                ))),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_tracker_types_remain_available_through_safe_wrapper() {
        let mut settings = TrackerSettings::default();
        settings.max_visible = 99;
        settings.normalize();
        assert_eq!(settings.max_visible, 12);
        let rule = TrackerRule::default();
        assert_eq!(rule.rule_id, 1);
    }
}
