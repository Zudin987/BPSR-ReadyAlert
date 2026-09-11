use crate::logging;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    cmp::Ordering,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::Duration,
};
use windows_sys::Win32::{
    Foundation::CloseHandle,
    System::Threading::{OpenProcess, WaitForSingleObject},
    UI::WindowsAndMessaging::{
        FindWindowW, MessageBoxW, PostMessageW, IDYES, MB_ICONERROR, MB_ICONINFORMATION,
        MB_OK, MB_YESNO, WM_COMMAND,
    },
};

const MANIFEST_URL: &str =
    "https://github.com/Zudin987/BPSR-ReadyAlert/releases/latest/download/update.json";
const RELEASE_PREFIX: &str =
    "https://github.com/Zudin987/BPSR-ReadyAlert/releases/download/";
const MAX_DOWNLOAD_BYTES: u64 = 50 * 1024 * 1024;
const CMD_EXIT: usize = 1099;
const SYNCHRONIZE_ACCESS: u32 = 0x0010_0000;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UpdatePreferences {
    pub auto_check: bool,
    pub auto_download: bool,
}

impl Default for UpdatePreferences {
    fn default() -> Self {
        Self {
            auto_check: true,
            auto_download: true,
        }
    }
}

#[derive(Debug, Deserialize)]
struct UpdateManifest {
    version: String,
    url: String,
    sha256: String,
}

pub fn load_preferences(root: &Path) -> UpdatePreferences {
    let path = preferences_path(root);
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<UpdatePreferences>(&text).ok())
        .unwrap_or_default()
}

pub fn save_preferences(root: &Path, prefs: &UpdatePreferences) -> Result<(), String> {
    let path = preferences_path(root);
    let pending = path.with_extension("json.new");
    let data = serde_json::to_vec_pretty(prefs).map_err(|e| e.to_string())?;
    fs::write(&pending, data).map_err(|e| format!("write update settings: {e}"))?;
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("replace update settings: {e}"))?;
    }
    fs::rename(&pending, &path).map_err(|e| format!("commit update settings: {e}"))?;
    Ok(())
}

pub fn spawn_startup(root: PathBuf) {
    let prefs = load_preferences(&root);
    if !prefs.auto_check {
        return;
    }
    thread::spawn(move || {
        if let Err(err) = run_check(&root, false) {
            logging::write(format!("updater: startup check failed: {err}"));
        }
    });
}

pub fn check_interactive(root: PathBuf) {
    thread::spawn(move || {
        if let Err(err) = run_check(&root, true) {
            logging::write(format!("updater: manual check failed: {err}"));
            message(
                "BPSR ReadyAlert - Updates",
                &format!("Could not check for updates.\n\n{err}"),
                true,
            );
        }
    });
}

pub fn handle_special_args() -> bool {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() >= 5 && args[1] == "--apply-update" {
        let pid = args[2].to_string_lossy().parse::<u32>().unwrap_or(0);
        let target = PathBuf::from(&args[3]);
        let downloaded = PathBuf::from(&args[4]);
        if let Err(err) = apply_update_helper(pid, &target, &downloaded) {
            message(
                "BPSR ReadyAlert - Update failed",
                &format!("The update could not be installed.\n\n{err}"),
                true,
            );
        }
        return true;
    }
    false
}

pub fn cleanup_stale_helper() {
    let helper = helper_path();
    let current = std::env::current_exe().ok();
    if current.as_ref() != Some(&helper) {
        let _ = fs::remove_file(helper);
    }
}

fn run_check(root: &Path, interactive: bool) -> Result<(), String> {
    let manifest = fetch_manifest()?;
    validate_manifest(&manifest)?;
    let current = env!("CARGO_PKG_VERSION");
    match compare_versions(&manifest.version, current)? {
        Ordering::Less | Ordering::Equal => {
            if interactive {
                message(
                    "BPSR ReadyAlert - Updates",
                    &format!("You're up to date.\n\nCurrent version: v{current}"),
                    false,
                );
            }
            return Ok(());
        }
        Ordering::Greater => {}
    }

    let prefs = load_preferences(root);
    let downloaded = if prefs.auto_download {
        let path = download_and_verify(&manifest)?;
        if !prompt_yes(&format!(
            "BPSR ReadyAlert v{} is available.\n\nThe update has been downloaded and verified.\n\nUpdate and restart now?\n\nYes = Update & Restart\nNo = Later",
            manifest.version
        )) {
            return Ok(());
        }
        path
    } else {
        if !prompt_yes(&format!(
            "BPSR ReadyAlert v{} is available.\n\nDownload, install, and restart now?\n\nYes = Update & Restart\nNo = Later",
            manifest.version
        )) {
            return Ok(());
        }
        download_and_verify(&manifest)?
    };

    launch_helper_and_exit(&downloaded)?;
    Ok(())
}

fn fetch_manifest() -> Result<UpdateManifest, String> {
    let response = ureq::get(MANIFEST_URL)
        .set("User-Agent", "BPSR-ReadyAlert-Updater")
        .call()
        .map_err(|e| format!("manifest request: {e}"))?;
    let text = response
        .into_string()
        .map_err(|e| format!("manifest body: {e}"))?;
    if text.len() > 64 * 1024 {
        return Err("update manifest was unexpectedly large".into());
    }
    serde_json::from_str(&text).map_err(|e| format!("invalid update manifest: {e}"))
}

fn validate_manifest(manifest: &UpdateManifest) -> Result<(), String> {
    version_tuple(&manifest.version).ok_or_else(|| "invalid manifest version".to_string())?;
    if !manifest.url.starts_with(RELEASE_PREFIX) || !manifest.url.ends_with("/BPSR-ReadyAlert.exe") {
        return Err("manifest download URL is not an official ReadyAlert release asset".into());
    }
    let hash = manifest.sha256.trim();
    if hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("manifest SHA-256 is invalid".into());
    }
    Ok(())
}

fn download_and_verify(manifest: &UpdateManifest) -> Result<PathBuf, String> {
    let dir = temp_root();
    fs::create_dir_all(&dir).map_err(|e| format!("create update temp folder: {e}"))?;
    let path = dir.join(format!("BPSR-ReadyAlert-v{}.exe", manifest.version));
    let response = ureq::get(&manifest.url)
        .set("User-Agent", "BPSR-ReadyAlert-Updater")
        .call()
        .map_err(|e| format!("update download: {e}"))?;
    let mut reader = response.into_reader();
    let mut file = File::create(&path).map_err(|e| format!("create update file: {e}"))?;
    let mut total = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = reader.read(&mut buffer).map_err(|e| format!("read update: {e}"))?;
        if count == 0 {
            break;
        }
        total = total.saturating_add(count as u64);
        if total > MAX_DOWNLOAD_BYTES {
            let _ = fs::remove_file(&path);
            return Err("update download exceeded the 50 MiB safety limit".into());
        }
        file.write_all(&buffer[..count])
            .map_err(|e| format!("write update: {e}"))?;
    }
    file.flush().map_err(|e| format!("flush update: {e}"))?;
    drop(file);

    let actual = sha256_file(&path)?;
    if !actual.eq_ignore_ascii_case(manifest.sha256.trim()) {
        let _ = fs::remove_file(&path);
        return Err("downloaded update failed SHA-256 verification".into());
    }
    logging::write(format!(
        "updater: downloaded verified v{} sha256={actual}",
        manifest.version
    ));
    Ok(path)
}

fn launch_helper_and_exit(downloaded: &Path) -> Result<(), String> {
    let current = std::env::current_exe().map_err(|e| format!("current executable: {e}"))?;
    let helper = helper_path();
    if let Some(parent) = helper.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create updater folder: {e}"))?;
    }
    let _ = fs::remove_file(&helper);
    fs::copy(&current, &helper).map_err(|e| format!("prepare updater helper: {e}"))?;
    Command::new(&helper)
        .arg("--apply-update")
        .arg(std::process::id().to_string())
        .arg(&current)
        .arg(downloaded)
        .spawn()
        .map_err(|e| format!("launch updater helper: {e}"))?;

    logging::write("updater: helper launched; requesting graceful shutdown");
    unsafe {
        let class = wide("BPSRReadyAlertRustMain");
        let hwnd = FindWindowW(class.as_ptr(), std::ptr::null());
        if !hwnd.is_null() && PostMessageW(hwnd, WM_COMMAND, CMD_EXIT, 0) != 0 {
            return Ok(());
        }
    }

    // The normal path above asks the native UI to close so capture/log workers can
    // stop cleanly. This fallback is only for the rare case where the main window
    // disappeared between the update prompt and installation request.
    std::process::exit(0);
}

fn apply_update_helper(pid: u32, target: &Path, downloaded: &Path) -> Result<(), String> {
    wait_for_process(pid);
    let backup = target.with_extension("exe.old");
    let _ = fs::remove_file(&backup);
    fs::rename(target, &backup).map_err(|e| format!("backup current EXE: {e}"))?;

    if let Err(err) = fs::copy(downloaded, target) {
        let _ = fs::rename(&backup, target);
        return Err(format!("replace EXE: {err}"));
    }

    match Command::new(target).spawn() {
        Ok(_) => {
            let _ = fs::remove_file(&backup);
            let _ = fs::remove_file(downloaded);
            Ok(())
        }
        Err(err) => {
            let _ = fs::remove_file(target);
            let _ = fs::rename(&backup, target);
            let _ = Command::new(target).spawn();
            Err(format!("restart updated app: {err}; previous version restored"))
        }
    }
}

fn wait_for_process(pid: u32) {
    if pid == 0 {
        thread::sleep(Duration::from_secs(2));
        return;
    }
    unsafe {
        let handle = OpenProcess(SYNCHRONIZE_ACCESS, 0, pid);
        if !handle.is_null() {
            let _ = WaitForSingleObject(handle, 30_000);
            let _ = CloseHandle(handle);
            return;
        }
    }
    thread::sleep(Duration::from_secs(2));
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| format!("open downloaded update: {e}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(|e| format!("hash update: {e}"))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hasher.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

fn compare_versions(left: &str, right: &str) -> Result<Ordering, String> {
    let left = version_tuple(left).ok_or_else(|| format!("invalid update version: {left}"))?;
    let right = version_tuple(right).ok_or_else(|| format!("invalid current version: {right}"))?;
    Ok(left.cmp(&right))
}

fn version_tuple(value: &str) -> Option<(u64, u64, u64)> {
    let clean = value.trim().trim_start_matches('v').split('-').next()?;
    let mut parts = clean.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

fn preferences_path(root: &Path) -> PathBuf {
    root.join("update-settings.json")
}

fn temp_root() -> PathBuf {
    std::env::temp_dir().join("BPSR-ReadyAlert")
}

fn helper_path() -> PathBuf {
    temp_root().join("BPSR-ReadyAlert-Updater.exe")
}

fn main_window() -> windows_sys::Win32::Foundation::HWND {
    unsafe {
        let class = wide("BPSRReadyAlertRustMain");
        FindWindowW(class.as_ptr(), std::ptr::null())
    }
}

fn prompt_yes(text: &str) -> bool {
    unsafe {
        MessageBoxW(
            main_window(),
            wide(text).as_ptr(),
            wide("BPSR ReadyAlert - Update available").as_ptr(),
            MB_YESNO | MB_ICONINFORMATION,
        ) == IDYES
    }
}

fn message(title: &str, text: &str, error: bool) {
    unsafe {
        MessageBoxW(
            main_window(),
            wide(text).as_ptr(),
            wide(title).as_ptr(),
            MB_OK | if error { MB_ICONERROR } else { MB_ICONINFORMATION },
        );
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_comparison_is_numeric() {
        assert_eq!(compare_versions("1.20.0", "1.19.9").unwrap(), Ordering::Greater);
        assert_eq!(compare_versions("v1.20.0", "1.20.0").unwrap(), Ordering::Equal);
        assert_eq!(compare_versions("1.9.9", "1.10.0").unwrap(), Ordering::Less);
    }

    #[test]
    fn manifest_rejects_foreign_downloads() {
        let bad = UpdateManifest {
            version: "1.20.0".into(),
            url: "https://example.com/BPSR-ReadyAlert.exe".into(),
            sha256: "a".repeat(64),
        };
        assert!(validate_manifest(&bad).is_err());
    }
}