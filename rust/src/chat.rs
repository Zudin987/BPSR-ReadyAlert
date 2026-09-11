use crate::{
    audio, logging,
    model::{AppEvent, ChatKind, ChatMessage, PlayerIdentity},
    paths::AppPaths,
    settings::AppSettings,
};
use regex::{Regex, RegexBuilder};
use serde_json::Value;
use std::{
    collections::{HashMap, VecDeque},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        mpsc::{self, SyncSender, TrySendError},
        Arc, Mutex, OnceLock, RwLock,
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use windows_sys::Win32::{Foundation::SYSTEMTIME, System::SystemInformation::GetLocalTime};

const MAX_FILTER_EXPRESSION: usize = 4096;
const MAX_FILTER_CACHE: usize = 256;

#[derive(Clone)]
pub struct ChatRuntime {
    log_tx: SyncSender<ChatMessage>,
    speech_tx: SyncSender<ChatMessage>,
    dedupe: Arc<Mutex<RecentDedupe>>,
}

#[derive(Default)]
struct RecentDedupe {
    entries: VecDeque<(u64, Instant)>,
}

impl RecentDedupe {
    fn accept(&mut self, message: &ChatMessage) -> bool {
        let now = Instant::now();
        while self.entries.front().is_some_and(|(_, at)| now.duration_since(*at) > Duration::from_secs(8)) {
            self.entries.pop_front();
        }
        let key = message_key(message);
        if self.entries.iter().any(|(seen, _)| *seen == key) { return false; }
        self.entries.push_back((key, now));
        while self.entries.len() > 512 { self.entries.pop_front(); }
        true
    }
}

#[derive(Default)]
struct ChatLogWriter {
    path: Option<PathBuf>,
    file: Option<File>,
}

impl ChatLogWriter {
    fn write(&mut self, dir: &Path, message: &ChatMessage) {
        let now = local_time();
        let filename = format!("chat-{:04}-{:02}-{:02}.txt", now.wYear, now.wMonth, now.wDay);
        let path = dir.join(filename);
        if self.path.as_ref() != Some(&path) || self.file.is_none() {
            self.path = Some(path.clone());
            self.file = None;
            if fs::create_dir_all(dir).is_ok() {
                self.file = OpenOptions::new().create(true).append(true).open(&path).ok();
            }
        }
        let Some(file) = self.file.as_mut() else { return; };
        let sender = message.sender_name.replace(|c: char| matches!(c, '\r' | '\n' | '\t'), " ");
        let text = message.text.replace(|c: char| matches!(c, '\r' | '\n' | '\t'), " ");
        if writeln!(file, "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}\t{}\t{}\t{}", now.wYear, now.wMonth, now.wDay, now.wHour, now.wMinute, now.wSecond, now.wMilliseconds, message.channel, sender, text).is_err() {
            self.file = None;
        }
    }
}

impl ChatRuntime {
    pub fn start(paths: AppPaths, settings: Arc<RwLock<AppSettings>>, identity: Arc<RwLock<Option<PlayerIdentity>>>, ui_tx: std::sync::mpsc::Sender<AppEvent>) -> Self {
        let (log_tx, log_rx) = mpsc::sync_channel::<ChatMessage>(4096);
        let log_settings = settings.clone();
        let log_dir = paths.chat_logs.clone();
        let _ = thread::Builder::new().name("readyalert-chatlog".into()).spawn(move || {
            cleanup_old_logs(&log_dir, &log_settings);
            let mut writer = ChatLogWriter::default();
            let mut next_cleanup = SystemTime::now() + Duration::from_secs(300);
            while let Ok(message) = log_rx.recv() {
                let enabled = log_settings.read().map(|s| s.chat.keep_local_chat_logs24_hours).unwrap_or(false);
                if enabled { writer.write(&log_dir, &message); }
                if SystemTime::now() >= next_cleanup {
                    cleanup_old_logs(&log_dir, &log_settings);
                    next_cleanup = SystemTime::now() + Duration::from_secs(300);
                }
            }
        });

        let (speech_tx, speech_rx) = mpsc::sync_channel::<ChatMessage>(24);
        let speech_settings = settings.clone();
        let _ = thread::Builder::new().name("readyalert-speech".into()).spawn(move || {
            let agent = ureq::AgentBuilder::new()
                .timeout_connect(Duration::from_secs(4))
                .timeout_read(Duration::from_secs(8))
                .timeout_write(Duration::from_secs(8))
                .build();
            while let Ok(message) = speech_rx.recv() {
                let snapshot = match speech_settings.read() { Ok(v) => v.clone(), Err(_) => continue };
                if snapshot.chat.is_blocked(message.sender_id) || skip_speech(&message) { continue; }
                let speech = snapshot.speech_translation.clone();
                let wants_translation = snapshot.chat_overlay_enabled
                    && speech.show_translation_in_overlay
                    && speech.translation_for(message.channel);
                let own_name = speech.ignore_own_username.trim().to_string();
                let detected = identity.read().ok().and_then(|g| g.clone()).map(|x| x.name).unwrap_or_default();
                let expected = if own_name.is_empty() { detected } else { own_name };
                let wants_tts = speech.tts_for(message.channel)
                    && speech.tts_volume > 0
                    && (expected.is_empty() || !expected.eq_ignore_ascii_case(message.sender_name.trim()));
                if !wants_translation && !wants_tts { continue; }

                let source = clean_text(&message.text, 1000);
                if source.is_empty() { continue; }
                let translated = translate(&agent, &source).unwrap_or_else(|err| {
                    logging::write(format!("translate: {err}"));
                    Translation { text: source.clone(), source_language: String::new(), translated: false }
                });
                if wants_translation && translated.translated {
                    let _ = ui_tx.send(AppEvent::Translation {
                        sequence_id: message.sequence_id,
                        text: translated.text.clone(),
                        source_language: translated.source_language.clone(),
                    });
                }
                if !wants_tts { continue; }
                let mut spoken = clean_text(&translated.text, 500);
                if speech.read_sender_name && !message.sender_name.trim().is_empty() {
                    spoken = format!("{}. {}", clean_text(&message.sender_name, 80), spoken);
                }
                for chunk in split_tts(&spoken, 200) {
                    match download_tts(&agent, &chunk) {
                        Ok(bytes) => if let Err(err) = audio::play_mp3(&bytes, speech.tts_volume) { logging::write(format!("tts: playback {err}")); },
                        Err(err) => { logging::write(format!("tts: {err}")); break; }
                    }
                }
            }
        });
        Self { log_tx, speech_tx, dedupe: Arc::new(Mutex::new(RecentDedupe::default())) }
    }

    pub fn handle(&self, message: &ChatMessage) {
        let accepted = self.dedupe.lock().map(|mut d| d.accept(message)).unwrap_or(true);
        if !accepted {
            logging::write(format!("chat: duplicate suppressed msg_id={} seq={}", message.message_id, message.sequence_id));
            return;
        }
        let _ = try_bounded(&self.log_tx, message.clone());
        let _ = try_bounded(&self.speech_tx, message.clone());
    }
}

pub fn should_hide_globally(settings: &AppSettings, message: &ChatMessage) -> bool {
    if settings.chat.is_blocked(message.sender_id) { return true; }
    if settings.chat.hide_stickers && matches!(message.kind, ChatKind::Sticker | ChatKind::Picture) { return true; }
    if settings.speech_translation.hide_emoji_messages && is_sprite_only(&message.text) { return true; }
    if settings.speech_translation.hide_linked_item_messages && is_linked_item(message) { return true; }
    if message.kind == ChatKind::Text && message.text.trim().is_empty() { return true; }
    false
}

pub fn should_hide_in_overlay(settings: &AppSettings, message: &ChatMessage) -> bool {
    if should_hide_globally(settings, message) { return true; }
    let tab = settings.chat.active_tab();
    if !tab.channels.contains(&message.channel) || message.sender_level < tab.min_level { return true; }
    let hay = format!("{} {}", message.sender_name, message.text);
    if !tab.show_if_matches.trim().is_empty() && !matches_expression(&hay, &tab.show_if_matches) { return true; }
    if !tab.hide_if_matches.trim().is_empty() && matches_expression(&hay, &tab.hide_if_matches) { return true; }
    false
}

#[derive(Clone)]
struct CompiledFilter {
    groups: Vec<Vec<Regex>>,
    error: Option<String>,
}

fn filter_cache() -> &'static Mutex<HashMap<String, CompiledFilter>> {
    static CACHE: OnceLock<Mutex<HashMap<String, CompiledFilter>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn or_splitter() -> &'static Regex {
    static SPLITTER: OnceLock<Regex> = OnceLock::new();
    SPLITTER.get_or_init(|| Regex::new(r"(?i)[\r\n]+|\s*(?:\|\||\bOR\b)\s*|\s+\|\s+").expect("filter OR splitter"))
}

fn and_splitter() -> &'static Regex {
    static SPLITTER: OnceLock<Regex> = OnceLock::new();
    SPLITTER.get_or_init(|| Regex::new(r"(?i)\s*(?:&&|\bAND\b)\s*").expect("filter AND splitter"))
}

fn compile_expression(expression: &str) -> CompiledFilter {
    let mut groups = Vec::new();
    for or_group in or_splitter().split(expression).map(str::trim).filter(|x| !x.is_empty()) {
        let mut atoms = Vec::new();
        for atom in and_splitter().split(or_group).map(str::trim).filter(|x| !x.is_empty()) {
            match RegexBuilder::new(atom).case_insensitive(true).build() {
                Ok(regex) => atoms.push(regex),
                Err(err) => return CompiledFilter { groups: Vec::new(), error: Some(err.to_string()) },
            }
        }
        if !atoms.is_empty() { groups.push(atoms); }
    }
    if groups.is_empty() {
        CompiledFilter { groups, error: Some("Enter at least one word or regular expression.".into()) }
    } else {
        CompiledFilter { groups, error: None }
    }
}

fn compiled_expression(expression: &str) -> CompiledFilter {
    if let Ok(mut cache) = filter_cache().lock() {
        if let Some(found) = cache.get(expression) { return found.clone(); }
        if cache.len() >= MAX_FILTER_CACHE { cache.clear(); }
        let compiled = compile_expression(expression);
        cache.insert(expression.to_string(), compiled.clone());
        return compiled;
    }
    compile_expression(expression)
}

pub(crate) fn matches_expression(hay: &str, expression: &str) -> bool {
    if expression.trim().is_empty() { return true; }
    if expression.len() > MAX_FILTER_EXPRESSION { return false; }
    let compiled = compiled_expression(expression);
    if compiled.error.is_some() { return false; }
    compiled.groups.iter().any(|group| group.iter().all(|atom| atom.is_match(hay)))
}

pub(crate) fn validate_expression(expression: &str) -> Result<(), String> {
    if expression.trim().is_empty() { return Ok(()); }
    if expression.len() > MAX_FILTER_EXPRESSION {
        return Err(format!("Filter is too long. Keep it under {MAX_FILTER_EXPRESSION} characters."));
    }
    let compiled = compiled_expression(expression);
    match compiled.error { Some(err) => Err(err), None => Ok(()) }
}

fn message_key(message: &ChatMessage) -> u64 {
    if message.message_id != 0 {
        return (message.message_id as u64) ^ (message.channel as u64).rotate_left(17) ^ 0xB5A4_9D21_6C7E_F013;
    }
    let mut hash = 14_695_981_039_346_656_037u64;
    for b in message.sender_id.to_le_bytes().into_iter()
        .chain(message.channel.to_le_bytes())
        .chain(message.unix_seconds.to_le_bytes())
        .chain((message.kind as i32).to_le_bytes())
        .chain(message.text.as_bytes().iter().copied())
    {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    hash
}

fn try_bounded(tx: &SyncSender<ChatMessage>, message: ChatMessage) -> bool {
    match tx.try_send(message) { Ok(()) => true, Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => false }
}

fn is_sprite_only(text: &str) -> bool {
    let mut rest = text.trim();
    let mut found = false;
    while let Some(after) = rest.strip_prefix("<sprite=") {
        let Some(close) = after.find('>') else { return false; };
        let number = &after[..close];
        if number.parse::<u32>().ok().filter(|n| *n >= 1 && *n <= 100).is_none() { return false; }
        found = true;
        rest = after[close + 1..].trim_start();
    }
    found && rest.is_empty()
}

fn is_linked_item(message: &ChatMessage) -> bool {
    message.kind == ChatKind::Hypertext || {
        let t = message.text.trim_start();
        t.starts_with("[Hypertext ") && t.find(']').is_some()
    }
}

fn skip_speech(message: &ChatMessage) -> bool {
    is_sprite_only(&message.text) || is_linked_item(message) || matches!(message.kind, ChatKind::Sticker | ChatKind::Picture) || message.text.trim().is_empty()
}

struct Translation { text: String, source_language: String, translated: bool }

fn translate(agent: &ureq::Agent, text: &str) -> Result<Translation, String> {
    let url = format!("https://translate.googleapis.com/translate_a/single?client=gtx&sl=auto&tl=en&dt=t&q={}", urlencoding::encode(text));
    let response = agent.get(&url).set("Referer", "https://translate.google.com/").call().map_err(|e| e.to_string())?;
    let body = response.into_string().map_err(|e| e.to_string())?;
    let root: Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    let mut output = String::new();
    if let Some(segments) = root.get(0).and_then(Value::as_array) {
        for segment in segments {
            if let Some(s) = segment.get(0).and_then(Value::as_str) { output.push_str(s); }
        }
    }
    let output = output.trim().to_string();
    if output.is_empty() { return Err("empty Google translation".into()); }
    let lang = root.get(2).and_then(Value::as_str).unwrap_or("").to_string();
    let english = lang.eq_ignore_ascii_case("en") || lang.to_ascii_lowercase().starts_with("en-");
    let translated = !english && output != text;
    Ok(Translation { text: output, source_language: lang, translated })
}

fn download_tts(agent: &ureq::Agent, text: &str) -> Result<Vec<u8>, String> {
    let url = format!("https://translate.google.com/translate_tts?ie=UTF-8&client=tw-ob&tl=en&total=1&idx=0&textlen={}&q={}", text.chars().count(), urlencoding::encode(text));
    let response = agent.get(&url).set("Accept", "audio/mpeg,*/*;q=0.8").call().map_err(|e| e.to_string())?;
    let content_type = response.header("content-type").unwrap_or("").to_ascii_lowercase();
    if !content_type.starts_with("audio/") { return Err(format!("unexpected TTS content-type {content_type}")); }
    let mut reader = response.into_reader();
    let mut bytes = Vec::new();
    reader.by_ref().take(4 * 1024 * 1024 + 1).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    if bytes.len() < 200 || bytes.len() > 4 * 1024 * 1024 { return Err(format!("invalid TTS payload {} bytes", bytes.len())); }
    Ok(bytes)
}

fn split_tts(text: &str, max: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut remaining = text.trim().to_string();
    while remaining.chars().count() > max {
        let mut split_byte = remaining.char_indices().nth(max).map(|x| x.0).unwrap_or(remaining.len());
        if let Some((idx, _)) = remaining[..split_byte].char_indices().rev().find(|(_,c)| matches!(c, ' ' | ',' | '.' | ';' | ':' | '!' | '?')) {
            if idx > split_byte / 2 { split_byte = idx + 1; }
        }
        let part = remaining[..split_byte].trim().to_string();
        if !part.is_empty() { out.push(part); }
        remaining = remaining[split_byte..].trim_start().to_string();
    }
    if !remaining.is_empty() { out.push(remaining); }
    out
}

fn clean_text(text: &str, max: usize) -> String {
    text.replace(|c: char| c == '\r' || c == '\n' || c == '\0', " ").split_whitespace().collect::<Vec<_>>().join(" ").chars().take(max).collect()
}

fn is_readyalert_chat_log(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|x| x.to_str()) else { return false; };
    let bytes = name.as_bytes();
    bytes.len() == 19
        && &bytes[0..5] == b"chat-"
        && &bytes[9..10] == b"-"
        && &bytes[12..13] == b"-"
        && &bytes[15..19] == b".txt"
        && bytes[5..9].iter().all(u8::is_ascii_digit)
        && bytes[10..12].iter().all(u8::is_ascii_digit)
        && bytes[13..15].iter().all(u8::is_ascii_digit)
}

fn cleanup_old_logs(dir: &PathBuf, settings: &Arc<RwLock<AppSettings>>) {
    let hours = settings.read().map(|s| s.chat.local_chat_log_retention_hours).unwrap_or(168).clamp(24,168) as u64;
    let cutoff = SystemTime::now().checked_sub(Duration::from_secs(hours * 3600)).unwrap_or(UNIX_EPOCH);
    let Ok(entries) = fs::read_dir(dir) else { return; };
    for entry in entries.flatten() {
        let path = entry.path();
        if !is_readyalert_chat_log(&path) { continue; }
        if let Ok(modified) = entry.metadata().and_then(|m| m.modified()) {
            if modified < cutoff { let _ = fs::remove_file(path); }
        }
    }
}

fn local_time() -> SYSTEMTIME {
    unsafe { let mut t: SYSTEMTIME = std::mem::zeroed(); GetLocalTime(&mut t); t }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(id: i64, text: &str) -> ChatMessage {
        ChatMessage {
            message_id: id,
            sequence_id: id.max(0) as u64 + 1,
            sender_id: 7,
            sender_name: "Tester".into(),
            sender_level: 60,
            channel: 1,
            unix_seconds: 123,
            kind: ChatKind::Text,
            text: text.into(),
        }
    }

    #[test] fn sprites() { assert!(is_sprite_only(" <sprite=1> <sprite=100> ")); assert!(!is_sprite_only("hi <sprite=1>")); }
    #[test] fn split_is_bounded() { assert!(split_tts(&"a ".repeat(250),200).iter().all(|x| x.chars().count() <= 200)); }
    #[test] fn duplicate_message_id_is_suppressed() {
        let mut d = RecentDedupe::default();
        assert!(d.accept(&message(99, "hello")));
        assert!(!d.accept(&message(99, "hello again")));
    }
    #[test] fn empty_plain_text_is_hidden() {
        let settings = AppSettings::default();
        assert!(should_hide_globally(&settings, &message(1, "   ")));
    }
    #[test] fn friendly_or_and_regex_filters_match_v136_semantics() {
        assert!(matches_expression("serum raid", "serum AND raid"));
        assert!(!matches_expression("serum only", "serum AND raid"));
        assert!(matches_expression("food ping", "serum | food"));
        assert!(matches_expression("FOOD ping", "food"));
        assert!(matches_expression("foo", "f(o|a)o"));
        assert!(matches_expression("bar", "foo OR bar"));
    }
    #[test] fn invalid_filter_fails_closed() {
        assert!(!matches_expression("anything", "["));
        assert!(validate_expression("[").is_err());
    }
    #[test] fn maintenance_cleanup_only_recognizes_owned_log_names() {
        assert!(is_readyalert_chat_log(Path::new("chat-2026-09-10.txt")));
        assert!(!is_readyalert_chat_log(Path::new("notes.txt")));
        assert!(!is_readyalert_chat_log(Path::new("chat-old.txt")));
        assert!(!is_readyalert_chat_log(Path::new("chat-2026-9-10.txt")));
    }
}
