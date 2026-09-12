use std::{fs, path::{Path, PathBuf}};

mod prior {
    include!("encounter_archive_v1272.rs");

    pub fn generate_prior(root: &std::path::Path) -> Result<std::path::PathBuf, String> {
        generate(root)
    }
}

/// Final v1.29 visual pass for the completely local Encounter History.
/// Data semantics and the existing archive pipeline stay owned by the prior renderer;
/// this layer only upgrades presentation and derives visual bars from metrics that are
/// already present in the saved encounter.
pub fn generate(root: &Path) -> Result<PathBuf, String> {
    let target = prior::generate_prior(root)?;
    let html = fs::read_to_string(&target)
        .map_err(|e| format!("read generated encounter history for Midnight Tactical upgrade: {e}"))?;

    let html = inject_once(html,"</style>",&format!("{MIDNIGHT_TACTICAL_STYLE}\n</style>"),"Midnight Tactical styles")?;
    let html = inject_once(html,"</body>",&format!("{MIDNIGHT_TACTICAL_SCRIPT}\n</body>"),"Midnight Tactical semantic script")?;

    let pending = target.with_extension("html.v1290.new");
    fs::write(&pending, html.as_bytes()).map_err(|e| format!("write Midnight Tactical encounter history: {e}"))?;
    if target.exists() {
        fs::remove_file(&target).map_err(|e| format!("replace encounter history for Midnight Tactical upgrade: {e}"))?;
    }
    fs::rename(&pending, &target).map_err(|e| format!("install Midnight Tactical encounter history: {e}"))?;
    Ok(target)
}

fn inject_once(mut source:String,anchor:&str,replacement:&str,label:&str)->Result<String,String>{
    let count=source.matches(anchor).count();
    if count!=1{return Err(format!("encounter-history v1.29 {label} expected one anchor, found {count}"));}
    source=source.replacen(anchor,replacement,1);Ok(source)
}

const MIDNIGHT_TACTICAL_STYLE:&str=r#"
/* midnight-tactical-v1290 - archive-only language, separate from native UI. */
:root{--bg:#080c11;--panel:#0f151c;--panel2:#141c25;--panel3:#19232d;--line:#26323e;--line-strong:#354655;--text:#e8eef5;--muted:#8291a2;--accent:#43b6c8;--accent-soft:#16333a;--damage:#f06464;--healing:#49ce8a;--tank:#6697e8;--warn:#d9ad58;--critical:#ff6169;--hover:#17222c;--shadow:0 10px 28px rgba(0,0,0,.22)}
html,body{background:var(--bg);color:var(--text);font:13px/1.42 "Segoe UI Variable Text","Segoe UI",system-ui,sans-serif}body{background-image:linear-gradient(rgba(255,255,255,.012) 1px,transparent 1px),linear-gradient(90deg,rgba(255,255,255,.009) 1px,transparent 1px);background-size:32px 32px}.shell{grid-template-columns:336px minmax(0,1fr)}
.left{background:#0b1117;border-right:1px solid var(--line);box-shadow:8px 0 30px rgba(0,0,0,.16)}.brand{padding:16px 16px 13px;border-bottom:1px solid var(--line);background:#0c1218}.brand h1{font-size:15px;letter-spacing:.045em;text-transform:uppercase}.brand p{font-size:11px;letter-spacing:.12em;text-transform:uppercase;color:#9aa8b7}.local{display:inline-flex;align-items:center;gap:6px;color:#86bea6;font-size:10px;letter-spacing:.04em;text-transform:uppercase}.local:before{content:"";width:6px;height:6px;border-radius:50%;background:var(--healing);box-shadow:0 0 0 3px rgba(73,206,138,.10)}
.tools{gap:6px;padding:9px 10px 0}.tool{min-height:30px;border:1px solid var(--line);background:#111923;border-radius:5px;padding:6px 10px;font-size:11px;font-weight:600;letter-spacing:.02em}.tool:hover{background:#192630;border-color:var(--line-strong)}.tool:focus-visible{outline:2px solid rgba(67,182,200,.42);outline-offset:1px}.tool.primary{border-color:#326575;background:#12252c}.tool.primary:not(:disabled):hover{background:#17313a}.tool:disabled{opacity:.38}.compare-count{font-variant-numeric:tabular-nums;font-size:10px}.search-wrap{padding:9px 10px 10px}.search{height:34px;border-radius:5px;border:1px solid var(--line);background:#101821;padding:0 10px;font-size:12px}.search:focus{border-color:var(--accent);box-shadow:0 0 0 2px rgba(67,182,200,.10)}
.enc-list{padding:6px 7px 10px}.enc-row{grid-template-columns:22px minmax(0,1fr);margin-bottom:2px;border-radius:5px}.compare-check{width:14px;height:14px;accent-color:var(--accent)}.enc-card{border:1px solid transparent;border-radius:5px;padding:8px 9px}.enc-card:hover{background:#121c25}.enc-card.active{position:relative;background:#14232c;border-color:#274653}.enc-card.active:before{content:"";position:absolute;left:-1px;top:7px;bottom:7px;width:2px;background:var(--accent);border-radius:2px}.enc-title{font-size:12px;font-weight:650}.enc-meta{font-size:10px;margin-top:3px}.enc-stat{font-size:11px;margin-top:5px;color:#aeb9c5}
.top{background:rgba(8,12,17,.98);backdrop-filter:none;border-bottom:1px solid var(--line);padding:14px 20px}.top h2{font-size:19px;letter-spacing:-.01em}.subtitle{font-size:11px}.pills{gap:5px}.pill{border:1px solid var(--line);border-radius:4px;padding:3px 7px;background:#101820;color:#aebbc8;font-size:10px;text-transform:uppercase;letter-spacing:.04em}.content{max-width:1320px;padding:15px 20px 52px}
.hero{gap:8px;margin-bottom:12px}.metric{position:relative;overflow:hidden;background:var(--panel);border:1px solid var(--line);border-radius:6px;padding:9px 10px 10px;box-shadow:var(--shadow)}.metric:before{content:"";position:absolute;left:0;right:0;top:0;height:2px;background:var(--line-strong)}.metric:nth-child(2):before,.metric.semantic-damage:before{background:var(--damage)}.metric:nth-child(3):before,.metric.semantic-healing:before{background:var(--healing)}.metric:nth-child(4):before,.metric.semantic-tank:before{background:var(--tank)}.metric.semantic-critical:before{background:var(--critical)}.metric .k{font-size:9px;letter-spacing:.11em;color:#7f8fa1}.metric .v{font-size:18px;margin-top:3px}
.layout{grid-template-columns:260px minmax(0,1fr);gap:11px}.players,.detail,.compare-panel{background:var(--panel);border:1px solid var(--line);border-radius:6px;box-shadow:var(--shadow)}.section-head{padding:9px 11px;border-bottom:1px solid var(--line);font-size:10px;letter-spacing:.09em;text-transform:uppercase;color:#aebbc8;background:#101821}.player-list{padding:5px}.player{position:relative;overflow:hidden;border-radius:4px;padding:7px 8px;grid-template-columns:26px minmax(0,1fr)}.player:after,.compare-player:after{content:"";position:absolute;left:0;bottom:0;height:2px;width:var(--share,0%);background:var(--damage);opacity:.78;pointer-events:none}.player:hover{background:#151f29}.player.active{background:#182630}.player.active:before{content:"";position:absolute;left:0;top:5px;bottom:5px;width:2px;background:var(--accent)}.rank{color:#8fa2b5;font-size:11px}.player-name{font-size:12px}.player-sub{font-size:10px}
.tabs{gap:0;padding:5px 7px;border-bottom:1px solid var(--line);background:#0e151c}.tab{position:relative;border-radius:4px;padding:6px 9px;font-size:11px;color:#96a5b5}.tab:hover{background:#16212b}.tab.active{background:#172831;color:#f1f6fa}.tab.active:after{content:"";position:absolute;left:9px;right:9px;bottom:1px;height:2px;background:var(--accent);border-radius:2px}.pane{padding:11px}.grid2{gap:7px}.mini{position:relative;background:var(--panel2);border:1px solid #24313d;border-radius:5px;padding:8px 9px}.mini:before{content:"";position:absolute;left:0;top:6px;bottom:6px;width:2px;background:#31404e;border-radius:2px}.mini.semantic-damage:before{background:var(--damage)}.mini.semantic-healing:before{background:var(--healing)}.mini.semantic-tank:before{background:var(--tank)}.mini.semantic-critical:before{background:var(--critical)}.mini .label{font-size:10px}.mini .value{font-size:15px;margin-top:1px}
.table-wrap{border:1px solid var(--line);border-radius:5px;background:#0c1218}.table{min-width:650px;font-size:11px}.table th,.table td{padding:7px 8px;border-bottom:1px solid #202b35}.table th{background:#111a22;color:#8191a2;font-size:9px;letter-spacing:.08em}.table tbody tr:nth-child(even){background:rgba(255,255,255,.012)}.table tbody tr:hover{background:#14202a}.table td.semantic-damage{color:#f59a9a}.table td.semantic-healing{color:#86dfad}.table td.semantic-tank{color:#9bbcf0}.table td.semantic-critical{color:#ff9297}.sort-btn:hover{color:#e7f4f8}.sort-arrow{color:var(--accent)}
.recap{border:1px solid #4b2a30;border-radius:5px;background:#110e12}.recap-head{padding:7px 9px;background:#2a171c;color:#ffadb1;font-size:11px}.event{grid-template-columns:66px 66px minmax(0,1fr) 92px;gap:7px;padding:6px 8px;border-top:1px solid #2b2024;font-size:11px}.heal{color:#75dba2}.damage{color:#ff8e92}.empty{padding:45px 18px}.empty strong{font-size:15px}.footer{padding:18px;color:#5f6e7d;font-size:10px;text-transform:uppercase;letter-spacing:.04em}
.compare-top{margin-bottom:10px}.compare-grid{gap:11px}.compare-panel{border-radius:6px}.compare-head{padding:10px 11px;background:#101821}.compare-head h3{font-size:14px}.compare-body{padding:10px}.compare-metrics{gap:6px;margin-bottom:9px}.compare-player{position:relative;overflow:hidden;padding:6px 2px;border-top:1px solid #202b35;font-size:11px}.compare-player .value{font-weight:650}.bench{color:#79c9e2}.compare-skill-list{border-top-color:var(--line)}.compare-skill-player{border:1px solid var(--line)!important;border-radius:5px!important;background:#0d141b!important}.compare-skill-player summary{padding:7px 9px!important}.compare-skill-player summary:hover{background:#14202a!important}.compare-skill-player[open] summary{background:#14212a!important;border-bottom-color:var(--line)!important}.compare-skill-meta{font-size:10px!important}
::-webkit-scrollbar{width:10px;height:10px}::-webkit-scrollbar-track{background:#0b1016}::-webkit-scrollbar-thumb{background:#273440;border:2px solid #0b1016;border-radius:6px}::-webkit-scrollbar-thumb:hover{background:#354754}
@media(max-width:1050px){.shell{grid-template-columns:286px minmax(0,1fr)}.layout{grid-template-columns:220px minmax(0,1fr)}}@media(max-width:720px){.left{background:#0b1117}.content{padding:10px 9px 36px}.hero{gap:6px}.metric{padding:8px}.player-list{padding:4px}.top{padding:12px 10px}}
@media print{:root{color-scheme:light;--bg:#fff;--panel:#fff;--panel2:#f7f9fb;--line:#cdd4dc;--text:#101820;--muted:#566472;--hover:#f2f5f7;--shadow:none}body{background:#fff!important;background-image:none!important;color:#101820}.shell{display:block}.left,.tools,.search-wrap,.footer{display:none!important}.main{width:100%}.top{position:static;background:#fff!important;padding:8px 0;border-bottom:1px solid #9aa6b2}.content{max-width:none;padding:10px 0}.metric,.players,.detail,.compare-panel,.mini,.table-wrap{box-shadow:none!important;background:#fff!important}.tabs{background:#fff}.tab:not(.active){display:none}.pane{padding:8px 0}.layout{grid-template-columns:220px minmax(0,1fr);gap:8px}.player,.compare-player,.recap,.table tr{break-inside:avoid}}
"#;

const MIDNIGHT_TACTICAL_SCRIPT:&str=r#"
<script>
(function(){
function semantic(label,context){const s=String(label||'').toLowerCase(),c=String(context||'').toLowerCase();if(s.includes('death'))return'critical';if(s.includes('heal')||s.includes('overheal'))return'healing';if(s.includes('taken')||s.includes('absorbed')||s.includes('shield')||c.includes('taken')||c.includes('absorbed'))return'tank';if(s.includes('damage')||s.includes('dps')||s==='boss'||s.includes('boss damage'))return'damage';return'neutral'}
function tagMetrics(root){root.querySelectorAll('.metric,.mini').forEach(el=>{const label=el.querySelector('.k,.label')?.textContent||'';el.classList.remove('semantic-damage','semantic-healing','semantic-tank','semantic-critical');const kind=semantic(label,'');if(kind!=='neutral')el.classList.add('semantic-'+kind)})}
function tagTables(root){root.querySelectorAll('table.table').forEach(table=>{const context=table.previousElementSibling?.textContent||table.parentElement?.previousElementSibling?.textContent||'';[...table.querySelectorAll('thead th')].forEach((head,index)=>{const kind=semantic(head.textContent,context);if(kind==='neutral')return;table.querySelectorAll(`tbody tr td:nth-child(${index+1})`).forEach(cell=>cell.classList.add('semantic-'+kind))})})}
function playerBars(rows,selector,root=document){const values=(rows||[]).map(r=>Math.max(0,Number(r.damage)||0)),leader=Math.max(1,...values);root.querySelectorAll(selector).forEach((el,index)=>{const share=Math.max(0,Math.min(100,100*(values[index]||0)/leader));el.style.setProperty('--share',share.toFixed(2)+'%')})}
function compareBars(){const encounters=compareIds.map(id=>DATA.find(e=>e.id===id)).filter(Boolean);document.querySelectorAll('.compare-panel').forEach((panel,index)=>{const encounter=encounters[index];if(encounter)playerBars(byDamage(encounter),'.compare-player',panel)})}
function tacticalTag(root=document){tagMetrics(root);tagTables(root)}
const basePlayerList=renderPlayerList;renderPlayerList=function(e,rows){basePlayerList(e,rows);playerBars(rows,'.player');tacticalTag(document)};
const basePane=renderPane;renderPane=function(e,r){basePane(e,r);tacticalTag(document)};
const baseDetail=renderDetail;renderDetail=function(){baseDetail();tacticalTag(document);if(compareMode)compareBars()};
const baseComparison=renderComparison;renderComparison=function(){baseComparison();tacticalTag(document);compareBars()};
tacticalTag(document);const current=selectedEncounter();if(current&&!compareMode)playerBars(byDamage(current),'.player');if(compareMode)compareBars();
})();
</script>
"#;

#[cfg(test)]
mod tests{
use super::*;use crate::{encounter_context::EncounterContextSnapshot,encounter_store,model::{DpsRow,DpsSnapshot}};use std::time::{SystemTime,UNIX_EPOCH};
#[test]fn midnight_tactical_upgrade_stays_offline_and_preserves_archive_features(){let nonce=SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();let root=std::env::temp_dir().join(format!("readyalert-encounter-html-v1290-{}-{nonce}",std::process::id()));let snapshot=DpsSnapshot{encounter_ms:30_000,total_damage:50_000,total_healing:5_000,total_damage_taken:9_000,rows:vec![DpsRow{uid:7,name:"Alice".into(),damage:50_000,healing:5_000,damage_taken:9_000,is_local:true,..Default::default()}],..Default::default()};let context=EncounterContextSnapshot{scene_id:1151,scene_name:"Void - Towering Ruin".into(),..Default::default()};encounter_store::archive(&root,&snapshot,&context,10).unwrap();let path=generate(&root).unwrap();let html=fs::read_to_string(path).unwrap();assert!(html.contains("midnight-tactical-v1290"));assert!(html.contains("semantic-damage"));assert!(html.contains("playerBars"));assert!(html.contains("comparePlayerSkillSections"));assert!(html.contains("connect-src 'none'"));assert!(!html.contains("<link "));assert!(!html.contains("src=\"http://"));assert!(!html.contains("src=\"https://"));let _=fs::remove_dir_all(root);}
}
