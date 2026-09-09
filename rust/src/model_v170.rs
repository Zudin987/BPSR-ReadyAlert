use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AlertKind {
    Queue,
    Ready,
    PartyInvite,
    PartyRequest,
    Error,
}

#[derive(Clone, Debug)]
pub struct AlertEvent {
    pub kind: AlertKind,
    pub title: String,
    pub message: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum ChatKind {
    Text = 0,
    TextNotice = 1,
    MultiLanguageNotice = 2,
    Sticker = 3,
    Picture = 4,
    Voice = 5,
    Hypertext = 6,
}

impl ChatKind {
    pub fn from_u64(v: u64) -> Self {
        match v {
            1 => Self::TextNotice,
            2 => Self::MultiLanguageNotice,
            3 => Self::Sticker,
            4 => Self::Picture,
            5 => Self::Voice,
            6 => Self::Hypertext,
            _ => Self::Text,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub message_id: i64,
    pub sequence_id: u64,
    pub sender_id: i64,
    pub sender_name: String,
    pub sender_level: i32,
    pub channel: i32,
    pub unix_seconds: i64,
    pub kind: ChatKind,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct PlayerIdentity {
    pub name: String,
    pub uid: i64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ImagineBadge {
    /// Resonance/Battle Imagine skill id resolved from the summon registry.
    pub skill_id: i32,
    pub name: String,
    /// Fantasy remodel level / tier observed on the summoned monster.
    pub tier: i32,
    pub icon_key: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SkillBreakdown {
    pub skill_id: i32,
    pub name: String,
    pub damage: i64,
    /// Portion of this skill's damage assigned to the stable boss/objective set.
    pub boss_damage: i64,
    pub healing: i64,
    /// Healing from this skill that actually modified HP when HpLessen was present.
    pub effective_healing: i64,
    /// Requested healing from this skill that did not modify HP.
    pub overhealing: i64,
    pub hits: u64,
    pub crits: u64,
    pub lucky_hits: u64,
    pub max_value: i64,
}

/// Encounter uptime for a named player buff. Only game-observed apply/remove
/// events are counted; unknown buff ids are intentionally not guessed.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct BuffUptime {
    pub buff_id: i32,
    pub name: String,
    pub uptime_ms: u64,
    pub activations: u32,
}

/// One event shown in the pre-death timeline. `is_heal` differentiates incoming
/// healing from incoming damage without introducing another serialized enum.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DeathRecapEvent {
    pub at_ms: u64,
    pub is_heal: bool,
    pub source_name: String,
    pub skill_id: i32,
    pub skill_name: String,
    pub value: i64,
    pub crit: bool,
    pub lucky: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DeathRecap {
    pub death_no: u32,
    pub at_ms: u64,
    pub events: Vec<DeathRecapEvent>,
}

/// Incoming damage split by attacker and skill for Tank analysis.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TakenSourceBreakdown {
    pub source_uuid: i64,
    pub source_name: String,
    pub skill_id: i32,
    pub skill_name: String,
    pub damage: i64,
    pub hits: u64,
    pub crits: u64,
    pub lucky_hits: u64,
    pub max_value: i64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DpsRow {
    pub actor_uuid: i64,
    pub uid: i64,
    pub name: String,
    pub profession_id: i32,
    pub subprofession_id: i32,
    pub subprofession_name: String,
    pub ability_score: i64,
    pub illusion_break: i64,
    pub hp: i64,
    pub max_hp: i64,
    pub damage: i64,
    pub healing: i64,
    pub damage_taken: i64,
    /// Direct type-5 Absorbed events kept separate from HP damage taken.
    pub absorbed_damage: i64,
    /// Damage dealt to the stable boss/objective target set for this encounter.
    pub boss_damage: i64,
    /// Healing that actually modified HP according to the packet HpLessen value.
    pub effective_healing: i64,
    /// Requested healing that did not modify HP.
    pub overhealing: i64,
    /// Direct character-sheet Block percentage in hundredths of a percent.
    pub block_pct: i64,
    /// Encounter DPS: total damage divided by the whole encounter duration.
    pub dps: f64,
    /// Active rates exclude long per-entity downtime and late starts.
    pub active_dps: f64,
    pub active_hps: f64,
    pub active_dtps: f64,
    pub damage_share: f64,
    pub healing_share: f64,
    pub tank_share: f64,
    pub hits: u64,
    pub crits: u64,
    pub lucky_hits: u64,
    pub deaths: u32,
    pub is_dead: bool,
    /// Expiry of buff 2110057 (Weakened: Wish Sealed) observed on this player.
    /// 0 means there is no active revive-blocking debuff.
    pub revive_blocked_until_ms: i64,
    /// True only for the character owned by this ReadyAlert process.
    pub is_local: bool,
    /// True when the player is known to belong to the current local party.
    pub is_party: bool,
    /// Current-scene attributes that can be safely shown by the inspector.
    pub attributes: Vec<TrackedAttribute>,
    pub imagines: Vec<ImagineBadge>,
    /// Outgoing damage/healing skill distribution.
    pub skills: Vec<SkillBreakdown>,
    /// Incoming damage grouped by the attack skill id.
    pub taken_skills: Vec<SkillBreakdown>,
    /// Named player buffs with current-encounter uptime.
    pub buff_uptimes: Vec<BuffUptime>,
    /// Up to the most recent deaths, each with the preceding combat timeline.
    pub death_recaps: Vec<DeathRecap>,
    /// Incoming HP damage split by source monster/entity and skill.
    pub taken_sources: Vec<TakenSourceBreakdown>,
    /// Direct shield/absorbed events split by source and skill. `damage` stores
    /// the absorbed amount; this does not include inferred mixed shield+HP hits.
    pub absorbed_sources: Vec<TakenSourceBreakdown>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TargetSnapshot {
    pub entity_uuid: i64,
    pub name: String,
    pub hp: i64,
    pub max_hp: i64,
    /// Only populated from a real observed game timer. Never estimated.
    pub enrage_remaining_ms: Option<i64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DpsSnapshot {
    pub encounter_ms: u64,
    pub total_damage: i64,
    pub total_healing: i64,
    pub total_damage_taken: i64,
    pub total_absorbed_damage: i64,
    pub total_boss_damage: i64,
    pub target: Option<TargetSnapshot>,
    pub rows: Vec<DpsRow>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TrackedAttribute {
    pub attr_id: i32,
    pub label: String,
    pub value: i64,
}

#[derive(Clone, Debug, Default)]
pub struct ConsumableStatus {
    pub buff_id: i32,
    pub name: String,
    /// Wall-clock expiry; 0 means the protocol did not expose a finite timer.
    pub expires_unix_ms: i64,
}

#[derive(Clone, Debug, Default)]
pub struct MechanicRow {
    pub key: String,
    pub label: String,
    pub target: Option<String>,
    pub created_unix_ms: i64,
    pub expires_unix_ms: i64,
    pub persistent: bool,
    pub priority: u8,
}

#[derive(Clone, Debug, Default)]
pub struct MechanicSnapshot {
    pub tracked_attributes: Vec<TrackedAttribute>,
    pub food: Option<ConsumableStatus>,
    pub serum: Option<ConsumableStatus>,
    pub rows: Vec<MechanicRow>,
}

#[derive(Clone, Debug)]
pub enum AppEvent {
    Alert(AlertEvent),
    Chat(ChatMessage),
    Translation { sequence_id: u64, text: String, source_language: String },
    Identity(PlayerIdentity),
    CaptureStatus(String),
    Dps(DpsSnapshot),
    Mechanics(MechanicSnapshot),
}

pub fn channel_name(channel: i32) -> &'static str {
    match channel {
        1 => "World",
        2 => "Local",
        3 => "Team",
        4 => "Guild",
        5 => "Private",
        6 => "Group",
        7 => "Top",
        8 => "Play",
        9 => "Newbie",
        99 => "System",
        _ => "Chat",
    }
}
