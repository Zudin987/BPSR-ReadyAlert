//! Add measured character-stat summaries to the existing offline History UI.
//! No graph, build simulator or network request is added.
use std::{fs, path::Path};

pub fn upgrade(path: &Path) -> Result<(), String> {
    let source = fs::read_to_string(path).map_err(|err| format!("read attribute history HTML: {err}"))?;
    if source.matches("</body>").count() != 1 || source.contains("readyalert-encounter-attributes-v1") {
        return Err("attribute history HTML expected one unmodified closing body".into());
    }
    let updated = source.replacen("</body>", &format!("{STYLES_AND_SCRIPT}\n</body>"), 1);
    fs::write(path, updated).map_err(|err| format!("write attribute history HTML: {err}"))
}

const STYLES_AND_SCRIPT: &str = r#"
<style id="readyalert-encounter-attributes-v1">
.attribute-note{color:var(--muted);font-size:11px;margin:0 0 10px;line-height:1.5}
.attribute-table{min-width:650px}.attribute-table th,.attribute-table td{white-space:nowrap}
.attribute-table .missing{color:var(--muted)}
.attribute-compare{border:1px solid var(--line);border-radius:4px;background:var(--panel);padding:12px;margin:0 0 12px;min-width:0}
.attribute-compare h3{font-size:13px;margin:0 0 7px}
.attribute-compare .table-wrap{max-width:100%}
.attribute-compare .table{min-width:760px;font-size:11px}
.attribute-compare .stat-pair{font-variant-numeric:tabular-nums;white-space:nowrap}
.attribute-compare .stat-delta{color:var(--accent);margin-left:5px}
</style>
<script>
(()=>{
  // Existing archive values are raw stat units. Percentage stats are hundredths
  // of 1%; do not infer whether increasing a stat increases actual DPS.
  const percentIds=new Set([11950,11730,11780,12530,12510,11710,11720,11930,11940,11970,11810,12540,11760,11960]);
  const numberOrNull=x=>x===null||x===undefined||x===''?null:Number.isFinite(Number(x))?Number(x):null;
  function statValue(id,x){const v=numberOrNull(x);return v===null?'—':percentIds.has(Number(id))?`${(v/100).toFixed(2)}%`:Math.round(v).toLocaleString();}
  function statDelta(id,a,b){a=numberOrNull(a);b=numberOrNull(b);if(a===null||b===null)return'';const delta=b-a,formatted=percentIds.has(Number(id))?`${(delta/100).toFixed(2)} pp`:Math.round(delta).toLocaleString();return ` (${delta>0?'+':''}${formatted})`;}
  function stats(r){return [...(r?.encounter_attributes||[])].filter(s=>Number(s.attr_id)>0).sort((a,b)=>String(a.label||'').localeCompare(String(b.label||'')));}
  function covered(s,ms){const observed=Number(s?.observed_ms)||0;return ms>0?`${Math.min(100,Math.max(0,100*observed/ms)).toFixed(0)}%`:'—';}
  function renderAttributes(e,r){
    const pane=document.querySelector('#pane');if(!pane)return;
    if(!r){pane.innerHTML='<div class="empty"><strong>No player selected</strong></div>';return;}
    const rows=stats(r);
    if(!rows.length){pane.innerHTML='<div class="empty"><strong>No saved attribute samples</strong>Older encounters do not contain initial or average stats. Complete a new encounter with the updated ReadyAlert.</div>';return;}
    const ms=Number(e?.snapshot?.encounter_ms)||0;
    pane.innerHTML=`<p class="attribute-note">Initial = pre-pull stat or first valid reading within 1 second. Average = time-weighted sampled stat during known intervals. Final = last observed value. Coverage shows how much of the fight contributed to each average; unknown is never treated as zero. These are observations, not predicted stat DPS contributions.</p><div class="table-wrap"><table class="table attribute-table"><thead><tr><th>Attribute</th><th>Initial</th><th>Average</th><th>Final</th><th>Coverage</th></tr></thead><tbody>${rows.map(s=>`<tr><td>${esc(s.label||`Attribute ${s.attr_id}`)}</td><td>${statValue(s.attr_id,s.initial)}</td><td>${statValue(s.attr_id,s.average)}</td><td>${statValue(s.attr_id,s.final_value)}</td><td>${covered(s,ms)}</td></tr>`).join('')}</tbody></table></div>`;
  }
  const originalTabs=renderTabs;
  renderTabs=function(){
    originalTabs();const tabs=document.querySelector('#tabs');if(!tabs)return;
    const button=document.createElement('button');button.className='tab'+(tab==='attributes'?' active':'');button.type='button';button.textContent='Attributes';
    button.onclick=()=>{tab='attributes';renderTabs();const e=selectedEncounter();renderPane(e,e?byDamage(e)[selectedPlayer]:null)};
    tabs.append(button);
  };
  const originalPane=renderPane;
  renderPane=function(e,r){if(tab==='attributes'){renderAttributes(e,r);return;}originalPane(e,r)};

  function key(r){return Number(r?.uid)>0?`u:${Number(r.uid)}`:`n:${String(r?.name||'').trim().toLocaleLowerCase()}`;}
  function compareStats(pair){
    document.querySelector('.attribute-compare')?.remove();
    if(pair.length!==2)return;
    const a=pair[0],b=pair[1],select=document.querySelector('#compare-player-select');
    const playerKey=select?.value||key(byDamage(a).find(r=>r.is_local)||byDamage(a)[0]);
    const playerA=byDamage(a).find(r=>key(r)===playerKey),playerB=byDamage(b).find(r=>key(r)===playerKey);
    const container=document.createElement('section');container.className='attribute-compare';
    const grid=document.querySelector('.compare-grid');if(!grid)return;
    grid.before(container);
    const first=stats(playerA),second=stats(playerB);
    if(!playerA||!playerB||(!first.length&&!second.length)){
      container.innerHTML='<h3>Character attribute comparison</h3><p class="attribute-note">No matching player or stored attributes in both runs. Older encounters cannot be reconstructed.</p>';return;
    }
    const am=new Map(first.map(s=>[Number(s.attr_id),s])),bm=new Map(second.map(s=>[Number(s.attr_id),s]));
    const ids=[...new Set([...am.keys(),...bm.keys()])].sort((x,y)=>x-y);
    const pairCell=(id,left,right)=>`${statValue(id,left)} → ${statValue(id,right)}<span class="stat-delta">${esc(statDelta(id,left,right))}</span>`;
    container.innerHTML=`<h3>Character attribute comparison · A → B</h3><p class="attribute-note">Identical stat IDs are aligned. Deltas are descriptive, not a claim that the stat change caused the DPS change. Only compare similar encounters and check coverage.</p><div class="table-wrap"><table class="table"><thead><tr><th>Attribute</th><th>Initial A → B</th><th>Average A → B</th><th>Final A → B</th><th>Coverage A → B</th></tr></thead><tbody>${ids.map(id=>{const x=am.get(id),y=bm.get(id),label=x?.label||y?.label||`Attribute ${id}`;return `<tr><td>${esc(label)}</td><td class="stat-pair">${pairCell(id,x?.initial,y?.initial)}</td><td class="stat-pair">${pairCell(id,x?.average,y?.average)}</td><td class="stat-pair">${pairCell(id,x?.final_value,y?.final_value)}</td><td>${x?covered(x,Number(a.snapshot?.encounter_ms)||0):'—'} → ${y?covered(y,Number(b.snapshot?.encounter_ms)||0):'—'}</td></tr>`}).join('')}</tbody></table></div>`;
  }
  const originalComparison=renderComparison;
  renderComparison=function(){
    originalComparison();const pair=compareIds.map(id=>DATA.find(e=>e.id===id)).filter(Boolean);
    compareStats(pair);
    // The existing player selector updates its own analysis without rebuilding
    // the whole comparison page. Keep the attributes panel synchronized too.
    document.querySelector('#compare-player-select')?.addEventListener('change',()=>compareStats(pair));
  };
  if(!compareMode){const e=selectedEncounter();if(e){renderTabs();renderPane(e,byDamage(e)[selectedPlayer])}}
})();
</script>
"#;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn injects_one_attribute_tab_and_compare_and_rejects_duplicates() {
        let root=std::env::temp_dir().join(format!("readyalert-attr-ui-{}",std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let path=root.join("index.html");
        fs::write(&path,"<html><body>example</body></html>").unwrap();
        upgrade(&path).unwrap();
        let text=fs::read_to_string(&path).unwrap();
        assert!(text.contains("Initial A → B"));
        assert!(text.contains("Coverage"));
        assert!(upgrade(&path).is_err());
        let _=fs::remove_dir_all(root);
    }
}
