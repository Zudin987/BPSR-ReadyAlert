use crate::{game_data_v1260, proto};
use serde::{Deserialize, Serialize};
use std::sync::{OnceLock, RwLock};

const ATTR_SCENE_BASIC_ID: u64 = 341;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct EncounterContextSnapshot {
    pub scene_id: i32,
    pub dungeon_id: i32,
    pub scene_name: String,
    pub difficulty: String,
    pub play_type: i32,
    /// Set only for explicitly started ReadyAlert benchmark captures.
    pub benchmark_name: String,
    pub benchmark_duration_ms: u64,
}

#[derive(Default)]
struct LiveContext {
    scene_id: i32,
    dungeon_id: i32,
    scene_name: String,
    difficulty: String,
    play_type: i32,
}

static LIVE: OnceLock<RwLock<LiveContext>> = OnceLock::new();

fn live() -> &'static RwLock<LiveContext> {
    LIVE.get_or_init(|| RwLock::new(LiveContext::default()))
}

pub fn observe_notify(service: u64, method: u32, body: &[u8]) {
    if service != proto::WORLD_SERVICE || method != proto::ENTER_SCENE_METHOD {
        return;
    }
    let Ok(mut state) = live().write() else { return; };
    *state = LiveContext::default();
    if let Some(scene_id) = find_attr_value(body, ATTR_SCENE_BASIC_ID, 4)
        .and_then(raw_varint)
        .and_then(|value| i32::try_from(value).ok())
    {
        apply_scene(&mut state, scene_id);
    }
}

pub fn snapshot() -> EncounterContextSnapshot {
    let Ok(state) = live().read() else { return EncounterContextSnapshot::default(); };
    EncounterContextSnapshot {
        scene_id: state.scene_id,
        dungeon_id: state.dungeon_id,
        scene_name: state.scene_name.clone(),
        difficulty: state.difficulty.clone(),
        play_type: state.play_type,
        benchmark_name: String::new(),
        benchmark_duration_ms: 0,
    }
}

fn apply_scene(state: &mut LiveContext, scene_id: i32) {
    state.scene_id = scene_id;
    let dungeon_id = game_data_v1260::unique_dungeon_id_for_scene(scene_id);
    if dungeon_id == 0 {
        // Multiple dungeon ids can reuse one scene. Keep the numeric scene id,
        // but never guess a name/difficulty from an ambiguous mapping.
        return;
    }
    if let Some(meta) = game_data_v1260::dungeon_by_id(dungeon_id) {
        state.dungeon_id = meta.dungeon_id;
        state.scene_name = meta.name.to_string();
        state.difficulty = meta.difficulty.to_string();
        state.play_type = meta.play_type;
    }
}

fn find_attr_value<'a>(buf: &'a [u8], wanted: u64, depth: u8) -> Option<&'a [u8]> {
    for attr in proto::len_fields(buf, 2) {
        if proto::get_varint_field(attr, 1) == Some(wanted) {
            if let Some(raw) = proto::get_len_field(attr, 2) {
                return Some(raw);
            }
        }
    }
    if depth == 0 {
        return None;
    }
    for field in 1..=20u32 {
        for nested in proto::len_fields(buf, field) {
            if let Some(raw) = find_attr_value(nested, wanted, depth - 1) {
                return Some(raw);
            }
        }
    }
    None
}

fn raw_varint(raw: &[u8]) -> Option<u64> {
    let mut position = 0usize;
    proto::read_varint(raw, &mut position)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ambiguous_scene_keeps_id_without_guessing_name() {
        let mut state = LiveContext::default();
        apply_scene(&mut state, 1031);
        assert_eq!(state.scene_id, 1031);
        assert_eq!(state.dungeon_id, 0);
        assert!(state.scene_name.is_empty());
    }

    #[test]
    fn benchmark_metadata_defaults_empty_for_old_records() {
        let parsed: EncounterContextSnapshot = serde_json::from_str(r#"{"scene_id":1151}"#).unwrap();
        assert_eq!(parsed.scene_id, 1151);
        assert!(parsed.benchmark_name.is_empty());
        assert_eq!(parsed.benchmark_duration_ms, 0);
    }
}
