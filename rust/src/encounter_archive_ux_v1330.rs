use std::{fs, path::Path};

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) -> Result<(), String> {
    let count = source.matches(from).count();
    if count != 1 {
        return Err(format!("encounter archive v1.33.0 {label} expected one anchor, found {count}"));
    }
    *source = source.replacen(from, to, 1);
    Ok(())
}

pub fn upgrade(path: &Path) -> Result<(), String> {
    let mut html = fs::read_to_string(path)
        .map_err(|e| format!("read encounter history for v1.33.0 UX upgrade: {e}"))?
        .replace("\r\n", "\n");

    replace_once(
        &mut html,
        r#"<div class="archive-brand-row"><p>Encounter History</p><a class="archive-switch" href="../ChatLogs/index.html" title="Open Chat Archive">Chat Archive</a></div>"#,
        r#"<nav class="archive-tabs" aria-label="Local archives"><a class="active" href="./index.html" aria-current="page">Encounter History</a><a href="../ChatLogs/index.html">Chat Archive</a></nav>"#,
        "segmented archive navigation",
    )?;

    replace_once(
        &mut html,
        "</style>",
        r#"
.archive-tabs{display:grid;grid-template-columns:1fr 1fr;gap:2px;margin-top:7px;padding:2px;border:1px solid var(--line);border-radius:6px;background:#080f14}.archive-tabs a{display:flex;align-items:center;justify-content:center;min-height:26px;border-radius:4px;padding:4px 6px;color:#91a0ae;text-decoration:none;font-size:10px;font-weight:650;white-space:nowrap}.archive-tabs a:hover{background:#17232c;color:#eef5fa}.archive-tabs a.active{background:#1b313a;color:#fff;box-shadow:inset 0 -2px 0 var(--accent)}
.enc-filter-panel{display:none;margin:0 10px 8px;padding:9px;border:1px solid var(--line);border-radius:6px;background:#0f171f}.enc-filter-panel.open{display:block}.enc-filter-grid{display:grid;grid-template-columns:1fr 1fr;gap:7px}.enc-filter-field{display:flex;flex-direction:column;gap:4px;min-width:0}.enc-filter-field.full{grid-column:1/-1}.enc-filter-field>span{color:#78899a;font-size:9px;font-weight:700;letter-spacing:.07em;text-transform:uppercase}.enc-filter-control{width:100%;height:31px;border:1px solid var(--line);background:#101821;border-radius:5px;padding:0 8px;color:#d6dfe8;font:11px "Segoe UI Variable Text","Segoe UI",system-ui,sans-serif;outline:none}.enc-filter-control:focus{border-color:var(--accent);box-shadow:0 0 0 2px rgba(67,182,200,.10)}.enc-filter-actions{display:flex;align-items:center;gap:6px;flex-wrap:wrap;margin-top:8px}.enc-filter-chip{border:1px solid #2f3d49;background:#111b23;color:#9dabb8;border-radius:999px;padding:4px 8px;font-size:9px;font-weight:650;cursor:pointer}.enc-filter-chip:hover{border-color:#49606f;color:#eff7fb}.enc-filter-chip.active{border-color:#357386;background:#16313a;color:#e7f9fc}.enc-filter-summary{margin-left:auto;color:#718293;font-size:9px}.tool.filter-active{border-color:#367383;background:#173039;color:#eaf8fb}.tool .filter-badge{display:inline-flex;align-items:center;justify-content:center;min-width:15px;height:15px;margin-left:4px;border-radius:999px;background:#285463;color:#e2fbff;font-size:9px}.enc-active-filters{display:flex;gap:4px;flex-wrap:wrap;margin-top:7px}.enc-active-filter{border:1px solid #334451;background:#14212a;color:#adbdca;border-radius:999px;padding:3px 7px;font-size:9px;cursor:pointer}.enc-active-filter:hover{border-color:#557083;color:#fff}
.compare-insight{margin-bottom:11px;border:1px solid var(--line);border-radius:7px;background:#0e151c;overflow:hidden;box-shadow:var(--shadow)}.compare-insight-head{display:flex;align-items:center;justify-content:space-between;gap:12px;padding:10px 11px;border-bottom:1px solid var(--line);background:#101a22}.compare-insight-head h3{margin:0;font-size:13px}.compare-insight-controls{display:flex;align-items:center;gap:8px;flex-wrap:wrap}.compare-insight-select{height:30px;max-width:240px;border:1px solid var(--line);background:#111a22;border-radius:5px;color:#dbe5ee;padding:0 8px;font-size:10px}.same-player-toggle{display:inline-flex;align-items:center;gap:5px;color:#a9b5c0;font-size:10px;white-space:nowrap}.same-player-toggle input{accent-color:var(--accent)}.delta-grid{display:grid;grid-template-columns:repeat(4,minmax(110px,1fr));gap:6px;padding:10px}.delta-card{position:relative;border:1px solid #24313d;border-radius:5px;background:var(--panel2);padding:8px 9px}.delta-card .label{color:#7f8fa1;font-size:9px;text-transform:uppercase;letter-spacing:.06em}.delta-card .route{margin-top:3px;color:#cbd7e1;font-size:11px;font-variant-numeric:tabular-nums}.delta-card .delta{margin-top:2px;font-size:11px;font-weight:700;font-variant-numeric:tabular-nums}.delta.positive{color:#65d699}.delta.negative{color:#ff8187}.delta.neutral{color:#94a4b4}.compare-builds{display:grid;grid-template-columns:1fr 1fr;gap:7px;padding:0 10px 10px}.compare-build{border:1px solid #24313d;border-radius:5px;background:#111921;padding:8px}.compare-build .label{color:#7f8fa1;font-size:9px;text-transform:uppercase;letter-spacing:.06em}.compare-build .value{margin-top:4px;color:#cbd6df;font-size:10px;overflow-wrap:anywhere}.skill-delta-wrap{padding:0 10px 10px}.skill-delta-title{color:#9facb8;font-size:10px;font-weight:700;margin:0 0 5px}.skill-delta-table{width:100%;border-collapse:collapse;font-size:10px}.skill-delta-table th,.skill-delta-table td{padding:6px 7px;border-top:1px solid #202b35;text-align:right}.skill-delta-table th:first-child,.skill-delta-table td:first-child{text-align:left}.skill-delta-table th{color:#778797;font-size:9px;text-transform:uppercase;letter-spacing:.05em}.compare-note{padding:10px;color:#8b99a7;font-size:10px}.raid-roster .players{min-width:0}
@media(min-width:1600px){.layout.raid-roster{grid-template-columns:460px minmax(0,1fr)}.raid-roster .player-list{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:2px}.raid-roster .player{min-width:0}}
@media(max-width:1000px){.delta-grid{grid-template-columns:repeat(2,minmax(110px,1fr))}.compare-builds{grid-template-columns:1fr}}@media(max-width:720px){.enc-filter-grid{grid-template-columns:1fr}.enc-filter-field.full{grid-column:auto}.compare-insight-head{align-items:flex-start;flex-direction:column}.delta-grid{grid-template-columns:1fr 1fr}.archive-tabs{max-width:360px}}
</style>"#,
        "styles",
    )?;

    replace_once(
        &mut html,
        "</body>",
        r#"
<script>
(()=>{
const STORE='bpsr-readyalert.encounter-history.filters.v2';
const tools=document.querySelector('.tools'),searchWrap=document.querySelector('.search-wrap');
if(!tools||!searchWrap||typeof DATA==='undefined')return;
const filterBtn=document.createElement('button');filterBtn.id='enc-filters-btn';filterBtn.className='tool';filterBtn.type='button';filterBtn.innerHTML='Filters <span class="filter-badge" hidden>0</span>';
const countAnchor=document.getElementById('compare-count');tools.insertBefore(filterBtn,countAnchor||null);
const panel=document.createElement('div');panel.className='enc-filter-panel';panel.innerHTML=`<div class="enc-filter-grid">
<label class="enc-filter-field"><span>Content</span><select id="enc-content" class="enc-filter-control"><option value="all">All content</option><option value="benchmark">Benchmark</option><option value="dungeon">Dungeon / Chaotic Realm</option><option value="raid">Raid / Season Challenge</option><option value="misc">Misc / other</option></select></label>
<label class="enc-filter-field"><span>Sort</span><select id="enc-sort" class="enc-filter-control"><option value="newest">Newest</option><option value="oldest">Oldest</option><option value="dps">Highest DPS</option><option value="damage">Highest damage</option><option value="longest">Longest duration</option><option value="shortest">Shortest duration</option></select></label>
<label class="enc-filter-field full"><span>Player / class / spec</span><input id="enc-player" class="enc-filter-control" type="search" autocomplete="off" placeholder="e.g. MrHard, Smite, Frostbeam"></label>
<label class="enc-filter-field"><span>From date</span><input id="enc-from" class="enc-filter-control" type="date"></label>
<label class="enc-filter-field"><span>To date</span><input id="enc-to" class="enc-filter-control" type="date"></label>
<label class="enc-filter-field full"><span>Minimum duration (seconds)</span><input id="enc-duration" class="enc-filter-control" type="number" min="0" step="1" placeholder="Any duration"></label>
</div><div class="enc-filter-actions"><button id="enc-benchmark-only" class="enc-filter-chip" type="button">Benchmark only</button><button id="enc-reset-filters" class="enc-filter-chip" type="button">Clear filters</button><span id="enc-filter-summary" class="enc-filter-summary"></span></div><div id="enc-active-filters" class="enc-active-filters"></div>`;
searchWrap.after(panel);
const content=panel.querySelector('#enc-content'),sort=panel.querySelector('#enc-sort'),player=panel.querySelector('#enc-player'),from=panel.querySelector('#enc-from'),to=panel.querySelector('#enc-to'),minDuration=panel.querySelector('#enc-duration'),benchmarkOnly=panel.querySelector('#enc-benchmark-only'),reset=panel.querySelector('#enc-reset-filters'),summary=panel.querySelector('#enc-filter-summary'),activeFilters=panel.querySelector('#enc-active-filters'),badge=filterBtn.querySelector('.filter-badge'),hideMisc=document.getElementById('hide-misc');
const lower=s=>String(s||'').toLocaleLowerCase();
const keyForRow=r=>Number(r?.uid)>0?`u:${Number(r.uid)}`:`n:${lower(r?.name).trim()}`;
const rowLabel=r=>String(r?.name||`Player ${r?.uid||''}`).trim();
const dateOf=e=>{const d=new Date(Number(e?.ended_unix_ms)||0);if(isNaN(d))return'';return [d.getFullYear(),String(d.getMonth()+1).padStart(2,'0'),String(d.getDate()).padStart(2,'0')].join('-')};
const totalDps=e=>{const ms=Number(e?.snapshot?.encounter_ms)||0,damage=Number(e?.snapshot?.total_damage)||0;return ms>0?damage/(ms/1000):0};
function state(){return{content:content.value||'all',sort:sort.value||'newest',player:player.value||'',from:from.value||'',to:to.value||'',duration:minDuration.value||''}}
function save(){try{localStorage.setItem(STORE,JSON.stringify(state()))}catch{}}
function load(){try{return JSON.parse(localStorage.getItem(STORE)||'{}')||{}}catch{return{}}}
function contentMatch(e,value){const pt=Number(e.context?.play_type)||0;if(value==='benchmark')return isBenchmark(e);if(value==='dungeon')return !isBenchmark(e)&&[2,9,17,19].includes(pt);if(value==='raid')return !isBenchmark(e)&&pt===18;if(value==='misc')return !isMainContent(e);return true}
function queryMatch(e,q){q=lower(q.trim());if(!q)return true;const rows=e.snapshot?.rows||[],hay=lower([encounterTitle(e),e.target_name,e.context?.difficulty,e.context?.benchmark_name,playLabel(e),...rows.map(r=>r.name),...rows.map(r=>r.subprofession_name)].join(' '));return q.split(/\s+/).filter(Boolean).every(term=>hay.includes(term))}
function playerMatch(e,q){q=lower(q.trim());if(!q)return true;return (e.snapshot?.rows||[]).some(r=>lower(`${r.name||''} ${r.subprofession_name||''}`).includes(q))}
function applySort(rows,mode){rows.sort((a,b)=>{if(mode==='oldest')return (Number(a.ended_unix_ms)||0)-(Number(b.ended_unix_ms)||0);if(mode==='dps')return totalDps(b)-totalDps(a);if(mode==='damage')return (Number(b.snapshot?.total_damage)||0)-(Number(a.snapshot?.total_damage)||0);if(mode==='longest')return (Number(b.snapshot?.encounter_ms)||0)-(Number(a.snapshot?.encounter_ms)||0);if(mode==='shortest')return (Number(a.snapshot?.encounter_ms)||0)-(Number(b.snapshot?.encounter_ms)||0);return (Number(b.ended_unix_ms)||0)-(Number(a.ended_unix_ms)||0)});return rows}
function renderFilterUi(s){const n=[s.content!=='all'?s.content:'',s.sort!=='newest'?s.sort:'',s.player,s.from,s.to,s.duration].filter(Boolean).length;badge.hidden=n===0;badge.textContent=String(n);filterBtn.classList.toggle('filter-active',n>0);benchmarkOnly.classList.toggle('active',s.content==='benchmark');summary.textContent=`${filtered.length.toLocaleString()} shown / ${DATA.length.toLocaleString()} saved`;activeFilters.replaceChildren();const defs=[];if(s.content!=='all')defs.push([content.options[content.selectedIndex]?.text||s.content,()=>{content.value='all'}]);if(s.sort!=='newest')defs.push(['Sort: '+(sort.options[sort.selectedIndex]?.text||s.sort),()=>{sort.value='newest'}]);if(s.player)defs.push(['Player: '+s.player,()=>{player.value=''}]);if(s.from)defs.push(['From: '+s.from,()=>{from.value=''}]);if(s.to)defs.push(['To: '+s.to,()=>{to.value=''}]);if(s.duration)defs.push(['Min: '+s.duration+'s',()=>{minDuration.value=''}]);for(const [label,clear] of defs){const b=document.createElement('button');b.type='button';b.className='enc-active-filter';b.textContent=label+' ×';b.onclick=()=>{clear();applySearch()};activeFilters.append(b)}}
applySearch=function(){const q=(search.value||'').trim(),s=state(),hide=hideMisc?.getAttribute('aria-pressed')==='true',minMs=Math.max(0,Number(s.duration)||0)*1000;filtered=DATA.filter(e=>{const d=dateOf(e);if(hide&&s.content!=='misc'&&!isMainContent(e))return false;if(!contentMatch(e,s.content))return false;if(!queryMatch(e,q)||!playerMatch(e,s.player))return false;if(s.from&&d&&d<s.from)return false;if(s.to&&d&&d>s.to)return false;if(minMs&&(Number(e.snapshot?.encounter_ms)||0)<minMs)return false;return true});applySort(filtered,s.sort);compareIds=compareIds.filter(id=>filtered.some(e=>e.id===id));if(!filtered.some(e=>e.id===selectedId))selectedId=filtered[0]?.id??null;selectedPlayer=0;tab='overview';compareMode=false;renderList();renderDetail();renderFilterUi(s);save()};
filterBtn.onclick=()=>{panel.classList.toggle('open');filterBtn.setAttribute('aria-expanded',panel.classList.contains('open')?'true':'false')};
benchmarkOnly.onclick=()=>{content.value=content.value==='benchmark'?'all':'benchmark';applySearch()};
reset.onclick=()=>{content.value='all';sort.value='newest';player.value='';from.value='';to.value='';minDuration.value='';search.value='';if(typeof setHideMisc==='function'){setHideMisc(false);if(typeof saveHideMisc==='function')saveHideMisc(false)}applySearch();search.focus()};
for(const el of [content,sort,player,from,to,minDuration])el.addEventListener(el.tagName==='SELECT'?'change':'input',()=>{if(el===content&&content.value==='misc'&&typeof setHideMisc==='function'){setHideMisc(false);if(typeof saveHideMisc==='function')saveHideMisc(false)}applySearch()});

let comparePlayerKey='',samePlayerOnly=true;
function fmtRate(v){return Number.isFinite(v)?`${v.toFixed(1)}%`:'—'}
function rowRate(n,d){return Number(d)>0?100*Number(n||0)/Number(d):0}
function imagineSummary(r){const xs=[...(r?.imagines||[])];return xs.length?xs.map(x=>`${x.name||`Imagine ${x.skill_id||'?'}`} T${Math.max(0,Number(x.tier)||0)}`).join(' · '):'None recorded'}
function deltaText(a,b,rate=false){const d=b-a;if(rate)return `${d>=0?'+':''}${d.toFixed(1)} pp`;if(!Number.isFinite(d))return'—';if(a===0)return d===0?'0':`${d>0?'+':''}${compact(d)}`;const p=100*d/Math.abs(a);return `${d>=0?'+':''}${p.toFixed(1)}%`}
function deltaClass(a,b,better='up'){if(b===a)return'neutral';const improved=better==='down'?b<a:b>a;return improved?'positive':'negative'}
function metricCard(label,a,b,format,better='up',rate=false){return `<div class="delta-card"><div class="label">${esc(label)}</div><div class="route">${esc(format(a))} → ${esc(format(b))}</div><div class="delta ${deltaClass(a,b,better)}">${esc(deltaText(a,b,rate))}</div></div>`}
function skillDeltas(a,b){const map=new Map();for(const [side,row] of [['a',a],['b',b]])for(const x of row?.skills||[]){const key=Number(x.skill_id)||lower(x.name);if(!map.has(key))map.set(key,{name:x.name||`Skill ${x.skill_id||''}`,a:0,b:0});map.get(key)[side]=Number(x.damage)||0}return [...map.values()].map(x=>({...x,d:x.b-x.a})).sort((x,y)=>Math.abs(y.d)-Math.abs(x.d)).slice(0,8)}
function commonPlayers(pair){const a=byDamage(pair[0]),b=byDamage(pair[1]),bm=new Map(b.map(r=>[keyForRow(r),r]));return a.filter(r=>bm.has(keyForRow(r))).map(r=>({key:keyForRow(r),label:rowLabel(r),a:r,b:bm.get(keyForRow(r))}))}
function applySamePlayerVisibility(pair,key,only){document.querySelectorAll('.compare-panel').forEach((panelEl,index)=>{const rows=byDamage(pair[index]);panelEl.querySelectorAll('.compare-player').forEach((el,i)=>{el.hidden=!!only&&keyForRow(rows[i])!==key});panelEl.querySelectorAll('.compare-skill-player').forEach((el,i)=>{el.hidden=!!only&&keyForRow(rows[i])!==key})})}
function renderCompareInsights(pair){const grid=document.querySelector('.compare-grid');if(!grid)return;document.querySelector('.compare-insight')?.remove();const common=commonPlayers(pair),preferred=common.find(x=>x.a?.is_local||x.b?.is_local)||common[0];if(!common.some(x=>x.key===comparePlayerKey))comparePlayerKey=preferred?.key||'';const picked=common.find(x=>x.key===comparePlayerKey)||null;const section=document.createElement('section');section.className='compare-insight';if(!picked){const a=pair[0].snapshot||{},b=pair[1].snapshot||{};section.innerHTML=`<div class="compare-insight-head"><h3>Encounter delta</h3></div><div class="compare-note">No same player appears in both encounters, so this summary compares encounter totals.</div><div class="delta-grid">${metricCard('Total damage',Number(a.total_damage)||0,Number(b.total_damage)||0,compact)}${metricCard('Total healing',Number(a.total_healing)||0,Number(b.total_healing)||0,compact)}${metricCard('Damage taken',Number(a.total_damage_taken)||0,Number(b.total_damage_taken)||0,compact,'down')}${metricCard('Duration',Number(a.encounter_ms)||0,Number(b.encounter_ms)||0,dur,'down')}</div>`;grid.before(section);return}const a=picked.a,b=picked.b,critA=rowRate(a.crits,a.hits),critB=rowRate(b.crits,b.hits),luckA=rowRate(a.lucky_hits,a.hits),luckB=rowRate(b.lucky_hits,b.hits),bossA=rowRate(a.boss_damage,a.damage),bossB=rowRate(b.boss_damage,b.damage),skills=skillDeltas(a,b);section.innerHTML=`<div class="compare-insight-head"><h3>${pair.every(isBenchmark)?'Benchmark player delta':'Same-player delta'}</h3><div class="compare-insight-controls"><select id="compare-player-select" class="compare-insight-select">${common.map(x=>`<option value="${esc(x.key)}" ${x.key===picked.key?'selected':''}>${esc(x.label)}</option>`).join('')}</select><label class="same-player-toggle"><input id="same-player-only" type="checkbox" ${samePlayerOnly?'checked':''}>Same player only</label></div></div><div class="delta-grid">${metricCard('DPS',Number(a.dps)||0,Number(b.dps)||0,compact)}${metricCard('Damage',Number(a.damage)||0,Number(b.damage)||0,compact)}${metricCard('Active DPS',Number(a.active_dps)||0,Number(b.active_dps)||0,compact)}${metricCard('Crit',critA,critB,fmtRate,'up',true)}${metricCard('Lucky',luckA,luckB,fmtRate,'up',true)}${metricCard('Boss damage',bossA,bossB,fmtRate,'up',true)}${metricCard('Effective healing',Number(a.effective_healing)||0,Number(b.effective_healing)||0,compact)}${metricCard('Deaths',Number(a.deaths)||0,Number(b.deaths)||0,n=>num(n),'down')}</div><div class="compare-builds"><div class="compare-build"><div class="label">First Imagines</div><div class="value">${esc(imagineSummary(a))}</div></div><div class="compare-build"><div class="label">Second Imagines</div><div class="value">${esc(imagineSummary(b))}</div></div></div>${skills.length?`<div class="skill-delta-wrap"><div class="skill-delta-title">Largest skill damage changes</div><table class="skill-delta-table"><thead><tr><th>Skill</th><th>First</th><th>Second</th><th>Delta</th></tr></thead><tbody>${skills.map(x=>`<tr><td>${esc(x.name)}</td><td>${compact(x.a)}</td><td>${compact(x.b)}</td><td class="${x.d>0?'delta positive':x.d<0?'delta negative':'delta neutral'}">${x.d>=0?'+':''}${compact(x.d)}</td></tr>`).join('')}</tbody></table></div>`:''}`;grid.before(section);section.querySelector('#compare-player-select').onchange=ev=>{comparePlayerKey=ev.target.value;renderCompareInsights(pair)};section.querySelector('#same-player-only').onchange=ev=>{samePlayerOnly=ev.target.checked;applySamePlayerVisibility(pair,comparePlayerKey,samePlayerOnly)};applySamePlayerVisibility(pair,comparePlayerKey,samePlayerOnly)}
const priorComparison=renderComparison;renderComparison=function(){priorComparison();const pair=compareIds.map(id=>DATA.find(e=>e.id===id)).filter(Boolean);if(pair.length!==2)return;const h=document.querySelector('.top h2');if(h&&pair.every(isBenchmark))h.textContent='Benchmark Comparison';renderCompareInsights(pair)};
const priorPlayerList=renderPlayerList;renderPlayerList=function(e,rows){priorPlayerList(e,rows);const layout=document.querySelector('.layout');if(layout)layout.classList.toggle('raid-roster',rows.length>=16)};

const saved=load();if(saved.content&&[...content.options].some(o=>o.value===saved.content))content.value=saved.content;if(saved.sort&&[...sort.options].some(o=>o.value===saved.sort))sort.value=saved.sort;if(saved.player)player.value=saved.player;if(saved.from)from.value=saved.from;if(saved.to)to.value=saved.to;if(saved.duration)minDuration.value=saved.duration;applySearch();
})();
</script>
</body>"#,
        "advanced filters and comparison script",
    )?;

    fs::write(path, html.as_bytes())
        .map_err(|e| format!("write encounter history v1.33.0 UX upgrade: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn upgrades_navigation_filters_and_compare_markers() {
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("readyalert-encounter-ux-v1330-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("index.html");
        fs::write(&path, r#"<html><head><style></style></head><body><div class="archive-brand-row"><p>Encounter History</p><a class="archive-switch" href="../ChatLogs/index.html" title="Open Chat Archive">Chat Archive</a></div></body></html>"#).unwrap();
        upgrade(&path).unwrap();
        let html = fs::read_to_string(&path).unwrap();
        assert!(html.contains("archive-tabs"));
        assert!(html.contains("enc-filter-panel"));
        assert!(html.contains("Benchmark only"));
        assert!(html.contains("Same player only"));
        assert!(html.contains("Largest skill damage changes"));
        assert!(html.contains("raid-roster"));
        fs::remove_dir_all(dir).unwrap();
    }
}
