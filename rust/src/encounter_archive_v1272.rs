use std::{fs, path::{Path, PathBuf}};

mod prior {
    include!("encounter_archive_v1270.rs");

    pub fn generate_prior(root: &std::path::Path) -> Result<std::path::PathBuf, String> {
        generate(root)
    }
}

pub fn generate(root: &Path) -> Result<PathBuf, String> {
    let target = prior::generate_prior(root)?;
    let html = fs::read_to_string(&target)
        .map_err(|e| format!("read generated encounter history for comparison upgrade: {e}"))?;

    let html = inject_once(
        html,
        "</style>",
        &format!("{COMPARE_STYLE}\n</style>"),
        "comparison skill styles",
    )?;
    let html = inject_once(
        html,
        "</body>",
        &format!("{COMPARE_SCRIPT}\n</body>"),
        "comparison skill script",
    )?;

    let pending = target.with_extension("html.v1272.new");
    fs::write(&pending, html.as_bytes())
        .map_err(|e| format!("write upgraded encounter history: {e}"))?;
    if target.exists() {
        fs::remove_file(&target)
            .map_err(|e| format!("replace encounter history for comparison upgrade: {e}"))?;
    }
    fs::rename(&pending, &target)
        .map_err(|e| format!("install upgraded encounter history: {e}"))?;
    Ok(target)
}

fn inject_once(mut source: String, anchor: &str, replacement: &str, label: &str) -> Result<String, String> {
    let count = source.matches(anchor).count();
    if count != 1 {
        return Err(format!("encounter-history v1.27.2 {label} expected one anchor, found {count}"));
    }
    source = source.replacen(anchor, replacement, 1);
    Ok(source)
}

const COMPARE_STYLE: &str = r#"
.compare-skill-list{margin-top:14px;padding-top:10px;border-top:1px solid var(--line)}
.compare-skill-player{margin-top:7px;border:1px solid #2b3540;border-radius:8px;background:#131920;overflow:hidden}
.compare-skill-player summary{display:grid;grid-template-columns:minmax(0,1fr) auto;gap:10px;align-items:center;padding:9px 10px;cursor:pointer;list-style:none;user-select:none}
.compare-skill-player summary::-webkit-details-marker{display:none}
.compare-skill-player summary:hover{background:#1a232d}
.compare-skill-player[open] summary{background:#1b252f;border-bottom:1px solid var(--line)}
.compare-skill-name{min-width:0;font-weight:650;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.compare-skill-meta{color:var(--muted);font-size:11px;white-space:nowrap}
.compare-skill-table{overflow:auto}
.compare-skill-table .table{min-width:1080px;font-size:11px}
.compare-skill-table .table th,.compare-skill-table .table td{padding:7px 8px}
.compare-skill-empty{padding:11px 10px;color:var(--muted);font-size:12px}
@media(max-width:1050px){.compare-skill-player summary{grid-template-columns:1fr}.compare-skill-meta{white-space:normal}}
"#;

const COMPARE_SCRIPT: &str = r#"
<script>
function compareSkillRows(r){
    const skills=[...(r?.skills||[])];
    skills.sort((a,b)=>((Number(b.damage)||0)+(Number(b.healing)||0))-((Number(a.damage)||0)+(Number(a.healing)||0)));
    if(!skills.length)return '<div class="compare-skill-empty">No outgoing skill events were retained for this player.</div>';
    return `<div class="compare-skill-table"><table class="table"><thead><tr><th>Skill</th><th>Damage</th><th>Dmg share</th><th>Boss</th><th>Healing</th><th>Effective heal</th><th>Overheal</th><th>Hits</th><th>Crit</th><th>Lucky</th><th>Min</th><th>Max</th></tr></thead><tbody>${skills.map(x=>`<tr><td>${esc(x.name||`Skill ${x.skill_id}`)}</td><td>${compact(x.damage)}</td><td>${pct(x.damage,r.damage)}</td><td>${compact(x.boss_damage)}</td><td>${compact(x.healing)}</td><td>${compact(x.effective_healing)}</td><td>${compact(x.overhealing)}</td><td>${num(x.hits)}</td><td>${pct(x.crits,x.hits)}</td><td>${pct(x.lucky_hits,x.hits)}</td><td>${x.min_value?compact(x.min_value):'—'}</td><td>${x.max_value?compact(x.max_value):'—'}</td></tr>`).join('')}</tbody></table></div>`;
}
function comparePlayerSkillSections(rows){
    if(!rows.length)return '<div class="muted">No player rows.</div>';
    const localIndex=rows.findIndex(r=>r.is_local);
    const openIndex=localIndex>=0?localIndex:0;
    return rows.map((r,i)=>`<details class="compare-skill-player" ${i===openIndex?'open':''}><summary><span class="compare-skill-name">${i+1}. ${esc(r.name||`Player ${r.uid||''}`)}<span class="muted"> · ${esc(r.subprofession_name||'Unknown spec')}</span></span><span class="compare-skill-meta">${compact(r.damage)} dmg · ${(r.skills||[]).length} skill${(r.skills||[]).length===1?'':'s'}</span></summary>${compareSkillRows(r)}</details>`).join('');
}
comparePanel=function(e){
    const s=e.snapshot||{},rows=byDamage(e);
    return `<section class="compare-panel"><div class="compare-head"><h3 class="${isBenchmark(e)?'bench':''}">${esc(encounterTitle(e))}</h3><div class="subtitle">${esc(when(e.ended_unix_ms))} · ${dur(s.encounter_ms)}</div></div><div class="compare-body"><div class="compare-metrics">${mini('Damage',compact(s.total_damage))}${mini('Healing',compact(s.total_healing))}${mini('Taken',compact(s.total_damage_taken))}${mini('Players',rows.length)}</div><div class="section-head" style="padding-left:0;border:0">Damage ranking</div>${rows.map((r,i)=>`<div class="compare-player"><span class="rank">${i+1}</span><span class="name">${esc(r.name||`Player ${r.uid||''}`)}<span class="muted"> · ${esc(r.subprofession_name||'Unknown spec')}</span></span><span class="value">${compact(r.damage)}</span></div>`).join('')||'<div class="muted">No player rows.</div>'}<div class="compare-skill-list"><div class="section-head" style="padding-left:0;border:0">Skill breakdown</div>${comparePlayerSkillSections(rows)}</div></div></section>`;
};
</script>
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        encounter_context::EncounterContextSnapshot,
        encounter_store,
        model::{DpsRow, DpsSnapshot, SkillBreakdown},
    };
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn comparison_includes_expandable_player_skill_details() {
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("readyalert-encounter-html-v1272-{}-{nonce}", std::process::id()));
        let snapshot = DpsSnapshot {
            encounter_ms: 30_000,
            total_damage: 50_000,
            rows: vec![DpsRow {
                uid: 7,
                name: "Alice".into(),
                damage: 50_000,
                is_local: true,
                skills: vec![SkillBreakdown {
                    skill_id: 42,
                    name: "Test Skill".into(),
                    damage: 50_000,
                    hits: 5,
                    crits: 2,
                    max_value: 15_000,
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        };
        let context = EncounterContextSnapshot { scene_id: 1151, scene_name: "Void - Towering Ruin".into(), ..Default::default() };
        encounter_store::archive(&root, &snapshot, &context, 10).unwrap();
        let path = generate(&root).unwrap();
        let html = fs::read_to_string(path).unwrap();
        assert!(html.contains("compare-skill-player"));
        assert!(html.contains("Effective heal"));
        assert!(html.contains("Dmg share"));
        assert!(html.contains("comparePlayerSkillSections"));
        let _ = fs::remove_dir_all(root);
    }
}
