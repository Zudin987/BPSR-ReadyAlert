use crate::model::channel_name;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

const ARCHIVE_FILE: &str = "index.html";

#[derive(Clone, Debug)]
struct ArchiveMessage {
    date: String,
    time: String,
    timestamp: String,
    channel: String,
    sender: String,
    text: String,
}

pub fn generate(dir: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(dir).map_err(|e| format!("create chat log folder: {e}"))?;

    let mut days: BTreeMap<String, Vec<ArchiveMessage>> = BTreeMap::new();
    let mut channels = BTreeSet::new();
    let mut senders = BTreeSet::new();
    let mut file_count = 0usize;
    let mut raw_lines = 0usize;

    let entries = fs::read_dir(dir).map_err(|e| format!("read chat log folder: {e}"))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !is_chat_log(&path) {
            continue;
        }
        file_count += 1;
        let fallback_date = date_from_filename(&path).unwrap_or_else(|| "Unknown date".into());
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(_) => continue,
        };
        let text = String::from_utf8_lossy(&bytes);
        for line in text.lines().map(str::trim_end).filter(|line| !line.trim().is_empty()) {
            let message = parse_line(line).unwrap_or_else(|| {
                raw_lines += 1;
                ArchiveMessage {
                    date: fallback_date.clone(),
                    time: String::new(),
                    timestamp: fallback_date.clone(),
                    channel: "Chat".into(),
                    sender: String::new(),
                    text: line.to_string(),
                }
            });
            channels.insert(message.channel.clone());
            if !message.sender.trim().is_empty() {
                senders.insert(message.sender.clone());
            }
            days.entry(message.date.clone()).or_default().push(message);
        }
    }

    for messages in days.values_mut() {
        messages.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    }

    let html = render_archive(&days, &channels, &senders, file_count, raw_lines);
    let target = dir.join(ARCHIVE_FILE);
    let pending = dir.join("index.html.new");
    fs::write(&pending, html.as_bytes()).map_err(|e| format!("write chat archive: {e}"))?;
    if target.exists() {
        fs::remove_file(&target).map_err(|e| format!("replace old chat archive: {e}"))?;
    }
    fs::rename(&pending, &target).map_err(|e| format!("install chat archive: {e}"))?;
    Ok(target)
}

fn is_chat_log(path: &Path) -> bool {
    path.is_file()
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("chat-") && name.ends_with(".txt"))
}

fn date_from_filename(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?.strip_prefix("chat-")?;
    if name.len() >= 10 {
        let prefix = &name[..10];
        if valid_date(prefix) {
            return Some(prefix.to_string());
        }
    }
    if name.len() >= 8 {
        let prefix = &name[..8];
        if prefix.bytes().all(|b| b.is_ascii_digit()) {
            return Some(format!("{}-{}-{}", &prefix[..4], &prefix[4..6], &prefix[6..8]));
        }
    }
    None
}

fn valid_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(i, b)| matches!(i, 4 | 7) || b.is_ascii_digit())
}

fn parse_line(line: &str) -> Option<ArchiveMessage> {
    parse_tsv(line).or_else(|| parse_legacy(line))
}

fn parse_tsv(line: &str) -> Option<ArchiveMessage> {
    let mut fields = line.splitn(4, '\t');
    let timestamp = fields.next()?.trim();
    let channel = fields.next()?.trim().parse::<i32>().ok()?;
    let sender = fields.next()?.trim();
    let text = fields.next()?.trim_end();
    let (date, time) = split_timestamp(timestamp)?;
    Some(ArchiveMessage {
        date: date.to_string(),
        time: display_time(time),
        timestamp: format!("{date} {time}"),
        channel: channel_name(channel).to_string(),
        sender: sender.to_string(),
        text: text.to_string(),
    })
}

fn parse_legacy(line: &str) -> Option<ArchiveMessage> {
    let first_close = line.find("] [")?;
    if !line.starts_with('[') || first_close <= 1 {
        return None;
    }
    let stamp = &line[1..first_close];
    let rest = &line[first_close + 3..];
    let channel_close = rest.find(']')?;
    let channel = rest[..channel_close].trim();
    let body = rest[channel_close + 1..].trim_start();
    let (sender, text) = body.split_once(": ").or_else(|| body.split_once(':'))?;
    let mut stamp_fields = stamp.split_whitespace();
    let date = stamp_fields.next()?;
    let time = stamp_fields.next()?;
    if !valid_date(date) {
        return None;
    }
    Some(ArchiveMessage {
        date: date.to_string(),
        time: display_time(time),
        timestamp: format!("{date} {time}"),
        channel: if channel.is_empty() { "Chat" } else { channel }.to_string(),
        sender: sender.trim().to_string(),
        text: text.trim_end().to_string(),
    })
}

fn split_timestamp(value: &str) -> Option<(&str, &str)> {
    let (date, time) = value.split_once(' ')?;
    if !valid_date(date) || time.len() < 8 {
        return None;
    }
    Some((date, time))
}

fn display_time(value: &str) -> String {
    value.chars().take(8).collect()
}

fn render_archive(
    days: &BTreeMap<String, Vec<ArchiveMessage>>,
    channels: &BTreeSet<String>,
    senders: &BTreeSet<String>,
    file_count: usize,
    raw_lines: usize,
) -> String {
    let total_messages: usize = days.values().map(Vec::len).sum();
    let latest = days.keys().next_back().cloned().unwrap_or_else(|| "all".into());
    let mut out = String::with_capacity(total_messages.saturating_mul(220).saturating_add(16_384));

    out.push_str(r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src data:; style-src 'unsafe-inline'; script-src 'unsafe-inline'; connect-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'">
<title>BPSR ReadyAlert Chat Archive</title>
<style>
:root{color-scheme:dark;--bg:#0f1115;--panel:#151920;--panel2:#1b2028;--line:#2a313b;--text:#e7ebf0;--muted:#8994a3;--accent:#63c7ff;--hover:#202731;--mark:#755f16;--shadow:0 8px 30px rgba(0,0,0,.28)}
*{box-sizing:border-box}[hidden]{display:none!important}html,body{margin:0;min-height:100%;background:var(--bg);color:var(--text);font:14px/1.45 "Segoe UI",system-ui,-apple-system,sans-serif}button,input,select{font:inherit;color:inherit}.app{display:grid;grid-template-columns:230px minmax(0,1fr);min-height:100vh}.sidebar{position:sticky;top:0;height:100vh;overflow:auto;background:#11151b;border-right:1px solid var(--line);padding:18px 12px}.brand{padding:4px 8px 16px}.brand h1{font-size:16px;margin:0 0 4px}.brand p{color:var(--muted);margin:0;font-size:12px}.local{display:inline-flex;align-items:center;gap:6px;margin-top:10px;color:#9fd6ae;font-size:12px}.local:before{content:"";width:7px;height:7px;border-radius:50%;background:#55c878;box-shadow:0 0 0 3px rgba(85,200,120,.12)}.side-title{padding:12px 8px 7px;color:var(--muted);font-size:11px;font-weight:700;letter-spacing:.08em;text-transform:uppercase}.date-btn{width:100%;display:flex;justify-content:space-between;gap:12px;border:0;background:transparent;border-radius:7px;padding:8px 9px;margin:1px 0;text-align:left;cursor:pointer;color:#c9d0d9}.date-btn:hover{background:var(--hover)}.date-btn.active{background:#202b35;color:#fff}.date-count{color:var(--muted);font-size:12px}.main{min-width:0}.toolbar{position:sticky;top:0;z-index:20;background:rgba(15,17,21,.96);backdrop-filter:blur(12px);border-bottom:1px solid var(--line);padding:16px 22px 13px}.toolbar-top{display:flex;align-items:end;justify-content:space-between;gap:16px;margin-bottom:12px}.toolbar h2{font-size:18px;margin:0}.toolbar-sub{color:var(--muted);font-size:12px;margin-top:2px}.filters{display:grid;grid-template-columns:minmax(220px,1fr) 150px 180px;gap:9px}.control{width:100%;height:36px;border:1px solid #343d49;background:var(--panel);border-radius:7px;padding:0 10px;outline:none}.control:focus{border-color:#4f91b8;box-shadow:0 0 0 2px rgba(99,199,255,.12)}.summary{color:var(--muted);font-size:12px;white-space:nowrap}.content{max-width:1180px;margin:0 auto;padding:12px 22px 60px}.day{margin-bottom:20px}.day-head{position:sticky;top:123px;z-index:10;display:flex;align-items:center;gap:10px;padding:10px 4px 7px;background:linear-gradient(var(--bg) 75%,transparent)}.day-head h3{font-size:13px;margin:0;color:#b9c2cd}.day-head span{color:var(--muted);font-size:11px}.message{display:grid;grid-template-columns:72px 190px minmax(0,1fr);gap:12px;align-items:start;padding:9px 10px;border-top:1px solid rgba(42,49,59,.58);border-radius:5px}.message:hover{background:var(--hover)}.time{color:#758293;font-variant-numeric:tabular-nums;font-size:12px;padding-top:3px}.who{display:flex;align-items:center;gap:8px;min-width:0}.channel{flex:0 0 auto;display:inline-flex;align-items:center;border-radius:4px;padding:2px 6px;font-size:10px;font-weight:700;letter-spacing:.04em;background:#303743;color:#dbe3ec}.ch-world{background:#173d55;color:#8bd7ff}.ch-local{background:#193b29;color:#9be8ad}.ch-team,.ch-group{background:#4a2931;color:#ffc2ce}.ch-guild{background:#4a3d05;color:#ffe45e}.ch-private{background:#4a274a;color:#ffb9ff}.ch-newbie{background:#303741;color:#b9c3ce}.ch-system{background:#4a2727;color:#ffaaaa}.sender{min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;border:0;background:transparent;padding:0;color:#e6ebf1;font-weight:600;text-align:left;cursor:pointer}.sender:hover{text-decoration:underline;color:#fff}.message-text{white-space:pre-wrap;overflow-wrap:anywhere;color:#d5dbe3}.raw .message-text{color:#aeb7c2;font-family:Consolas,"Cascadia Mono",monospace;font-size:12px}.empty{margin:70px auto;text-align:center;color:var(--muted)}.empty strong{display:block;color:#cbd3dd;font-size:16px;margin-bottom:5px}mark{background:var(--mark);color:#fff;border-radius:2px;padding:0 1px}.footer{color:#66717f;text-align:center;font-size:11px;padding:24px 12px 36px}.kbd{border:1px solid #3a424d;background:#1a1f26;border-radius:4px;padding:1px 5px;font-size:10px;color:#aeb8c4}@media(max-width:760px){.app{display:block}.sidebar{position:relative;height:auto;border-right:0;border-bottom:1px solid var(--line);max-height:220px}.filters{grid-template-columns:1fr}.toolbar{position:relative}.day-head{top:0}.message{grid-template-columns:58px minmax(0,1fr)}.who{grid-column:2}.message-text{grid-column:2}.content{padding:10px 10px 40px}}
</style>
</head>
<body>
<div class="app">
<aside class="sidebar">
<div class="brand"><h1>BPSR ReadyAlert</h1><p>Chat Archive</p><div class="local">Offline / local only</div></div>
<div class="side-title">Chat logs</div>
<button class="date-btn" data-date="all"><span>All retained logs</span><span class="date-count">"#);
    let _ = write!(out, "{}", total_messages);
    out.push_str("</span></button>\n");
    for (date, messages) in days.iter().rev() {
        let _ = writeln!(
            out,
            "<button class=\"date-btn{}\" data-date=\"{}\"><span>{}</span><span class=\"date-count\">{}</span></button>",
            if date == &latest { " active" } else { "" },
            escape_attr(date),
            escape_html(date),
            messages.len()
        );
    }
    out.push_str("</aside>\n<main class=\"main\">\n<header class=\"toolbar\"><div class=\"toolbar-top\"><div><h2 id=\"view-title\">");
    out.push_str(&escape_html(&latest));
    out.push_str("</h2><div class=\"toolbar-sub\">Search and filter the logs stored on this PC. Nothing is uploaded.</div></div><div class=\"summary\" id=\"result-count\"></div></div><div class=\"filters\"><input class=\"control\" id=\"search\" type=\"search\" autocomplete=\"off\" placeholder=\"Search messages or players…\" aria-label=\"Search messages or players\"><select class=\"control\" id=\"channel\" aria-label=\"Channel filter\"><option value=\"\">All channels</option>");
    for channel in channels {
        let _ = write!(
            out,
            "<option value=\"{}\">{}</option>",
            escape_attr(channel),
            escape_html(channel)
        );
    }
    out.push_str("</select><input class=\"control\" id=\"sender\" list=\"sender-list\" autocomplete=\"off\" placeholder=\"Filter sender…\" aria-label=\"Sender filter\"><datalist id=\"sender-list\">");
    for sender in senders {
        let _ = write!(out, "<option value=\"{}\"></option>", escape_attr(sender));
    }
    out.push_str("</datalist></div></header><div class=\"content\" id=\"content\">\n");

    if days.is_empty() {
        out.push_str("<div class=\"empty\" id=\"empty\"><strong>No chat logs yet</strong>ReadyAlert will list retained local chat here after messages are logged.</div>");
    } else {
        out.push_str("<div class=\"empty\" id=\"empty\" hidden><strong>No matching messages</strong>Try another keyword, sender, channel, or date.</div>\n<div id=\"days\">\n");
        for (date, messages) in days.iter().rev() {
            let _ = writeln!(out, "<section class=\"day\" data-date=\"{}\">", escape_attr(date));
            let _ = writeln!(
                out,
                "<div class=\"day-head\"><h3>{}</h3><span>{} messages</span></div>",
                escape_html(date),
                messages.len()
            );
            for message in messages {
                let raw = message.sender.trim().is_empty();
                let _ = write!(
                    out,
                    "<article class=\"message{}\" data-date=\"{}\" data-channel=\"{}\" data-sender=\"{}\"><time class=\"time\" title=\"{}\">{}</time><div class=\"who\"><span class=\"channel {}\">{}</span>",
                    if raw { " raw" } else { "" },
                    escape_attr(&message.date),
                    escape_attr(&message.channel),
                    escape_attr(&message.sender),
                    escape_attr(&message.timestamp),
                    escape_html(if message.time.is_empty() { "--:--:--" } else { &message.time }),
                    channel_class(&message.channel),
                    escape_html(&message.channel),
                );
                if raw {
                    out.push_str("<span class=\"sender\">Raw log</span>");
                } else {
                    let _ = write!(out, "<button class=\"sender\" type=\"button\">{}</button>", escape_html(&message.sender));
                }
                let _ = writeln!(out, "</div><div class=\"message-text\">{}</div></article>", escape_html(&message.text));
            }
            out.push_str("</section>\n");
        }
        out.push_str("</div>\n");
    }

    let _ = write!(
        out,
        "<div class=\"footer\">Generated locally from {} chat log file{} · {} message{}{} · <span class=\"kbd\">/</span> Search · <span class=\"kbd\">Esc</span> Clear filters</div>",
        file_count,
        if file_count == 1 { "" } else { "s" },
        total_messages,
        if total_messages == 1 { "" } else { "s" },
        if raw_lines > 0 { format!(" · {raw_lines} raw line{} preserved", if raw_lines == 1 { "" } else { "s" }) } else { String::new() }
    );
    out.push_str("</div></main></div>\n");
    let _ = write!(out, "<script>window.READYALERT_LATEST={:?};</script>\n", latest);
    out.push_str(r#"<script>
(()=>{
const messages=[...document.querySelectorAll('.message')];
const days=[...document.querySelectorAll('.day')];
const dateButtons=[...document.querySelectorAll('.date-btn')];
const search=document.getElementById('search');
const channel=document.getElementById('channel');
const sender=document.getElementById('sender');
const result=document.getElementById('result-count');
const title=document.getElementById('view-title');
const empty=document.getElementById('empty');
let activeDate=window.READYALERT_LATEST||'all';
for(const el of document.querySelectorAll('.message-text')) el.dataset.original=el.textContent;
const lower=s=>(s||'').toLocaleLowerCase();
const senderNames=new Set(messages.map(msg=>lower(msg.dataset.sender)).filter(Boolean));
function highlight(el,q){const original=el.dataset.original||'';el.replaceChildren();if(!q){el.textContent=original;return;}const hay=lower(original);let from=0;while(from<original.length){const at=hay.indexOf(q,from);if(at<0){el.append(document.createTextNode(original.slice(from)));break;}if(at>from)el.append(document.createTextNode(original.slice(from,at)));const mark=document.createElement('mark');mark.textContent=original.slice(at,at+q.length);el.append(mark);from=at+q.length;}if(original.length===0)el.textContent='';}
function apply(){const q=lower(search?.value.trim());const wantedChannel=channel?.value||'';const wantedSender=lower(sender?.value.trim());const exactSender=senderNames.has(wantedSender);let shown=0;for(const msg of messages){const name=lower(msg.dataset.sender);const textEl=msg.querySelector('.message-text');const body=lower(textEl?.dataset.original);const senderMatch=!wantedSender||(exactSender?name===wantedSender:name.includes(wantedSender));const visible=(activeDate==='all'||msg.dataset.date===activeDate)&&(!wantedChannel||msg.dataset.channel===wantedChannel)&&senderMatch&&(!q||(name+' '+body).includes(q));msg.hidden=!visible;if(visible){shown++;highlight(textEl,q);}else if(textEl&&q)highlight(textEl,'');}for(const day of days)day.hidden=!day.querySelector('.message:not([hidden])');if(result)result.textContent=shown.toLocaleString()+' message'+(shown===1?'':'s');if(empty)empty.hidden=shown!==0;title.textContent=activeDate==='all'?'All retained logs':activeDate;}
for(const button of dateButtons)button.addEventListener('click',()=>{activeDate=button.dataset.date||'all';for(const x of dateButtons)x.classList.toggle('active',x===button);apply();window.scrollTo({top:0,behavior:'instant'});});
search?.addEventListener('input',apply);channel?.addEventListener('change',apply);sender?.addEventListener('input',apply);
for(const button of document.querySelectorAll('button.sender'))button.addEventListener('click',()=>{sender.value=button.textContent||'';apply();sender?.focus();});
document.addEventListener('keydown',event=>{if(event.key==='Escape'){if(search)search.value='';if(channel)channel.value='';if(sender)sender.value='';apply();search?.focus();}else if(event.key==='/'&&!/^(INPUT|SELECT|TEXTAREA)$/.test(document.activeElement?.tagName||'')){event.preventDefault();search?.focus();}});
apply();
})();
</script>
</body>
</html>
"#);
    out
}

fn channel_class(channel: &str) -> &'static str {
    match channel.to_ascii_lowercase().as_str() {
        "world" => "ch-world",
        "local" => "ch-local",
        "team" => "ch-team",
        "group" => "ch-group",
        "guild" => "ch-guild",
        "private" => "ch-private",
        "newbie" => "ch-newbie",
        "system" => "ch-system",
        _ => "",
    }
}

fn escape_html(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

fn escape_attr(value: &str) -> String {
    escape_html(value).replace(['\r', '\n', '\t'], " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_current_tsv_format() {
        let msg = parse_line("2026-09-12 11:50:12.581\t1\tAlice\tPA9").unwrap();
        assert_eq!(msg.date, "2026-09-12");
        assert_eq!(msg.time, "11:50:12");
        assert_eq!(msg.channel, "World");
        assert_eq!(msg.sender, "Alice");
        assert_eq!(msg.text, "PA9");
    }

    #[test]
    fn parses_legacy_bracket_format() {
        let msg = parse_line("[2026-09-04 12:39:17.267 +08:00] [Guild] 亗Vivi: ggty").unwrap();
        assert_eq!(msg.date, "2026-09-04");
        assert_eq!(msg.channel, "Guild");
        assert_eq!(msg.sender, "亗Vivi");
        assert_eq!(msg.text, "ggty");
    }

    #[test]
    fn generated_archive_is_offline_searchable_and_html_escaped() {
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("readyalert-chat-archive-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("chat-2026-09-12.txt"),
            "2026-09-12 11:50:12.581\t1\tAlice\t<script>alert(1)</script>\n",
        )
        .unwrap();
        fs::write(
            root.join("chat-20260904-04Z.txt"),
            "[2026-09-04 12:39:17.267 +08:00] [Guild] Vivi: ggty\n",
        )
        .unwrap();
        let path = generate(&root).unwrap();
        let html = fs::read_to_string(path).unwrap();
        assert!(html.contains("connect-src 'none'"));
        assert!(html.contains("id=\"search\""));
        assert!(html.contains("[hidden]{display:none!important}"));
        assert!(html.contains("const exactSender=senderNames.has(wantedSender)"));
        assert!(html.contains("2026-09-04"));
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(!html.contains("<script>alert(1)</script>"));
        fs::remove_dir_all(root).unwrap();
    }
}
