use std::{fs, path::{Path, PathBuf}};

mod prior {
    include!("encounter_archive_v1290.rs");

    pub fn generate_prior(root: &std::path::Path) -> Result<std::path::PathBuf, String> {
        generate(root)
    }
}

/// v1.31.2 archive extension: expose the Imagine loadout already retained in each
/// saved DPS row. The archive remains completely local and requires no new schema.
pub fn generate(root: &Path) -> Result<PathBuf, String> {
    let target = prior::generate_prior(root)?;
    let html = fs::read_to_string(&target)
        .map_err(|e| format!("read generated encounter history for Imagine upgrade: {e}"))?;
    let count = html.matches("</body>").count();
    if count != 1 {
        return Err(format!("encounter-history v1.31.2 Imagine script expected one body anchor, found {count}"));
    }
    let html = html.replacen("</body>", &format!("{IMAGINE_ARCHIVE_SCRIPT}\n</body>"), 1);

    let pending = target.with_extension("html.v1312.new");
    fs::write(&pending, html.as_bytes()).map_err(|e| format!("write Imagine encounter history: {e}"))?;
    if target.exists() {
        fs::remove_file(&target).map_err(|e| format!("replace encounter history for Imagine upgrade: {e}"))?;
    }
    fs::rename(&pending, &target).map_err(|e| format!("install Imagine encounter history: {e}"))?;
    Ok(target)
}

const IMAGINE_ARCHIVE_SCRIPT: &str = r#"
<script>
(function(){
function imagineName(x){const name=String(x?.name||'').trim();return name||`Imagine ${Number(x?.skill_id)||'Unknown'}`}
function imagineTier(x){return `T${Math.max(0,Number(x?.tier)||0)}`}
function imagineSummary(r){const items=[...(r?.imagines||[])];return items.length?items.map(x=>`${imagineName(x)} ${imagineTier(x)}`).join(' · '):'None recorded'}
function renderImaginePane(pane,r){
  const items=[...(r?.imagines||[])];
  if(!items.length){pane.innerHTML='<div class="empty"><strong>No Imagine data</strong>No Imagine summon was retained for this player in this encounter.</div>';return;}
  pane.innerHTML=`<div class="table-wrap"><table class="table"><thead><tr><th>Imagine</th><th>Tier</th><th>Skill ID</th></tr></thead><tbody>${items.map(x=>`<tr><td>${esc(imagineName(x))}</td><td>${esc(imagineTier(x))}</td><td>${num(x.skill_id)}</td></tr>`).join('')}</tbody></table></div>`;
}
renderTabs=function(){
  const tabs=[['overview','Overview'],['imagines','Imagines'],['skills','Skills'],['buffs','Buffs'],['deaths','Deaths'],['taken','Damage Taken']];
  const el=$('#tabs');if(!el)return;el.innerHTML='';
  tabs.forEach(([id,label])=>{const b=document.createElement('button');b.className='tab'+(id===tab?' active':'');b.textContent=label;b.onclick=()=>{tab=id;renderTabs();const e=selectedEncounter(),rows=e?byDamage(e):[];renderPane(e,rows[selectedPlayer])};el.appendChild(b)});
};
const v1312BasePane=renderPane;
renderPane=function(e,r){
  const pane=$('#pane');if(!pane)return;
  if(tab==='imagines'){if(!r){pane.innerHTML='<div class="empty"><strong>No player rows</strong>This encounter did not contain player contribution data.</div>';}else{renderImaginePane(pane,r);}if(typeof tacticalTag==='function')tacticalTag(document);return;}
  v1312BasePane(e,r);
  if(tab==='overview'&&r){const grid=pane.querySelector('.grid2');if(grid&&!grid.querySelector('[data-imagines-summary]')){const card=document.createElement('div');card.setAttribute('data-imagines-summary','1');card.innerHTML=mini('Imagines',imagineSummary(r));const child=card.firstElementChild;if(child)grid.appendChild(child);}}
};
const current=selectedEncounter();if(current&&!compareMode&&document.querySelector('#tabs')){const rows=byDamage(current);renderTabs();renderPane(current,rows[selectedPlayer]);}
})();
</script>
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{encounter_context::EncounterContextSnapshot, encounter_store, model::{DpsRow, DpsSnapshot, ImagineBadge}};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn archive_exposes_saved_imagine_name_tier_and_tab() {
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("readyalert-encounter-html-v1312-{}-{nonce}", std::process::id()));
        let row = DpsRow {
            uid: 1,
            name: "Alice".into(),
            damage: 12_345,
            imagines: vec![ImagineBadge { skill_id: 3_898, name: "Test Imagine".into(), tier: 5, icon_key: "BI".into() }],
            ..Default::default()
        };
        let snapshot = DpsSnapshot { encounter_ms: 1_000, total_damage: 12_345, rows: vec![row], ..Default::default() };
        encounter_store::archive(&root, &snapshot, &EncounterContextSnapshot::default(), 10).unwrap();
        let path = generate(&root).unwrap();
        let html = fs::read_to_string(path).unwrap();
        assert!(html.contains("Test Imagine"));
        assert!(html.contains("['imagines','Imagines']"));
        assert!(html.contains("imagineTier"));
        let _ = fs::remove_dir_all(root);
    }
}
