use crate::{event_tracker, model::AppEvent, proto};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{SystemTime, UNIX_EPOCH};

mod previous {
    include!("telemetry_v1110.rs");
}

const SYNC_NEAR_ENTITIES: u32 = 0x06;
const SYNC_NEAR_DELTA_INFO: u32 = 0x2d;
const SYNC_TO_ME_DELTA_INFO: u32 = 0x2e;
const ENTITY_PLAYER: i64 = 10;

pub fn request_manual_reset() { previous::request_manual_reset(); }

pub struct TelemetryRuntime {
    inner: previous::TelemetryRuntime,
    inner_rx: Receiver<AppEvent>,
    tx: Sender<AppEvent>,
    local_uuid: i64,
}

impl TelemetryRuntime {
    pub fn new(tx: Sender<AppEvent>) -> Self {
        let (inner_tx, inner_rx) = mpsc::channel();
        Self { inner: previous::TelemetryRuntime::new(inner_tx), inner_rx, tx, local_uuid: 0 }
    }

    pub fn handle_notify(&mut self, service: u64, method: u32, body: &[u8]) {
        if service == proto::WORLD_SERVICE && method == proto::ENTER_SCENE_METHOD {
            self.local_uuid = 0;
            event_tracker::clear_scene();
        }

        self.inner.handle_notify(service, method, body);
        let mut pending = Vec::new();
        while let Ok(event) = self.inner_rx.try_recv() {
            if let AppEvent::Dps(snapshot) = &event { event_tracker::update_roster(&snapshot.rows); }
            pending.push(event);
        }

        self.observe_notify(service, method, body);
        for event in pending { let _ = self.tx.send(event); }
    }

    fn observe_notify(&mut self, service: u64, method: u32, body: &[u8]) {
        if service != proto::WORLD_SERVICE { return; }
        match method {
            proto::ENTER_SCENE_METHOD => {
                if let Some(info) = proto::get_len_field(body, 1) {
                    if let Some(player) = proto::get_len_field(info, 2) {
                        let uuid = signed(proto::get_varint_field(player, 1).unwrap_or(0));
                        if uuid != 0 {
                            self.local_uuid = uuid;
                            if let Some(buffs) = proto::get_len_field(player, 7) { observe_buff_snapshot(uuid, buffs); }
                        }
                    }
                }
            }
            SYNC_NEAR_ENTITIES => {
                for entity in proto::len_fields(body, 1) {
                    let uuid = signed(proto::get_varint_field(entity, 1).unwrap_or(0));
                    if uuid == 0 { continue; }
                    if let Some(buffs) = proto::get_len_field(entity, 7) { observe_buff_snapshot(uuid, buffs); }
                }
            }
            SYNC_NEAR_DELTA_INFO => {
                for delta in proto::len_fields(body, 1) { self.observe_delta(delta, None); }
            }
            SYNC_TO_ME_DELTA_INFO => {
                if let Some(wrapper) = proto::get_len_field(body, 1) {
                    if let Some(delta) = proto::get_len_field(wrapper, 1) {
                        self.observe_delta(delta, (self.local_uuid != 0).then_some(self.local_uuid));
                    }
                }
            }
            _ => {}
        }
    }

    fn observe_delta(&self, delta: &[u8], fallback_uuid: Option<i64>) {
        let raw_uuid = signed(proto::get_varint_field(delta, 1).unwrap_or(0));
        let target_uuid = if raw_uuid != 0 { raw_uuid } else { fallback_uuid.unwrap_or(0) };
        if target_uuid == 0 { return; }
        if let Some(effects) = proto::get_len_field(delta, 7) {
            for damage in proto::len_fields(effects, 2) { observe_skill(target_uuid, damage); }
        }
        if let Some(buff_effect) = proto::get_len_field(delta, 10) {
            for effect in proto::len_fields(buff_effect, 2) { observe_buff_effect(target_uuid, effect); }
        }
    }
}

fn observe_skill(target_uuid: i64, damage: &[u8]) {
    let miss = proto::get_varint_field(damage, 2).unwrap_or(0) != 0 || proto::get_varint_field(damage, 4).unwrap_or(0) == 1;
    if miss { return; }
    let skill_id = proto::get_varint_field(damage, 12).unwrap_or(0).min(i32::MAX as u64) as i32;
    if skill_id <= 0 { return; }
    let top_summoner = signed(proto::get_varint_field(damage, 21).unwrap_or(0));
    let raw_attacker = signed(proto::get_varint_field(damage, 11).unwrap_or(0));
    let source_uuid = if top_summoner != 0 { top_summoner } else { raw_attacker };
    let source_uid = player_uid(source_uuid);
    let target_uid = player_uid(target_uuid);
    event_tracker::observe_skill(skill_id, source_uid, target_uid);
}

fn observe_buff_snapshot(host_uuid: i64, sync: &[u8]) {
    let uid = player_uid(host_uuid);
    if uid <= 0 { return; }
    for info in proto::len_fields(sync, 2) {
        let buff_uuid = proto::get_varint_field(info, 1).unwrap_or(0).min(i32::MAX as u64) as i32;
        let base_id = proto::get_varint_field(info, 2).unwrap_or(0).min(i32::MAX as u64) as i32;
        if buff_uuid == 0 || base_id == 0 { continue; }
        event_tracker::observe_buff(uid, buff_uuid, base_id, false, packet_expiry(info));
    }
}

fn observe_buff_effect(host_uuid: i64, effect: &[u8]) {
    let uid = player_uid(host_uuid);
    if uid <= 0 { return; }
    let event_type = proto::get_varint_field(effect, 1).unwrap_or(0) as i32;
    let buff_uuid = proto::get_varint_field(effect, 2).unwrap_or(0).min(i32::MAX as u64) as i32;
    if buff_uuid == 0 { return; }
    if event_type == 2 {
        event_tracker::observe_buff(uid, buff_uuid, 0, true, 0);
        return;
    }
    if !matches!(event_type, 1 | 3 | 4 | 5) { return; }
    for logic in proto::len_fields(effect, 5) {
        if proto::get_varint_field(logic, 1).unwrap_or(0) != 18 { continue; }
        let Some(info) = proto::get_len_field(logic, 2) else { continue; };
        let base_id = proto::get_varint_field(info, 2).unwrap_or(0).min(i32::MAX as u64) as i32;
        if base_id > 0 { event_tracker::observe_buff(uid, buff_uuid, base_id, false, packet_expiry(info)); }
    }
}

fn packet_expiry(info: &[u8]) -> i64 {
    let duration_ms = proto::get_varint_field(info, 11).unwrap_or(0).min(24 * 60 * 60 * 1000) as i64;
    if duration_ms <= 0 { 0 } else { now_ms().saturating_add(duration_ms) }
}

fn player_uid(uuid: i64) -> i64 {
    if uuid != 0 && entity_kind(uuid) == ENTITY_PLAYER { uuid >> 16 } else { 0 }
}
fn entity_kind(uuid: i64) -> i64 { (uuid >> 6) & 0x1f }
fn signed(value: u64) -> i64 { value as i64 }
fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis().min(i64::MAX as u128) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_uid_rejects_non_player_entities() {
        let player_uuid = (12345_i64 << 16) | (ENTITY_PLAYER << 6);
        assert_eq!(player_uid(player_uuid), player_uuid >> 16);
        assert_eq!(player_uid(1 << 6), 0);
    }

    #[test]
    fn zero_duration_never_fabricates_tracker_timer() {
        assert_eq!(packet_expiry(&[]), 0);
    }
}
