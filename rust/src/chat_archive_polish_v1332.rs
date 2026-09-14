use std::{fs, path::Path};

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) -> Result<(), String> {
    let count = source.matches(from).count();
    if count != 1 {
        return Err(format!("chat archive v1.33.2 {label} expected one anchor, found {count}"));
    }
    *source = source.replacen(from, to, 1);
    Ok(())
}

pub fn upgrade(path: &Path) -> Result<(), String> {
    let mut html = fs::read_to_string(path)
        .map_err(|e| format!("read chat archive for v1.33.2 polish: {e}"))?
        .replace("\r\n", "\n");

    replace_once(
        &mut html,
        "</style>",
        r#"
/* v1.33.2: make Chat Archive use Encounter History as the sidebar/theme source of truth. */
:root{--bg:#0e1116;--panel:#151a21;--panel2:#1b222b;--line:#29323d;--text:#e8edf3;--muted:#8793a3;--accent:#65c9ff;--hover:#202833}
.app{grid-template-columns:350px minmax(0,1fr)}
.sidebar{padding:0;background:#11161c;border-right:1px solid var(--line)}
.brand{padding:17px 16px 12px;border-bottom:1px solid var(--line)}
.brand h1{font-size:16px;margin:0}
.brand p{font-size:12px;color:var(--muted)}
.local{margin-top:7px;font-size:11px}
.archive-tabs{display:grid;grid-template-columns:1fr 1fr;gap:2px;margin-top:7px;padding:2px;border:1px solid var(--line);border-radius:6px;background:#080f14}
.archive-tabs a{display:flex;align-items:center;justify-content:center;min-height:26px;border-radius:4px;padding:4px 6px;color:#91a0ae;text-decoration:none;font-size:10px;font-weight:650;white-space:nowrap}
.archive-tabs a:hover{background:#17232c;color:#eef5fa}
.archive-tabs a.active{background:#1b313a;color:#fff;box-shadow:inset 0 -2px 0 var(--accent)}
.side-title{padding:12px 16px 7px;color:var(--muted);font-size:11px;font-weight:700;letter-spacing:.08em;text-transform:uppercase}
.date-btn{width:calc(100% - 16px);margin:1px 8px;border-radius:8px;padding:9px 10px}
.date-btn:hover{background:var(--hover)}
.date-btn.active{background:#1e2a35;color:#fff}
@media(max-width:1050px) and (min-width:761px){.app{grid-template-columns:290px minmax(0,1fr)}}
@media(max-width:760px){.app{display:block}.sidebar{position:relative;height:auto;border-right:0;border-bottom:1px solid var(--line);max-height:220px}.brand{padding:17px 16px 12px}.date-btn{width:calc(100% - 16px);margin:1px 8px}}
</style>"#,
        "theme/sidebar styles",
    )?;

    fs::write(path, html.as_bytes())
        .map_err(|e| format!("write chat archive v1.33.2 polish: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn injects_encounter_history_sidebar_metrics_and_theme() {
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("readyalert-chat-polish-v1332-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("index.html");
        fs::write(&path, "<html><head><style>.archive-tabs{}</style></head><body></body></html>").unwrap();
        upgrade(&path).unwrap();
        let html = fs::read_to_string(&path).unwrap();
        assert!(html.contains(".app{grid-template-columns:350px minmax(0,1fr)}"));
        assert!(html.contains(".brand{padding:17px 16px 12px;border-bottom:1px solid var(--line)}"));
        assert!(html.contains("background:#080f14"));
        assert!(html.contains("background:#1b313a"));
        assert!(html.contains("--bg:#0e1116"));
        fs::remove_dir_all(dir).unwrap();
    }
}
