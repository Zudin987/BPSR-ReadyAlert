mod archive_impl {
    include!("encounter_archive_v1312.rs");

    pub fn generate_inner(root: &std::path::Path) -> Result<std::path::PathBuf, String> {
        generate(root)
    }
}

use crate::encounter_store;
use std::{fs, os::windows::ffi::OsStrExt, path::{Path, PathBuf}, ptr::null};
use windows_sys::Win32::{
    Foundation::HWND,
    UI::{
        Shell::ShellExecuteW,
        WindowsAndMessaging::SW_SHOWNORMAL,
    },
};

/// Guard the previous HTML before the multi-stage archive renderer starts. The
/// legacy stages replace index.html more than once; if any final rename fails,
/// restore the last-known-good page rather than leaving history missing/partial.
pub fn generate(root: &Path) -> Result<PathBuf, String> {
    let dir = encounter_store::dir(root);
    fs::create_dir_all(&dir).map_err(|e| format!("create encounter-history folder: {e}"))?;
    let target = dir.join("index.html");
    let guard = dir.join("index.html.last-good");
    let had_target = target.is_file();

    if had_target {
        fs::copy(&target, &guard)
            .map_err(|e| format!("preserve previous encounter history: {e}"))?;
    } else {
        let _ = fs::remove_file(&guard);
    }

    match archive_impl::generate_inner(root) {
        Ok(path) => {
            let _ = fs::remove_file(&guard);
            Ok(path)
        }
        Err(err) => {
            let restore = if had_target {
                fs::copy(&guard, &target)
                    .map(|_| ())
                    .map_err(|e| format!("restore previous encounter history: {e}"))
            } else {
                Ok(())
            };
            let _ = fs::remove_file(&guard);
            match restore {
                Ok(()) => Err(err),
                Err(restore_err) => Err(format!("{err}; {restore_err}")),
            }
        }
    }
}

/// Regenerate the offline encounter-history page and open it in the user's
/// default browser. This is intentionally user-triggered and never runs on the
/// packet capture path.
pub unsafe fn open_local(owner: HWND) {
    let root = encounter_store::default_root();
    let path = match generate(&root) {
        Ok(path) => path,
        Err(err) => {
            crate::logging::write(format!("encounter-history: manual open generation failed: {err}"));
            crate::win::message_box(
                "BPSR ReadyAlert - Encounter History",
                &format!("Could not prepare Encounter History.\n\n{err}"),
                true,
            );
            return;
        }
    };

    let operation: Vec<u16> = "open".encode_utf16().chain(std::iter::once(0)).collect();
    let file: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let result = ShellExecuteW(
        owner,
        operation.as_ptr(),
        file.as_ptr(),
        null(),
        null(),
        SW_SHOWNORMAL,
    );
    if result as isize <= 32 {
        crate::logging::write(format!(
            "encounter-history: ShellExecuteW failed code={} path={}",
            result as isize,
            path.display()
        ));
        crate::win::message_box(
            "BPSR ReadyAlert - Encounter History",
            "Encounter History was generated, but Windows could not open it in your default browser.",
            true,
        );
    }
}

#[cfg(test)]
mod v1315_archive_guard_tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn normal_generation_does_not_leave_guard_file() {
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("readyalert-encounter-safe-{}-{nonce}", std::process::id()));
        fs::create_dir_all(encounter_store::dir(&root)).unwrap();
        let path = generate(&root).unwrap();
        assert!(path.is_file());
        assert!(!encounter_store::dir(&root).join("index.html.last-good").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
