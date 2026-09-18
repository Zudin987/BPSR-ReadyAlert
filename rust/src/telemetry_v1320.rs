use crate::{model::AppEvent, proto};
use std::sync::mpsc::Sender;

mod previous {
    include!(concat!(env!("OUT_DIR"), "/telemetry_v1270_lifecycle.rs"));
}

pub use previous::{
    arm_benchmark, benchmark_status, benchmark_completion_pending, default_benchmark_seconds, request_manual_reset, BenchmarkStatus,
};

const SYNC_NEAR_ENTITIES: u32 = 0x06;
const SYNC_NEAR_DELTA_INFO: u32 = 0x2d;
const SYNC_TO_ME_DELTA_INFO: u32 = 0x2e;

const ATTR_MONSTER_ID: u64 = 0x0a;
const ATTR_SKILL_ID: u64 = 0x64;
const ATTR_PROFESSION_ID: u64 = 0xdc;
const ATTR_SEASON_STRENGTH: u64 = 11_440;
const ATTR_SEASON_STRENGTH_TOTAL: u64 = 11_441;
const SEASON_ATTR_PROBE_END: u64 = 11_449;

/// Thin forward-compatibility wrapper. The proven telemetry implementation still
/// owns all combat calculations. This layer only observes IDs that already exist
/// in packets so an unknown Season 4 value can be diagnosed without changing,
/// rejecting or delaying the original packet.
pub struct TelemetryRuntime {
    inner: previous::TelemetryRuntime,
}

impl TelemetryRuntime {
    pub fn new(tx: Sender<AppEvent>) -> Self {
        Self { inner: previous::TelemetryRuntime::new(tx) }
    }

    pub fn handle_notify(&mut self, service: u64, method: u32, body: &[u8]) {
        probe_forward_compat_ids(service, method, body);
        self.inner.handle_notify(service, method, body);
    }
}

fn probe_forward_compat_ids(service: u64, method: u32, body: &[u8]) {
    if service != proto::WORLD_SERVICE {
        return;
    }
    match method {
        proto::ENTER_SCENE_METHOD => {
            if let Some(info) = proto::get_len_field(body, 1) {
                if let Some(player) = proto::get_len_field(info, 2) {
                    probe_attrs(proto::get_len_field(player, 3));
                }
            }
        }
        SYNC_NEAR_ENTITIES => {
            for entity in proto::len_fields(body, 1) {
                probe_attrs(proto::get_len_field(entity, 3));
            }
        }
        SYNC_NEAR_DELTA_INFO => {
            for delta in proto::len_fields(body, 1) {
                probe_delta(delta);
            }
        }
        SYNC_TO_ME_DELTA_INFO => {
            if let Some(wrapper) = proto::get_len_field(body, 1) {
                if let Some(delta) = proto::get_len_field(wrapper, 1) {
                    probe_delta(delta);
                }
            }
        }
        _ => {}
    }
}

fn probe_delta(delta: &[u8]) {
    probe_attrs(proto::get_len_field(delta, 2));

    if let Some(effects) = proto::get_len_field(delta, 7) {
        for damage in proto::len_fields(effects, 2) {
            if let Some(id) = proto::get_varint_field(damage, 12).map(|id| id as i32) {
                let _ = crate::telemetry::game_names_v1300::skill_name(id);
            }
        }
    }

    if let Some(buff_effects) = proto::get_len_field(delta, 10) {
        for effect in proto::len_fields(buff_effects, 2) {
            if let Some(id) = proto::get_varint_field(effect, 2).map(|id| id as i32) {
                let _ = crate::telemetry::game_names_v1300::buff_name(id);
            }
        }
    }
}

fn probe_attrs(attrs: Option<&[u8]>) {
    let Some(attrs) = attrs else { return; };
    for attr in proto::len_fields(attrs, 2) {
        let id = proto::get_varint_field(attr, 1).unwrap_or(0);
        let Some(raw) = proto::get_len_field(attr, 2) else { continue; };
        match id {
            ATTR_MONSTER_ID => {
                if let Some(id) = raw_varint(raw).and_then(|value| i32::try_from(value).ok()) {
                    let _ = crate::telemetry::game_names_v1300::monster_name(id);
                }
            }
            ATTR_SKILL_ID => {
                if let Some(id) = raw_varint(raw).map(|value| value as i32) {
                    let _ = crate::telemetry::game_names_v1300::skill_name(id);
                }
            }
            ATTR_PROFESSION_ID => {
                if let Some(id) = raw_varint(raw).and_then(|value| i32::try_from(value).ok()) {
                    let _ = crate::telemetry::game_names_v1300::class_name(id);
                }
            }
            ATTR_SEASON_STRENGTH..=SEASON_ATTR_PROBE_END => {
                // 11440/11441 are the known generic Season Strength final/total
                // pair. Nearby ids are intentionally watched because a future
                // season could introduce another member of the same stat family.
                let id = id as i32;
                if crate::telemetry::game_names_v1300::attribute_name(id).is_none()
                    && id != ATTR_SEASON_STRENGTH as i32
                    && id != ATTR_SEASON_STRENGTH_TOTAL as i32
                {
                    crate::telemetry::game_names_v1300::note_unknown_protocol_id(b'A', id);
                }
            }
            _ => {}
        }
    }
}

fn raw_varint(raw: &[u8]) -> Option<u64> {
    let mut position = 0usize;
    proto::read_varint(raw, &mut position)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn season_strength_probe_range_keeps_known_final_and_total_ids() {
        assert_eq!(ATTR_SEASON_STRENGTH, 11_440);
        assert_eq!(ATTR_SEASON_STRENGTH_TOTAL, 11_441);
        assert!(ATTR_SEASON_STRENGTH_TOTAL <= SEASON_ATTR_PROBE_END);
    }

    #[test]
    fn unknown_probe_never_changes_or_rejects_packets() {
        // A malformed/empty notify is simply observed then forwarded by runtime;
        // the probe itself is deliberately best-effort and side-effect-free apart
        // from diagnostic name lookups.
        probe_forward_compat_ids(proto::WORLD_SERVICE, SYNC_NEAR_DELTA_INFO, &[]);
    }
}
