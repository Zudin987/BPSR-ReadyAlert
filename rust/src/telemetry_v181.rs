use crate::{
    feature_settings::{ATTR_CURRENT_HP, ATTR_FIGHT_POINT, ATTR_MAX_HP, ATTR_SEASON_STRENGTH},
    model::{AppEvent, DpsSnapshot, MechanicSnapshot},
    proto,
};
use std::{
    collections::{HashMap, HashSet},
    sync::mpsc::{self, Receiver, Sender},
};

// Keep the audited v1.8.1 adapter intact and add a small scene-freshness guard
// around it. This avoids duplicating the packet/DPS implementation while
// preventing attributes omitted by a new scene's full-sync from leaking values
// cached in the previous map.
mod legacy {
    include!("telemetry_adapter_v170.rs");
}

const SYNC_NEAR_ENTITIES: u32 = 0x06;
const SYNC_CONTAINER_DATA: u32 = 0x15;
const SYNC_NEAR_DELTA_INFO: u32 = 0x2d;
const SYNC_TO_ME_DELTA_INFO: u32 = 0x2e;
const ATTR_PROFESSION_ID: i32 = 0xdc;
const ENTITY_PLAYER: i64 = 10;

pub fn request_manual_reset() {
    legacy::request_manual_reset();
}

pub struct TelemetryRuntime {
    inner: legacy::TelemetryRuntime,
    inner_rx: Receiver<AppEvent>,
    tx: Sender<AppEvent>,
    fresh_attrs: HashMap<i64, HashSet<i32>>,
    local_uid: i64,
}

impl TelemetryRuntime {
    pub fn new(tx: Sender<AppEvent>) -> Self {
        let (inner_tx, inner_rx) = mpsc::channel();
        Self {
            inner: legacy::TelemetryRuntime::new(inner_tx),
            inner_rx,
            tx,
            fresh_attrs: HashMap::new(),
            local_uid: 0,
        }
    }

    pub fn handle_notify(&mut self, service: u64, method: u32, body: &[u8]) {
        self.observe_freshness(service, method, body);
        self.inner.handle_notify(service, method, body);
        self.forward_inner_events();
    }

    fn forward_inner_events(&mut self) {
        while let Ok(event) = self.inner_rx.try_recv() {
            let event = match event {
                AppEvent::Dps(mut snapshot) => {
                    sanitize_dps(&mut snapshot, &self.fresh_attrs);
                    AppEvent::Dps(snapshot)
                }
                AppEvent::Mechanics(mut snapshot) => {
                    sanitize_mechanics(&mut snapshot, self.local_uid, &self.fresh_attrs);
                    AppEvent::Mechanics(snapshot)
                }
                other => other,
            };
            let _ = self.tx.send(event);
        }
    }

    fn observe_freshness(&mut self, service: u64, method: u32, body: &[u8]) {
        if service != proto::WORLD_SERVICE {
            return;
        }

        match method {
            proto::ENTER_SCENE_METHOD => {
                // A map/scene full-sync is allowed to omit attributes whose values
                // did not change. Never let omitted fields inherit the prior map.
                self.fresh_attrs.clear();
                self.local_uid = 0;
                if let Some(info) = proto::get_len_field(body, 1) {
                    if let Some(player) = proto::get_len_field(info, 2) {
                        let uuid = signed(proto::get_varint_field(player, 1).unwrap_or(0));
                        if is_player(uuid) {
                            self.local_uid = uuid >> 16;
                            self.observe_attr_block(uuid, proto::get_len_field(player, 3));
                        }
                    }
                }
            }
            SYNC_NEAR_ENTITIES => {
                for entity in proto::len_fields(body, 1) {
                    let uuid = signed(proto::get_varint_field(entity, 1).unwrap_or(0));
                    self.observe_attr_block(uuid, proto::get_len_field(entity, 3));
                }
            }
            SYNC_CONTAINER_DATA => {
                if let Some(data) = proto::get_len_field(body, 1) {
                    if let Some(uid) = proto::get_varint_field(data, 1).map(signed).filter(|v| *v > 0) {
                        self.local_uid = uid;
                        if proto::get_len_field(data, 2)
                            .and_then(|base| proto::get_varint_field(base, 35))
                            .is_some()
                        {
                            self.fresh_attrs.entry(uid).or_default().insert(ATTR_FIGHT_POINT);
                        }
                    }
                }
            }
            SYNC_NEAR_DELTA_INFO => {
                for delta in proto::len_fields(body, 1) {
                    self.observe_delta(delta, None);
                }
            }
            SYNC_TO_ME_DELTA_INFO => {
                if let Some(wrapper) = proto::get_len_field(body, 1) {
                    if let Some(delta) = proto::get_len_field(wrapper, 1) {
                        let fallback = (self.local_uid > 0).then_some(canonical_player_uuid(self.local_uid));
                        self.observe_delta(delta, fallback);
                    }
                }
            }
            _ => {}
        }
    }

    fn observe_delta(&mut self, delta: &[u8], fallback_uuid: Option<i64>) {
        let raw_uuid = signed(proto::get_varint_field(delta, 1).unwrap_or(0));
        let uuid = if raw_uuid != 0 { raw_uuid } else { fallback_uuid.unwrap_or(0) };
        self.observe_attr_block(uuid, proto::get_len_field(delta, 2));
    }

    fn observe_attr_block(&mut self, uuid: i64, attrs: Option<&[u8]>) {
        if !is_player(uuid) {
            return;
        }
        let uid = uuid >> 16;
        if uid <= 0 {
            return;
        }
        let Some(attrs) = attrs else { return; };
        let fresh = self.fresh_attrs.entry(uid).or_default();
        for attr in proto::len_fields(attrs, 2) {
            let Some(id) = proto::get_varint_field(attr, 1) else { continue; };
            if id <= i32::MAX as u64 && proto::get_len_field(attr, 2).is_some() {
                fresh.insert(id as i32);
            }
        }
    }
}

fn sanitize_dps(snapshot: &mut DpsSnapshot, fresh_attrs: &HashMap<i64, HashSet<i32>>) {
    for row in &mut snapshot.rows {
        let fresh = fresh_attrs.get(&row.uid);
        if !fresh.is_some_and(|set| set.contains(&ATTR_FIGHT_POINT)) {
            row.ability_score = 0;
        }
        if !fresh.is_some_and(|set| set.contains(&ATTR_SEASON_STRENGTH)) {
            row.illusion_break = 0;
        }
        if !fresh.is_some_and(|set| set.contains(&ATTR_CURRENT_HP)) {
            row.hp = 0;
        }
        if !fresh.is_some_and(|set| set.contains(&ATTR_MAX_HP)) {
            row.max_hp = 0;
        }
        for attr in &mut row.attributes {
            if !fresh.is_some_and(|set| set.contains(&attr.attr_id)) {
                attr.value = 0;
            }
        }
        // Current-scene combat skills are themselves fresh evidence of a spec,
        // so keep adapter inference when available. Otherwise do not show a
        // profession cached from the previous map.
        if row.skills.is_empty() && !fresh.is_some_and(|set| set.contains(&ATTR_PROFESSION_ID)) {
            row.profession_id = 0;
            row.subprofession_id = 0;
            row.subprofession_name.clear();
        }
    }
}

fn sanitize_mechanics(
    snapshot: &mut MechanicSnapshot,
    local_uid: i64,
    fresh_attrs: &HashMap<i64, HashSet<i32>>,
) {
    let fresh = fresh_attrs.get(&local_uid);
    for attr in &mut snapshot.tracked_attributes {
        if !fresh.is_some_and(|set| set.contains(&attr.attr_id)) {
            attr.value = 0;
        }
    }
}

fn signed(value: u64) -> i64 {
    value as i64
}

fn is_player(uuid: i64) -> bool {
    uuid != 0 && ((uuid >> 6) & 0x1f) == ENTITY_PLAYER && (uuid >> 16) > 0
}

fn canonical_player_uuid(uid: i64) -> i64 {
    (uid << 16) | (ENTITY_PLAYER << 6)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DpsRow, SkillBreakdown, TrackedAttribute};

    #[test]
    fn stale_scene_metadata_is_hidden() {
        let mut snapshot = DpsSnapshot {
            rows: vec![DpsRow {
                uid: 123,
                profession_id: 5,
                subprofession_id: 5001,
                subprofession_name: "Smite".into(),
                ability_score: 58_404,
                illusion_break: 7_200,
                hp: 242_288,
                max_hp: 242_288,
                attributes: vec![TrackedAttribute { attr_id: ATTR_CURRENT_HP, label: "HP".into(), value: 242_288 }],
                ..Default::default()
            }],
            ..Default::default()
        };
        sanitize_dps(&mut snapshot, &HashMap::new());
        let row = &snapshot.rows[0];
        assert_eq!(row.profession_id, 0);
        assert_eq!(row.ability_score, 0);
        assert_eq!(row.illusion_break, 0);
        assert_eq!(row.hp, 0);
        assert_eq!(row.max_hp, 0);
        assert_eq!(row.attributes[0].value, 0);
        assert!(row.subprofession_name.is_empty());
    }

    #[test]
    fn current_scene_skill_inference_is_preserved() {
        let mut snapshot = DpsSnapshot {
            rows: vec![DpsRow {
                uid: 123,
                profession_id: 5,
                subprofession_id: 5001,
                subprofession_name: "Smite".into(),
                skills: vec![SkillBreakdown { skill_id: 1518, ..Default::default() }],
                ..Default::default()
            }],
            ..Default::default()
        };
        sanitize_dps(&mut snapshot, &HashMap::new());
        assert_eq!(snapshot.rows[0].profession_id, 5);
        assert_eq!(snapshot.rows[0].subprofession_name, "Smite");
    }

    #[test]
    fn mechanics_drops_values_not_observed_in_current_scene() {
        let mut snapshot = MechanicSnapshot {
            tracked_attributes: vec![TrackedAttribute {
                attr_id: ATTR_FIGHT_POINT,
                label: "Ability Score".into(),
                value: 58_404,
            }],
            ..Default::default()
        };
        sanitize_mechanics(&mut snapshot, 123, &HashMap::new());
        assert_eq!(snapshot.tracked_attributes[0].value, 0);
    }
}
