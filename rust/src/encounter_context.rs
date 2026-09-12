use crate::{game_data_v1260, proto};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::{OnceLock, RwLock}};

const SYNC_NEAR_ENTITIES: u32 = 0x06;
const SYNC_NEAR_DELTA_INFO: u32 = 0x2d;
const SYNC_TO_ME_DELTA_INFO: u32 = 0x2e;
const ATTR_EQUIP_DATA: u64 = 200;
const ATTR_SCENE_BASIC_ID: u64 = 341;
const ENTITY_PLAYER: i64 = 10;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ObservedGear {
    pub slot: i32,
    pub equip_id: i32,
    pub part: i32,
    pub gear_level: i32,
    pub suit_id: i32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PlayerGear {
    pub actor_uuid: i64,
    pub uid: i64,
    pub items: Vec<ObservedGear>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct EncounterContextSnapshot {
    pub scene_id: i32,
    pub dungeon_id: i32,
    pub scene_name: String,
    pub difficulty: String,
    pub play_type: i32,
    pub player_gear: Vec<PlayerGear>,
}

#[derive(Default)]
struct LiveContext {
    scene_id: i32,
    dungeon_id: i32,
    scene_name: String,
    difficulty: String,
    play_type: i32,
    local_uuid: i64,
    gear: HashMap<i64, Vec<ObservedGear>>,
}

static LIVE: OnceLock<RwLock<LiveContext>> = OnceLock::new();

fn live() -> &'static RwLock<LiveContext> {
    LIVE.get_or_init(|| RwLock::new(LiveContext::default()))
}

pub fn observe_notify(service: u64, method: u32, body: &[u8]) {
    if service != proto::WORLD_SERVICE {
        return;
    }
    let Ok(mut state) = live().write() else { return; };
    match method {
        proto::ENTER_SCENE_METHOD => {
            state.gear.clear();
            state.local_uuid = 0;
            state.scene_id = 0;
            state.dungeon_id = 0;
            state.scene_name.clear();
            state.difficulty.clear();
            state.play_type = 0;

            if let Some(scene_id) = find_attr_value(body, ATTR_SCENE_BASIC_ID, 4)
                .and_then(raw_varint)
                .and_then(|value| i32::try_from(value).ok())
            {
                apply_scene(&mut state, scene_id);
            }

            if let Some(info) = proto::get_len_field(body, 1) {
                if let Some(player) = proto::get_len_field(info, 2) {
                    let uuid = signed(proto::get_varint_field(player, 1).unwrap_or(0));
                    if uuid != 0 {
                        state.local_uuid = uuid;
                        observe_entity_attrs(&mut state, uuid, proto::get_len_field(player, 3));
                    }
                }
            }
        }
        SYNC_NEAR_ENTITIES => {
            for entity in proto::len_fields(body, 1) {
                let uuid = signed(proto::get_varint_field(entity, 1).unwrap_or(0));
                if uuid != 0 {
                    observe_entity_attrs(&mut state, uuid, proto::get_len_field(entity, 3));
                }
            }
        }
        SYNC_NEAR_DELTA_INFO => {
            for delta in proto::len_fields(body, 1) {
                observe_delta(&mut state, delta, None);
            }
        }
        SYNC_TO_ME_DELTA_INFO => {
            if let Some(wrapper) = proto::get_len_field(body, 1) {
                if let Some(delta) = proto::get_len_field(wrapper, 1) {
                    let fallback = (state.local_uuid != 0).then_some(state.local_uuid);
                    observe_delta(&mut state, delta, fallback);
                }
            }
        }
        _ => {}
    }
}

pub fn snapshot() -> EncounterContextSnapshot {
    let Ok(state) = live().read() else { return EncounterContextSnapshot::default(); };
    let mut player_gear: Vec<PlayerGear> = state.gear.iter().map(|(&actor_uuid, items)| {
        let uid = if entity_kind(actor_uuid) == ENTITY_PLAYER { actor_uuid >> 16 } else { 0 };
        let mut items = items.clone();
        items.sort_by_key(|item| (item.part, item.slot, item.equip_id));
        PlayerGear { actor_uuid, uid, items }
    }).collect();
    player_gear.sort_by_key(|player| (player.uid, player.actor_uuid));
    EncounterContextSnapshot {
        scene_id: state.scene_id,
        dungeon_id: state.dungeon_id,
        scene_name: state.scene_name.clone(),
        difficulty: state.difficulty.clone(),
        play_type: state.play_type,
        player_gear,
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

fn observe_delta(state: &mut LiveContext, delta: &[u8], fallback_uuid: Option<i64>) {
    let raw_uuid = signed(proto::get_varint_field(delta, 1).unwrap_or(0));
    let uuid = if raw_uuid != 0 { raw_uuid } else { fallback_uuid.unwrap_or(0) };
    if uuid == 0 {
        return;
    }
    observe_entity_attrs(state, uuid, proto::get_len_field(delta, 2));
}

fn observe_entity_attrs(state: &mut LiveContext, uuid: i64, attrs: Option<&[u8]>) {
    let Some(attrs) = attrs else { return; };
    for attr in proto::len_fields(attrs, 2) {
        let id = proto::get_varint_field(attr, 1).unwrap_or(0);
        let Some(raw) = proto::get_len_field(attr, 2) else { continue; };
        if id == ATTR_EQUIP_DATA {
            state.gear.insert(uuid, parse_equipment(raw));
        }
    }
}

fn parse_equipment(raw: &[u8]) -> Vec<ObservedGear> {
    let mut out = Vec::new();
    let mut position = 0usize;
    while position < raw.len() {
        let before = position;
        let Some(length) = proto::read_varint(raw, &mut position) else { break; };
        let Ok(length) = usize::try_from(length) else { break; };
        let Some(end) = position.checked_add(length) else { break; };
        if length == 0 || end > raw.len() {
            position = before;
            break;
        }
        push_equipment(&mut out, &raw[position..end]);
        position = end;
    }

    // Defensive fallback for captures where the attr payload is one bare
    // EquipNine protobuf rather than the game's concatenated length form.
    if out.is_empty() {
        push_equipment(&mut out, raw);
    }
    out.sort_by_key(|item| (item.part, item.slot, item.equip_id));
    out.dedup_by_key(|item| (item.slot, item.equip_id));
    out
}

fn push_equipment(out: &mut Vec<ObservedGear>, message: &[u8]) {
    let slot = proto::get_varint_field(message, 1).unwrap_or(0) as i32;
    let equip_id = proto::get_varint_field(message, 2).unwrap_or(0) as i32;
    if equip_id <= 0 {
        return;
    }
    let meta = game_data_v1260::equip_meta(equip_id).unwrap_or_default();
    out.push(ObservedGear {
        slot,
        equip_id,
        part: meta.part,
        gear_level: meta.gear_level,
        suit_id: meta.suit_id,
    });
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

fn signed(value: u64) -> i64 { value as i64 }
fn entity_kind(uuid: i64) -> i64 { (uuid >> 6) & 0x1f }

#[cfg(test)]
mod tests {
    use super::*;

    fn varint(mut value: u64) -> Vec<u8> {
        let mut out = Vec::new();
        loop {
            let mut byte = (value & 0x7f) as u8;
            value >>= 7;
            if value != 0 { byte |= 0x80; }
            out.push(byte);
            if value == 0 { break; }
        }
        out
    }

    #[test]
    fn equip_nine_length_stream_resolves_supplied_metadata() {
        let mut msg = vec![0x08];
        msg.extend(varint(1));
        msg.push(0x10);
        msg.extend(varint(2011440));
        let mut raw = varint(msg.len() as u64);
        raw.extend(msg);
        let items = parse_equipment(&raw);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].equip_id, 2011440);
        assert_eq!(items[0].gear_level, 260);
        assert_eq!(items[0].suit_id, 102);
    }

    #[test]
    fn ambiguous_scene_keeps_id_without_guessing_name() {
        let mut state = LiveContext::default();
        apply_scene(&mut state, 1031);
        assert_eq!(state.scene_id, 1031);
        assert_eq!(state.dungeon_id, 0);
        assert!(state.scene_name.is_empty());
    }
}
