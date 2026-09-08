use crate::{
    feature_settings::{
        attr_label, is_trackable_attr, tracked_attr_ids, ATTR_CURRENT_HP, ATTR_FIGHT_POINT,
        ATTR_MAX_HP, ATTR_SEASON_STRENGTH,
    },
    model::{
        AppEvent, ConsumableStatus, DpsRow, DpsSnapshot, ImagineBadge, MechanicRow,
        MechanicSnapshot, SkillBreakdown, TargetSnapshot, TrackedAttribute,
    },
    proto,
};
use std::{
    collections::{HashMap, HashSet},
    sync::mpsc::Sender,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const SYNC_NEAR_ENTITIES: u32 = 0x06;
const SYNC_CONTAINER_DATA: u32 = 0x15;
const SYNC_NEAR_DELTA_INFO: u32 = 0x2d;
const SYNC_TO_ME_DELTA_INFO: u32 = 0x2e;

const TEAM_INFO: u32 = 0x01;
const TEAM_MEMBER_INFO: u32 = 0x02;
const TEAM_JOIN: u32 = 0x03;
const TEAM_LEAVE: u32 = 0x04;
const TEAM_DISSOLVE: u32 = 0x0d;

const ATTR_NAME: u64 = 0x01;
const ATTR_MONSTER_ID: u64 = 0x0a;
const ATTR_ACTOR_STATE: u64 = 0x0b;
const ATTR_TOP_SUMMONER_ID: u64 = 0x5b;
const ATTR_SKILL_ID: u64 = 0x64;
const ATTR_SKILL_REMODEL_LEVEL: u64 = 0x79;
const ATTR_PROFESSION_ID: u64 = 0xdc;
const ACTOR_STATE_DEAD: i64 = 9;

const ENTITY_PLAYER: i64 = 10;
const ENTITY_MONSTER: i64 = 1;
const FANTASY_MARKER_BUFF_ID: i32 = 2_199_999;
const SEGMENT_BOUNDARY_DELAY: Duration = Duration::from_secs(3);
const MAX_SEGMENT: Duration = Duration::from_secs(20 * 60);

#[derive(Clone, Debug, Default)]
struct EntityMeta {
    name: String,
    monster_id: i32,
    hp: i64,
    max_hp: i64,
    actor_state: i64,
    owner_uuid: i64,
    remodel_level: i32,
}

#[derive(Clone, Debug, Default)]
struct PlayerMeta {
    actor_uuid: i64,
    name: String,
    profession_id: i32,
    subprofession_id: i32,
    ability_score: i64,
    illusion_break: i64,
    hp: i64,
    max_hp: i64,
    actor_state: i64,
    attrs: HashMap<i32, i64>,
}

#[derive(Clone, Debug, Default)]
struct SkillStat {
    damage: i64,
    healing: i64,
    hits: u64,
    crits: u64,
    luckies: u64,
    max_value: i64,
}

#[derive(Clone, Debug, Default)]
struct CombatActor {
    damage: i64,
    healing: i64,
    damage_taken: i64,
    hits: u64,
    crits: u64,
    lucky_hits: u64,
    deaths: u32,
    is_dead: bool,
    skills: HashMap<i32, SkillStat>,
    imagines: HashMap<i32, i32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ConsumableKind {
    Food,
    Serum,
}

pub struct TelemetryRuntime {
    tx: Sender<AppEvent>,
    entities: HashMap<i64, EntityMeta>,
    players: HashMap<i64, PlayerMeta>,
    team: HashSet<i64>,
    combat: HashMap<i64, CombatActor>,
    local_uid: i64,
    encounter_started: Option<Instant>,
    pending_boundary: Option<Instant>,
    last_dps_emit: Instant,
    last_target: i64,
    mechanics: HashMap<String, MechanicRow>,
    buff_instances: HashMap<(i64, i32), String>,
    consumable_instances: HashMap<(i64, i32), ConsumableKind>,
    food: Option<ConsumableStatus>,
    serum: Option<ConsumableStatus>,
    entity_mechanic_keys: HashMap<i64, Vec<String>>,
    last_mechanic_emit: Instant,
}

impl TelemetryRuntime {
    pub fn new(tx: Sender<AppEvent>) -> Self {
        let now = Instant::now();
        Self {
            tx,
            entities: HashMap::new(),
            players: HashMap::new(),
            team: HashSet::new(),
            combat: HashMap::new(),
            local_uid: 0,
            encounter_started: None,
            pending_boundary: None,
            last_dps_emit: now.checked_sub(Duration::from_secs(1)).unwrap_or(now),
            last_target: 0,
            mechanics: HashMap::new(),
            buff_instances: HashMap::new(),
            consumable_instances: HashMap::new(),
            food: None,
            serum: None,
            entity_mechanic_keys: HashMap::new(),
            last_mechanic_emit: now.checked_sub(Duration::from_secs(1)).unwrap_or(now),
        }
    }

    pub fn handle_notify(&mut self, service: u64, method: u32, body: &[u8]) {
        if service == proto::TEAM_SERVICE {
            self.handle_team(method, body);
            return;
        }
        if service != proto::WORLD_SERVICE {
            return;
        }

        match method {
            proto::ENTER_SCENE_METHOD => {
                self.reset_scene();
                self.handle_enter_scene(body);
            }
            SYNC_NEAR_ENTITIES => self.handle_near_entities(body),
            SYNC_CONTAINER_DATA => self.handle_container_snapshot(body),
            SYNC_NEAR_DELTA_INFO => {
                for delta in proto::len_fields(body, 1) {
                    self.handle_delta(delta, None);
                }
            }
            SYNC_TO_ME_DELTA_INFO => {
                if let Some(wrapper) = proto::get_len_field(body, 1) {
                    if let Some(delta) = proto::get_len_field(wrapper, 1) {
                        let local = (self.local_uid > 0).then_some(canonical_player_uuid(self.local_uid));
                        self.handle_delta(delta, local);
                    }
                }
            }
            _ => {}
        }
        self.prune_expired_mechanics();
    }

    /// User-requested training-dummy reset. Keeps roster and current scene context.
    pub fn manual_reset(&mut self) {
        self.reset_encounter_keep_roster();
        self.pending_boundary = None;
        self.emit_dps();
    }

    fn reset_scene(&mut self) {
        self.entities.clear();
        self.combat.clear();
        self.encounter_started = None;
        self.pending_boundary = None;
        self.last_target = 0;
        self.mechanics.clear();
        self.buff_instances.clear();
        self.consumable_instances.clear();
        self.food = None;
        self.serum = None;
        self.entity_mechanic_keys.clear();
        for uid in self.team.clone() {
            self.combat.entry(uid).or_default();
        }
        let _ = self.tx.send(AppEvent::Dps(DpsSnapshot::default()));
        self.emit_mechanics(true);
    }

    fn handle_enter_scene(&mut self, body: &[u8]) {
        if let Some(info) = proto::get_len_field(body, 1) {
            if let Some(player) = proto::get_len_field(info, 2) {
                let uuid = signed(proto::get_varint_field(player, 1).unwrap_or(0));
                if uuid != 0 {
                    self.local_uid = uuid >> 16;
                    self.team.insert(self.local_uid);
                    self.combat.entry(self.local_uid).or_default();
                    self.update_entity_from_attrs(uuid, proto::get_len_field(player, 3));
                    if let Some(buffs) = proto::get_len_field(player, 7) {
                        self.handle_buff_snapshot(uuid, buffs);
                    }
                }
            }
        }
        self.emit_dps();
        self.emit_mechanics(true);
    }

    /// CN protocol: SyncContainerData.v_data = field 1 (CharSerialize),
    /// CharSerialize.char_base = field 2, CharBaseInfo.fight_point = field 35.
    fn handle_container_snapshot(&mut self, body: &[u8]) {
        let Some(data) = proto::get_len_field(body, 1) else { return; };
        if let Some(char_id) = proto::get_varint_field(data, 1).map(signed).filter(|v| *v > 0) {
            self.local_uid = char_id;
            self.team.insert(char_id);
            self.combat.entry(char_id).or_default();
            let player = self.players.entry(char_id).or_default();
            if player.actor_uuid == 0 {
                player.actor_uuid = canonical_player_uuid(char_id);
            }
        }
        let Some(base) = proto::get_len_field(data, 2) else { return; };
        let Some(fight_point) = proto::get_varint_field(base, 35).map(signed) else { return; };
        if self.local_uid > 0 {
            let player = self.players.entry(self.local_uid).or_default();
            player.ability_score = fight_point;
            player.attrs.insert(ATTR_FIGHT_POINT, fight_point);
            self.emit_dps();
            self.emit_mechanics(false);
        }
    }

    fn handle_team(&mut self, method: u32, body: &[u8]) {
        if method == TEAM_DISSOLVE {
            self.team.clear();
            if self.local_uid > 0 {
                self.team.insert(self.local_uid);
            }
            self.retain_team_rows();
            self.emit_dps();
            return;
        }
        if method == TEAM_LEAVE {
            let before = self.team.clone();
            self.scan_team_members(body);
            if self.team == before {
                self.team.retain(|uid| *uid == self.local_uid);
            }
            self.retain_team_rows();
            self.emit_dps();
            return;
        }
        if matches!(method, TEAM_INFO | TEAM_MEMBER_INFO | TEAM_JOIN) {
            self.scan_team_members(body);
            for uid in self.team.clone() {
                self.combat.entry(uid).or_default();
            }
            self.emit_dps();
        }
    }

    fn retain_team_rows(&mut self) {
        if self.encounter_started.is_none() {
            self.combat.retain(|uid, _| self.team.contains(uid));
        }
    }

    fn scan_team_members(&mut self, data: &[u8]) {
        let mut found = Vec::new();
        scan_member_messages(data, 0, &mut found);
        for member in found {
            if member.uid <= 0 {
                continue;
            }
            self.team.insert(member.uid);
            let player = self.players.entry(member.uid).or_default();
            if !member.name.is_empty() {
                player.name = member.name;
            }
            if member.profession_id > 0 {
                player.profession_id = member.profession_id;
            }
            if member.hp > 0 {
                player.hp = member.hp;
            }
            if member.max_hp > 0 {
                player.max_hp = member.max_hp;
            }
            self.combat.entry(member.uid).or_default();
        }
    }

    fn handle_near_entities(&mut self, body: &[u8]) {
        let mut changed = false;
        for entity in proto::len_fields(body, 1) {
            let uuid = signed(proto::get_varint_field(entity, 1).unwrap_or(0));
            if uuid == 0 {
                continue;
            }
            self.update_entity_from_attrs(uuid, proto::get_len_field(entity, 3));
            let mut marker_source = None;
            if let Some(buffs) = proto::get_len_field(entity, 7) {
                marker_source = self.handle_buff_snapshot(uuid, buffs);
            }
            self.register_fantasy_summon(uuid, marker_source);

            let monster_id = self.entities.get(&uuid).map(|meta| meta.monster_id).unwrap_or(0);
            if let Some((label, duration, priority)) = monster_rule(monster_id) {
                let key = format!("entity:{uuid}:{monster_id}");
                self.mechanics.insert(
                    key.clone(),
                    mechanic_row(key.clone(), label, None, duration, duration == 0, priority),
                );
                self.entity_mechanic_keys.entry(uuid).or_default().push(key);
                changed = true;
            }
        }
        for disappeared in proto::len_fields(body, 2) {
            let uuid = signed(proto::get_varint_field(disappeared, 1).unwrap_or(0));
            if uuid == self.last_target && self.encounter_started.is_some() {
                self.arm_boundary();
            }
            self.entities.remove(&uuid);
            if let Some(keys) = self.entity_mechanic_keys.remove(&uuid) {
                for key in keys {
                    changed |= self.mechanics.remove(&key).is_some();
                }
            }
        }
        if changed {
            self.emit_mechanics(true);
        }
        self.emit_dps();
    }

    fn handle_delta(&mut self, delta: &[u8], fallback_uuid: Option<i64>) {
        let raw_uuid = signed(proto::get_varint_field(delta, 1).unwrap_or(0));
        let uuid = if raw_uuid != 0 { raw_uuid } else { fallback_uuid.unwrap_or(0) };
        if uuid == 0 {
            return;
        }

        if let Some(attrs) = proto::get_len_field(delta, 2) {
            self.update_entity_from_attrs(uuid, Some(attrs));
            if let Some(skill) = attr_varint(attrs, ATTR_SKILL_ID).map(|value| value as i32) {
                if let Some((label, duration, priority)) = skill_rule(skill) {
                    let key = format!("skill:{uuid}:{skill}:{}", now_ms() / 250);
                    self.mechanics.insert(
                        key.clone(),
                        mechanic_row(key, label, self.display_target(uuid), duration, false, priority),
                    );
                    self.emit_mechanics(false);
                }
            }
            self.register_fantasy_summon(uuid, None);
        }
        if let Some(effects) = proto::get_len_field(delta, 7) {
            for damage in proto::len_fields(effects, 2) {
                self.apply_damage(uuid, damage);
            }
        }
        if let Some(buff_effect) = proto::get_len_field(delta, 10) {
            for effect in proto::len_fields(buff_effect, 2) {
                self.apply_buff(uuid, effect);
            }
        }
    }

    fn update_entity_from_attrs(&mut self, uuid: i64, attrs: Option<&[u8]>) {
        let Some(attrs) = attrs else { return; };
        if entity_kind(uuid) == ENTITY_PLAYER {
            let uid = uuid >> 16;
            let old_state = self.players.get(&uid).map(|p| p.actor_state).unwrap_or_default();
            let mut actor_state_seen = false;
            {
                let player = self.players.entry(uid).or_default();
                player.actor_uuid = uuid;
                for attr in proto::len_fields(attrs, 2) {
                    let id = proto::get_varint_field(attr, 1).unwrap_or(0);
                    let Some(raw) = proto::get_len_field(attr, 2) else { continue; };
                    let value = raw_signed(raw);
                    match id {
                        ATTR_NAME => {
                            if let Some(name) = decode_name(raw) {
                                player.name = name;
                            }
                        }
                        ATTR_ACTOR_STATE => {
                            if let Some(value) = value {
                                player.actor_state = value;
                                actor_state_seen = true;
                            }
                        }
                        ATTR_PROFESSION_ID => {
                            if let Some(value) = value {
                                player.profession_id = value as i32;
                            }
                        }
                        x if x == ATTR_FIGHT_POINT as u64 => {
                            if let Some(value) = value {
                                player.ability_score = value;
                                player.attrs.insert(ATTR_FIGHT_POINT, value);
                            }
                        }
                        x if x == ATTR_SEASON_STRENGTH as u64 => {
                            if let Some(value) = value {
                                player.illusion_break = value;
                                player.attrs.insert(ATTR_SEASON_STRENGTH, value);
                            }
                        }
                        x if x == ATTR_CURRENT_HP as u64 => {
                            if let Some(value) = value {
                                player.hp = value;
                                player.attrs.insert(ATTR_CURRENT_HP, value);
                            }
                        }
                        x if x == ATTR_MAX_HP as u64 => {
                            if let Some(value) = value {
                                player.max_hp = value;
                                player.attrs.insert(ATTR_MAX_HP, value);
                            }
                        }
                        _ if is_trackable_attr(id as i32) => {
                            if let Some(value) = value {
                                player.attrs.insert(id as i32, value);
                            }
                        }
                        _ => {}
                    }
                }
            }
            if actor_state_seen {
                let new_state = self.players.get(&uid).map(|p| p.actor_state).unwrap_or_default();
                self.apply_player_actor_state(uid, old_state, new_state);
            }
            if self.team.contains(&uid) || uid == self.local_uid {
                self.combat.entry(uid).or_default();
            }
        } else {
            let old_state = self.entities.get(&uuid).map(|m| m.actor_state).unwrap_or_default();
            let mut actor_state_seen = false;
            {
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
                        ATTR_ACTOR_STATE => {
                            if let Some(value) = raw_signed(raw) {
                                meta.actor_state = value;
                                actor_state_seen = true;
                            }
                        }
                        ATTR_TOP_SUMMONER_ID => {
                            if let Some(value) = raw_signed(raw) {
                                meta.owner_uuid = value;
                            }
                        }
                        ATTR_SKILL_REMODEL_LEVEL => {
                            if let Some(value) = raw_signed(raw) {
                                meta.remodel_level = value.clamp(0, i32::MAX as i64) as i32;
                            }
                        }
                        x if x == ATTR_CURRENT_HP as u64 => {
                            if let Some(value) = raw_signed(raw) {
                                meta.hp = value;
                            }
                        }
                        x if x == ATTR_MAX_HP as u64 => {
                            if let Some(value) = raw_signed(raw) {
                                meta.max_hp = value;
                            }
                        }
                        _ => {}
                    }
                }
            }
            if actor_state_seen {
                let new_state = self.entities.get(&uuid).map(|m| m.actor_state).unwrap_or_default();
                if old_state != ACTOR_STATE_DEAD && new_state == ACTOR_STATE_DEAD && uuid == self.last_target {
                    self.arm_boundary();
                }
            }
        }
    }

    fn apply_player_actor_state(&mut self, uid: i64, old_state: i64, new_state: i64) {
        let combat = self.combat.entry(uid).or_default();
        if old_state != ACTOR_STATE_DEAD && new_state == ACTOR_STATE_DEAD {
            combat.deaths = combat.deaths.saturating_add(1);
            combat.is_dead = true;
        } else if old_state == ACTOR_STATE_DEAD && new_state != ACTOR_STATE_DEAD {
            combat.is_dead = false;
        } else {
            combat.is_dead = new_state == ACTOR_STATE_DEAD;
        }
        if new_state == ACTOR_STATE_DEAD && self.party_is_wiped() {
            self.arm_boundary();
        }
    }

    fn party_is_wiped(&self) -> bool {
        let mut roster: Vec<i64> = self.team.iter().copied().collect();
        if self.local_uid > 0 && !roster.contains(&self.local_uid) {
            roster.push(self.local_uid);
        }
        !roster.is_empty()
            && roster
                .iter()
                .all(|uid| self.combat.get(uid).is_some_and(|actor| actor.is_dead))
    }

    fn apply_damage(&mut self, target_uuid: i64, damage: &[u8]) {
        let damage_type = proto::get_varint_field(damage, 4).unwrap_or(0) as i32;
        let miss = proto::get_varint_field(damage, 2).unwrap_or(0) != 0 || damage_type == 1;
        if miss || matches!(damage_type, 3 | 5) {
            return;
        }

        let top_summoner = signed(proto::get_varint_field(damage, 21).unwrap_or(0));
        let raw_attacker = signed(proto::get_varint_field(damage, 11).unwrap_or(0));
        let summon_owner = self.entities.get(&raw_attacker).map(|m| m.owner_uuid).unwrap_or(0);
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

        let skill = proto::get_varint_field(damage, 12).unwrap_or(0) as i32;
        let owner_level = proto::get_varint_field(damage, 13).unwrap_or(0).min(i32::MAX as u64) as i32;
        let raw_value = signed(proto::get_varint_field(damage, 6).unwrap_or(0));
        let lucky_value = signed(proto::get_varint_field(damage, 8).unwrap_or(0));
        let hp_lessen = signed(proto::get_varint_field(damage, 9).unwrap_or(0));
        let mut value = if lucky_value != 0 { lucky_value } else { raw_value };
        if value < 0 {
            value = hp_lessen.max(0);
        }
        if value <= 0 {
            return;
        }

        let crit = proto::get_varint_field(damage, 3).unwrap_or(0) != 0
            || (proto::get_varint_field(damage, 5).unwrap_or(0) & 1) != 0;
        let lucky = lucky_value != 0;
        let is_heal = damage_type == 2;
        let target_kind = entity_kind(target_uuid);
        let now = Instant::now();

        // CN-style segment lifecycle: start on eligible outgoing player damage,
        // split only after an observed boundary (scene/target death/disappear/wipe)
        // and a short guard delay. Ordinary idle time never wipes the meter.
        if !is_heal && target_kind == ENTITY_MONSTER && attacker_uid > 0 {
            self.prepare_segment(now);
            self.last_target = target_uuid;
        }

        if is_heal {
            if attacker_uid > 0 && self.encounter_started.is_some() {
                let actor = self.combat.entry(attacker_uid).or_default();
                actor.healing = actor.healing.saturating_add(value);
                self.apply_skill(attacker_uid, skill, value, true, lucky, crit);
                self.record_imagine_from_damage(attacker_uid, raw_attacker, skill, owner_level);
            }
        } else {
            if target_kind == ENTITY_MONSTER && attacker_uid > 0 && self.encounter_started.is_some() {
                let actor = self.combat.entry(attacker_uid).or_default();
                actor.damage = actor.damage.saturating_add(value);
                actor.hits = actor.hits.saturating_add(1);
                if crit {
                    actor.crits = actor.crits.saturating_add(1);
                }
                if lucky {
                    actor.lucky_hits = actor.lucky_hits.saturating_add(1);
                }
                self.apply_skill(attacker_uid, skill, value, false, lucky, crit);
                self.record_imagine_from_damage(attacker_uid, raw_attacker, skill, owner_level);
            }
            if target_kind == ENTITY_PLAYER && self.encounter_started.is_some() {
                let uid = target_uuid >> 16;
                self.combat.entry(uid).or_default().damage_taken = self
                    .combat
                    .get(&uid)
                    .map(|actor| actor.damage_taken)
                    .unwrap_or(0)
                    .saturating_add(value);
            }
        }

        if self.last_dps_emit.elapsed() >= Duration::from_millis(100) {
            self.emit_dps();
        }
    }

    fn prepare_segment(&mut self, now: Instant) {
        if self.encounter_started.is_some_and(|started| started.elapsed() >= MAX_SEGMENT) {
            self.reset_encounter_keep_roster();
        }
        if let Some(boundary) = self.pending_boundary.take() {
            if now.saturating_duration_since(boundary) >= SEGMENT_BOUNDARY_DELAY {
                self.reset_encounter_keep_roster();
            }
        }
        self.encounter_started.get_or_insert(now);
    }

    fn arm_boundary(&mut self) {
        if self.encounter_started.is_some() && self.pending_boundary.is_none() {
            self.pending_boundary = Some(Instant::now());
        }
    }

    fn apply_skill(&mut self, uid: i64, skill: i32, value: i64, heal: bool, lucky: bool, crit: bool) {
        let stat = self.combat.entry(uid).or_default().skills.entry(skill).or_default();
        if heal {
            stat.healing = stat.healing.saturating_add(value);
        } else {
            stat.damage = stat.damage.saturating_add(value);
        }
        stat.hits = stat.hits.saturating_add(1);
        if crit {
            stat.crits = stat.crits.saturating_add(1);
        }
        if lucky {
            stat.luckies = stat.luckies.saturating_add(1);
        }
        stat.max_value = stat.max_value.max(value);
    }

    fn record_imagine_from_damage(&mut self, uid: i64, raw_attacker: i64, skill: i32, owner_level: i32) {
        if let Some(meta) = self.entities.get(&raw_attacker) {
            if let Some(fantasy_skill) = fantasy_skill_for_monster(meta.monster_id) {
                let tier = meta.remodel_level.max(owner_level);
                self.record_imagine(uid, fantasy_skill, tier);
                return;
            }
        }
        // Compatibility fallback for packets that identify the Fantasy parent skill directly.
        if is_fantasy_skill(skill) {
            self.record_imagine(uid, skill, owner_level);
        }
    }

    fn register_fantasy_summon(&mut self, summon_uuid: i64, marker_source: Option<i32>) {
        let Some(meta) = self.entities.get(&summon_uuid).cloned() else { return; };
        if !(3_000_000..=3_009_999).contains(&meta.monster_id) || meta.owner_uuid == 0 {
            return;
        }
        let owner_uid = if entity_kind(meta.owner_uuid) == ENTITY_PLAYER {
            meta.owner_uuid >> 16
        } else {
            return;
        };
        let skill = marker_source
            .and_then(normalize_fantasy_source)
            .or_else(|| fantasy_skill_for_monster(meta.monster_id));
        if let Some(skill) = skill {
            self.record_imagine(owner_uid, skill, meta.remodel_level);
        }
    }

    fn record_imagine(&mut self, uid: i64, skill: i32, tier: i32) {
        self.combat
            .entry(uid)
            .or_default()
            .imagines
            .entry(skill)
            .and_modify(|old| *old = (*old).max(tier))
            .or_insert(tier);
    }

    fn reset_encounter_keep_roster(&mut self) {
        self.combat.clear();
        for uid in self.team.clone() {
            self.combat.entry(uid).or_default();
        }
        if self.local_uid > 0 {
            self.combat.entry(self.local_uid).or_default();
        }
        self.encounter_started = None;
        self.pending_boundary = None;
        self.last_target = 0;
    }

    fn emit_dps(&mut self) {
        self.last_dps_emit = Instant::now();
        let encounter_ms = self
            .encounter_started
            .map(|start| start.elapsed().as_millis() as u64)
            .unwrap_or(0);
        let seconds = (encounter_ms.max(1) as f64 / 1000.0).max(0.001);
        let total_damage: i64 = self.combat.values().map(|actor| actor.damage).sum();
        let total_healing: i64 = self.combat.values().map(|actor| actor.healing).sum();
        let total_taken: i64 = self.combat.values().map(|actor| actor.damage_taken).sum();

        let mut uids: HashSet<i64> = self.combat.keys().copied().collect();
        uids.extend(self.team.iter().copied());
        if self.local_uid > 0 {
            uids.insert(self.local_uid);
        }

        let mut rows = Vec::with_capacity(uids.len());
        for uid in uids {
            let stat = self.combat.get(&uid).cloned().unwrap_or_default();
            let meta = self.players.get(&uid).cloned().unwrap_or_default();
            let mut skills: Vec<SkillBreakdown> = stat
                .skills
                .iter()
                .map(|(id, value)| SkillBreakdown {
                    skill_id: *id,
                    name: skill_name(*id),
                    damage: value.damage,
                    healing: value.healing,
                    hits: value.hits,
                    crits: value.crits,
                    lucky_hits: value.luckies,
                    max_value: value.max_value,
                })
                .collect();
            skills.sort_by(|a, b| (b.damage + b.healing).cmp(&(a.damage + a.healing)));

            let mut imagines: Vec<ImagineBadge> = stat
                .imagines
                .iter()
                .map(|(id, tier)| {
                    let (name, icon_key) = imagine_info(*id);
                    ImagineBadge { skill_id: *id, name, tier: *tier, icon_key }
                })
                .collect();
            imagines.sort_by_key(|badge| badge.skill_id);

            rows.push(DpsRow {
                actor_uuid: meta.actor_uuid,
                uid,
                name: if meta.name.trim().is_empty() { format!("Player {uid}") } else { meta.name },
                profession_id: meta.profession_id,
                subprofession_id: meta.subprofession_id,
                // Adapter infers the active specialization from observed profession skills.
                subprofession_name: String::new(),
                ability_score: meta.ability_score,
                illusion_break: meta.illusion_break,
                damage: stat.damage,
                healing: stat.healing,
                damage_taken: stat.damage_taken,
                dps: stat.damage as f64 / seconds,
                damage_share: pct(stat.damage, total_damage),
                healing_share: pct(stat.healing, total_healing),
                tank_share: pct(stat.damage_taken, total_taken),
                hits: stat.hits,
                crits: stat.crits,
                lucky_hits: stat.lucky_hits,
                deaths: stat.deaths,
                is_dead: stat.is_dead,
                is_local: uid == self.local_uid && self.local_uid > 0,
                imagines,
                skills,
            });
        }
        rows.sort_by(|a, b| b.damage.cmp(&a.damage).then_with(|| b.healing.cmp(&a.healing)));
        let target = self.target_snapshot();
        let _ = self.tx.send(AppEvent::Dps(DpsSnapshot {
            encounter_ms,
            total_damage,
            total_healing,
            total_damage_taken: total_taken,
            target,
            rows,
        }));
    }

    fn target_snapshot(&self) -> Option<TargetSnapshot> {
        if self.last_target == 0 {
            return None;
        }
        let meta = self.entities.get(&self.last_target)?;
        Some(TargetSnapshot {
            entity_uuid: self.last_target,
            name: if meta.name.trim().is_empty() {
                if meta.monster_id > 0 { format!("Target {}", meta.monster_id) } else { "Target".into() }
            } else {
                meta.name.clone()
            },
            hp: meta.hp,
            max_hp: meta.max_hp,
            enrage_remaining_ms: None,
        })
    }

    /// Returns the Fantasy marker source_config_id when the snapshot contains it.
    fn handle_buff_snapshot(&mut self, host: i64, sync: &[u8]) -> Option<i32> {
        let mut marker_source = None;
        for info in proto::len_fields(sync, 2) {
            let buff_uuid = proto::get_varint_field(info, 1).unwrap_or(0) as i32;
            let base_id = proto::get_varint_field(info, 2).unwrap_or(0) as i32;
            if base_id == FANTASY_MARKER_BUFF_ID {
                marker_source = proto::get_len_field(info, 12)
                    .and_then(|source| proto::get_varint_field(source, 2))
                    .map(|value| value.min(i32::MAX as u64) as i32);
            }
            self.observe_consumable(host, buff_uuid, base_id, info);
        }
        marker_source
    }

    fn apply_buff(&mut self, host: i64, effect: &[u8]) {
        let event_type = proto::get_varint_field(effect, 1).unwrap_or(0) as i32;
        let buff_uuid = proto::get_varint_field(effect, 2).unwrap_or(0) as i32;
        if event_type == 2 {
            if let Some(key) = self.buff_instances.remove(&(host, buff_uuid)) {
                if self.mechanics.remove(&key).is_some() {
                    self.emit_mechanics(false);
                }
            }
            if let Some(kind) = self.consumable_instances.remove(&(host, buff_uuid)) {
                match kind {
                    ConsumableKind::Food => self.food = None,
                    ConsumableKind::Serum => self.serum = None,
                }
                self.emit_mechanics(true);
            }
            return;
        }
        if !matches!(event_type, 1 | 3 | 4 | 5) {
            return;
        }

        for logic in proto::len_fields(effect, 5) {
            if proto::get_varint_field(logic, 1).unwrap_or(0) != 18 {
                continue;
            }
            let Some(info) = proto::get_len_field(logic, 2) else { continue; };
            let base_id = proto::get_varint_field(info, 2).unwrap_or(0) as i32;
            self.observe_consumable(host, buff_uuid, base_id, info);
            let Some((label, default_duration, priority)) = buff_rule(base_id) else { continue; };
            let duration = proto::get_varint_field(info, 11).unwrap_or(0).min(300_000) as u64;
            let duration = if duration > 0 { duration } else { default_duration };
            let key = format!("buff:{host}:{buff_uuid}:{base_id}");
            self.mechanics.insert(
                key.clone(),
                mechanic_row(key.clone(), label, self.display_target(host), duration, duration == 0, priority),
            );
            self.buff_instances.insert((host, buff_uuid), key);
            self.emit_mechanics(false);
        }
    }

    fn observe_consumable(&mut self, host: i64, buff_uuid: i32, base_id: i32, info: &[u8]) {
        if entity_kind(host) != ENTITY_PLAYER || host >> 16 != self.local_uid {
            return;
        }
        let Some((kind, name)) = consumable_info(base_id) else { return; };
        let status = ConsumableStatus {
            buff_id: base_id,
            name: name.into(),
            expires_unix_ms: observed_buff_expiry(info),
        };
        self.consumable_instances.insert((host, buff_uuid), kind);
        match kind {
            ConsumableKind::Food => self.food = Some(status),
            ConsumableKind::Serum => self.serum = Some(status),
        }
        self.emit_mechanics(false);
    }

    fn display_target(&self, uuid: i64) -> Option<String> {
        if entity_kind(uuid) != ENTITY_PLAYER {
            return None;
        }
        let uid = uuid >> 16;
        self.players
            .get(&uid)
            .map(|player| player.name.trim())
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .or_else(|| Some(format!("Player {uid}")))
    }

    fn prune_expired_mechanics(&mut self) {
        let now = now_ms();
        let before = self.mechanics.len();
        self.mechanics
            .retain(|_, row| row.persistent || row.expires_unix_ms <= 0 || row.expires_unix_ms > now);
        let old_food = self.food.is_some();
        let old_serum = self.serum.is_some();
        if self.food.as_ref().is_some_and(|x| x.expires_unix_ms > 0 && x.expires_unix_ms <= now) {
            self.food = None;
        }
        if self.serum.as_ref().is_some_and(|x| x.expires_unix_ms > 0 && x.expires_unix_ms <= now) {
            self.serum = None;
        }
        if before != self.mechanics.len() || old_food != self.food.is_some() || old_serum != self.serum.is_some() {
            self.emit_mechanics(true);
        }
    }

    fn emit_mechanics(&mut self, force: bool) {
        if !force && self.last_mechanic_emit.elapsed() < Duration::from_millis(50) {
            return;
        }
        self.last_mechanic_emit = Instant::now();
        let now = now_ms();
        let mut rows: Vec<MechanicRow> = self
            .mechanics
            .values()
            .filter(|row| row.persistent || row.expires_unix_ms <= 0 || row.expires_unix_ms > now)
            .cloned()
            .collect();
        rows.sort_by(|a, b| b.priority.cmp(&a.priority).then_with(|| a.expires_unix_ms.cmp(&b.expires_unix_ms)));
        rows.truncate(20);

        let tracked_attributes = self
            .players
            .get(&self.local_uid)
            .map(|player| {
                tracked_attr_ids()
                    .map(|id| TrackedAttribute {
                        attr_id: id,
                        label: attr_label(id).into(),
                        value: *player.attrs.get(&id).unwrap_or(&0),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let _ = self.tx.send(AppEvent::Mechanics(MechanicSnapshot {
            tracked_attributes,
            food: self.food.clone(),
            serum: self.serum.clone(),
            rows,
        }));
    }
}

#[derive(Default)]
struct TeamCandidate {
    uid: i64,
    name: String,
    profession_id: i32,
    hp: i64,
    max_hp: i64,
}

fn scan_member_messages(data: &[u8], depth: usize, out: &mut Vec<TeamCandidate>) {
    if depth > 5 || data.is_empty() || data.len() > 1024 * 1024 {
        return;
    }
    let uid = signed(proto::get_varint_field(data, 1).unwrap_or(0));
    let name = proto_string(data, 3).unwrap_or_default();
    let profession_id = proto::get_varint_field(data, 4).unwrap_or(0) as i32;
    let hp = signed(proto::get_varint_field(data, 8).unwrap_or(0));
    let max_hp = signed(proto::get_varint_field(data, 9).unwrap_or(0));
    if uid > 0 && !name.trim().is_empty() && (profession_id > 0 || max_hp > 0) {
        out.push(TeamCandidate { uid, name, profession_id, hp, max_hp });
    }
    for field in 1..=24 {
        for child in proto::len_fields(data, field) {
            if child.len() < data.len() {
                scan_member_messages(child, depth + 1, out);
            }
        }
    }
}

fn proto_string(data: &[u8], field: u32) -> Option<String> {
    let bytes = proto::get_len_field(data, field)?;
    if bytes.is_empty() || bytes.len() > 256 {
        return None;
    }
    let text = std::str::from_utf8(bytes)
        .ok()?
        .replace(|c: char| matches!(c, '\0' | '\r' | '\n'), " ")
        .trim()
        .to_string();
    (!text.is_empty()).then_some(text)
}

fn attr_varint(attrs: &[u8], wanted: u64) -> Option<u64> {
    for attr in proto::len_fields(attrs, 2) {
        if proto::get_varint_field(attr, 1) == Some(wanted) {
            return raw_varint(proto::get_len_field(attr, 2)?);
        }
    }
    None
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

fn canonical_player_uuid(uid: i64) -> i64 {
    (uid << 16) | (ENTITY_PLAYER << 6)
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or(0)
}

fn pct(value: i64, total: i64) -> f64 {
    if total > 0 { value.max(0) as f64 * 100.0 / total as f64 } else { 0.0 }
}

fn mechanic_row(
    key: String,
    label: &str,
    target: Option<String>,
    duration: u64,
    persistent: bool,
    priority: u8,
) -> MechanicRow {
    let now = now_ms();
    MechanicRow {
        key,
        label: label.into(),
        target,
        created_unix_ms: now,
        expires_unix_ms: if duration > 0 { now.saturating_add(duration as i64) } else { 0 },
        persistent,
        priority,
    }
}

fn observed_buff_expiry(info: &[u8]) -> i64 {
    let duration = proto::get_varint_field(info, 11).unwrap_or(0).min(24 * 60 * 60 * 1000) as i64;
    if duration <= 0 {
        return 0;
    }
    let create = proto::get_varint_field(info, 6).map(signed).unwrap_or(0);
    if create >= 1_000_000_000_000 {
        return create.saturating_add(duration);
    }
    if create >= 1_000_000_000 {
        return create.saturating_mul(1000).saturating_add(duration);
    }
    now_ms().saturating_add(duration)
}

fn normalize_fantasy_source(source: i32) -> Option<i32> {
    if (3_800..=4_100).contains(&source) {
        return Some(source);
    }
    let compact = source / 100;
    (3_800..=4_100).contains(&compact).then_some(compact)
}

fn is_fantasy_skill(id: i32) -> bool {
    (3_898..=3_999).contains(&id)
}

/// CN FantasyMonsterSkillMap.json fallback, used when the marker source id is absent.
fn fantasy_skill_for_monster(monster_id: i32) -> Option<i32> {
    Some(match monster_id {
        3_000_000 => 3901,
        3_000_008 => 3908,
        3_000_009 => 3903,
        3_000_010 | 3_000_011 | 3_000_021 | 3_000_040 => 3946,
        3_000_013 => 3917,
        3_000_019 => 3906,
        3_000_020 => 3902,
        3_000_022 => 3923,
        3_000_023 => 3938,
        3_000_024 => 3943,
        3_000_025 => 3942,
        3_000_026 => 3939,
        3_000_027 => 3905,
        3_000_028 => 3904,
        3_000_029 => 3937,
        3_000_030 => 3928,
        3_000_031 => 3934,
        3_000_032 => 3922,
        3_000_033 => 3921,
        3_000_034 => 3920,
        3_000_035 => 3910,
        3_000_036 => 3945,
        3_000_037 => 3947,
        3_000_038 => 3944,
        3_000_039 => 3907,
        3_000_041 => 3911,
        3_000_042 => 3936,
        3_000_043 => 3948,
        3_000_044 => 3949,
        3_000_045 => 3950,
        3_000_046 => 3951,
        3_000_047 => 3952,
        3_000_048 => 3953,
        3_000_049 => 3954,
        3_000_051 => 3955,
        3_000_052 => 3956,
        3_000_053 => 3957,
        3_000_054 => 3958,
        3_000_055 => 3959,
        3_000_056 => 3960,
        3_000_057 => 3962,
        3_000_058 => 3961,
        3_000_059 => 3963,
        3_000_060 => 3964,
        3_000_061 => 3965,
        3_000_062 => 3966,
        3_000_063 => 3968,
        3_000_064 => 3969,
        3_000_065..=3_000_068 => 3970,
        3_000_069 => 3971,
        3_000_070 => 3972,
        3_000_071 => 3973,
        3_000_072 => 3974,
        3_000_073 => 3975,
        3_000_074 => 3976,
        3_000_075 => 3977,
        3_000_076 => 3978,
        3_000_077 => 3979,
        3_000_078 => 3980,
        3_000_079 => 3981,
        3_000_080 => 3982,
        3_000_081 => 3983,
        3_000_082 => 3988,
        3_000_083 | 3_000_084 => 3989,
        3_000_085 => 3990,
        3_000_086 => 3991,
        3_000_087 => 3992,
        3_000_088 => 3993,
        3_000_089 => 3994,
        3_000_090 => 3995,
        3_000_091 => 3996,
        3_000_092 => 3997,
        3_000_093 => 3998,
        3_000_094 => 3999,
        3_000_095 => 3898,
        3_000_096 => 3899,
        _ => return None,
    })
}

fn imagine_info(id: i32) -> (String, String) {
    let name = match id {
        3903 => "Brigand Leader",
        3920 => "Airona",
        3921 => "Tina",
        3944 => "Celestial Flier",
        _ => return (format!("Battle Imagine {id}"), "BI".into()),
    };
    let icon = name
        .split_whitespace()
        .filter_map(|part| part.chars().next())
        .take(2)
        .collect::<String>()
        .to_ascii_uppercase();
    (name.into(), if icon.is_empty() { "BI".into() } else { icon })
}

/// Frequently observed player skill names sourced from BPSR-ZDPS's MIT-licensed
/// SkillOverrides.en.json. Unknown ids remain visible rather than being hidden.
fn skill_name(id: i32) -> String {
    let name = match id {
        0 => "Unknown",
        1201 => "Raincall Surge - Stage 1",
        1202 => "Raincall Surge - Stage 2",
        1203 => "Raincall Surge - Stage 3",
        1204 => "Raincall Surge - Stage 4",
        1210 | 1250 => "Maelstrom",
        1211 => "Crystal Veil",
        1216 => "Ultra Cold: Crystal Veil",
        1219 => "Ice Tornado",
        1222 | 1223 => "Phantom Dash",
        1238 => "Tidal Vortex - Passive Skill",
        1239 | 1260 => "Meteor Storm",
        1240 => "Frozen Gale",
        1241 => "Frostbeam",
        1242 => "Frost Lance",
        1243 => "Permafrost",
        1244 => "Blizzard",
        1245 => "Frost Shelter",
        1246 | 1256 => "Tidepool",
        1247 => "Frost Comet (Talent)",
        1248 => "Glacier Hymn",
        1249 => "Cooperative Crystal",
        1251 => "Water Vortex",
        1257 => "Blizzard",
        1258 => "Icy Bolt",
        1259 => "Frost Comet",
        1261 => "Blizzard (Glacier Fury)",
        1262 => "Meteor Storm (Expansion)",
        1263 => "Glacier Hymn (Comet)",
        1401 => "Windborne Grace - Sweep",
        1402 => "Windborne Grace - Rush/Smash",
        1403 => "Windborne Grace - Spinning Kick",
        1404 => "Windborne Grace - Down Smash",
        1405 | 1418 => "Gale Thrust",
        1406 => "Windborne Grace",
        1407 => "Typhoon",
        1409 => "Typhoon Cleave (Battle Cry Leap)",
        1410 | 1426 => "Typhoon Cleave",
        1411 => "Swift Blade",
        1419 => "Skyfall",
        1420 => "Galeform",
        1421 => "Spiral Thrust",
        1422 => "Breach Pursuit",
        1423 => "Aegis Gale",
        1424 => "Instant Edge",
        1425 => "Falcon Toss (Leap)",
        1427 => "Breach Pursuit (Sweeping Slash)",
        1428 => "Spear Thrust",
        1429 => "Tornado",
        1431 => "Sharp Impact (Crash Down)",
        1433 => "Azure Sever (Leap)",
        1434 => "Vortex Strike",
        1435 => "Drake Cannon",
        1501 => "Vines' Embrace - Stage 1",
        1502 => "Vines' Embrace - Stage 2",
        1503 => "Vines' Embrace - Stage 3",
        1504 => "Vines' Embrace - Stage 4",
        1517 => "Vines' Embrace - Lethal Skill Counter",
        1551 => "Regen Bud: Wild Seed",
        1556 => "Bloomheal",
        1560 => "Regen Pulse",
        1561 => "Regen Pulse (Pulse Retrospect)",
        1562 => "Healing Ring",
        1601 => "Blazing Swing - Stage 1",
        1602 => "Blazing Swing - Stage 2",
        1603 => "Blazing Swing - Stage 3",
        1604 => "Blazing Swing - Stage 4",
        1605 => "Blazing Ascension",
        1606 => "Frenzied Slash",
        1607 => "Blazing Assault",
        1608 => "Rage Cleave - Stage 1",
        1609 => "Rage Cleave - Stage 2",
        1610 => "Rage Cleave - Stage 3",
        1611 => "Rage Cleave - Stage 4",
        1612 => "Axe Wind",
        1613 => "Wildfire Dance",
        1614 => "Unbound Meteor",
        1615 => "Wasteland's End",
        1714 => "Iaido Signature",
        1715 => "Moonstrike Signature",
        1734 => "Iaido Signature",
        1738 => "Moonstrike Signature",
        1518 => "Smite Signature",
        1541 => "Smite Signature",
        1930 | 1931 | 1934 | 1935 => "Block Signature",
        1941 => "Earthfort Signature",
        2292 => "Wildpack Signature",
        2301 | 2336 | 2361 => "Concerto Signature",
        2321 | 2335 => "Dissonance Signature",
        2405 | 2411 => "Recovery Signature",
        2406 => "Shield Signature",
        3_898..=3_999 => return imagine_info(id).0,
        _ => return format!("Skill {id}"),
    };
    name.into()
}

fn consumable_info(id: i32) -> Option<(ConsumableKind, &'static str)> {
    use ConsumableKind::{Food, Serum};
    Some(match id {
        2_010_003 => (Food, "Sea Breeze Feast"),
        2_010_022 => (Food, "Adventure Food"),
        2_010_071..=2_010_079 | 2_010_081..=2_010_086 => (Food, "Foodie's Grace"),
        2_032_011..=2_032_018 => (Food, "Physical ATK Cuisine"),
        2_032_021..=2_032_028 => (Food, "Magic ATK Cuisine"),
        2_032_031..=2_032_038 => (Food, "Armor Cuisine"),
        2_032_041..=2_032_048 => (Food, "Endurance Cuisine"),
        2_032_051..=2_032_058 => (Food, "HP Recovery Cuisine"),
        2_032_061..=2_032_286 | 2_032_311..=2_032_383 => (Food, "Cuisine"),
        2_033_011..=2_033_019 => (Serum, "Fire Strength Serum"),
        2_033_021..=2_033_029 => (Serum, "Ice Strength Serum"),
        2_033_031..=2_033_039 => (Serum, "Forest Strength Serum"),
        2_033_041..=2_033_049 => (Serum, "Rock Strength Serum"),
        2_033_051..=2_033_059 => (Serum, "Wind Strength Serum"),
        2_033_061..=2_033_069 => (Serum, "Thunder Strength Serum"),
        2_033_071..=2_033_079 => (Serum, "Light Strength Serum"),
        2_033_081..=2_033_089 => (Serum, "Dark Strength Serum"),
        2_033_091..=2_033_099 => (Serum, "Fire Resistance Serum"),
        2_033_101..=2_033_109 => (Serum, "Ice Resistance Serum"),
        2_033_111..=2_033_119 => (Serum, "Forest Resistance Serum"),
        2_033_121..=2_033_129 => (Serum, "Rock Resistance Serum"),
        2_033_131..=2_033_139 => (Serum, "Wind Resistance Serum"),
        2_033_141..=2_033_149 => (Serum, "Thunder Resistance Serum"),
        2_033_151..=2_033_159 => (Serum, "Light Resistance Serum"),
        2_033_161..=2_033_169 => (Serum, "Dark Resistance Serum"),
        2_033_174..=2_033_179 => (Serum, "Physical Enhancement Serum"),
        2_033_184..=2_033_189 => (Serum, "Magic Enhancement Serum"),
        2_033_211..=2_033_383 => (Serum, "S4 Alchemy Serum"),
        _ => return None,
    })
}

fn monster_rule(id: i32) -> Option<(&'static str, u64, u8)> {
    Some(match id {
        2106 => ("Correct portal", 0, 3),
        300086 => ("Pizza danger - SLOW", 0, 3),
        300089 => ("Pizza danger - FAST", 0, 3),
        33904 | 33905 => ("Tower activating", 40_000, 2),
        3340219 => ("Ice wave", 0, 3),
        3340220 => ("Water wave", 0, 3),
        10330051 => ("Pinball", 6_000, 2),
        _ => return None,
    })
}

fn skill_rule(id: i32) -> Option<(&'static str, u64, u8)> {
    Some(match id {
        111103 => ("Gravity Blast", 8_500, 3),
        3390117 | 3390123 => ("Clone charge - LEFT", 10_000, 3),
        3390118 | 3390124 => ("Clone charge - RIGHT", 10_000, 3),
        3340245 => ("Pizza danger", 7_000, 3),
        10310062 => ("Electromagnetic ring - INNER", 6_000, 3),
        10310063 => ("Electromagnetic ring - MID", 6_000, 3),
        10310064 => ("Electromagnetic ring - OUTER", 6_000, 3),
        470125 | 470132 => ("Pair mechanic - SETTLE", 5_000, 3),
        470119 => ("Shadow phase", 8_000, 3),
        470112 => ("Chain - NEAR circle", 4_000, 3),
        470113 => ("Chain - FAR circle", 4_000, 3),
        _ => return None,
    })
}

fn buff_rule(id: i32) -> Option<(&'static str, u64, u8)> {
    Some(match id {
        821076 => ("Sticky Bomb", 8_000, 3),
        884101 | 884106 | 884122 => ("Tower activating", 40_000, 2),
        884102 => ("Tower complete - BLUE", 5_000, 1),
        884103 => ("Tower complete - GOLD", 5_000, 1),
        884129 => ("Energy Pillar", 10_000, 3),
        884141 => ("Energy Pillar - SHORT", 8_000, 3),
        884162 => ("Charge target - LEFT", 10_000, 3),
        884163 => ("Charge target - RIGHT", 10_000, 3),
        884168 => ("Charge target - RANDOM", 10_000, 3),
        884169 => ("Puzzle mark #1", 12_000, 3),
        884170 => ("Puzzle mark #2", 12_000, 3),
        510571 => ("Heavy Wound", 10_000, 3),
        841519 => ("Red Bind", 10_000, 3),
        841509 => ("Wudi Slash", 10_000, 3),
        829214 => ("Phase - EDGE", 10_000, 2),
        829215 => ("Phase - CORNER", 10_000, 2),
        829327 => ("Phase mirage - TOP LEFT", 10_000, 2),
        829328 => ("Phase mirage - MID LEFT", 10_000, 2),
        829329 => ("Phase mirage - BOTTOM LEFT", 10_000, 2),
        829330 => ("Phase mirage - TOP RIGHT", 10_000, 2),
        829331 => ("Phase mirage - MID RIGHT", 10_000, 2),
        829332 => ("Phase mirage - BOTTOM RIGHT", 10_000, 2),
        829104 => ("Electromagnetic Pulse A", 10_000, 3),
        829105 => ("Electromagnetic Pulse B", 10_000, 3),
        829106 => ("Electromagnetic Pulse C", 10_000, 3),
        829115 | 829116 | 829304 | 829305 => ("Share", 10_000, 3),
        829306 | 829307 => ("Decay", 10_000, 3),
        829308 | 829309 => ("Spread", 10_000, 3),
        829316 => ("Causal Jump", 10_000, 3),
        829217 => ("Normal target", 10_000, 3),
        829245 => ("Decay target", 10_000, 3),
        829226 => ("Hit order #1", 10_000, 3),
        829227 => ("Hit order #2", 10_000, 3),
        829228 => ("Hit order #3", 10_000, 3),
        829323 | 829324 | 829326 => ("Kill mark", 10_000, 3),
        829314 => ("Pinball cast", 6_000, 3),
        883602 => ("Dual mark - ICE", 10_000, 3),
        883603 => ("Dual mark - WATER", 10_000, 3),
        883633 => ("Pizza - ORANGE", 8_000, 2),
        883634 => ("Pizza - PURPLE", 8_000, 2),
        884659 => ("Pair mark", 25_000, 3),
        884664 => ("Pair swap", 25_000, 2),
        884660 => ("Pair settle", 6_000, 3),
        884661 => ("Pair penalty", 6_000, 3),
        884614 => ("Wheel - BLUE / GROUP", 10_000, 3),
        884615 => ("Wheel - RED / SOLO", 10_000, 3),
        884616 => ("Wheel - DOOM", 10_000, 3),
        884641 => ("Energy Ball target", 10_000, 3),
        884609 => ("Chain - NEAR", 8_000, 3),
        884610 => ("Chain - FAR", 8_000, 3),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_layout() {
        let player = canonical_player_uuid(123);
        let monster = (456_i64 << 16) | (ENTITY_MONSTER << 6);
        assert_eq!(entity_kind(player), ENTITY_PLAYER);
        assert_eq!(entity_kind(monster), ENTITY_MONSTER);
    }

    #[test]
    fn actor_state_dead_is_authoritative() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let mut runtime = TelemetryRuntime::new(tx);
        runtime.team.insert(123);
        runtime.apply_player_actor_state(123, 0, ACTOR_STATE_DEAD);
        assert!(runtime.combat[&123].is_dead);
        assert_eq!(runtime.combat[&123].deaths, 1);
        runtime.apply_player_actor_state(123, ACTOR_STATE_DEAD, 0);
        assert!(!runtime.combat[&123].is_dead);
    }

    #[test]
    fn fantasy_registry_maps_3944() {
        assert_eq!(fantasy_skill_for_monster(3_000_038), Some(3944));
        assert_eq!(imagine_info(3944).0, "Celestial Flier");
    }

    #[test]
    fn current_attribute_ids_are_used() {
        assert_eq!(ATTR_FIGHT_POINT, 0x272e);
        assert_eq!(ATTR_SEASON_STRENGTH, 0x2cb0);
    }

    #[test]
    fn shares() {
        assert!((pct(25, 100) - 25.0).abs() < 0.001);
    }
}
