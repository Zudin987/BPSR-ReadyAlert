use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AlertKind { Queue, Ready, PartyInvite, PartyRequest, Error }
#[derive(Clone, Debug)]
pub struct AlertEvent { pub kind: AlertKind, pub title: String, pub message: String }
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum ChatKind { Text = 0, TextNotice = 1, MultiLanguageNotice = 2, Sticker = 3, Picture = 4, Voice = 5, Hypertext = 6 }
impl ChatKind { pub fn from_u64(v: u64) -> Self { match v { 1 => Self::TextNotice, 2 => Self::MultiLanguageNotice, 3 => Self::Sticker, 4 => Self::Picture, 5 => Self::Voice, 6 => Self::Hypertext, _ => Self::Text } } }
#[derive(Clone, Debug)]
pub struct ChatMessage { pub message_id: i64, pub sequence_id: u64, pub sender_id: i64, pub sender_name: String, pub sender_level: i32, pub channel: i32, pub unix_seconds: i64, pub kind: ChatKind, pub text: String }
#[derive(Clone, Debug)]
pub struct PlayerIdentity { pub name: String, pub uid: i64 }

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ImagineBadge {
    pub skill_id: i32,
    pub name: String,
    /// The protocol tier is already zero-based: 0 = T0 and 5 = T5.
    pub tier: i32,
    pub icon_key: String,
}
pub fn imagine_tier_display(raw: i32) -> i32 { raw.max(0) }
pub fn imagine_tier_label(raw: i32) -> String { format!("T{}", imagine_tier_display(raw)) }

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SkillBreakdown {
    pub skill_id: i32, pub name: String, pub damage: i64, pub boss_damage: i64,
    pub healing: i64, pub effective_healing: i64, pub overhealing: i64,
    pub hits: u64, pub crits: u64, pub lucky_hits: u64,
    /// Zero means min/max were not retained in an older history record.
    pub min_value: i64, pub max_value: i64,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct BuffUptime { pub buff_id: i32, pub name: String, pub uptime_ms: u64, pub activations: u32 }
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DeathRecapEvent { pub at_ms: u64, pub is_heal: bool, pub source_name: String, pub skill_id: i32, pub skill_name: String, pub value: i64, pub crit: bool, pub lucky: bool }
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DeathRecap { pub death_no: u32, pub at_ms: u64, pub events: Vec<DeathRecapEvent> }
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TakenSourceBreakdown { pub source_uuid: i64, pub source_name: String, pub skill_id: i32, pub skill_name: String, pub damage: i64, pub hits: u64, pub crits: u64, pub lucky_hits: u64, pub max_value: i64 }
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ConsumableStatus { pub buff_id: i32, pub name: String, pub expires_unix_ms: i64, pub duration_ms: i64 }

/// Summary of an observed character-sheet stat during one encounter. This is
/// observational data, NOT a damage-formula prediction. Values use the same raw
/// units as TrackedAttribute (percentage attributes are hundredths of 1%).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct EncounterAttributeSummary {
    pub attr_id: i32,
    pub label: String,
    /// Value observed at the start, or None if no trustworthy start value exists.
    pub initial: Option<i64>,
    /// Time-weighted mean of known intervals; None when coverage is zero.
    pub average: Option<f64>,
    /// Last observed value of this encounter; not necessarily the end-of-fight value.
    pub final_value: Option<i64>,
    /// Only intervals with an observed value count toward the mean.
    pub observed_ms: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DpsRow {
    pub actor_uuid: i64, pub uid: i64, pub name: String,
    pub profession_id: i32, pub subprofession_id: i32, pub subprofession_name: String,
    pub ability_score: i64, pub illusion_break: i64, pub hp: i64, pub max_hp: i64,
    pub damage: i64, pub healing: i64, pub damage_taken: i64,
    /// Direct type-5 absorption, separate from HP damage taken.
    pub absorbed_damage: i64,
    pub boss_damage: i64,
    pub effective_healing: i64, pub overhealing: i64,
    /// Character-sheet Block percentage in hundredths of a percent.
    pub block_pct: i64,
    /// Total encounter damage divided by the full encounter duration.
    pub dps: f64,
    pub active_dps: f64, pub active_hps: f64, pub active_dtps: f64,
    pub damage_share: f64, pub healing_share: f64, pub tank_share: f64,
    pub hits: u64, pub crits: u64, pub lucky_hits: u64, pub deaths: u32,
    pub is_dead: bool, pub revive_blocked_until_ms: i64,
    pub is_local: bool, pub is_party: bool,
    pub attributes: Vec<TrackedAttribute>,
    /// Per-encounter stat observations; empty in legacy saves.
    pub encounter_attributes: Vec<EncounterAttributeSummary>,
    pub food: Option<ConsumableStatus>, pub serum: Option<ConsumableStatus>,
    pub imagines: Vec<ImagineBadge>,
    pub skills: Vec<SkillBreakdown>, pub taken_skills: Vec<SkillBreakdown>,
    pub buff_uptimes: Vec<BuffUptime>, pub death_recaps: Vec<DeathRecap>,
    pub taken_sources: Vec<TakenSourceBreakdown>, pub absorbed_sources: Vec<TakenSourceBreakdown>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TargetSnapshot { pub entity_uuid: i64, pub name: String, pub hp: i64, pub max_hp: i64, pub enrage_remaining_ms: Option<i64> }
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DpsSnapshot {
    pub encounter_ms: u64, pub total_damage: i64, pub total_healing: i64,
    pub total_damage_taken: i64, pub total_absorbed_damage: i64, pub total_boss_damage: i64,
    pub target: Option<TargetSnapshot>, pub rows: Vec<DpsRow>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TrackedAttribute { pub attr_id: i32, pub label: String, pub value: i64 }
#[derive(Clone, Debug, Default)]
pub struct MechanicRow { pub key: String, pub label: String, pub target: Option<String>, pub created_unix_ms: i64, pub expires_unix_ms: i64, pub persistent: bool, pub priority: u8 }
#[derive(Clone, Debug, Default)]
pub struct MechanicSnapshot { pub tracked_attributes: Vec<TrackedAttribute>, pub food: Option<ConsumableStatus>, pub serum: Option<ConsumableStatus>, pub rows: Vec<MechanicRow> }
#[derive(Clone, Debug)]
pub enum AppEvent {
    Alert(AlertEvent), Chat(ChatMessage),
    Translation { sequence_id: u64, text: String, source_language: String },
    Identity(PlayerIdentity), CaptureStatus(String), Dps(DpsSnapshot), Mechanics(MechanicSnapshot),
}
pub fn channel_name(channel: i32) -> &'static str {
    match channel { 1 => "World", 2 => "Local", 3 => "Team", 4 => "Guild", 5 => "Private", 6 => "Group", 7 => "Top", 8 => "Play", 9 => "Newbie", 99 => "System", _ => "Chat" }
}
#[cfg(test)]
mod imagine_tier_tests {
    use super::*;
    #[test]
    fn imagine_tier_keeps_protocol_t0_and_t5() {
        assert_eq!(imagine_tier_display(0), 0);
        assert_eq!(imagine_tier_label(0), "T0");
        assert_eq!(imagine_tier_display(5), 5);
        assert_eq!(imagine_tier_label(5), "T5");
    }
    #[test]
    fn imagine_tier_never_exposes_negative_protocol_noise() {
        assert_eq!(imagine_tier_label(-1), "T0");
    }
    #[test]
    fn older_records_without_attribute_summaries_still_deserialize() {
        let row: DpsRow = serde_json::from_str(r#"{"uid":42,"damage":100}"#).unwrap();
        assert!(row.encounter_attributes.is_empty());
    }
}
