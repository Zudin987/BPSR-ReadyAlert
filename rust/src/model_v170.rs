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

#[derive(Clone, Debug, Default)]
pub struct ImagineBadge {
    /// Resonance/Battle Imagine skill id resolved from the summon registry.
    pub skill_id: i32,
    pub name: String,
    /// Fantasy remodel level / tier observed on the summoned monster.
    pub tier: i32,
    pub icon_key: String,
}

#[derive(Clone, Debug, Default)]
pub struct SkillBreakdown {
    pub skill_id: i32,
    pub name: String,
    pub damage: i64,
    pub healing: i64,
    pub hits: u64,
    pub crits: u64,
    pub lucky_hits: u64,
    pub max_value: i64,
}

#[derive(Clone, Debug, Default)]
pub struct DpsRow {
    pub actor_uuid: i64,
    pub uid: i64,
    pub name: String,
    pub profession_id: i32,
    pub subprofession_id: i32,
    pub subprofession_name: String,
    pub ability_score: i64,
    pub illusion_break: i64,
    pub damage: i64,
    pub healing: i64,
    pub damage_taken: i64,
    pub dps: f64,
    pub damage_share: f64,
    pub healing_share: f64,
    pub tank_share: f64,
    pub hits: u64,
    pub crits: u64,
    pub lucky_hits: u64,
    pub deaths: u32,
    pub is_dead: bool,
    /// True only for the character owned by this ReadyAlert process.
    pub is_local: bool,
    pub imagines: Vec<ImagineBadge>,
    pub skills: Vec<SkillBreakdown>,
}

#[derive(Clone, Debug, Default)]
pub struct TargetSnapshot {
    pub entity_uuid: i64,
    pub name: String,
    pub hp: i64,
    pub max_hp: i64,
    /// Only populated from a real observed game timer. Never estimated.
    pub enrage_remaining_ms: Option<i64>,
}

#[derive(Clone, Debug, Default)]
pub struct DpsSnapshot {
    pub encounter_ms: u64,
    pub total_damage: i64,
    pub total_healing: i64,
    pub total_damage_taken: i64,
    pub target: Option<TargetSnapshot>,
    pub rows: Vec<DpsRow>,
}

#[derive(Clone, Debug, Default)]
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
