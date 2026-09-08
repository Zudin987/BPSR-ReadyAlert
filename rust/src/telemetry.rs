use crate::{
    model::{AppEvent, DpsRow, DpsSnapshot, MechanicRow, MechanicSnapshot},
    proto,
};
use std::{
    collections::HashMap,
    sync::mpsc::Sender,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const SYNC_NEAR_ENTITIES: u32 = 0x06;
const SYNC_NEAR_DELTA_INFO: u32 = 0x2d;
const SYNC_TO_ME_DELTA_INFO: u32 = 0x2e;

const ATTR_NAME: u64 = 0x01;
const ATTR_MONSTER_ID: u64 = 0x0a;
const ATTR_SKILL_ID: u64 = 0x64;

const ENTITY_PLAYER: i64 = 10;
const ENTITY_MONSTER: i64 = 1;

#[derive(Clone, Debug, Default)]
struct EntityMeta {
    name: String,
    monster_id: i32,
}

#[derive(Clone, Debug, Default)]
struct CombatActor {
    damage: i64,
    hits: u64,
    crits: u64,
    lucky_hits: u64,
}

pub struct TelemetryRuntime {
    tx: Sender<AppEvent>,
    entities: HashMap<i64, EntityMeta>,
    combat: HashMap<i64, CombatActor>,
    encounter_started: Option<Instant>,
    last_damage: Option<Instant>,
    last_dps_emit: Instant,
    mechanics: HashMap<String, MechanicRow>,
    buff_instances: HashMap<(i64, i32), String>,
    entity_mechanic_keys: HashMap<i64, Vec<String>>,
    last_mechanic_emit: Instant,
}

impl TelemetryRuntime {
    pub fn new(tx: Sender<AppEvent>) -> Self {
        let now = Instant::now();
        Self {
            tx,
            entities: HashMap::new(),
            combat: HashMap::new(),
            encounter_started: None,
            last_damage: None,
            last_dps_emit: now.checked_sub(Duration::from_secs(1)).unwrap_or(now),
            mechanics: HashMap::new(),
            buff_instances: HashMap::new(),
            entity_mechanic_keys: HashMap::new(),
            last_mechanic_emit: now.checked_sub(Duration::from_secs(1)).unwrap_or(now),
        }
    }

    pub fn handle_notify(&mut self, service: u64, method: u32, body: &[u8]) {
        if service != proto::WORLD_SERVICE { return; }
        match method {
            proto::ENTER_SCENE_METHOD => self.reset_scene(),
            SYNC_NEAR_ENTITIES => self.handle_near_entities(body),
            SYNC_NEAR_DELTA_INFO => {
                for delta in proto::len_fields(body, 1) {
                    self.handle_delta(delta, None);
                }
            }
            SYNC_TO_ME_DELTA_INFO => {
                if let Some(wrapper) = proto::get_len_field(body, 1) {
                    let outer_uuid = signed(proto::get_varint_field(wrapper, 5).unwrap_or(0));
                    if let Some(delta) = proto::get_len_field(wrapper, 1) {
                        self.handle_delta(delta, (outer_uuid != 0).then_some(outer_uuid));
                    }
                }
            }
            _ => {}
        }
        self.prune_expired_mechanics();
    }

    fn reset_scene(&mut self) {
        self.entities.clear();
        self.combat.clear();
        self.encounter_started = None;
        self.last_damage = None;
        self.mechanics.clear();
        self.buff_instances.clear();
        self.entity_mechanic_keys.clear();
        let _ = self.tx.send(AppEvent::Dps(DpsSnapshot::default()));
        let _ = self.tx.send(AppEvent::Mechanics(MechanicSnapshot::default()));
    }

    fn handle_near_entities(&mut self, body: &[u8]) {
        let mut mechanics_changed = false;
        for entity in proto::len_fields(body, 1) {
            let uuid = signed(proto::get_varint_field(entity, 1).unwrap_or(0));
            if uuid == 0 { continue; }
            let attrs = proto::get_len_field(entity, 3);
            self.update_entity_from_attrs(uuid, attrs);
            let monster_id = self.entities.get(&uuid).map(|m| m.monster_id).unwrap_or(0);
            if let Some((label, duration_ms, priority)) = monster_rule(monster_id) {
                let key = format!("entity:{uuid}:{monster_id}");
                let row = mechanic_row(key.clone(), label, None, duration_ms, duration_ms == 0, priority);
                self.mechanics.insert(key.clone(), row);
                self.entity_mechanic_keys.entry(uuid).or_default().push(key);
                mechanics_changed = true;
            }
        }
        for gone in proto::len_fields(body, 2) {
            let uuid = signed(proto::get_varint_field(gone, 1).unwrap_or(0));
            self.entities.remove(&uuid);
            if let Some(keys) = self.entity_mechanic_keys.remove(&uuid) {
                for key in keys { mechanics_changed |= self.mechanics.remove(&key).is_some(); }
            }
        }
        if mechanics_changed { self.emit_mechanics(true); }
    }

    fn handle_delta(&mut self, delta: &[u8], outer_uuid: Option<i64>) {
        let raw_uuid = signed(proto::get_varint_field(delta, 1).unwrap_or(0));
        let uuid = if raw_uuid != 0 { raw_uuid } else { outer_uuid.unwrap_or(0) };
        if uuid == 0 { return; }

        if let Some(attrs) = proto::get_len_field(delta, 2) {
            self.update_entity_from_attrs(uuid, Some(attrs));
            if let Some(skill_id) = attr_varint(attrs, ATTR_SKILL_ID).map(|v| v as i32) {
                if let Some((label, duration_ms, priority)) = skill_rule(skill_id) {
                    let key = format!("skill:{uuid}:{skill_id}:{}", now_unix_ms() / 250);
                    self.mechanics.insert(key.clone(), mechanic_row(key, label, self.display_target(uuid), duration_ms, false, priority));
                    self.emit_mechanics(false);
                }
            }
        }

        if let Some(effects) = proto::get_len_field(delta, 7) {
            for dmg in proto::len_fields(effects, 2) {
                self.apply_damage(uuid, dmg);
            }
        }

        if let Some(buff_sync) = proto::get_len_field(delta, 10) {
            for effect in proto::len_fields(buff_sync, 2) {
                self.apply_buff(uuid, effect);
            }
        }
    }

    fn update_entity_from_attrs(&mut self, uuid: i64, attrs: Option<&[u8]>) {
        let Some(attrs) = attrs else { return; };
        let meta = self.entities.entry(uuid).or_default();
        for attr in proto::len_fields(attrs, 2) {
            let id = proto::get_varint_field(attr, 1).unwrap_or(0);
            let Some(raw) = proto::get_len_field(attr, 2) else { continue; };
            match id {
                ATTR_NAME => if let Some(name) = decode_name(raw) { meta.name = name; },
                ATTR_MONSTER_ID => if let Some(v) = raw_varint(raw) { meta.monster_id = v.min(i32::MAX as u64) as i32; },
                _ => {}
            }
        }
    }

    fn apply_damage(&mut self, target_uuid: i64, dmg: &[u8]) {
        let damage_type = proto::get_varint_field(dmg, 4).unwrap_or(0) as i32;
        let is_miss = proto::get_varint_field(dmg, 2).unwrap_or(0) != 0 || damage_type == 1;
        if is_miss || matches!(damage_type, 2 | 3 | 5) { return; }
        if entity_kind(target_uuid) != ENTITY_MONSTER { return; }

        let owner_id = proto::get_varint_field(dmg, 12).unwrap_or(0) as i32;
        if owner_id == 0 { return; }
        let top_summoner = signed(proto::get_varint_field(dmg, 21).unwrap_or(0));
        let raw_attacker = signed(proto::get_varint_field(dmg, 11).unwrap_or(0));
        let attacker = if top_summoner != 0 { top_summoner } else { raw_attacker };
        if attacker == 0 || entity_kind(attacker) != ENTITY_PLAYER { return; }

        let raw_value = signed(proto::get_varint_field(dmg, 6).unwrap_or(0));
        let lucky_value = signed(proto::get_varint_field(dmg, 8).unwrap_or(0));
        let hp_lessen = signed(proto::get_varint_field(dmg, 9).unwrap_or(0));
        let mut value = if lucky_value != 0 { lucky_value } else { raw_value };
        if value < 0 { value = hp_lessen.max(0); }
        if value <= 0 { return; }

        let now = Instant::now();
        if self.last_damage.is_some_and(|last| last.elapsed() >= Duration::from_secs(8)) {
            self.combat.clear();
            self.encounter_started = None;
        }
        self.encounter_started.get_or_insert(now);
        self.last_damage = Some(now);

        let actor = self.combat.entry(attacker).or_default();
        actor.damage = actor.damage.saturating_add(value);
        actor.hits = actor.hits.saturating_add(1);
        if proto::get_varint_field(dmg, 5).unwrap_or(0) & 1 != 0 { actor.crits = actor.crits.saturating_add(1); }
        if lucky_value != 0 { actor.lucky_hits = actor.lucky_hits.saturating_add(1); }

        if self.last_dps_emit.elapsed() >= Duration::from_millis(100) {
            self.emit_dps();
        }
    }

    fn emit_dps(&mut self) {
        self.last_dps_emit = Instant::now();
        let encounter_ms = self.encounter_started.map(|x| x.elapsed().as_millis() as u64).unwrap_or(0).max(1);
        let total_damage: i64 = self.combat.values().map(|x| x.damage).sum();
        let seconds = encounter_ms as f64 / 1000.0;
        let mut rows: Vec<DpsRow> = self.combat.iter().map(|(uuid, stat)| {
            let uid = *uuid >> 16;
            let name = self.entities.get(uuid).map(|m| m.name.trim()).filter(|n| !n.is_empty())
                .map(str::to_string).unwrap_or_else(|| format!("Player {uid}"));
            DpsRow {
                actor_uuid: *uuid,
                uid,
                name,
                damage: stat.damage,
                dps: stat.damage as f64 / seconds.max(0.001),
                share: if total_damage > 0 { stat.damage as f64 * 100.0 / total_damage as f64 } else { 0.0 },
                hits: stat.hits,
                crits: stat.crits,
                lucky_hits: stat.lucky_hits,
            }
        }).collect();
        rows.sort_by(|a,b| b.damage.cmp(&a.damage));
        rows.truncate(24);
        let _ = self.tx.send(AppEvent::Dps(DpsSnapshot { encounter_ms, total_damage, rows }));
    }

    fn apply_buff(&mut self, host_uuid: i64, effect: &[u8]) {
        let event_type = proto::get_varint_field(effect, 1).unwrap_or(0) as i32;
        let buff_uuid = proto::get_varint_field(effect, 2).unwrap_or(0) as i32;
        if matches!(event_type, 2 | 6) {
            if let Some(key) = self.buff_instances.remove(&(host_uuid, buff_uuid)) {
                if self.mechanics.remove(&key).is_some() { self.emit_mechanics(false); }
            }
            return;
        }
        if !matches!(event_type, 1 | 3 | 4 | 5) { return; }

        for logic in proto::len_fields(effect, 5) {
            if proto::get_varint_field(logic, 1).unwrap_or(0) != 18 { continue; }
            let Some(info) = proto::get_len_field(logic, 2) else { continue; };
            let base_id = proto::get_varint_field(info, 2).unwrap_or(0) as i32;
            let Some((label, default_duration, priority)) = buff_rule(base_id) else { continue; };
            let duration = proto::get_varint_field(info, 11).unwrap_or(0).min(300_000) as u64;
            let duration_ms = if duration > 0 { duration } else { default_duration };
            let key = format!("buff:{host_uuid}:{buff_uuid}:{base_id}");
            let row = mechanic_row(key.clone(), label, self.display_target(host_uuid), duration_ms, duration_ms == 0, priority);
            self.mechanics.insert(key.clone(), row);
            self.buff_instances.insert((host_uuid, buff_uuid), key);

            if matches!(base_id, 884102 | 884103) {
                let tower_keys: Vec<String> = self.mechanics.keys().filter(|k| k.starts_with("entity:") && self.mechanics.get(*k).is_some_and(|r| r.label == "Tower activating")).cloned().collect();
                for k in tower_keys { self.mechanics.remove(&k); }
            }
            self.emit_mechanics(false);
        }
    }

    fn display_target(&self, uuid: i64) -> Option<String> {
        if entity_kind(uuid) != ENTITY_PLAYER { return None; }
        self.entities.get(&uuid).map(|m| m.name.trim()).filter(|n| !n.is_empty())
            .map(str::to_string).or_else(|| Some(format!("Player {}", uuid >> 16)))
    }

    fn prune_expired_mechanics(&mut self) {
        let now = now_unix_ms();
        let before = self.mechanics.len();
        self.mechanics.retain(|_, row| row.persistent || row.expires_unix_ms <= 0 || row.expires_unix_ms > now);
        if self.mechanics.len() != before { self.emit_mechanics(true); }
    }

    fn emit_mechanics(&mut self, force: bool) {
        if !force && self.last_mechanic_emit.elapsed() < Duration::from_millis(50) { return; }
        self.last_mechanic_emit = Instant::now();
        let now = now_unix_ms();
        let mut rows: Vec<MechanicRow> = self.mechanics.values().filter(|r| r.persistent || r.expires_unix_ms <= 0 || r.expires_unix_ms > now).cloned().collect();
        rows.sort_by(|a,b| b.priority.cmp(&a.priority).then_with(|| a.expires_unix_ms.cmp(&b.expires_unix_ms)));
        rows.truncate(12);
        let _ = self.tx.send(AppEvent::Mechanics(MechanicSnapshot { rows }));
    }
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
    let mut p = 0usize;
    proto::read_varint(raw, &mut p)
}

fn decode_name(raw: &[u8]) -> Option<String> {
    if raw.is_empty() { return None; }
    let mut p = 0usize;
    let decoded = if let Some(len) = proto::read_varint(raw, &mut p) {
        let len = usize::try_from(len).ok()?;
        if len > 0 && p.checked_add(len) == Some(raw.len()) { &raw[p..] }
        else if raw.len() > 1 { &raw[1..] } else { return None; }
    } else if raw.len() > 1 { &raw[1..] } else { return None; };
    let text = std::str::from_utf8(decoded).ok()?.replace(|c: char| matches!(c, '\r'|'\n'|'\0'), " ").trim().to_string();
    (!text.is_empty()).then_some(text.chars().take(128).collect())
}

fn signed(v: u64) -> i64 { v as i64 }
fn entity_kind(uuid: i64) -> i64 { (uuid >> 6) & 0x1f }
fn now_unix_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis().min(i64::MAX as u128) as i64).unwrap_or(0)
}

fn mechanic_row(key: String, label: &str, target: Option<String>, duration_ms: u64, persistent: bool, priority: u8) -> MechanicRow {
    let now = now_unix_ms();
    MechanicRow {
        key,
        label: label.into(),
        target,
        created_unix_ms: now,
        expires_unix_ms: if duration_ms > 0 { now.saturating_add(duration_ms as i64) } else { 0 },
        persistent,
        priority,
    }
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
        // S3 Giant Tower
        821076 => ("Sticky Bomb", 8_000, 3),

        // S3 Cursed Tomb
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

        // S3 Tina Mindrealm
        510571 => ("Heavy Wound", 10_000, 3),
        841519 => ("Red Bind", 10_000, 3),
        841509 => ("Wudi Slash", 10_000, 3),

        // S3 Raid
        829214 => ("Phase - EDGE", 10_000, 2),
        829215 => ("Phase - CORNER", 10_000, 2),
        829327 => ("Phase mirage - TOP LEFT", 10_000, 2),
        829328 => ("Phase mirage - MID LEFT", 10_000, 2),
        829329 => ("Phase mirage - BOTTOM LEFT", 10_000, 2),
        829330 => ("Phase mirage - TOP RIGHT", 10_000, 2),
        829331 => ("Phase mirage - MID RIGHT", 10_000, 2),
        829332 => ("Phase mirage - BOTTOM RIGHT", 10_000, 2),
        829372 => ("Preset Return #1", 10_000, 2),
        829373 => ("Preset Return #2", 10_000, 2),
        829374 => ("Preset Return #3", 10_000, 2),
        829104 => ("Electromagnetic Pulse A", 10_000, 3),
        829105 => ("Electromagnetic Pulse B", 10_000, 3),
        829106 => ("Electromagnetic Pulse C", 10_000, 3),
        829115 => ("Share - MIRAGE", 10_000, 3),
        829116 => ("Mirage Share", 10_000, 3),
        829304 => ("Share", 10_000, 3),
        829305 => ("Mirage Share", 10_000, 3),
        829306 => ("Decay", 10_000, 3),
        829307 => ("Mirage Decay", 10_000, 3),
        829308 => ("Spread", 10_000, 3),
        829309 => ("Mirage Spread", 10_000, 3),
        829316 => ("Causal Jump", 10_000, 3),
        829217 => ("Normal target", 10_000, 3),
        829245 => ("Decay target", 10_000, 3),
        829226 => ("Hit order #1", 10_000, 3),
        829227 => ("Hit order #2", 10_000, 3),
        829228 => ("Hit order #3", 10_000, 3),
        829323 => ("Divine Trick kill mark", 10_000, 3),
        829324 => ("Kill mark", 10_000, 3),
        829326 => ("Mirage kill mark", 10_000, 3),
        829314 => ("Pinball cast", 6_000, 3),

        // S3 Sea-Ringed Reef
        883602 => ("Dual mark - ICE", 10_000, 3),
        883603 => ("Dual mark - WATER", 10_000, 3),
        883633 => ("Pizza - ORANGE", 8_000, 2),
        883634 => ("Pizza - PURPLE", 8_000, 2),

        // S4 Wasteland Court
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
    fn entity_kind_layout_matches_bpsr() {
        let player = (123_i64 << 16) | (10 << 6);
        let monster = (456_i64 << 16) | (1 << 6);
        assert_eq!(entity_kind(player), ENTITY_PLAYER);
        assert_eq!(entity_kind(monster), ENTITY_MONSTER);
    }

    #[test]
    fn known_mechanics_are_registered() {
        assert_eq!(buff_rule(821076).unwrap().0, "Sticky Bomb");
        assert_eq!(skill_rule(111103).unwrap().0, "Gravity Blast");
        assert_eq!(monster_rule(2106).unwrap().0, "Correct portal");
    }
}
