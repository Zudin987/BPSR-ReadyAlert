use std::{fs, path::Path};

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) -> Result<(), String> {
    let count = source.matches(from).count();
    if count != 1 {
        return Err(format!("chat archive v1.33.0 {label} expected one anchor, found {count}"));
    }
    *source = source.replacen(from, to, 1);
    Ok(())
}

pub fn upgrade(path: &Path) -> Result<(), String> {
    let mut html = fs::read_to_string(path)
        .map_err(|e| format!("read chat archive for v1.33.0 UX upgrade: {e}"))?
        .replace("\r\n", "\n");

    replace_once(
        &mut html,
        r#"<div class="archive-brand-row"><p>Chat Archive</p><a class="archive-switch" href="../EncounterHistory/index.html" title="Open Encounter History">Encounter History</a></div>"#,
        r#"<nav class="archive-tabs" aria-label="Local archives"><a href="../EncounterHistory/index.html">Encounter History</a><a class="active" href="./index.html" aria-current="page">Chat Archive</a></nav>"#,
        "segmented archive navigation",
    )?;

    replace_once(
        &mut html,
        "</style>",
        r#"
.archive-tabs{display:grid;grid-template-columns:1fr 1fr;gap:2px;margin-top:7px;padding:2px;border:1px solid var(--line);border-radius:6px;background:#0e141a}.archive-tabs a{display:flex;align-items:center;justify-content:center;min-height:26px;border-radius:4px;padding:4px 6px;color:#93a0ae;text-decoration:none;font-size:10px;font-weight:650;white-space:nowrap}.archive-tabs a:hover{background:#19232c;color:#eef4f9}.archive-tabs a.active{background:#20303b;color:#fff;box-shadow:inset 0 -2px 0 var(--accent)}
.filters.enhanced{grid-template-columns:minmax(240px,1fr) auto auto}.archive-filter-btn,.archive-clear-btn{height:36px;border:1px solid #343d49;background:var(--panel);border-radius:7px;padding:0 11px;cursor:pointer;font-size:12px;font-weight:650;white-space:nowrap}.archive-filter-btn:hover,.archive-clear-btn:hover{background:var(--hover)}.archive-filter-btn.active{border-color:#3e748f;background:#19303c;color:#e9f7ff}.archive-filter-count{display:inline-flex;align-items:center;justify-content:center;min-width:17px;height:17px;margin-left:5px;padding:0 5px;border-radius:999px;background:#28506a;color:#dff4ff;font-size:10px}.advanced-filter-panel{display:none;margin-top:9px;padding:10px;border:1px solid var(--line);border-radius:8px;background:#121820;box-shadow:var(--shadow)}.advanced-filter-panel.open{display:block}.advanced-filter-grid{display:grid;grid-template-columns:repeat(4,minmax(130px,1fr));gap:9px}.filter-field{display:flex;flex-direction:column;gap:5px;min-width:0}.filter-field>span{color:#8f9baa;font-size:10px;font-weight:700;letter-spacing:.06em;text-transform:uppercase}.filter-checks{display:flex;align-items:center;gap:14px;flex-wrap:wrap;margin-top:10px}.filter-check{display:inline-flex;align-items:center;gap:6px;color:#b6c0cb;font-size:12px;cursor:pointer}.filter-check input{accent-color:var(--accent)}.quick-filters{display:flex;align-items:center;gap:6px;flex-wrap:wrap;margin-top:10px}.quick-filter{border:1px solid #303a45;background:#171e26;color:#aab6c3;border-radius:999px;padding:4px 8px;font-size:10px;cursor:pointer}.quick-filter:hover{border-color:#466274;color:#edf7fd}.quick-filter.active{border-color:#3d7691;background:#193444;color:#e7f8ff}.active-filter-chips{display:flex;gap:5px;flex-wrap:wrap;margin-top:8px}.active-filter-chip{border:1px solid #34434f;background:#17232c;color:#b9c7d3;border-radius:999px;padding:3px 7px;font-size:10px;cursor:pointer}.active-filter-chip:hover{border-color:#557083;color:#fff}.search-hint{color:#687584;font-size:10px;margin-top:8px}.summary strong{color:#c7d2de;font-weight:650}
@media(max-width:980px){.advanced-filter-grid{grid-template-columns:repeat(2,minmax(130px,1fr))}}@media(max-width:760px){.filters.enhanced{grid-template-columns:1fr auto auto}.advanced-filter-grid{grid-template-columns:1fr}.archive-tabs{max-width:360px}}
</style>"#,
        "styles",
    )?;

    replace_once(
        &mut html,
        "</body>",
        r#"
<script>
(()=>{
const STORE='bpsr-readyalert.chat-archive.filters.v2';
const messages=[...document.querySelectorAll('.message')];
const days=[...document.querySelectorAll('.day')];
const dateButtons=[...document.querySelectorAll('.date-btn')];
const filters=document.querySelector('.filters');
const search=document.getElementById('search');
const channel=document.getElementById('channel');
const sender=document.getElementById('sender');
const result=document.getElementById('result-count');
const empty=document.getElementById('empty');
if(!filters||!search||!channel||!sender)return;
filters.classList.add('enhanced');

const filterBtn=document.createElement('button');filterBtn.type='button';filterBtn.className='archive-filter-btn';filterBtn.innerHTML='Filters <span class="archive-filter-count" hidden>0</span>';
const clearBtn=document.createElement('button');clearBtn.type='button';clearBtn.className='archive-clear-btn';clearBtn.textContent='Clear';clearBtn.title='Clear search and filters';
filters.append(filterBtn,clearBtn);

const panel=document.createElement('div');panel.className='advanced-filter-panel';panel.innerHTML=`<div class="advanced-filter-grid">
<label class="filter-field"><span>Channel</span><div id="adv-channel-slot"></div></label>
<label class="filter-field"><span>Sender</span><div id="adv-sender-slot"></div></label>
<label class="filter-field"><span>From date</span><input class="control" id="adv-date-from" type="date"></label>
<label class="filter-field"><span>To date</span><input class="control" id="adv-date-to" type="date"></label>
</div><div class="filter-checks">
<label class="filter-check"><input id="adv-exact" type="checkbox">Exact phrase</label>
<label class="filter-check"><input id="adv-image" type="checkbox">Has image</label>
</div><div class="quick-filters" id="adv-quick"></div><div class="active-filter-chips" id="adv-active-chips"></div><div class="search-hint">Tip: normal search matches all typed words in any order. Enable Exact phrase when word order matters. <b>Ctrl+F</b> or <b>/</b> focuses this search.</div>`;
filters.after(panel);
panel.querySelector('#adv-channel-slot').append(channel);panel.querySelector('#adv-sender-slot').append(sender);
const dateFrom=panel.querySelector('#adv-date-from'),dateTo=panel.querySelector('#adv-date-to'),exact=panel.querySelector('#adv-exact'),hasImage=panel.querySelector('#adv-image'),quick=panel.querySelector('#adv-quick'),chips=panel.querySelector('#adv-active-chips'),countBadge=filterBtn.querySelector('.archive-filter-count');
channel.classList.add('control');sender.classList.add('control');
for(const el of document.querySelectorAll('.message-text'))if(!el.dataset.original)el.dataset.original=el.textContent||'';
const lower=s=>String(s||'').toLocaleLowerCase();
const activeDate=()=>document.querySelector('.date-btn.active')?.dataset.date||'all';
const senderNames=new Set(messages.map(m=>lower(m.dataset.sender)).filter(Boolean));

function state(){return{q:search.value||'',channel:channel.value||'',sender:sender.value||'',from:dateFrom.value||'',to:dateTo.value||'',exact:!!exact.checked,image:!!hasImage.checked,date:activeDate()}}
function save(){try{localStorage.setItem(STORE,JSON.stringify(state()))}catch{}}
function load(){try{return JSON.parse(localStorage.getItem(STORE)||'{}')||{}}catch{return{}}}
function matchQuery(name,body,q,isExact){q=lower(q.trim());if(!q)return true;const hay=lower(`${name} ${body}`);if(isExact)return hay.includes(q);const terms=q.split(/\s+/).filter(Boolean);return terms.every(term=>hay.includes(term))}
function highlight(el,q,isExact){if(!el)return;const original=el.dataset.original||'';el.replaceChildren();const query=lower(q.trim());if(!query){el.textContent=original;return;}const terms=isExact?[query]:[...new Set(query.split(/\s+/).filter(Boolean))].sort((a,b)=>b.length-a.length);if(!terms.length){el.textContent=original;return;}const hay=lower(original);let pos=0;while(pos<original.length){let best=-1,bestTerm='';for(const term of terms){const at=hay.indexOf(term,pos);if(at>=0&&(best<0||at<best||(at===best&&term.length>bestTerm.length))){best=at;bestTerm=term}}if(best<0){el.append(document.createTextNode(original.slice(pos)));break}if(best>pos)el.append(document.createTextNode(original.slice(pos,best)));const mark=document.createElement('mark');mark.textContent=original.slice(best,best+bestTerm.length);el.append(mark);pos=best+bestTerm.length}if(!original)el.textContent=''}
function setChannel(value){channel.value=value||'';applyAdvanced()}
function dateLabel(v){return v||''}
function renderQuick(){quick.replaceChildren();const preferred=['World','Team','Guild','Private'];for(const name of preferred){if(![...channel.options].some(o=>o.value===name))continue;const b=document.createElement('button');b.type='button';b.className='quick-filter'+(channel.value===name?' active':'');b.textContent=name;b.onclick=()=>setChannel(channel.value===name?'':name);quick.append(b)}const today=new Date(),yesterday=new Date(today);yesterday.setDate(today.getDate()-1);for(const [label,d] of [['Today',today],['Yesterday',yesterday]]){const iso=[d.getFullYear(),String(d.getMonth()+1).padStart(2,'0'),String(d.getDate()).padStart(2,'0')].join('-');const dateButton=dateButtons.find(b=>b.dataset.date===iso);if(!dateButton)continue;const b=document.createElement('button');b.type='button';b.className='quick-filter'+(activeDate()===iso?' active':'');b.textContent=label;b.onclick=()=>dateButton.click();quick.append(b)}}
function renderChips(s){chips.replaceChildren();const defs=[];if(s.channel)defs.push(['Channel: '+s.channel,()=>{channel.value=''}]);if(s.sender)defs.push(['Sender: '+s.sender,()=>{sender.value=''}]);if(s.from)defs.push(['From: '+dateLabel(s.from),()=>{dateFrom.value=''}]);if(s.to)defs.push(['To: '+dateLabel(s.to),()=>{dateTo.value=''}]);if(s.exact&&s.q.trim())defs.push(['Exact phrase',()=>{exact.checked=false}]);if(s.image)defs.push(['Has image',()=>{hasImage.checked=false}]);for(const [label,clear] of defs){const b=document.createElement('button');b.type='button';b.className='active-filter-chip';b.textContent=label+' ×';b.onclick=()=>{clear();applyAdvanced()};chips.append(b)}}
function applyAdvanced(){const s=state(),wantedSender=lower(s.sender.trim()),exactSender=senderNames.has(wantedSender);let shown=0;for(const msg of messages){const name=msg.dataset.sender||'',bodyEl=msg.querySelector('.message-text'),body=bodyEl?.dataset.original||'',msgDate=msg.dataset.date||'',dateOk=(s.date==='all'||msgDate===s.date)&&(!s.from||msgDate>=s.from)&&(!s.to||msgDate<=s.to),senderOk=!wantedSender||(exactSender?lower(name)===wantedSender:lower(name).includes(wantedSender)),imageOk=!s.image||/\[\s*image\s*\(/i.test(body),visible=dateOk&&(!s.channel||msg.dataset.channel===s.channel)&&senderOk&&imageOk&&matchQuery(name,body,s.q,s.exact);msg.hidden=!visible;if(visible){shown++;highlight(bodyEl,s.q,s.exact)}else highlight(bodyEl,'',false)}for(const day of days)day.hidden=!day.querySelector('.message:not([hidden])');if(result)result.innerHTML=`<strong>${shown.toLocaleString()}</strong> match${shown===1?'':'es'} / ${messages.length.toLocaleString()} retained`;if(empty)empty.hidden=shown!==0;const n=[s.channel,s.sender,s.from,s.to,s.image?'1':'',s.exact&&s.q.trim()?'1':''].filter(Boolean).length;countBadge.hidden=n===0;countBadge.textContent=String(n);filterBtn.classList.toggle('active',n>0);renderQuick();renderChips(s);save()}

filterBtn.onclick=()=>{panel.classList.toggle('open');filterBtn.setAttribute('aria-expanded',panel.classList.contains('open')?'true':'false')};
clearBtn.onclick=()=>{search.value='';channel.value='';sender.value='';dateFrom.value='';dateTo.value='';exact.checked=false;hasImage.checked=false;applyAdvanced();search.focus()};
for(const el of [search,channel,sender,dateFrom,dateTo,exact,hasImage])el.addEventListener(el.tagName==='SELECT'||el.type==='checkbox'?'change':'input',applyAdvanced);
for(const b of dateButtons)b.addEventListener('click',()=>setTimeout(applyAdvanced,0));
document.addEventListener('keydown',ev=>{if((ev.ctrlKey||ev.metaKey)&&lower(ev.key)==='f'){ev.preventDefault();search.focus();search.select()}else if(ev.key==='/'&&!/^(INPUT|SELECT|TEXTAREA)$/.test(document.activeElement?.tagName||'')){ev.preventDefault();search.focus()}});
const saved=load();if(saved.q!=null)search.value=saved.q;if(saved.channel!=null&&[...channel.options].some(o=>o.value===saved.channel))channel.value=saved.channel;if(saved.sender!=null)sender.value=saved.sender;if(saved.from)dateFrom.value=saved.from;if(saved.to)dateTo.value=saved.to;exact.checked=!!saved.exact;hasImage.checked=!!saved.image;if(saved.date&&saved.date!=='all'){const b=dateButtons.find(x=>x.dataset.date===saved.date);if(b)b.click()}applyAdvanced();
})();
</script>
</body>"#,
        "advanced filter script",
    )?;

    fs::write(path, html.as_bytes())
        .map_err(|e| format!("write chat archive v1.33.0 UX upgrade: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn upgrades_navigation_and_filter_markers() {
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("readyalert-chat-ux-v1330-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("index.html");
        fs::write(&path, r#"<html><head><style></style></head><body><div class="archive-brand-row"><p>Chat Archive</p><a class="archive-switch" href="../EncounterHistory/index.html" title="Open Encounter History">Encounter History</a></div></body></html>"#).unwrap();
        upgrade(&path).unwrap();
        let html = fs::read_to_string(&path).unwrap();
        assert!(html.contains("archive-tabs"));
        assert!(html.contains("advanced-filter-panel"));
        assert!(html.contains("bpsr-readyalert.chat-archive.filters.v2"));
        assert!(html.contains("Exact phrase"));
        assert!(html.contains("Has image"));
        fs::remove_dir_all(dir).unwrap();
    }
}
