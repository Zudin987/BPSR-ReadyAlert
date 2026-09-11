use crate::model::{ChatKind, ChatMessage, PlayerIdentity};

pub const CHAT_SERVICE: u64 = 164_931_432;
pub const CHAT_NOTIFY_NEWEST: u32 = 0x01;
pub const WORLD_SERVICE: u64 = 1_664_308_034;
pub const MATCH_SERVICE: u64 = 822_849_903;
pub const TEAM_SERVICE: u64 = 966_773_353;
pub const READY_ALL_METHOD: u32 = 0x46;
pub const READY_CAPTAIN_METHOD: u32 = 0x47;
pub const ENTER_SCENE_METHOD: u32 = 0x03;
pub const TEAM_APPLY_JOIN_METHOD: u32 = 0x05;
pub const TEAM_INVITATION_METHOD: u32 = 0x06;
pub const TEAM_ACTIVITY_METHOD: u32 = 0x0E;
pub const MATCH_ENTER_RESULT_METHOD: u32 = 0x04;

pub fn get_varint_field(data: &[u8], wanted: u32) -> Option<u64> {
    let mut p = 0usize;
    while p < data.len() {
        let key = read_varint(data, &mut p)?;
        let field = (key >> 3) as u32;
        let wire = (key & 7) as u8;
        if wire == 0 {
            let value = read_varint(data, &mut p)?;
            if field == wanted { return Some(value); }
        } else {
            skip_field(data, &mut p, wire)?;
        }
    }
    None
}

pub fn get_len_field(data: &[u8], wanted: u32) -> Option<&[u8]> {
    let mut p = 0usize;
    while p < data.len() {
        let key = read_varint(data, &mut p)?;
        let field = (key >> 3) as u32;
        let wire = (key & 7) as u8;
        if wire == 2 {
            let len = usize::try_from(read_varint(data, &mut p)?).ok()?;
            let end = p.checked_add(len)?;
            if end > data.len() { return None; }
            if field == wanted { return Some(&data[p..end]); }
            p = end;
        } else {
            skip_field(data, &mut p, wire)?;
        }
    }
    None
}

pub fn len_fields(data: &[u8], wanted: u32) -> Vec<&[u8]> {
    let mut result = Vec::new();
    let mut p = 0usize;
    while p < data.len() {
        let Some(key) = read_varint(data, &mut p) else { break; };
        let field = (key >> 3) as u32;
        let wire = (key & 7) as u8;
        if wire == 2 {
            let Some(raw_len) = read_varint(data, &mut p) else { break; };
            let Ok(len) = usize::try_from(raw_len) else { break; };
            let Some(end) = p.checked_add(len) else { break; };
            if end > data.len() { break; }
            if field == wanted { result.push(&data[p..end]); }
            p = end;
        } else if skip_field(data, &mut p, wire).is_none() {
            break;
        }
    }
    result
}

pub fn read_varint(data: &[u8], p: &mut usize) -> Option<u64> {
    let mut value = 0u64;
    let mut shift = 0u32;
    while *p < data.len() && shift < 64 {
        let b = data[*p];
        *p += 1;
        value |= u64::from(b & 0x7f) << shift;
        if b & 0x80 == 0 { return Some(value); }
        shift += 7;
    }
    None
}

fn skip_field(data: &[u8], p: &mut usize, wire: u8) -> Option<()> {
    match wire {
        0 => { read_varint(data, p)?; }
        1 => { *p = p.checked_add(8)?; if *p > data.len() { return None; } }
        2 => {
            let len = usize::try_from(read_varint(data, p)?).ok()?;
            *p = p.checked_add(len)?;
            if *p > data.len() { return None; }
        }
        5 => { *p = p.checked_add(4)?; if *p > data.len() { return None; } }
        _ => return None,
    }
    Some(())
}

pub fn parse_match_status(payload: &[u8]) -> Option<u64> {
    let request = get_len_field(payload, 1)?;
    let info = get_len_field(request, 2)?;
    get_varint_field(info, 2)
}

pub fn parse_match_wait_ready(payload: &[u8]) -> bool {
    parse_match_status(payload) == Some(2)
}

pub fn parse_team_activity_state(payload: &[u8]) -> Option<u64> {
    let request = get_len_field(payload, 1)?;
    let activity = get_len_field(request, 1)?;
    get_varint_field(activity, 2)
}

pub fn parse_team_activity_voting(payload: &[u8]) -> bool {
    parse_team_activity_state(payload) == Some(3)
}

pub fn parse_chat(payload: &[u8], sequence_id: u64) -> Option<ChatMessage> {
    let request = get_len_field(payload, 1)?;
    let channel = get_varint_field(request, 1).unwrap_or(0).min(i32::MAX as u64) as i32;
    let chat = get_len_field(request, 2)?;

    let message_id = get_varint_field(chat, 1).unwrap_or(0) as i64;
    let mut sender_id = 0i64;
    let mut sender_name = String::new();
    let mut sender_level = 0i32;
    if let Some(sender) = get_len_field(chat, 2) {
        sender_id = get_varint_field(sender, 1).unwrap_or(0) as i64;
        sender_name = get_string_field(sender, 2).unwrap_or_default();
        sender_level = get_varint_field(sender, 5).unwrap_or(0).min(i32::MAX as u64) as i32;
    }

    let unix_seconds = get_varint_field(chat, 3).unwrap_or(0).min(i64::MAX as u64) as i64;
    let info = get_len_field(chat, 4)?;
    let kind = ChatKind::from_u64(get_varint_field(info, 1).unwrap_or(0));
    let mut text = get_string_field(info, 3).unwrap_or_default();

    match kind {
        ChatKind::Sticker => {
            if let Some(sticker) = get_len_field(info, 5) {
                if let Some(id) = get_varint_field(sticker, 1) { text = format!("[Image({id})]"); }
            }
            if text.trim().is_empty() { text = "[Sticker]".into(); }
        }
        ChatKind::MultiLanguageNotice => {
            if let Some(notice) = get_len_field(info, 4) { text = combine(text, decode_notice(notice)); }
            if text.trim().is_empty() { text = "[Multi-language notice]".into(); }
        }
        ChatKind::Voice => {
            if let Some(voice) = get_len_field(info, 6) {
                if let Some(v) = get_string_field(voice, 2) { text = combine(text, v); }
            }
            if text.trim().is_empty() { text = "[Voice]".into(); }
        }
        ChatKind::Hypertext => {
            if let Some(hyper) = get_len_field(info, 7) { text = combine(text, decode_hypertext(hyper)); }
            if text.trim().is_empty() { text = "[Hypertext]".into(); }
        }
        ChatKind::TextNotice if text.trim().is_empty() => text = "[Notice]".into(),
        ChatKind::Picture if text.trim().is_empty() => text = "[Picture]".into(),
        ChatKind::Text if text.trim().is_empty() => {
            text = fallback_plain_text(info).unwrap_or_default();
        }
        _ => {}
    }

    Some(ChatMessage {
        message_id,
        sequence_id,
        sender_id,
        sender_name,
        sender_level,
        channel,
        unix_seconds,
        kind,
        text,
    })
}

pub fn parse_identity(payload: &[u8]) -> Option<PlayerIdentity> {
    let info = get_len_field(payload, 1)?;
    let player = get_len_field(info, 2)?;
    let raw_uuid = get_varint_field(player, 1)?;
    if raw_uuid > i64::MAX as u64 { return None; }
    let attrs = get_len_field(player, 3)?;
    let name_raw = find_attribute_raw_data(attrs, 1)?;
    let mut p = 0usize;
    let byte_len = usize::try_from(read_varint(name_raw, &mut p)?).ok()?;
    let end = p.checked_add(byte_len)?;
    if byte_len == 0 || byte_len > 512 || end > name_raw.len() { return None; }
    let name = std::str::from_utf8(&name_raw[p..end]).ok()?
        .replace(|c: char| matches!(c, '\r' | '\n' | '\0'), " ")
        .trim().to_string();
    if name.is_empty() { return None; }
    let uid = (raw_uuid as i64) >> 16;
    if uid <= 0 { return None; }
    Some(PlayerIdentity { name: name.chars().take(128).collect(), uid })
}

fn find_attribute_raw_data(attrs: &[u8], wanted_id: u64) -> Option<&[u8]> {
    for attr in len_fields(attrs, 2) {
        if get_varint_field(attr, 1) == Some(wanted_id) { return get_len_field(attr, 2); }
    }
    None
}

fn get_string_field(data: &[u8], field: u32) -> Option<String> {
    let raw = get_len_field(data, field)?;
    if raw.len() > 64 * 1024 { return None; }
    Some(String::from_utf8_lossy(raw).into_owned())
}

fn fallback_plain_text(data: &[u8]) -> Option<String> {
    let mut p = 0usize;
    while p < data.len() {
        let key = read_varint(data, &mut p)?;
        let field = (key >> 3) as u32;
        let wire = (key & 7) as u8;
        if wire == 2 {
            let len = usize::try_from(read_varint(data, &mut p)?).ok()?;
            let end = p.checked_add(len)?;
            if end > data.len() { return None; }
            if field >= 3 {
                if let Some(v) = printable_utf8(&data[p..end]) { return Some(v); }
            }
            p = end;
        } else {
            skip_field(data, &mut p, wire)?;
        }
    }
    None
}

fn decode_notice(data: &[u8]) -> String {
    let mut parts = Vec::<String>::new();
    if let Some(id) = get_varint_field(data, 1).filter(|x| *x != 0) { parts.push(format!("[Notice {id}]")); }
    for raw in len_fields(data, 2) {
        if let Some(v) = printable_utf8(raw) { add_unique(&mut parts, v); }
    }
    parts.join(" ")
}

fn decode_hypertext(data: &[u8]) -> String {
    let mut parts = Vec::<String>::new();
    if let Some(id) = get_varint_field(data, 1).filter(|x| *x != 0) { parts.push(format!("[Hypertext {id}]")); }
    for holder in len_fields(data, 2) {
        let kind = get_varint_field(holder, 1).unwrap_or(0);
        let Some(content) = get_len_field(holder, 2) else { continue; };
        if kind == 7 {
            if let Some(v) = printable_utf8(content) { add_unique(&mut parts, v); }
        } else {
            collect_printable_proto(content, 0, &mut parts);
        }
    }
    parts.join(" ")
}

fn collect_printable_proto(data: &[u8], depth: usize, out: &mut Vec<String>) {
    if depth > 2 || data.is_empty() || data.len() > 64 * 1024 { return; }
    let mut p = 0usize;
    while p < data.len() {
        let Some(key) = read_varint(data, &mut p) else { return; };
        let wire = (key & 7) as u8;
        if wire == 2 {
            let Some(raw_len) = read_varint(data, &mut p) else { return; };
            let Ok(len) = usize::try_from(raw_len) else { return; };
            let Some(end) = p.checked_add(len) else { return; };
            if end > data.len() { return; }
            let child = &data[p..end];
            if let Some(v) = printable_utf8(child) { add_unique(out, v); }
            else { collect_printable_proto(child, depth + 1, out); }
            p = end;
        } else if skip_field(data, &mut p, wire).is_none() {
            return;
        }
    }
}

fn printable_utf8(raw: &[u8]) -> Option<String> {
    if raw.is_empty() || raw.len() > 2048 { return None; }
    let decoded = std::str::from_utf8(raw).ok()?.trim();
    if decoded.is_empty() || decoded.chars().count() > 512 { return None; }
    let mut useful = false;
    for c in decoded.chars() {
        if c.is_control() && !c.is_whitespace() { return None; }
        if c.is_alphanumeric() { useful = true; }
    }
    useful.then(|| decoded.to_string())
}

fn combine(first: String, second: String) -> String {
    let first = first.trim();
    let second = second.trim();
    if first.is_empty() { return second.to_string(); }
    if second.is_empty() || first.to_ascii_lowercase().contains(&second.to_ascii_lowercase()) { return first.to_string(); }
    format!("{first} {second}")
}

fn add_unique(values: &mut Vec<String>, value: String) {
    let value = value.trim();
    if value.is_empty() { return; }
    if !values.iter().any(|x| x.eq_ignore_ascii_case(value)) { values.push(value.to_string()); }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varint_and_fields() {
        let data = [0x08, 0x96, 0x01, 0x12, 0x03, b'a', b'b', b'c'];
        assert_eq!(get_varint_field(&data, 1), Some(150));
        assert_eq!(get_len_field(&data, 2), Some(&b"abc"[..]));
    }

    #[test]
    fn queue_status_parser() {
        let payload = [0x0a, 0x06, 0x12, 0x04, 0x10, 0x02, 0x18, 0x01];
        assert_eq!(parse_match_status(&payload), Some(2));
        assert!(parse_match_wait_ready(&payload));
    }

    #[test]
    fn team_activity_state_parser() {
        let payload = [0x0a, 0x04, 0x0a, 0x02, 0x10, 0x03];
        assert_eq!(parse_team_activity_state(&payload), Some(3));
        assert!(parse_team_activity_voting(&payload));
    }

    #[test]
    fn fallback_recovers_schema_drift_text() {
        let info = [0x08, 0x00, 0x42, 0x05, b'h', b'e', b'l', b'l', b'o'];
        assert_eq!(fallback_plain_text(&info).as_deref(), Some("hello"));
    }
}
