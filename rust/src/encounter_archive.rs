mod archive_impl {
    include!("encounter_archive_v1312.rs");

    pub fn generate_inner(root: &std::path::Path) -> Result<std::path::PathBuf, String> {
        generate(root)
    }
}

mod ux_v1330 {
    include!("encounter_archive_ux_v1330.rs");
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

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) -> Result<(), String> {
    let count = source.matches(from).count();
    if count != 1 {
        return Err(format!("encounter-history archive UX {label} expected one anchor, found {count}"));
    }
    *source = source.replacen(from, to, 1);
    Ok(())
}

fn upgrade_archive_ui(path: &Path) -> Result<(), String> {
    let mut html = fs::read_to_string(path)
        .map_err(|e| format!("read encounter history for archive UX upgrade: {e}"))?
        .replace("\r\n", "\n");

    replace_once(
        &mut html,
        r#"<div class="brand"><h1>BPSR ReadyAlert</h1><p>Encounter History</p><div class="local">Offline / stored only on this PC</div></div>"#,
        r#"<div class="brand"><h1>BPSR ReadyAlert</h1><div class="archive-brand-row"><p>Encounter History</p><a class="archive-switch" href="../ChatLogs/index.html" title="Open Chat Archive">Chat Archive</a></div><div class="local">Offline / stored only on this PC</div></div>"#,
        "archive navigation",
    )?;

    replace_once(
        &mut html,
        r#"<div class="tools"><button id="refresh" class="tool" type="button">Refresh</button><button id="compare-btn" class="tool primary" type="button" disabled>Compare</button><span id="compare-count" class="compare-count">0 / 2</span></div>"#,
        r#"<div class="tools"><button id="refresh" class="tool" type="button">Refresh</button><button id="compare-btn" class="tool primary" type="button" disabled>Compare</button><button id="hide-misc" class="tool toggle" type="button" aria-pressed="false" title="Show only dungeons, raids, Chaotic Realm and benchmarks">Hide Misc</button><span id="compare-count" class="compare-count">0 / 2</span></div>"#,
        "Hide Misc control",
    )?;

    replace_once(
        &mut html,
        "No saved encounter matches this search.",
        "No saved encounter matches the current filters.",
        "empty filter message",
    )?;

    replace_once(
        &mut html,
        "</style>",
        r#"
.archive-brand-row{display:flex;align-items:center;gap:8px;margin-top:3px}.archive-brand-row p{margin:0;min-width:0}.archive-switch{display:inline-flex;align-items:center;justify-content:center;min-height:22px;border:1px solid var(--line);border-radius:4px;padding:3px 7px;background:#111923;color:#aebbc8;text-decoration:none;font-size:10px;font-weight:600;letter-spacing:.02em;white-space:nowrap}.archive-switch:hover{background:#192630;border-color:var(--line-strong,#354655);color:#eef5fb}.archive-switch:focus-visible{outline:2px solid rgba(67,182,200,.42);outline-offset:1px}.tool.toggle[aria-pressed="true"],.tool.toggle.active{border-color:#367383;background:#173039;color:#eaf8fb}.tool.toggle[aria-pressed="true"]:hover,.tool.toggle.active:hover{background:#1b3a45}.tools .toggle{white-space:nowrap}
</style>"#,
        "archive UX styles",
    )?;

    let old_apply_search = r#"function applySearch(){const q=(search.value||'').trim().toLocaleLowerCase();filtered=DATA.filter(e=>{if(!q)return true;const rows=e.snapshot?.rows||[];return [encounterTitle(e),e.target_name,e.context?.difficulty,e.context?.benchmark_name,...rows.map(r=>r.name),...rows.map(r=>r.subprofession_name)].join(' ').toLocaleLowerCase().includes(q)});if(!filtered.some(e=>e.id===selectedId))selectedId=filtered[0]?.id??null;selectedPlayer=0;tab='overview';compareMode=false;renderList();renderDetail();}"#;
    let new_apply_search = r#"function isMainContent(e){if(isBenchmark(e))return true;const playType=Number(e.context?.play_type)||0;return [2,9,17,18,19].includes(playType);}
function applySearch(){const q=(search.value||'').trim().toLocaleLowerCase(),hideMisc=$('#hide-misc')?.getAttribute('aria-pressed')==='true';filtered=DATA.filter(e=>{if(hideMisc&&!isMainContent(e))return false;if(!q)return true;const rows=e.snapshot?.rows||[];return [encounterTitle(e),e.target_name,e.context?.difficulty,e.context?.benchmark_name,...rows.map(r=>r.name),...rows.map(r=>r.subprofession_name)].join(' ').toLocaleLowerCase().includes(q)});compareIds=compareIds.filter(id=>filtered.some(e=>e.id===id));if(!filtered.some(e=>e.id===selectedId))selectedId=filtered[0]?.id??null;selectedPlayer=0;tab='overview';compareMode=false;renderList();renderDetail();}"#;
    replace_once(
        &mut html,
        old_apply_search,
        new_apply_search,
        "main-content filter",
    )?;

    let old_init = r#"$('#refresh').onclick=()=>location.reload();compareBtn.onclick=()=>{if(compareIds.length===2){compareMode=true;renderDetail()}};search.addEventListener('input',applySearch);document.addEventListener('keydown',ev=>{if(ev.key==='/'&&document.activeElement!==search){ev.preventDefault();search.focus()}else if(ev.key==='Escape'&&search.value){search.value='';applySearch();search.focus()}});renderList();renderDetail();"#;
    let new_init = r#"const HIDE_MISC_STORAGE_KEY='bpsr-readyalert.encounter-history.hide-misc',hideMiscBtn=$('#hide-misc');
function loadHideMisc(){try{return localStorage.getItem(HIDE_MISC_STORAGE_KEY)==='1'}catch{return false}}
function saveHideMisc(value){try{localStorage.setItem(HIDE_MISC_STORAGE_KEY,value?'1':'0')}catch{}}
function setHideMisc(value){if(!hideMiscBtn)return;hideMiscBtn.setAttribute('aria-pressed',value?'true':'false');hideMiscBtn.classList.toggle('active',value)}
setHideMisc(loadHideMisc());hideMiscBtn?.addEventListener('click',()=>{const next=hideMiscBtn.getAttribute('aria-pressed')!=='true';setHideMisc(next);saveHideMisc(next);applySearch()});$('#refresh').onclick=()=>location.reload();compareBtn.onclick=()=>{if(compareIds.length===2){compareMode=true;renderDetail()}};search.addEventListener('input',applySearch);document.addEventListener('keydown',ev=>{if(ev.key==='/'&&document.activeElement!==search){ev.preventDefault();search.focus()}else if(ev.key==='Escape'&&search.value){search.value='';applySearch();search.focus()}});applySearch();"#;
    replace_once(
        &mut html,
        old_init,
        new_init,
        "Hide Misc persistence",
    )?;

    fs::write(path, html.as_bytes())
        .map_err(|e| format!("write upgraded encounter-history archive UX: {e}"))
}

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

    let generated = archive_impl::generate_inner(root)
        .and_then(|path| {
            upgrade_archive_ui(&path)?;
            ux_v1330::upgrade(&path)?;
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

    #[test]
    fn archive_has_filters_compare_insights_and_chat_navigation() {
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("readyalert-encounter-archive-nav-{}-{nonce}", std::process::id()));
        fs::create_dir_all(encounter_store::dir(&root)).unwrap();
        let path = generate(&root).unwrap();
        let html = fs::read_to_string(path).unwrap();
        assert!(html.contains("id=\"hide-misc\""));
        assert!(html.contains("../ChatLogs/index.html"));
        assert!(html.contains("archive-tabs"));
        assert!(html.contains("bpsr-readyalert.encounter-history.hide-misc"));
        assert!(html.contains("bpsr-readyalert.encounter-history.filters.v2"));
        assert!(html.contains("[2,9,17,18,19].includes(playType)"));
        assert!(!html.contains("[2,8,9,17,18,19].includes(playType)"));
        assert!(html.contains("Benchmark only"));
        assert!(html.contains("Same player only"));
        assert!(html.contains("Largest skill damage changes"));
        assert!(html.contains("raid-roster"));
        assert!(html.contains("No saved encounter matches the current filters."));
        fs::remove_dir_all(root).unwrap();
    }
}
