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
.attribute-note{color:var(--muted);font-size:11px;margin:0 0 10px;line-height:1.45}
.attribute-table{min-width:560px}.attribute-table th,.attribute-table td{white-space:nowrap}
.attribute-table .attribute-name{font-weight:650;color:#dfe7ee}.attribute-table .attribute-value{color:#c9d4de;font-variant-numeric:tabular-nums}
.attribute-table .coverage{color:var(--muted);font-variant-numeric:tabular-nums}
.attribute-compare{border:1px solid var(--line);border-radius:4px;background:#0e151c;margin:0 0 12px;min-width:0;overflow:hidden;box-shadow:var(--shadow)}
.attribute-compare-head{display:flex;align-items:flex-start;justify-content:space-between;gap:12px;padding:10px 11px;border-bottom:1px solid var(--line);background:#101a22}
.attribute-compare-title{min-width:0}.attribute-compare h3{font-size:13px;margin:0;color:#e7eef5}.attribute-compare h3 span{color:var(--muted);font-weight:600}
.attribute-compare-sub{margin:3px 0 0;color:#8291a0;font-size:10px;line-height:1.4}
.attribute-summary{flex:0 0 auto;border:1px solid #31414d;border-radius:3px;background:#111b23;color:#b9c7d2;padding:4px 7px;font-size:10px;font-weight:700;white-space:nowrap}
.attribute-compare-body{padding:9px 10px 10px}
.attribute-compare .table-wrap{max-width:100%;border-radius:4px}
.attribute-compare .table{min-width:610px;font-size:10px}.attribute-change-table th,.attribute-change-table td{padding:7px 8px}
.attribute-change-table .attribute-name{font-weight:650;color:#dfe7ee}
.attribute-change-table .stat-pair{display:flex;align-items:center;justify-content:flex-end;gap:7px;min-height:22px;font-variant-numeric:tabular-nums;white-space:nowrap}
.attribute-change-table .stat-values{color:#c7d2dc}.attribute-change-table .stat-arrow{color:#667789;margin:0 2px}
.attribute-change-table .stat-delta{display:inline-flex;align-items:center;justify-content:center;min-width:50px;border:1px solid transparent;border-radius:3px;padding:2px 5px;font-size:9px;font-weight:750;line-height:1.25}
.attribute-change-table .stat-delta.up{color:#65d699;border-color:rgba(101,214,153,.25);background:rgba(101,214,153,.07)}
.attribute-change-table .stat-delta.down{color:#ff8187;border-color:rgba(255,129,135,.25);background:rgba(255,129,135,.07)}
.attribute-change-table .stat-delta.missing{color:var(--warn);border-color:rgba(240,197,95,.25);background:rgba(240,197,95,.07)}
.attribute-change-table .coverage{color:#94a4b4;font-variant-numeric:tabular-nums;white-space:nowrap}
.attribute-no-changes{padding:18px 12px;text-align:center;color:var(--muted);font-size:11px}.attribute-no-changes strong{display:block;color:#d7e0e8;font-size:13px;margin-bottom:3px}
@media(max-width:720px){.attribute-compare-head{flex-direction:column}.attribute-summary{align-self:flex-start}.attribute-compare .table{min-width:560px}}
</style>
<script>
(()=>{
  // Existing archive values are raw stat units. Percentage stats are hundredths
  // of 1%; colors below show numeric direction only, never "better" or "worse".
  const percentIds=new Set([11950,11730,11780,12530,12510,11710,11720,11930,11940,11970,11810,12540,11760,11960]);
  const numberOrNull=x=>x===null||x===undefined||x===''?null:Number.isFinite(Number(x))?Number(x):null;
  function statValue(id,x){const v=numberOrNull(x);return v===null?'—':percentIds.has(Number(id))?`${(v/100).toFixed(2)}%`:Math.round(v).toLocaleString();}
  function sameDisplayed(id,a,b){return statValue(id,a)===statValue(id,b);}
  function statDeltaInfo(id,a,b){
    a=numberOrNull(a);b=numberOrNull(b);
    if(a===null||b===null)return{kind:'missing',text:a===b?'':a===null?'A missing':'B missing'};
    if(sameDisplayed(id,a,b))return{kind:'same',text:''};
    const delta=b-a,formatted=percentIds.has(Number(id))?`${(delta/100).toFixed(2)} pp`:Math.round(delta).toLocaleString();
    return{kind:delta>0?'up':'down',text:`${delta>0?'+':''}${formatted}`};
  }
  function stats(r){return [...(r?.encounter_attributes||[])].filter(s=>Number(s.attr_id)>0).sort((a,b)=>String(a.label||'').localeCompare(String(b.label||'')));}
  function coveragePct(s,ms){const observed=Number(s?.observed_ms)||0;return s&&ms>0?Math.min(100,Math.max(0,100*observed/ms)):null;}
  function covered(s,ms){const value=coveragePct(s,ms);return value===null?'—':`${value.toFixed(0)}%`;}
  function renderAttributes(e,r){
    const pane=document.querySelector('#pane');if(!pane)return;
    if(!r){pane.innerHTML='<div class="empty"><strong>No player selected</strong></div>';return;}
    const rows=stats(r);
    if(!rows.length){pane.innerHTML='<div class="empty"><strong>No saved attribute samples</strong>Older encounters do not contain start or fight-average stats. Complete a new encounter with the updated ReadyAlert.</div>';return;}
    const ms=Number(e?.snapshot?.encounter_ms)||0;
    pane.innerHTML=`<p class="attribute-note">Start = pre-pull stat or first valid reading within 1 second. Fight average = time-weighted observed value. Coverage shows how much of the fight was sampled; unknown values are ignored.</p><div class="table-wrap"><table class="table attribute-table"><thead><tr><th>Attribute</th><th>Start</th><th>Fight average</th><th>Coverage</th></tr></thead><tbody>${rows.map(s=>`<tr><td class="attribute-name">${esc(s.label||`Attribute ${s.attr_id}`)}</td><td class="attribute-value">${statValue(s.attr_id,s.initial)}</td><td class="attribute-value">${statValue(s.attr_id,s.average)}</td><td class="coverage">${covered(s,ms)}</td></tr>`).join('')}</tbody></table></div>`;
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
  function pairCell(id,left,right){
    const delta=statDeltaInfo(id,left,right);
    const badge=delta.text?`<span class="stat-delta ${delta.kind}">${esc(delta.text)}</span>`:'';
    return `<span class="stat-pair"><span class="stat-values">${statValue(id,left)} <span class="stat-arrow">→</span> ${statValue(id,right)}</span>${badge}</span>`;
  }
  function changeMagnitude(id,x,y){
    const a=numberOrNull(x?.average)??numberOrNull(x?.initial),b=numberOrNull(y?.average)??numberOrNull(y?.initial);
    if(a===null||b===null)return 1e12;
    return percentIds.has(Number(id))?Math.abs(b-a):Math.abs(b-a)/Math.max(1,Math.abs(a));
  }
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
      container.innerHTML='<div class="attribute-compare-head"><div class="attribute-compare-title"><h3>Attribute changes</h3><p class="attribute-compare-sub">No matching player or stored attributes in both runs. Older encounters cannot be reconstructed.</p></div></div>';return;
    }
    const am=new Map(first.map(s=>[Number(s.attr_id),s])),bm=new Map(second.map(s=>[Number(s.attr_id),s]));
    const allIds=[...new Set([...am.keys(),...bm.keys()])];
    const ids=allIds.filter(id=>{const x=am.get(id),y=bm.get(id);return !sameDisplayed(id,x?.initial,y?.initial)||!sameDisplayed(id,x?.average,y?.average);});
    ids.sort((left,right)=>{
      const diff=changeMagnitude(right,am.get(right),bm.get(right))-changeMagnitude(left,am.get(left),bm.get(left));
      if(Number.isFinite(diff)&&diff!==0)return diff;
      const lx=am.get(left)?.label||bm.get(left)?.label||`Attribute ${left}`,rx=am.get(right)?.label||bm.get(right)?.label||`Attribute ${right}`;
      return String(lx).localeCompare(String(rx));
    });
    const msA=Number(a.snapshot?.encounter_ms)||0,msB=Number(b.snapshot?.encounter_ms)||0;
    const showCoverage=ids.some(id=>{const x=am.get(id),y=bm.get(id),ca=coveragePct(x,msA),cb=coveragePct(y,msB);return ca===null||cb===null||ca<95||cb<95;});
    const hidden=Math.max(0,allIds.length-ids.length);
    const head=`<div class="attribute-compare-head"><div class="attribute-compare-title"><h3>Attribute changes <span>· A → B</span></h3><p class="attribute-compare-sub">Only changed attributes are shown. Green/red means numeric increase/decrease, not better/worse.</p></div><div class="attribute-summary">${ids.length} changed${hidden?` · ${hidden} hidden`:''}</div></div>`;
    if(!ids.length){
      container.innerHTML=head+'<div class="attribute-no-changes"><strong>No attribute changes</strong>Start and fight-average values match at the displayed precision.</div>';return;
    }
    const coverageHead=showCoverage?'<th>Coverage A → B</th>':'';
    const body=ids.map(id=>{const x=am.get(id),y=bm.get(id),label=x?.label||y?.label||`Attribute ${id}`,coverage=showCoverage?`<td class="coverage">${x?covered(x,msA):'—'} → ${y?covered(y,msB):'—'}</td>`:'';return `<tr><td class="attribute-name">${esc(label)}</td><td>${pairCell(id,x?.initial,y?.initial)}</td><td>${pairCell(id,x?.average,y?.average)}</td>${coverage}</tr>`;}).join('');
    container.innerHTML=head+`<div class="attribute-compare-body"><div class="table-wrap"><table class="table attribute-change-table"><thead><tr><th>Attribute</th><th>Start A → B</th><th>Fight average A → B</th>${coverageHead}</tr></thead><tbody>${body}</tbody></table></div></div>`;
  }
  const originalComparison=renderComparison;
  renderComparison=function(){
    originalComparison();const pair=compareIds.map(id=>DATA.find(e=>e.id===id)).filter(Boolean);
    compareStats(pair);
  };
  // The upstream selector replaces its own DOM node whenever the player
  // changes. Delegate at document level so changes after the first still sync.
  document.addEventListener('change',event=>{
    if(event.target?.id!=='compare-player-select')return;
    compareStats(compareIds.map(id=>DATA.find(e=>e.id===id)).filter(Boolean));
  });
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
        assert!(text.contains("Fight average A → B"));
        assert!(text.contains("Only changed attributes are shown"));
        assert!(text.contains("Coverage"));
        assert!(!text.contains("<th>Final</th>"));
        assert!(text.contains("document.addEventListener('change',event=>"));
        assert!(!text.contains("querySelector('#compare-player-select')?.addEventListener"));
        assert!(upgrade(&path).is_err());
        let _=fs::remove_dir_all(root);
    }
}
