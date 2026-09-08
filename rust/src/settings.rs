use crate::{logging, paths::AppPaths};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, io, path::Path};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    pub queue_pop_alert: bool,
    pub ready_check_alert: bool,
    pub party_invite_alert: bool,
    pub party_request_alert: bool,
    pub desktop_notification: bool,
    pub auto_launch_resonance_logs: bool,
    pub resonance_logs_path: String,
    pub npcap_device_name: String,
    pub alert_volume: i32,
    pub chat_overlay_enabled: bool,
    pub chat: ChatOverlaySettings,
    pub speech_translation: SpeechSettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            queue_pop_alert: true,
            ready_check_alert: true,
            party_invite_alert: true,
            party_request_alert: true,
            desktop_notification: true,
            auto_launch_resonance_logs: true,
            resonance_logs_path: String::new(),
            npcap_device_name: String::new(),
            alert_volume: 100,
            chat_overlay_enabled: true,
            chat: ChatOverlaySettings::default(),
            speech_translation: SpeechSettings::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ChatOverlaySettings {
    pub top_most: bool,
    pub compact_mode: bool,
    pub show_time: bool,
    pub show_time_as_ago: bool,
    pub hide_stickers: bool,
    pub background_opacity: i32,
    pub toolbar_opacity: i32,
    pub text_opacity: i32,
    pub window_opacity: i32,
    pub font_family: String,
    pub font_size: f32,
    pub bold_message_text: bool,
    pub text_shadow: bool,
    pub show_separators: bool,
    pub show_zebra_stripes: bool,
    pub show_color_band: bool,
    pub click_through: bool,
    pub click_through_hotkey: String,
    pub collapse_hotkey: String,
    pub collapse_side: String,
    pub highlight_if_matches: String,
    pub highlight_color: String,
    pub highlight_sound_rules: Vec<ChatSoundRule>,
    pub highlight_sound_enabled: bool,
    pub highlight_sound_path: String,
    pub private_highlight_enabled: bool,
    pub private_highlight_color: String,
    pub private_sound_enabled: bool,
    pub private_sound_path: String,
    pub chat_sound_volume: i32,
    pub channel_colors: HashMap<i32, String>,
    pub max_history: usize,
    pub keep_local_chat_logs24_hours: bool,
    pub local_chat_log_retention_hours: i32,
    pub window_x: i32,
    pub window_y: i32,
    pub window_width: i32,
    pub window_height: i32,
    pub last_selected_tab_id: i64,
    pub tabs: Vec<ChatTabSettings>,
    pub blocked_users: Vec<ChatBlockedUser>,
}

impl Default for ChatOverlaySettings {
    fn default() -> Self {
        let mut value = Self {
            top_most: true,
            compact_mode: false,
            show_time: true,
            show_time_as_ago: true,
            hide_stickers: true,
            background_opacity: 82,
            toolbar_opacity: 92,
            text_opacity: 100,
            window_opacity: 100,
            font_family: "Segoe UI".into(),
            font_size: 12.0,
            bold_message_text: false,
            text_shadow: true,
            show_separators: true,
            show_zebra_stripes: true,
            show_color_band: true,
            click_through: false,
            click_through_hotkey: "Ctrl+Shift+F10".into(),
            collapse_hotkey: String::new(),
            collapse_side: "Left".into(),
            highlight_if_matches: String::new(),
            highlight_color: "#6B5A3A".into(),
            highlight_sound_rules: Vec::new(),
            highlight_sound_enabled: false,
            highlight_sound_path: String::new(),
            private_highlight_enabled: true,
            private_highlight_color: "#56355D".into(),
            private_sound_enabled: false,
            private_sound_path: String::new(),
            chat_sound_volume: 100,
            channel_colors: default_channel_colors(),
            max_history: 200,
            keep_local_chat_logs24_hours: true,
            local_chat_log_retention_hours: 168,
            window_x: i32::MIN,
            window_y: i32::MIN,
            window_width: 700,
            window_height: 430,
            last_selected_tab_id: 639233255393111833,
            tabs: default_tabs(),
            blocked_users: Vec::new(),
        };
        value.normalize();
        value
    }
}

impl ChatOverlaySettings {
    pub fn normalize(&mut self) {
        self.background_opacity = 82;
        self.toolbar_opacity = 92;
        self.text_opacity = 100;
        self.window_opacity = self.window_opacity.clamp(25, 100);
        self.font_family = clamp_text(&self.font_family, "Segoe UI", 100);
        if !self.font_size.is_finite() { self.font_size = 12.0; }
        self.font_size = self.font_size.clamp(8.0, 24.0);
        self.click_through_hotkey = clamp_text(&self.click_through_hotkey, "Ctrl+Shift+F10", 80);
        self.collapse_hotkey.clear();
        if !matches!(self.collapse_side.to_ascii_lowercase().as_str(), "left" | "right" | "top" | "bottom") {
            self.collapse_side = "Left".into();
        }
        truncate(&mut self.highlight_if_matches, 4096);
        self.highlight_color = normalize_hex(&self.highlight_color, "#6B5A3A");
        self.private_highlight_color = normalize_hex(&self.private_highlight_color, "#56355D");
        truncate(&mut self.private_sound_path, 1024);
        self.chat_sound_volume = self.chat_sound_volume.clamp(0, 100);
        self.highlight_sound_rules.truncate(2);
        for rule in &mut self.highlight_sound_rules {
            rule.match_text = rule.match_text.trim().to_string();
            truncate(&mut rule.match_text, 4096);
            truncate(&mut rule.sound_path, 1024);
            if rule.match_text.is_empty() { rule.enabled = false; }
        }
        self.highlight_sound_enabled = false;
        self.highlight_sound_path.clear();
        self.max_history = self.max_history.clamp(10, 500);
        self.local_chat_log_retention_hours = match self.local_chat_log_retention_hours {
            24 | 72 | 168 => self.local_chat_log_retention_hours,
            _ => 168,
        };
        self.window_width = self.window_width.clamp(360, 2400);
        self.window_height = self.window_height.clamp(180, 1600);
        let defaults = default_channel_colors();
        for (key, color) in defaults {
            let current = self.channel_colors.get(&key).cloned().unwrap_or_else(|| color.clone());
            self.channel_colors.insert(key, normalize_hex(&current, &color));
        }
        for tab in &mut self.tabs {
            tab.name = clamp_text(&tab.name, "Chat", 40);
            tab.min_level = tab.min_level.clamp(1, 100);
            tab.channels.retain(|c| matches!(*c, 0..=9 | 99));
            tab.channels.sort_unstable();
            tab.channels.dedup();
            truncate(&mut tab.show_if_matches, 4096);
            truncate(&mut tab.hide_if_matches, 4096);
        }
        if self.tabs.is_empty() { self.tabs = default_tabs(); }
        if !self.tabs.iter().any(|t| t.id == self.last_selected_tab_id) {
            self.last_selected_tab_id = self.tabs[0].id;
        }
        self.blocked_users.retain(|u| u.id != 0);
        self.blocked_users.sort_by_key(|u| u.id);
        self.blocked_users.dedup_by_key(|u| u.id);
    }

    pub fn is_blocked(&self, sender_id: i64) -> bool {
        sender_id != 0 && self.blocked_users.iter().any(|x| x.id == sender_id)
    }

    pub fn active_tab(&self) -> &ChatTabSettings {
        self.tabs.iter().find(|t| t.id == self.last_selected_tab_id).unwrap_or(&self.tabs[0])
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ChatSoundRule {
    pub enabled: bool,
    #[serde(rename = "match")]
    pub match_text: String,
    pub sound_path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ChatTabSettings {
    pub id: i64,
    pub name: String,
    pub channels: Vec<i32>,
    pub min_level: i32,
    pub show_if_matches: String,
    pub hide_if_matches: String,
}

impl Default for ChatTabSettings {
    fn default() -> Self {
        Self { id: 1, name: "Chat".into(), channels: vec![1,2,3,4,5,6,9], min_level: 1, show_if_matches: String::new(), hide_if_matches: String::new() }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ChatBlockedUser {
    pub id: i64,
    pub name: String,
    pub blocked_at_utc: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SpeechSettings {
    pub translation_enabled: bool,
    pub translation_world: bool,
    pub translation_guild: bool,
    pub translation_party_team: bool,
    pub show_translation_in_overlay: bool,
    pub tts_enabled: bool,
    pub tts_guild: bool,
    pub tts_party_team: bool,
    pub read_sender_name: bool,
    pub ignore_own_username: String,
    pub tts_volume: i32,
    pub hide_emoji_messages: bool,
    pub hide_linked_item_messages: bool,
}

impl Default for SpeechSettings {
    fn default() -> Self {
        Self {
            translation_enabled: true,
            translation_world: true,
            translation_guild: true,
            translation_party_team: true,
            show_translation_in_overlay: true,
            tts_enabled: true,
            tts_guild: false,
            tts_party_team: true,
            read_sender_name: true,
            ignore_own_username: String::new(),
            tts_volume: 100,
            hide_emoji_messages: true,
            hide_linked_item_messages: true,
        }
    }
}

impl SpeechSettings {
    pub fn normalize(&mut self) {
        self.ignore_own_username = self.ignore_own_username.replace(|c: char| matches!(c, '\r' | '\n' | '\0'), " ").trim().to_string();
        truncate(&mut self.ignore_own_username, 128);
        self.tts_volume = self.tts_volume.clamp(0, 100);
    }

    pub fn translation_for(&self, channel: i32) -> bool {
        self.translation_enabled && ((self.translation_world && channel == 1)
            || (self.translation_guild && channel == 4)
            || (self.translation_party_team && matches!(channel, 3 | 6)))
    }

    pub fn tts_for(&self, channel: i32) -> bool {
        self.tts_enabled && ((self.tts_guild && channel == 4)
            || (self.tts_party_team && matches!(channel, 3 | 6)))
    }
}

pub fn load(paths: &AppPaths) -> AppSettings {
    let backup = paths.settings.with_extension("json.bak");
    for path in [&paths.settings, &backup] {
        if let Ok(text) = fs::read_to_string(path) {
            match serde_json::from_str::<AppSettings>(&text) {
                Ok(mut settings) => {
                    settings.normalize();
                    if path == &backup {
                        let _ = save(paths, &settings);
                        logging::write("settings: recovered from backup");
                    }
                    return settings;
                }
                Err(err) => logging::write(format!("settings: load failed {}: {err}", path.display())),
            }
        }
    }
    let mut settings = AppSettings::default();
    settings.normalize();
    let _ = save(paths, &settings);
    settings
}

pub fn save(paths: &AppPaths, settings: &AppSettings) -> io::Result<()> {
    let mut normalized = settings.clone();
    normalized.normalize();
    let temp = paths.settings.with_extension("json.new");
    let backup = paths.settings.with_extension("json.bak");
    let json = serde_json::to_string_pretty(&normalized).map_err(io::Error::other)?;
    fs::write(&temp, json.as_bytes())?;
    let _: AppSettings = serde_json::from_slice(&fs::read(&temp)?).map_err(io::Error::other)?;
    if paths.settings.exists() {
        let _ = fs::copy(&paths.settings, &backup);
        fs::remove_file(&paths.settings)?;
    }
    fs::rename(temp, &paths.settings)?;
    Ok(())
}

impl AppSettings {
    pub fn normalize(&mut self) {
        self.alert_volume = self.alert_volume.clamp(0, 100);
        self.npcap_device_name = self.npcap_device_name.trim().to_string();
        truncate(&mut self.npcap_device_name, 512);
        self.resonance_logs_path = self.resonance_logs_path.trim().to_string();
        truncate(&mut self.resonance_logs_path, 2048);
        self.chat.normalize();
        self.speech_translation.normalize();
    }
}

fn default_tabs() -> Vec<ChatTabSettings> {
    vec![
        ChatTabSettings { id: 639233255393111833, name: "All".into(), channels: vec![1,2,3,4,5,6,9], min_level: 50, show_if_matches: String::new(), hide_if_matches: String::new() },
        ChatTabSettings { id: 639233255393111900, name: "Guild&Team".into(), channels: vec![3,4,5,6], min_level: 1, show_if_matches: String::new(), hide_if_matches: String::new() },
        ChatTabSettings { id: 639233255393111918, name: "Guild".into(), channels: vec![4], min_level: 1, show_if_matches: String::new(), hide_if_matches: String::new() },
        ChatTabSettings { id: 639235625391474596, name: "Team".into(), channels: vec![3,6], min_level: 1, show_if_matches: String::new(), hide_if_matches: String::new() },
    ]
}

fn default_channel_colors() -> HashMap<i32, String> {
    [(0,"#C7C7C7"),(1,"#63C7FF"),(2,"#8FED8F"),(3,"#FFB5C2"),(4,"#FFD600"),(5,"#FFA1FF"),(6,"#ADD8E6"),(7,"#FF8C00"),(8,"#C6A8FF"),(9,"#9FA8B2"),(99,"#FF6347")]
        .into_iter().map(|(k,v)| (k, v.to_string())).collect()
}

fn normalize_hex(value: &str, fallback: &str) -> String {
    let value = value.trim();
    let bytes = value.as_bytes();
    if bytes.len() == 7 && bytes[0] == b'#' && bytes[1..].iter().all(u8::is_ascii_hexdigit) {
        value.to_ascii_uppercase()
    } else {
        fallback.to_string()
    }
}

fn clamp_text(value: &str, fallback: &str, max: usize) -> String {
    let mut out = value.trim().to_string();
    if out.is_empty() { out = fallback.to_string(); }
    truncate(&mut out, max);
    out
}

fn truncate(value: &mut String, max: usize) {
    if value.chars().count() <= max { return; }
    *value = value.chars().take(max).collect();
}

#[allow(dead_code)]
fn _is_json_file(path: &Path) -> bool { path.extension().and_then(|x| x.to_str()) == Some("json") }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_existing_profile() {
        let s = AppSettings::default();
        assert!(s.queue_pop_alert && s.ready_check_alert && s.party_invite_alert && s.party_request_alert);
        assert!(s.chat_overlay_enabled);
        assert_eq!(s.chat.local_chat_log_retention_hours, 168);
        assert_eq!(s.chat.tabs.len(), 4);
        assert!(s.speech_translation.translation_enabled);
        assert!(s.speech_translation.tts_enabled);
    }
}
