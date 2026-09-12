include!("encounter_archive_v1270.rs");

use std::{os::windows::ffi::OsStrExt, ptr::null};
use windows_sys::Win32::{
    Foundation::HWND,
    UI::{
        Shell::ShellExecuteW,
        WindowsAndMessaging::SW_SHOWNORMAL,
    },
};

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
