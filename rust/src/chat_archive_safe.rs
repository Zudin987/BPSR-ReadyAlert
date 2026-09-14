use std::{fs, path::{Path, PathBuf}};

mod legacy {
    include!("chat_archive.rs");

    pub fn generate_legacy(dir: &Path) -> Result<PathBuf, String> {
        generate(dir)
    }
}

mod ux_v1330 {
    include!("chat_archive_ux_v1330.rs");
}

mod polish_v1332 {
    include!("chat_archive_polish_v1332.rs");
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) -> Result<(), String> {
    let count = source.matches(from).count();
    if count != 1 {
        return Err(format!("chat archive navigation {label} expected one anchor, found {count}"));
    }
    *source = source.replacen(from, to, 1);
    Ok(())
}

fn upgrade_archive_navigation(path: &Path) -> Result<(), String> {
    let mut html = fs::read_to_string(path)
        .map_err(|e| format!("read chat archive for navigation upgrade: {e}"))?
        .replace("\r\n", "\n");

    replace_once(
        &mut html,
        r#"<div class="brand"><h1>BPSR ReadyAlert</h1><p>Chat Archive</p><div class="local">Offline / local only</div></div>"#,
        r#"<div class="brand"><h1>BPSR ReadyAlert</h1><div class="archive-brand-row"><p>Chat Archive</p><a class="archive-switch" href="../EncounterHistory/index.html" title="Open Encounter History">Encounter History</a></div><div class="local">Offline / local only</div></div>"#,
        "brand link",
    )?;

    replace_once(
        &mut html,
        "</style>",
        r#"
.archive-brand-row{display:flex;align-items:center;gap:8px;margin-top:4px}.archive-brand-row p{margin:0;min-width:0}.archive-switch{display:inline-flex;align-items:center;justify-content:center;min-height:22px;border:1px solid var(--line);border-radius:4px;padding:3px 7px;background:#171e26;color:#aeb8c4;text-decoration:none;font-size:10px;font-weight:600;letter-spacing:.02em;white-space:nowrap}.archive-switch:hover{background:#202b35;border-color:#3b4a58;color:#f0f5fa}.archive-switch:focus-visible{outline:2px solid rgba(99,199,255,.42);outline-offset:1px}
</style>"#,
        "styles",
    )?;

    fs::write(path, html.as_bytes())
        .map_err(|e| format!("write chat archive navigation upgrade: {e}"))
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

    let generated = legacy::generate_legacy(dir)
        .and_then(|path| {
            upgrade_archive_navigation(&path)?;
            ux_v1330::upgrade(&path)?;
            polish_v1332::upgrade(&path)?;
            Ok(path)
        });

    match generated {
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

    #[test]
    fn generated_chat_archive_has_navigation_filters_and_matching_sidebar() {
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("readyalert-chat-archive-nav-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("chat-2026-09-14.txt"), "2026-09-14 00:00:00\t1\tAlice\t[Image(11016)] Hello\n").unwrap();
        let path = generate(&dir).unwrap();
        let html = fs::read_to_string(path).unwrap();
        assert!(html.contains("../EncounterHistory/index.html"));
        assert!(html.contains("archive-tabs"));
        assert!(html.contains("advanced-filter-panel"));
        assert!(html.contains("Exact phrase"));
        assert!(html.contains("Has image"));
        assert!(html.contains("bpsr-readyalert.chat-archive.filters.v2"));
        assert!(html.contains(".app{grid-template-columns:350px minmax(0,1fr)}"));
        assert!(html.contains(".brand{padding:17px 16px 12px;border-bottom:1px solid var(--line)}"));
        assert!(html.contains("background:#080f14"));
        assert!(html.contains("background:#1b313a"));
        fs::remove_dir_all(dir).unwrap();
    }
}
