use crate::{
    model::{AppEvent, DpsSnapshot, TakenSourceBreakdown},
    proto,
};
use std::{
    collections::{HashMap, HashSet},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    time::Instant,
};

mod previous {
    include!("telemetry_v1100.rs");
}

const SYNC_NEAR_ENTITIES: u32 = 0x06;
const SYNC_NEAR_DELTA_INFO: u32 = 0x2d;
const SYNC_TO_ME_DELTA_INFO: u32 = 0x2e;

const ATTR_NAME: u64 = 0x01;
const ATTR_MONSTER_ID: u64 = 0x0a;
const ATTR_TOP_SUMMONER_ID: u64 = 0x5b;

const ENTITY_MONSTER: i64 = 1;
const ENTITY_PLAYER: i64 = 10;
const DAMAGE_TYPE_HEAL: i32 = 2;
const DAMAGE_TYPE_IMMUNE: i32 = 3;
const DAMAGE_TYPE_ABSORBED: i32 = 5;

static ANALYSIS_RESET_REQUESTED: AtomicBool = AtomicBool::new(false);

pub fn request_manual_reset() {
    ANALYSIS_RESET_REQUESTED.store(true, Ordering::Release);
    previous::request_manual_reset();
}

#[derive(Clone, Debug, Default)]
struct EntityMeta {
    name: String,
    monster_id: i32,
    owner_uuid: i64,
}

#[derive(Clone, Debug, Default)]
struct HealingExtra {
    effective: i64,
    overheal: i64,
}

#[derive(Clone, Debug, Default)]
struct AbsorbedStat {
    source_name: String,
    skill_name: String,
    damage: i64,
    hits: u64,
    crits: u64,
    lucky_hits: u64,
    max_value: i64,
}

#[derive(Clone, Debug, Default)]
struct AnalysisActor {
    /// (skill id, target uuid) -> damage. Keeping the target dimension lets a
    /// later boss/objective promotion reclassify early trash without losing data.
    skill_target_damage: HashMap<(i32, i64), i64>,
    healing: HashMap<i32, HealingExtra>,
    /// Direct EDamageType::Absorbed events only. Mixed shield+HP reconstruction
    /// needs buff-derived shield state and is intentionally not guessed here.
    absorbed: HashMap<(i64, i32), AbsorbedStat>,
}

pub struct TelemetryRuntime {
    inner: previous::TelemetryRuntime,
    inner_rx: Receiver<AppEvent>,
    tx: Sender<AppEvent>,
    entities: HashMap<i64, EntityMeta>,
    actors: HashMap<i64, AnalysisActor>,
    analysis_started: Option<Instant>,
    last_totals: Option<(u64, i64, i64, i64)>,
    boss_targets: HashSet<i64>,
    primary_target: Option<(i64, i64, i64)>,
}

impl TelemetryRuntime {
    pub fn new(tx: Sender<AppEvent>) -> Self {
        let (inner_tx, inner_rx) = mpsc::channel();
        Self {
            inner: previous::TelemetryRuntime::new(inner_tx),
            inner_rx,
            tx,
            entities: HashMap::new(),
            actors: HashMap::new(),
            analysis_started: None,
            last_totals: None,
            boss_targets: HashSet::new(),
            primary_target: None,
        }
    }

    pub fn handle_notify(&mut self, service: u64, method: u32, body: &[u8]) {
        if ANALYSIS_RESET_REQUESTED.swap(false, Ordering::AcqRel) {
            self.reset_encounter_analysis();
        }
        if service == proto::WORLD_SERVICE && method == proto::ENTER_SCENE_METHOD {
            self.reset_scene_analysis();
        }

        self.inner.handle_notify(service, method, body);
        let mut pending = Vec::new();
        while let Ok(event) = self.inner_rx.try_recv() {
            pending.push(event);
        }

        if pending.iter().any(|event| match event {
            AppEvent::Dps(snapshot) => self.snapshot_rolls_encounter(snapshot),
            _ => false,
        }) {
            self.reset_encounter_analysis();
        }

        self.observe_notify(service, method, body);

        for event in pending {
            let event = match event {
                AppEvent::Dps(mut snapshot) => {
                    self.enrich_snapshot(&mut snapshot);
                    AppEvent::Dps(snapshot)
                }
                other => other,
            };
            let _ = self.tx.send(event);
        }
    }

    fn snapshot_rolls_encounter(&self, snapshot: &DpsSnapshot) -> bool {
        let Some(previous) = self.last_totals else { return false; };
        encounter_changed(
            previous,
            (
                snapshot.encounter_ms,
                snapshot.total_damage,
                snapshot.total_healing,
                snapshot.total_damage_taken,
            ),
        )
    }

    fn observe_notify(&mut self, service: u64, method: u32, body: &[u8]) {
        if service != proto::WORLD_SERVICE {
            return;
        }
        match method {
            proto::ENTER_SCENE_METHOD => {
                if let Some(info) = proto::get_len_field(body, 1) {
                    if let Some(player) = proto::get_len_field(info, 2) {
                        let uuid = signed(proto::get_varint_field(player, 1).unwrap_or(0));
                        if uuid != 0 {
                            self.observe_attrs(uuid, proto::get_len_field(player, 3));
                        }
                    }
                }
            }
            SYNC_NEAR_ENTITIES => {
                for entity in proto::len_fields(body, 1) {
                    let uuid = signed(proto::get_varint_field(entity, 1).unwrap_or(0));
                    if uuid != 0 {
                        self.observe_attrs(uuid, proto::get_len_field(entity, 3));
                    }
                }
                for disappeared in proto::len_fields(body, 2) {
                    let uuid = signed(proto::get_varint_field(disappeared, 1).unwrap_or(0));
                    if uuid != 0 {
                        self.entities.remove(&uuid);
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
                        self.observe_delta(delta, None);
                    }
                }
            }
            _ => {}
        }
    }

    fn observe_delta(&mut self, delta: &[u8], fallback_uuid: Option<i64>) {
        let raw_uuid = signed(proto::get_varint_field(delta, 1).unwrap_or(0));
        let uuid = if raw_uuid != 0 { raw_uuid } else { fallback_uuid.unwrap_or(0) };
        if uuid == 0 {
            return;
        }
        if let Some(attrs) = proto::get_len_field(delta, 2) {
            self.observe_attrs(uuid, Some(attrs));
        }
        if let Some(effects) = proto::get_len_field(delta, 7) {
            for damage in proto::len_fields(effects, 2) {
                self.observe_damage(uuid, damage);
            }
        }
    }

    fn observe_attrs(&mut self, uuid: i64, attrs: Option<&[u8]>) {
        let Some(attrs) = attrs else { return; };
        let meta = self.entities.entry(uuid).or_default();
        for attr in proto::len_fields(attrs, 2) {
            let id = proto::get_varint_field(attr, 1).unwrap_or(0);
            let Some(raw) = proto::get_len_field(attr, 2) else { continue; };
            match id {
                ATTR_NAME => {
                    if let Some(name) = decode_name(raw) {
                        meta.name = name;
                    }
                }
                ATTR_MONSTER_ID => {
                    if let Some(value) = raw_varint(raw) {
                        meta.monster_id = value.min(i32::MAX as u64) as i32;
                    }
                }
                ATTR_TOP_SUMMONER_ID => {
                    if let Some(value) = raw_signed(raw) {
                        meta.owner_uuid = value;
                    }
                }
                _ => {}
            }
        }
    }

    fn observe_damage(&mut self, target_uuid: i64, damage: &[u8]) {
        let damage_type = proto::get_varint_field(damage, 4).unwrap_or(0) as i32;
        let miss = proto::get_varint_field(damage, 2).unwrap_or(0) != 0 || damage_type == 1;
        if miss || damage_type == DAMAGE_TYPE_IMMUNE {
            return;
        }

        let top_summoner = signed(proto::get_varint_field(damage, 21).unwrap_or(0));
        let raw_attacker = signed(proto::get_varint_field(damage, 11).unwrap_or(0));
        let summon_owner = self.entities.get(&raw_attacker).map(|meta| meta.owner_uuid).unwrap_or(0);
        let attacker_uuid = if top_summoner != 0 {
            top_summoner
        } else if summon_owner != 0 {
            summon_owner
        } else {
            raw_attacker
        };
        let attacker_uid = if attacker_uuid != 0 && entity_kind(attacker_uuid) == ENTITY_PLAYER {
            attacker_uuid >> 16
        } else {
            0
        };
        let target_kind = entity_kind(target_uuid);
        let skill = proto::get_varint_field(damage, 12).unwrap_or(0) as i32;
        let raw_value = signed(proto::get_varint_field(damage, 6).unwrap_or(0));
        let lucky_value = signed(proto::get_varint_field(damage, 8).unwrap_or(0));
        let hp_lessen = proto::get_varint_field(damage, 9).map(signed);
        let mut value = if lucky_value != 0 { lucky_value } else { raw_value };
        if value < 0 {
            value = hp_lessen.unwrap_or(0).max(0);
        }
        if value <= 0 {
            return;
        }
        let crit = proto::get_varint_field(damage, 3).unwrap_or(0) != 0
            || (proto::get_varint_field(damage, 5).unwrap_or(0) & 1) != 0;
        let lucky = lucky_value != 0;

        if damage_type == DAMAGE_TYPE_ABSORBED {
            if target_kind == ENTITY_PLAYER && self.analysis_started.is_some() {
                let uid = target_uuid >> 16;
                let source_uuid = if raw_attacker != 0 { raw_attacker } else { attacker_uuid };
                let source_name = self.entity_name(source_uuid);
                let skill_name = skill_display_name(skill);
                let stat = self.actors.entry(uid).or_default().absorbed.entry((source_uuid, skill)).or_default();
                if stat.source_name.is_empty() {
                    stat.source_name = source_name;
                }
                if stat.skill_name.is_empty() {
                    stat.skill_name = skill_name;
                }
                stat.damage = stat.damage.saturating_add(value);
                stat.hits = stat.hits.saturating_add(1);
                if crit {
                    stat.crits = stat.crits.saturating_add(1);
                }
                if lucky {
                    stat.lucky_hits = stat.lucky_hits.saturating_add(1);
                }
                stat.max_value = stat.max_value.max(value);
            }
            return;
        }

        if damage_type != DAMAGE_TYPE_HEAL && target_kind == ENTITY_MONSTER && attacker_uid > 0 {
            self.ensure_analysis_started();
            let value_ref = self
                .actors
                .entry(attacker_uid)
                .or_default()
                .skill_target_damage
                .entry((skill, target_uuid))
                .or_default();
            *value_ref = value_ref.saturating_add(value);
        }

        if damage_type == DAMAGE_TYPE_HEAL && attacker_uid > 0 && self.analysis_started.is_some() {
            let (effective, overheal) = split_healing(value, hp_lessen);
            let extra = self.actors.entry(attacker_uid).or_default().healing.entry(skill).or_default();
            extra.effective = extra.effective.saturating_add(effective);
            extra.overheal = extra.overheal.saturating_add(overheal);
        }
    }

    fn enrich_snapshot(&mut self, snapshot: &mut DpsSnapshot) {
        self.update_boss_target(snapshot);
        let mut total_absorbed = 0_i64;

        for row in &mut snapshot.rows {
            let Some(actor) = self.actors.get(&row.uid) else { continue; };

            for skill in &mut row.skills {
                let boss = actor
                    .skill_target_damage
                    .iter()
                    .filter(|((skill_id, target_uuid), _)| {
                        *skill_id == skill.skill_id && self.boss_targets.contains(target_uuid)
                    })
                    .map(|(_, value)| *value)
                    .fold(0_i64, i64::saturating_add)
                    .min(skill.damage.max(0));
                skill.boss_damage = boss;

                if let Some(heal) = actor.healing.get(&skill.skill_id) {
                    skill.effective_healing = heal.effective.min(skill.healing.max(0));
                    skill.overhealing = heal
                        .overheal
                        .min(skill.healing.saturating_sub(skill.effective_healing).max(0));
                }
            }

            row.absorbed_sources = actor
                .absorbed
                .iter()
                .map(|((source_uuid, skill_id), stat)| TakenSourceBreakdown {
                    source_uuid: *source_uuid,
                    source_name: stat.source_name.clone(),
                    skill_id: *skill_id,
                    skill_name: stat.skill_name.clone(),
                    damage: stat.damage,
                    hits: stat.hits,
                    crits: stat.crits,
                    lucky_hits: stat.lucky_hits,
                    max_value: stat.max_value,
                })
                .collect();
            row.absorbed_sources.sort_by(|a, b| {
                b.damage
                    .cmp(&a.damage)
                    .then_with(|| b.max_value.cmp(&a.max_value))
                    .then_with(|| a.source_name.cmp(&b.source_name))
            });
            row.absorbed_damage = row
                .absorbed_sources
                .iter()
                .map(|source| source.damage)
                .fold(0_i64, i64::saturating_add);
            total_absorbed = total_absorbed.saturating_add(row.absorbed_damage);
        }

        snapshot.total_absorbed_damage = total_absorbed;
        let empty = snapshot.encounter_ms == 0
            && snapshot.total_damage == 0
            && snapshot.total_healing == 0
            && snapshot.total_damage_taken == 0;
        self.last_totals = if empty {
            None
        } else {
            Some((
                snapshot.encounter_ms,
                snapshot.total_damage,
                snapshot.total_healing,
                snapshot.total_damage_taken,
            ))
        };
    }

    fn update_boss_target(&mut self, snapshot: &DpsSnapshot) {
        let Some(target) = snapshot.target.as_ref() else { return; };
        if target.entity_uuid == 0 {
            return;
        }
        let candidate = (target.entity_uuid, target.hp, target.max_hp);
        match self.primary_target {
            None => {
                self.boss_targets.clear();
                self.boss_targets.insert(target.entity_uuid);
                self.primary_target = Some(candidate);
            }
            Some((uuid, _, _)) if uuid == target.entity_uuid => {
                self.primary_target = Some(candidate);
            }
            Some((_, previous_hp, previous_max)) => {
                if target.max_hp > previous_max.max(0) {
                    self.boss_targets.clear();
                    self.boss_targets.insert(target.entity_uuid);
                } else {
                    let phase_like = previous_hp <= 0
                        && (previous_max <= 0
                            || target.max_hp.saturating_mul(5) >= previous_max.saturating_mul(2));
                    if phase_like {
                        self.boss_targets.insert(target.entity_uuid);
                    }
                }
                self.primary_target = Some(candidate);
            }
        }
    }

    fn ensure_analysis_started(&mut self) {
        self.analysis_started.get_or_insert_with(Instant::now);
    }

    fn reset_encounter_analysis(&mut self) {
        self.analysis_started = None;
        self.last_totals = None;
        self.boss_targets.clear();
        self.primary_target = None;
        for actor in self.actors.values_mut() {
            actor.skill_target_damage.clear();
            actor.healing.clear();
            actor.absorbed.clear();
        }
    }

    fn reset_scene_analysis(&mut self) {
        self.entities.clear();
        self.actors.clear();
        self.analysis_started = None;
        self.last_totals = None;
        self.boss_targets.clear();
        self.primary_target = None;
    }

    fn entity_name(&self, uuid: i64) -> String {
        if uuid == 0 {
            return "Unknown".into();
        }
        if let Some(meta) = self.entities.get(&uuid) {
            if !meta.name.trim().is_empty() {
                return meta.name.clone();
            }
            if meta.monster_id > 0 {
                return format!("Monster {}", meta.monster_id);
            }
        }
        if entity_kind(uuid) == ENTITY_PLAYER {
            format!("Player {}", uuid >> 16)
        } else {
            format!("Entity {uuid}")
        }
    }
}

fn split_healing(requested: i64, hp_lessen: Option<i64>) -> (i64, i64) {
    let requested = requested.max(0);
    if requested == 0 {
        return (0, 0);
    }
    let Some(actual) = hp_lessen else {
        return (requested, 0);
    };
    let effective = actual.unsigned_abs().min(requested as u64) as i64;
    (effective, requested.saturating_sub(effective))
}

fn skill_display_name(id: i32) -> String {
    // The v1.10 inner layer already has the complete pinned ZDPS name table for
    // outgoing/taken UI. Absorbed events can still be useful when an id is new,
    // so never hide an unknown id.
    if id == 0 {
        "Unknown".into()
    } else {
        format!("Skill {id}")
    }
}

fn encounter_changed(previous: (u64, i64, i64, i64), next: (u64, i64, i64, i64)) -> bool {
    next.0 + 250 < previous.0 || next.1 < previous.1 || next.2 < previous.2 || next.3 < previous.3
}

fn raw_varint(raw: &[u8]) -> Option<u64> {
    let mut position = 0;
    proto::read_varint(raw, &mut position)
}

fn raw_signed(raw: &[u8]) -> Option<i64> {
    raw_varint(raw).map(signed)
}

fn decode_name(raw: &[u8]) -> Option<String> {
    if raw.is_empty() {
        return None;
    }
    let mut position = 0;
    let decoded = if let Some(length) = proto::read_varint(raw, &mut position) {
        let length = usize::try_from(length).ok()?;
        if length > 0 && position.checked_add(length) == Some(raw.len()) {
            &raw[position..]
        } else if raw.len() > 1 {
            &raw[1..]
        } else {
            return None;
        }
    } else if raw.len() > 1 {
        &raw[1..]
    } else {
        return None;
    };
    let text = std::str::from_utf8(decoded)
        .ok()?
        .replace(|c: char| matches!(c, '\r' | '\n' | '\0'), " ")
        .trim()
        .to_string();
    (!text.is_empty()).then(|| text.chars().take(128).collect())
}

fn signed(value: u64) -> i64 {
    value as i64
}

fn entity_kind(uuid: i64) -> i64 {
    (uuid >> 6) & 0x1f
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_healing_split_matches_packet_actual() {
        assert_eq!(split_healing(10_000, Some(6_400)), (6_400, 3_600));
        assert_eq!(split_healing(10_000, Some(0)), (0, 10_000));
        assert_eq!(split_healing(10_000, None), (10_000, 0));
    }

    #[test]
    fn direct_absorbed_type_is_separate_from_hp_damage() {
        assert_eq!(DAMAGE_TYPE_ABSORBED, 5);
        assert_ne!(DAMAGE_TYPE_ABSORBED, DAMAGE_TYPE_HEAL);
    }

    #[test]
    fn encounter_rollover_matches_inner_generation() {
        assert!(encounter_changed((8_000, 900_000, 80_000, 100_000), (400, 20_000, 0, 1_000)));
        assert!(!encounter_changed((8_000, 900_000, 80_000, 100_000), (9_000, 1_000_000, 90_000, 110_000)));
    }

    #[test]
    fn boss_phase_threshold_rejects_tiny_add() {
        let previous_max = 10_000_000_i64;
        let tiny_add = 500_000_i64;
        assert!(tiny_add.saturating_mul(5) < previous_max.saturating_mul(2));
    }
}
