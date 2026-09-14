use std::{fs, path::{Path, PathBuf}};

mod legacy {
    include!("chat_archive.rs");

    pub fn generate_legacy(dir: &Path) -> Result<PathBuf, String> {
        generate(dir)
    }
}

/// The legacy renderer writes to index.html.new and then replaces index.html.
/// Keep a last-known-good guard outside that replacement sequence so a disk,
/// antivirus, permission, or rename failure cannot leave the user's archive gone.
pub fn generate(dir: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(dir).map_err(|e| format!("create chat log folder: {e}"))?;
    let target = dir.join("index.html");
    let guard = dir.join("index.html.last-good");
    let had_target = target.is_file();

    if had_target {
        fs::copy(&target, &guard)
            .map_err(|e| format!("preserve previous chat archive: {e}"))?;
    } else {
        let _ = fs::remove_file(&guard);
    }

    match legacy::generate_legacy(dir) {
        Ok(path) => {
            let _ = fs::remove_file(&guard);
            Ok(path)
        }
        Err(err) => {
            let restore = if had_target {
                fs::copy(&guard, &target)
                    .map(|_| ())
                    .map_err(|e| format!("restore previous chat archive: {e}"))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn wrapper_generates_archive_and_removes_guard() {
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("readyalert-chat-archive-safe-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("chat-2026-09-14.txt"), "2026-09-14 00:00:00\t1\tAlice\tHello\n").unwrap();
        let path = generate(&dir).unwrap();
        assert!(path.is_file());
        assert!(!dir.join("index.html.last-good").exists());
        fs::remove_dir_all(dir).unwrap();
    }
}
