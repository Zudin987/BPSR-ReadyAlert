use std::{fs, path::{Path, PathBuf}};

pub fn generate(chat_logs: &Path) -> Result<PathBuf, String> {
    let root = chat_logs.parent().ok_or_else(|| "chat log folder has no app-data parent".to_string())?;
    crate::chat_archive::generate(chat_logs)?;
    let encounter_path = crate::encounter_archive::generate(root)?;

    let dir = root.join("Archives");
    fs::create_dir_all(&dir).map_err(|e| format!("create archive hub folder: {e}"))?;
    let target = dir.join("index.html");
    let pending = dir.join("index.html.new");
    let encounter_count = crate::encounter_store::load_recent(root, crate::encounter_store::DEFAULT_LIMIT).len();
    let html = format!(r#"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; base-uri 'none'; object-src 'none'">
<title>BPSR ReadyAlert Local Archives</title><style>
:root{{color-scheme:dark;--bg:#0f1115;--panel:#171c23;--line:#2b343f;--text:#e8edf3;--muted:#8c98a8;--accent:#66c9ff}}*{{box-sizing:border-box}}body{{margin:0;min-height:100vh;background:var(--bg);color:var(--text);font:14px/1.45 "Segoe UI",system-ui,sans-serif;display:grid;place-items:center;padding:24px}}main{{width:min(760px,100%)}}h1{{font-size:24px;margin:0 0 5px}}.sub{{color:var(--muted);margin-bottom:22px}}.grid{{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:12px}}a{{display:block;text-decoration:none;color:inherit;background:var(--panel);border:1px solid var(--line);border-radius:11px;padding:18px;min-height:150px}}a:hover{{border-color:#42627a;background:#1c242d}}h2{{font-size:17px;margin:0 0 7px}}p{{color:var(--muted);margin:0}}.meta{{margin-top:18px;color:var(--accent);font-size:12px}}.local{{color:#8fd3a7;font-size:12px;margin-top:8px}}@media(max-width:620px){{.grid{{grid-template-columns:1fr}}}}
</style></head><body><main><h1>BPSR ReadyAlert</h1><div class="sub">Local Archives</div><div class="grid"><a href="../ChatLogs/index.html"><h2>Chat Archive</h2><p>Browse retained chat by date, channel, sender and keyword.</p><div class="meta">Open chat history →</div></a><a href="../EncounterHistory/index.html"><h2>Encounter History</h2><p>Inspect saved combat summaries, players, skills, observed gear, buffs, deaths and incoming damage.</p><div class="meta">{} saved encounter{} →</div></a></div><div class="local">Offline only · these pages read files stored on this PC and make no network requests.</div></main></body></html>"#, encounter_count, if encounter_count == 1 { "" } else { "s" });
    fs::write(&pending, html.as_bytes()).map_err(|e| format!("write archive hub: {e}"))?;
    if target.exists() {
        fs::remove_file(&target).map_err(|e| format!("replace archive hub: {e}"))?;
    }
    fs::rename(&pending, &target).map_err(|e| format!("install archive hub: {e}"))?;

    // Keep the result explicitly tied to the generated encounter path so a
    // future refactor cannot silently stop creating it while the hub still links.
    if !encounter_path.exists() {
        return Err("encounter archive was not generated".into());
    }
    Ok(target)
}
