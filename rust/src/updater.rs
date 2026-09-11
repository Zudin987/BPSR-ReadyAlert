use crate::logging;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    cmp::Ordering,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicBool, Ordering as AtomicOrdering},
    thread,
    time::Duration,
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, HWND},
    System::Threading::{OpenProcess, WaitForSingleObject},
    UI::WindowsAndMessaging::{
        MessageBoxW, PostMessageW, IDYES, MB_ICONERROR, MB_ICONINFORMATION, MB_OK, MB_YESNO,
        WM_COMMAND,
    },
};

const MANIFEST_URL: &str =
    "https://github.com/Zudin987/BPSR-ReadyAlert/releases/latest/download/update.json";
const RELEASE_PREFIX: &str =
    "https://github.com/Zudin987/BPSR-ReadyAlert/releases/download/";
const MAX_DOWNLOAD_BYTES: u64 = 50 * 1024 * 1024;
const CMD_EXIT: usize = 1099;
const SYNCHRONIZE_ACCESS: u32 = 0x0010_0000;
const WAIT_OBJECT_0_CODE: u32 = 0;
const WAIT_TIMEOUT_CODE: u32 = 0x0000_0102;
const WAIT_FAILED_CODE: u32 = 0xffff_ffff;
static UPDATE_BUSY: AtomicBool = AtomicBool::new(false);

struct UpdateBusyGuard;
impl Drop for UpdateBusyGuard {
    fn drop(&mut self) {
        UPDATE_BUSY.store(false, AtomicOrdering::Release);
    }
}

fn begin_update_operation() -> Option<UpdateBusyGuard> {
    UPDATE_BUSY
        .compare_exchange(false, true, AtomicOrdering::AcqRel, AtomicOrdering::Acquire)
        .ok()
        .map(|_| UpdateBusyGuard)
}

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
    let backup = path.with_extension("json.bak");
    let data = serde_json::to_vec_pretty(prefs).map_err(|e| e.to_string())?;
    fs::write(&pending, data).map_err(|e| format!("write update settings: {e}"))?;
    let _: UpdatePreferences = serde_json::from_slice(
        &fs::read(&pending).map_err(|e| format!("verify update settings: {e}"))?,
    )
    .map_err(|e| format!("verify update settings JSON: {e}"))?;
    if path.exists() {
        if fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<UpdatePreferences>(&bytes).ok())
            .is_some()
        {
            let _ = fs::copy(&path, &backup);
        }
        fs::remove_file(&path).map_err(|e| format!("replace update settings: {e}"))?;
    }
    fs::rename(&pending, &path).map_err(|e| format!("commit update settings: {e}"))?;
    Ok(())
}

pub fn spawn_startup(root: PathBuf, owner: isize) {
    let prefs = load_preferences(&root);
    if !prefs.auto_check {
        return;
    }
    thread::spawn(move || {
        let Some(_busy) = begin_update_operation() else {
            logging::write("updater: startup check skipped because another update operation is active");
            return;
        };
        if let Err(err) = run_check(&root, false, owner) {
            logging::write(format!("updater: startup check failed: {err}"));
        }
    });
}

pub fn check_interactive(root: PathBuf, owner: isize) {
    thread::spawn(move || {
        let Some(_busy) = begin_update_operation() else {
            message(
                owner,
                "BPSR ReadyAlert - Updates",
                "An update check or installation is already in progress.",
                false,
            );
            return;
        };
        if let Err(err) = run_check(&root, true, owner) {
            logging::write(format!("updater: manual check failed: {err}"));
            message(
                owner,
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
                0,
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

fn run_check(root: &Path, interactive: bool, owner: isize) -> Result<(), String> {
    let manifest = fetch_manifest()?;
    validate_manifest(&manifest)?;
    let current = env!("CARGO_PKG_VERSION");
    match compare_versions(&manifest.version, current)? {
        Ordering::Less | Ordering::Equal => {
            if interactive {
                message(
                    owner,
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
        if !prompt_yes(
            owner,
            &format!(
                "BPSR ReadyAlert v{} is available.\n\nThe update has been downloaded and verified.\n\nUpdate and restart now?\n\nYes = Update & Restart\nNo = Later",
                manifest.version
            ),
        ) {
            return Ok(());
        }
        path
    } else {
        if !prompt_yes(
            owner,
            &format!(
                "BPSR ReadyAlert v{} is available.\n\nDownload, install, and restart now?\n\nYes = Update & Restart\nNo = Later",
                manifest.version
            ),
        ) {
            return Ok(());
        }
        download_and_verify(&manifest)?
    };

    launch_helper_and_exit(&downloaded, owner)?;
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
    let (major, minor, patch) =
        version_tuple(&manifest.version).ok_or_else(|| "invalid manifest version".to_string())?;
    let canonical = format!("{major}.{minor}.{patch}");
    let expected_url = format!("{RELEASE_PREFIX}v{canonical}/BPSR-ReadyAlert.exe");
    if manifest.url != expected_url {
        return Err("manifest download URL does not match its ReadyAlert release version".into());
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

fn launch_helper_and_exit(downloaded: &Path, owner: isize) -> Result<(), String> {
    let current = std::env::current_exe().map_err(|e| format!("current executable: {e}"))?;
    let helper = helper_path();
    if let Some(parent) = helper.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create updater folder: {e}"))?;
    }
    let _ = fs::remove_file(&helper);
    fs::copy(&current, &helper).map_err(|e| format!("prepare updater helper: {e}"))?;
    let mut helper_process = Command::new(&helper)
        .arg("--apply-update")
        .arg(std::process::id().to_string())
        .arg(&current)
        .arg(downloaded)
        .spawn()
        .map_err(|e| format!("launch updater helper: {e}"))?;

    logging::write("updater: helper launched; requesting graceful shutdown");
    let hwnd = owner as HWND;
    if !hwnd.is_null() && unsafe { PostMessageW(hwnd, WM_COMMAND, CMD_EXIT, 0) } != 0 {
        return Ok(());
    }

    let _ = helper_process.kill();
    Err("could not request graceful ReadyAlert shutdown; update was not installed".into())
}

fn apply_update_helper(pid: u32, target: &Path, downloaded: &Path) -> Result<(), String> {
    wait_for_process(pid)?;
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

fn wait_for_process(pid: u32) -> Result<(), String> {
    if pid == 0 {
        thread::sleep(Duration::from_secs(2));
        return Ok(());
    }
    unsafe {
        let handle = OpenProcess(SYNCHRONIZE_ACCESS, 0, pid);
        if handle.is_null() {
            // The process may already have exited between helper launch and here.
            thread::sleep(Duration::from_millis(250));
            return Ok(());
        }
        let result = WaitForSingleObject(handle, 30_000);
        let _ = CloseHandle(handle);
        match result {
            WAIT_OBJECT_0_CODE => Ok(()),
            WAIT_TIMEOUT_CODE => Err("ReadyAlert did not exit within 30 seconds; update cancelled".into()),
            WAIT_FAILED_CODE => Err("Windows failed while waiting for ReadyAlert to exit".into()),
            other => Err(format!("unexpected process wait result {other}; update cancelled")),
        }
    }
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

fn prompt_yes(owner: isize, text: &str) -> bool {
    unsafe {
        MessageBoxW(
            owner as HWND,
            wide(text).as_ptr(),
            wide("BPSR ReadyAlert - Update available").as_ptr(),
            MB_YESNO | MB_ICONINFORMATION,
        ) == IDYES
    }
}

fn message(owner: isize, title: &str, text: &str, error: bool) {
    unsafe {
        MessageBoxW(
            owner as HWND,
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

    fn good_manifest(version: &str) -> UpdateManifest {
        UpdateManifest {
            version: version.into(),
            url: format!("{RELEASE_PREFIX}v{version}/BPSR-ReadyAlert.exe"),
            sha256: "a".repeat(64),
        }
    }

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

    #[test]
    fn manifest_url_must_match_version_exactly() {
        let mut manifest = good_manifest("1.23.0");
        assert!(validate_manifest(&manifest).is_ok());
        manifest.url = format!("{RELEASE_PREFIX}v1.22.1/BPSR-ReadyAlert.exe");
        assert!(validate_manifest(&manifest).is_err());
    }
}