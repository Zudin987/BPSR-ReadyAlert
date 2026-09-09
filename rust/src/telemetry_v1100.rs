use crate::{
    model::{
        AppEvent, BuffUptime, DeathRecap, DeathRecapEvent, DpsSnapshot, TakenSourceBreakdown,
    },
    proto,
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    time::{Duration, Instant},
};

mod previous {
    include!("telemetry_v190.rs");
}

mod names {
    include!(concat!(env!("OUT_DIR"), "/analysis_names_v1100.rs"));
}

const SYNC_NEAR_ENTITIES: u32 = 0x06;
const SYNC_NEAR_DELTA_INFO: u32 = 0x2d;
const SYNC_TO_ME_DELTA_INFO: u32 = 0x2e;

const ATTR_NAME: u64 = 0x01;
const ATTR_MONSTER_ID: u64 = 0x0a;
const ATTR_ACTOR_STATE: u64 = 0x0b;
const ATTR_TOP_SUMMONER_ID: u64 = 0x5b;
const ATTR_CURRENT_HP: u64 = 0x2c2e;
const ATTR_MAX_HP: u64 = 0x2c38;
const ATTR_BLOCK_PCT: u64 = 11_970;

const ACTOR_STATE_DEAD: i64 = 9;
const ENTITY_MONSTER: i64 = 1;
const ENTITY_PLAYER: i64 = 10;
const DEATH_RECAP_WINDOW_MS: u64 = 10_000;
const RECENT_EVENT_RETENTION_MS: u64 = 12_000;
const MAX_RECENT_EVENTS: usize = 64;
const MAX_DEATH_RECAPS: usize = 8;

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
    actor_state: i64,
    hp: i64,
    max_hp: i64,
    block_pct: i64,
}

#[derive(Clone, Debug)]
struct ActiveBuffInstance {
    base_id: i32,
    expires_at: Option<Instant>,
}

#[derive(Clone, Debug, Default)]
struct BuffAggregate {
    active_instances: HashSet<i32>,
    active_since_ms: Option<u64>,
    total_ms: u64,
    activations: u32,
}

#[derive(Clone, Debug, Default)]
struct TakenStat {
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
    target_damage: HashMap<i64, i64>,
    effective_healing: i64,
    overhealing: i64,
    buff_instances: HashMap<i32, ActiveBuffInstance>,
    buffs: HashMap<i32, BuffAggregate>,
    recent_events: VecDeque<DeathRecapEvent>,
    death_recaps: Vec<DeathRecap>,
    taken_sources: HashMap<(i64, i32), TakenStat>,
}

pub struct TelemetryRuntime {
    inner: previous::TelemetryRuntime,
    inner_rx: Receiver<AppEvent>,
    tx: Sender<AppEvent>,
    entities: HashMap<i64, EntityMeta>,
    actors: HashMap<i64, AnalysisActor>,
    local_uuid: i64,
    analysis_started: Option<Instant>,
    last_totals: Option<(u64, i64, i64, i64)>,
    boss_targets: HashSet<i64>,
    primary_target: Option<(i64, i64, i64)>, // uuid, hp, max_hp
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
            local_uuid: 0,
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

        // Let the audited v1.9 pipeline parse first. If this packet starts a new
        // pull after a wipe, its outgoing snapshot tells us to reset analysis
        // before we account the same packet below, so the first hit is not lost.
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
        let next = (
            snapshot.encounter_ms,
            snapshot.total_damage,
            snapshot.total_healing,
            snapshot.total_damage_taken,
        );
        encounter_changed(previous, next)
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
                            self.local_uuid = uuid;
                            self.observe_attrs(uuid, proto::get_len_field(player, 3));
                            if let Some(buffs) = proto::get_len_field(player, 7) {
                                self.observe_buff_snapshot(uuid, buffs);
                            }
                        }
                    }
                }
            }
            SYNC_NEAR_ENTITIES => {
                for entity in proto::len_fields(body, 1) {
                    let uuid = signed(proto::get_varint_field(entity, 1).unwrap_or(0));
                    if uuid == 0 { continue; }
                    self.observe_attrs(uuid, proto::get_len_field(entity, 3));
                    if let Some(buffs) = proto::get_len_field(entity, 7) {
                        self.observe_buff_snapshot(uuid, buffs);
                    }
                }
                for disappeared in proto::len_fields(body, 2) {
                    let uuid = signed(proto::get_varint_field(disappeared, 1).unwrap_or(0));
                    if uuid != 0 { self.entities.remove(&uuid); }
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
                        self.observe_delta(delta, (self.local_uuid != 0).then_some(self.local_uuid));
                    }
                }
            }
            _ => {}
        }
        self.expire_buffs();
    }

    fn observe_delta(&mut self, delta: &[u8], fallback_uuid: Option<i64>) {
        let raw_uuid = signed(proto::get_varint_field(delta, 1).unwrap_or(0));
        let uuid = if raw_uuid != 0 { raw_uuid } else { fallback_uuid.unwrap_or(0) };
        if uuid == 0 { return; }

        if let Some(attrs) = proto::get_len_field(delta, 2) {
            self.observe_attrs(uuid, Some(attrs));
        }
        if let Some(effects) = proto::get_len_field(delta, 7) {
            for damage in proto::len_fields(effects, 2) {
                self.observe_damage(uuid, damage);
            }
        }
        if let Some(buff_effect) = proto::get_len_field(delta, 10) {
            for effect in proto::len_fields(buff_effect, 2) {
                self.observe_buff_effect(uuid, effect);
            }
        }
    }

    fn observe_attrs(&mut self, uuid: i64, attrs: Option<&[u8]>) {
        let Some(attrs) = attrs else { return; };
        let old_state = self.entities.get(&uuid).map(|meta| meta.actor_state).unwrap_or(0);
        let meta = self.entities.entry(uuid).or_default();
        let mut actor_state_seen = false;
        for attr in proto::len_fields(attrs, 2) {
            let id = proto::get_varint_field(attr, 1).unwrap_or(0);
            let Some(raw) = proto::get_len_field(attr, 2) else { continue; };
            match id {
                ATTR_NAME => {
                    if let Some(name) = decode_name(raw) { meta.name = name; }
                }
                ATTR_MONSTER_ID => {
                    if let Some(value) = raw_varint(raw) { meta.monster_id = value.min(i32::MAX as u64) as i32; }
                }
                ATTR_TOP_SUMMONER_ID => {
                    if let Some(value) = raw_signed(raw) { meta.owner_uuid = value; }
                }
                ATTR_ACTOR_STATE => {
                    if let Some(value) = raw_signed(raw) {
                        meta.actor_state = value;
                        actor_state_seen = true;
                    }
                }
                ATTR_CURRENT_HP => {
                    if let Some(value) = raw_signed(raw) { meta.hp = value; }
                }
                ATTR_MAX_HP => {
                    if let Some(value) = raw_signed(raw) { meta.max_hp = value; }
                }
                ATTR_BLOCK_PCT => {
                    if let Some(value) = raw_signed(raw) { meta.block_pct = value; }
                }
                _ => {}
            }
        }
        let new_state = meta.actor_state;
        if actor_state_seen && entity_kind(uuid) == ENTITY_PLAYER && old_state != ACTOR_STATE_DEAD && new_state == ACTOR_STATE_DEAD {
            self.capture_death(uuid >> 16);
        }
    }

    fn observe_damage(&mut self, target_uuid: i64, damage: &[u8]) {
        let damage_type = proto::get_varint_field(damage, 4).unwrap_or(0) as i32;
        let miss = proto::get_varint_field(damage, 2).unwrap_or(0) != 0 || damage_type == 1;
        if miss || matches!(damage_type, 3 | 5) { return; }

        let top_summoner = signed(proto::get_varint_field(damage, 21).unwrap_or(0));
        let raw_attacker = signed(proto::get_varint_field(damage, 11).unwrap_or(0));
        let summon_owner = self.entities.get(&raw_attacker).map(|m| m.owner_uuid).unwrap_or(0);
        let attacker_uuid = if top_summoner != 0 { top_summoner } else if summon_owner != 0 { summon_owner } else { raw_attacker };
        let attacker_uid = if attacker_uuid != 0 && entity_kind(attacker_uuid) == ENTITY_PLAYER { attacker_uuid >> 16 } else { 0 };
        let target_kind = entity_kind(target_uuid);

        let skill = proto::get_varint_field(damage, 12).unwrap_or(0) as i32;
        let raw_value = signed(proto::get_varint_field(damage, 6).unwrap_or(0));
        let lucky_value = signed(proto::get_varint_field(damage, 8).unwrap_or(0));
        let hp_lessen = proto::get_varint_field(damage, 9).map(signed);
        let mut value = if lucky_value != 0 { lucky_value } else { raw_value };
        if value < 0 {
            value = hp_lessen.unwrap_or(0).max(0);
        }
        if value <= 0 { return; }

        let crit = proto::get_varint_field(damage, 3).unwrap_or(0) != 0
            || (proto::get_varint_field(damage, 5).unwrap_or(0) & 1) != 0;
        let lucky = lucky_value != 0;
        let is_heal = damage_type == 2;

        if !is_heal && target_kind == ENTITY_MONSTER && attacker_uid > 0 {
            self.ensure_analysis_started();
            let actor = self.actors.entry(attacker_uid).or_default();
            let entry = actor.target_damage.entry(target_uuid).or_default();
            *entry = entry.saturating_add(value);
        }

        if is_heal && attacker_uid > 0 && self.analysis_started.is_some() {
            let (effective, overheal) = split_healing(value, hp_lessen);
            let actor = self.actors.entry(attacker_uid).or_default();
            actor.effective_healing = actor.effective_healing.saturating_add(effective);
            actor.overhealing = actor.overhealing.saturating_add(overheal);
        }

        if target_kind == ENTITY_PLAYER && self.analysis_started.is_some() {
            let target_uid = target_uuid >> 16;
            let at_ms = self.analysis_elapsed_ms();
            let source_uuid = if raw_attacker != 0 { raw_attacker } else { attacker_uuid };
            let source_name = self.entity_name(source_uuid);
            let skill_name = names::skill_name(skill);
            if is_heal {
                let (effective, _) = split_healing(value, hp_lessen);
                if effective > 0 {
                    self.push_recent_event(target_uid, DeathRecapEvent {
                        at_ms,
                        is_heal: true,
                        source_name,
                        skill_id: skill,
                        skill_name,
                        value: effective,
                        crit,
                        lucky,
                    });
                }
            } else {
                self.push_recent_event(target_uid, DeathRecapEvent {
                    at_ms,
                    is_heal: false,
                    source_name: source_name.clone(),
                    skill_id: skill,
                    skill_name: skill_name.clone(),
                    value,
                    crit,
                    lucky,
                });
                let stat = self.actors.entry(target_uid).or_default().taken_sources.entry((source_uuid, skill)).or_default();
                if stat.source_name.is_empty() { stat.source_name = source_name; }
                if stat.skill_name.is_empty() { stat.skill_name = skill_name; }
                stat.damage = stat.damage.saturating_add(value);
                stat.hits = stat.hits.saturating_add(1);
                if crit { stat.crits = stat.crits.saturating_add(1); }
                if lucky { stat.lucky_hits = stat.lucky_hits.saturating_add(1); }
                stat.max_value = stat.max_value.max(value);
            }
        }
    }

    fn observe_buff_snapshot(&mut self, host: i64, sync: &[u8]) {
        if entity_kind(host) != ENTITY_PLAYER { return; }
        for info in proto::len_fields(sync, 2) {
            let buff_uuid = proto::get_varint_field(info, 1).unwrap_or(0) as i32;
            let base_id = proto::get_varint_field(info, 2).unwrap_or(0) as i32;
            self.apply_named_buff(host >> 16, buff_uuid, base_id, info, false);
        }
    }

    fn observe_buff_effect(&mut self, host: i64, effect: &[u8]) {
        if entity_kind(host) != ENTITY_PLAYER { return; }
        let uid = host >> 16;
        let event_type = proto::get_varint_field(effect, 1).unwrap_or(0) as i32;
        let buff_uuid = proto::get_varint_field(effect, 2).unwrap_or(0) as i32;
        if event_type == 2 {
            self.remove_buff_instance(uid, buff_uuid);
            return;
        }
        if !matches!(event_type, 1 | 3 | 4 | 5) { return; }
        for logic in proto::len_fields(effect, 5) {
            if proto::get_varint_field(logic, 1).unwrap_or(0) != 18 { continue; }
            let Some(info) = proto::get_len_field(logic, 2) else { continue; };
            let base_id = proto::get_varint_field(info, 2).unwrap_or(0) as i32;
            self.apply_named_buff(uid, buff_uuid, base_id, info, event_type == 1);
        }
    }

    fn apply_named_buff(&mut self, uid: i64, buff_uuid: i32, base_id: i32, info: &[u8], explicit_apply: bool) {
        if uid <= 0 || buff_uuid == 0 || base_id == 0 || names::buff_name(base_id).is_none() { return; }
        let duration_ms = proto::get_varint_field(info, 11).unwrap_or(0).min(24 * 60 * 60 * 1000) as u64;
        let expires_at = (duration_ms > 0).then(|| Instant::now() + Duration::from_millis(duration_ms));
        let now = self.analysis_elapsed_ms();
        let actor = self.actors.entry(uid).or_default();

        if let Some(existing) = actor.buff_instances.get_mut(&buff_uuid) {
            if existing.base_id == base_id {
                existing.expires_at = expires_at.or(existing.expires_at);
                return;
            }
        }
        if actor.buff_instances.contains_key(&buff_uuid) {
            self.remove_buff_instance(uid, buff_uuid);
        }

        let actor = self.actors.entry(uid).or_default();
        actor.buff_instances.insert(buff_uuid, ActiveBuffInstance { base_id, expires_at });
        let aggregate = actor.buffs.entry(base_id).or_default();
        let was_inactive = aggregate.active_instances.is_empty();
        aggregate.active_instances.insert(buff_uuid);
        if was_inactive && self.analysis_started.is_some() {
            aggregate.active_since_ms = Some(now);
            aggregate.activations = aggregate.activations.saturating_add(1);
        } else if explicit_apply && self.analysis_started.is_some() && aggregate.activations == 0 {
            aggregate.activations = 1;
        }
    }

    fn remove_buff_instance(&mut self, uid: i64, buff_uuid: i32) {
        let now = self.analysis_elapsed_ms();
        let Some(actor) = self.actors.get_mut(&uid) else { return; };
        let Some(instance) = actor.buff_instances.remove(&buff_uuid) else { return; };
        let Some(aggregate) = actor.buffs.get_mut(&instance.base_id) else { return; };
        aggregate.active_instances.remove(&buff_uuid);
        if aggregate.active_instances.is_empty() {
            if let Some(start) = aggregate.active_since_ms.take() {
                aggregate.total_ms = aggregate.total_ms.saturating_add(now.saturating_sub(start));
            }
        }
    }

    fn expire_buffs(&mut self) {
        let now = Instant::now();
        let mut expired = Vec::new();
        for (uid, actor) in &self.actors {
            for (uuid, instance) in &actor.buff_instances {
                if instance.expires_at.is_some_and(|expiry| expiry <= now) {
                    expired.push((*uid, *uuid));
                }
            }
        }
        for (uid, uuid) in expired { self.remove_buff_instance(uid, uuid); }
    }

    fn push_recent_event(&mut self, uid: i64, event: DeathRecapEvent) {
        let now = event.at_ms;
        let actor = self.actors.entry(uid).or_default();
        actor.recent_events.push_back(event);
        while actor.recent_events.len() > MAX_RECENT_EVENTS
            || actor.recent_events.front().is_some_and(|event| now.saturating_sub(event.at_ms) > RECENT_EVENT_RETENTION_MS)
        {
            actor.recent_events.pop_front();
        }
    }

    fn capture_death(&mut self, uid: i64) {
        if self.analysis_started.is_none() || uid <= 0 { return; }
        let now = self.analysis_elapsed_ms();
        let actor = self.actors.entry(uid).or_default();
        let events: Vec<DeathRecapEvent> = actor.recent_events.iter()
            .filter(|event| now.saturating_sub(event.at_ms) <= DEATH_RECAP_WINDOW_MS)
            .cloned()
            .collect();
        let death_no = actor.death_recaps.last().map(|death| death.death_no.saturating_add(1)).unwrap_or(1);
        actor.death_recaps.push(DeathRecap { death_no, at_ms: now, events });
        if actor.death_recaps.len() > MAX_DEATH_RECAPS {
            actor.death_recaps.remove(0);
        }
    }

    fn enrich_snapshot(&mut self, snapshot: &mut DpsSnapshot) {
        self.expire_buffs();
        self.update_boss_target(snapshot);
        let now = self.analysis_elapsed_ms().max(snapshot.encounter_ms);
        let mut total_boss = 0_i64;

        for row in &mut snapshot.rows {
            let meta = self.entities.get(&row.actor_uuid);
            if let Some(meta) = meta { row.block_pct = meta.block_pct; }
            let Some(actor) = self.actors.get(&row.uid) else { continue; };

            row.boss_damage = actor.target_damage.iter()
                .filter(|(target, _)| self.boss_targets.contains(target))
                .map(|(_, value)| *value)
                .fold(0_i64, i64::saturating_add)
                .min(row.damage.max(0));
            total_boss = total_boss.saturating_add(row.boss_damage);
            row.effective_healing = actor.effective_healing.min(row.healing.max(0));
            row.overhealing = actor.overhealing.min(row.healing.saturating_sub(row.effective_healing).max(0));
            row.buff_uptimes = buff_snapshot(actor, now, snapshot.encounter_ms);
            row.death_recaps = actor.death_recaps.clone();
            row.taken_sources = actor.taken_sources.iter().map(|((source_uuid, skill_id), stat)| TakenSourceBreakdown {
                source_uuid: *source_uuid,
                source_name: stat.source_name.clone(),
                skill_id: *skill_id,
                skill_name: stat.skill_name.clone(),
                damage: stat.damage,
                hits: stat.hits,
                crits: stat.crits,
                lucky_hits: stat.lucky_hits,
                max_value: stat.max_value,
            }).collect();
            row.taken_sources.sort_by(|a, b| b.damage.cmp(&a.damage).then_with(|| b.max_value.cmp(&a.max_value)));
        }
        snapshot.total_boss_damage = total_boss.min(snapshot.total_damage.max(0));

        let empty = snapshot.encounter_ms == 0
            && snapshot.total_damage == 0
            && snapshot.total_healing == 0
            && snapshot.total_damage_taken == 0;
        self.last_totals = if empty { None } else { Some((snapshot.encounter_ms, snapshot.total_damage, snapshot.total_healing, snapshot.total_damage_taken)) };
    }

    fn update_boss_target(&mut self, snapshot: &DpsSnapshot) {
        let Some(target) = snapshot.target.as_ref() else { return; };
        if target.entity_uuid == 0 { return; }
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
                    // Initial trash/add was promoted to a stronger objective: the
                    // old provisional target must not count as boss damage.
                    self.boss_targets.clear();
                    self.boss_targets.insert(target.entity_uuid);
                } else {
                    // Accept lower-HP replacements only when they look like a
                    // real phase/objective rather than a tiny leftover add.
                    let phase_like = previous_hp <= 0
                        && (previous_max <= 0 || target.max_hp.saturating_mul(5) >= previous_max.saturating_mul(2));
                    if phase_like { self.boss_targets.insert(target.entity_uuid); }
                }
                self.primary_target = Some(candidate);
            }
        }
    }

    fn ensure_analysis_started(&mut self) {
        if self.analysis_started.is_some() { return; }
        self.analysis_started = Some(Instant::now());
        for actor in self.actors.values_mut() {
            for aggregate in actor.buffs.values_mut() {
                aggregate.total_ms = 0;
                aggregate.activations = 0;
                aggregate.active_since_ms = None;
                if !aggregate.active_instances.is_empty() {
                    aggregate.active_since_ms = Some(0);
                    aggregate.activations = 1;
                }
            }
        }
    }

    fn reset_encounter_analysis(&mut self) {
        self.analysis_started = None;
        self.last_totals = None;
        self.boss_targets.clear();
        self.primary_target = None;
        for actor in self.actors.values_mut() {
            actor.target_damage.clear();
            actor.effective_healing = 0;
            actor.overhealing = 0;
            actor.recent_events.clear();
            actor.death_recaps.clear();
            actor.taken_sources.clear();
            for aggregate in actor.buffs.values_mut() {
                aggregate.total_ms = 0;
                aggregate.activations = 0;
                aggregate.active_since_ms = None;
            }
        }
    }

    fn reset_scene_analysis(&mut self) {
        self.entities.clear();
        self.actors.clear();
        self.local_uuid = 0;
        self.analysis_started = None;
        self.last_totals = None;
        self.boss_targets.clear();
        self.primary_target = None;
    }

    fn analysis_elapsed_ms(&self) -> u64 {
        self.analysis_started.map(|start| start.elapsed().as_millis().min(u64::MAX as u128) as u64).unwrap_or(0)
    }

    fn entity_name(&self, uuid: i64) -> String {
        if uuid == 0 { return "Unknown".into(); }
        if let Some(meta) = self.entities.get(&uuid) {
            if !meta.name.trim().is_empty() { return meta.name.clone(); }
            if meta.monster_id > 0 { return format!("Monster {}", meta.monster_id); }
        }
        if entity_kind(uuid) == ENTITY_PLAYER { format!("Player {}", uuid >> 16) } else { format!("Entity {uuid}") }
    }
}

fn split_healing(requested: i64, hp_lessen: Option<i64>) -> (i64, i64) {
    let requested = requested.max(0);
    if requested == 0 { return (0, 0); }
    let Some(actual) = hp_lessen else {
        // Missing HpLessen is not evidence of overheal; keep the event effective
        // rather than fabricating waste from absent protocol data.
        return (requested, 0);
    };
    let effective = actual.unsigned_abs().min(requested as u64) as i64;
    (effective, requested.saturating_sub(effective))
}

fn buff_snapshot(actor: &AnalysisActor, now_ms: u64, encounter_ms: u64) -> Vec<BuffUptime> {
    let cap = encounter_ms.max(now_ms);
    let mut rows: Vec<BuffUptime> = actor.buffs.iter().filter_map(|(base_id, aggregate)| {
        let name = names::buff_name(*base_id)?;
        let ongoing = aggregate.active_since_ms.map(|start| now_ms.saturating_sub(start)).unwrap_or(0);
        let uptime = aggregate.total_ms.saturating_add(ongoing).min(cap);
        (uptime > 0 || aggregate.activations > 0).then(|| BuffUptime {
            buff_id: *base_id,
            name: name.into(),
            uptime_ms: uptime,
            activations: aggregate.activations,
        })
    }).collect();
    rows.sort_by(|a, b| b.uptime_ms.cmp(&a.uptime_ms).then_with(|| b.activations.cmp(&a.activations)).then_with(|| a.name.cmp(&b.name)));
    rows
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
    if raw.is_empty() { return None; }
    let mut position = 0;
    let decoded = if let Some(length) = proto::read_varint(raw, &mut position) {
        let length = usize::try_from(length).ok()?;
        if length > 0 && position.checked_add(length) == Some(raw.len()) {
            &raw[position..]
        } else if raw.len() > 1 {
            &raw[1..]
        } else { return None; }
    } else if raw.len() > 1 {
        &raw[1..]
    } else { return None; };
    let text = std::str::from_utf8(decoded).ok()?
        .replace(|c: char| matches!(c, '\r' | '\n' | '\0'), " ")
        .trim().to_string();
    (!text.is_empty()).then(|| text.chars().take(128).collect())
}

fn signed(value: u64) -> i64 { value as i64 }
fn entity_kind(uuid: i64) -> i64 { (uuid >> 6) & 0x1f }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn healing_split_uses_actual_hp_modification() {
        assert_eq!(split_healing(10_000, Some(7_500)), (7_500, 2_500));
        assert_eq!(split_healing(10_000, Some(0)), (0, 10_000));
        assert_eq!(split_healing(10_000, Some(-7_500)), (7_500, 2_500));
        assert_eq!(split_healing(10_000, None), (10_000, 0));
    }

    #[test]
    fn encounter_regression_resets_analysis_generation() {
        assert!(encounter_changed((12_000, 5_000, 2_000, 1_000), (500, 50, 0, 0)));
        assert!(!encounter_changed((12_000, 5_000, 2_000, 1_000), (13_000, 6_000, 2_500, 1_200)));
    }

    #[test]
    fn death_window_keeps_only_recent_events() {
        let events = [1_000_u64, 5_000, 11_000];
        let now = 12_000_u64;
        let kept: Vec<_> = events.into_iter().filter(|at| now.saturating_sub(*at) <= DEATH_RECAP_WINDOW_MS).collect();
        assert_eq!(kept, vec![5_000, 11_000]);
    }

    #[test]
    fn boss_promotion_can_drop_initial_trash_damage() {
        let mut targets = HashSet::from([10_i64]);
        let previous_max = 500_000_i64;
        let candidate_max = 10_000_000_i64;
        if candidate_max > previous_max {
            targets.clear();
            targets.insert(20);
        }
        assert_eq!(targets, HashSet::from([20_i64]));
    }
}
