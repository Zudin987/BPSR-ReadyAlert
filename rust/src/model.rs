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
    /// BPSR ChitChatMsg.msg_id. Strong duplicate key when present.
    pub message_id: i64,
    /// Local capture sequence used to associate asynchronous translation results.
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

#[derive(Clone, Debug)]
pub enum AppEvent {
    Alert(AlertEvent),
    Chat(ChatMessage),
    Translation { sequence_id: u64, text: String, source_language: String },
    Identity(PlayerIdentity),
    CaptureStatus(String),
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
