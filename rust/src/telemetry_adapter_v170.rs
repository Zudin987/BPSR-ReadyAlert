use crate::{model::{AppEvent, DpsRow, DpsSnapshot}, proto};
use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
};

mod inner {
    include!(concat!(env!("OUT_DIR"), "/telemetry_v170_fixed.rs"));
}

static MANUAL_RESET_REQUESTED: AtomicBool = AtomicBool::new(false);

/// Called by the native DPS overlay Reset button. The capture worker consumes
/// this on its next packet without introducing another thread or runtime.
pub fn request_manual_reset() {
    MANUAL_RESET_REQUESTED.store(true, Ordering::Release);
}

pub struct TelemetryRuntime {
    inner: inner::TelemetryRuntime,
    inner_rx: Receiver<AppEvent>,
    tx: Sender<AppEvent>,
    team_uids: HashSet<i64>,
}

impl TelemetryRuntime {
    pub fn new(tx: Sender<AppEvent>) -> Self {
        let (inner_tx, inner_rx) = mpsc::channel();
        Self {
            inner: inner::TelemetryRuntime::new(inner_tx),
            inner_rx,
            tx,
            team_uids: HashSet::new(),
        }
    }

    pub fn handle_notify(&mut self, service: u64, method: u32, body: &[u8]) {
        if MANUAL_RESET_REQUESTED.swap(false, Ordering::AcqRel) {
            self.inner.manual_reset();
            self.forward_inner_events();
        }
        if service == proto::TEAM_SERVICE {
            self.update_team(method, body);
        }
        self.inner.handle_notify(service, method, body);
        self.forward_inner_events();
    }

    fn update_team(&mut self, method: u32, body: &[u8]) {
        match method {
            0x02 => {
                if let Some(req) = proto::get_len_field(body, 1) {
                    for field in [5_u32, 6_u32] {
                        for member in proto::len_fields(req, field) {
                            self.add_char_id(proto::get_varint_field(member, 1));
                        }
                    }
                }
            }
            0x03 => {
                if let Some(req) = proto::get_len_field(body, 1) {
                    for member in proto::len_fields(req, 2) {
                        self.add_char_id(proto::get_varint_field(member, 1));
                    }
                    for entry in proto::len_fields(req, 6) {
                        self.add_char_id(proto::get_varint_field(entry, 1));
                        if let Some(value) = proto::get_len_field(entry, 2) {
                            self.add_char_id(proto::get_varint_field(value, 1));
                        }
                    }
                }
            }
            0x04 => {
                if let Some(req) = proto::get_len_field(body, 1) {
                    if let Some(id) = proto::get_varint_field(req, 1).and_then(valid_uid) {
                        self.team_uids.remove(&id);
                    }
                }
            }
            0x0d => self.team_uids.clear(),
            _ => {}
        }
    }

    fn add_char_id(&mut self, value: Option<u64>) {
        if let Some(uid) = value.and_then(valid_uid) {
            self.team_uids.insert(uid);
        }
    }

    fn forward_inner_events(&mut self) {
        while let Ok(event) = self.inner_rx.try_recv() {
            let event = match event {
                AppEvent::Dps(mut snapshot) => {
                    enrich_snapshot(&mut snapshot, &self.team_uids);
                    AppEvent::Dps(snapshot)
                }
                other => other,
            };
            let _ = self.tx.send(event);
        }
    }
}

fn valid_uid(raw: u64) -> Option<i64> {
    if raw == 0 || raw > i64::MAX as u64 {
        return None;
    }
    let uid = raw as i64;
    (uid > 0 && uid < 10_000_000_000_000).then_some(uid)
}

fn enrich_snapshot(snapshot: &mut DpsSnapshot, team: &HashSet<i64>) {
    for row in &mut snapshot.rows {
        // The compact telemetry core intentionally keeps a small fallback table.
        // Normalize it here from the maintained BPSR-ZDPS English overrides so
        // the drill-down does not expose raw wire ids for ordinary player skills.
        for skill in &mut row.skills {
            if let Some(name) = known_skill_name(skill.skill_id) {
                skill.name = name.into();
            }
        }

        for imagine in &mut row.imagines {
            if let Some((name, icon)) = imagine_metadata(imagine.skill_id) {
                imagine.name = name.into();
                imagine.icon_key = icon.into();
            } else {
                // Preserve v1.7 compatibility aliases for records produced before
                // the expanded metadata table existed.
                if imagine.name.eq_ignore_ascii_case("Tina") || imagine.icon_key.eq_ignore_ascii_case("TI") {
                    imagine.icon_key = "TN".into();
                }
                if imagine.name.eq_ignore_ascii_case("Brigand Leader") {
                    imagine.icon_key = "BL".into();
                }
            }
        }

        if row.subprofession_name.trim().is_empty() {
            if let Some((sub_id, profession_id, name)) = infer_spec(row) {
                row.subprofession_id = sub_id;
                row.subprofession_name = name.into();
                if row.profession_id == 0 {
                    row.profession_id = profession_id;
                }
            }
        }
    }

    for uid in team.iter().copied() {
        if snapshot.rows.iter().any(|row| row.uid == uid) {
            continue;
        }
        snapshot.rows.push(DpsRow {
            actor_uuid: canonical_player_uuid(uid),
            uid,
            name: format!("Player {uid}"),
            ..DpsRow::default()
        });
    }
    snapshot.rows.sort_by(|a, b| {
        b.damage
            .cmp(&a.damage)
            .then_with(|| b.healing.cmp(&a.healing))
            .then_with(|| a.name.cmp(&b.name))
    });
}

/// Player-skill overrides and common child/effect ids from BPSR-ZDPS
/// `Data/SkillOverrides.en.json`. Child ids deliberately collapse to the
/// player-facing ability name where that makes the distribution easier to read.
fn known_skill_name(id: i32) -> Option<&'static str> {
    Some(match id {
        // Frost Mage child/effect ids.
        120201 | 120301 | 120401 | 120501 => "Raincall Surge",
        120901 => "Piercing Ice Spear",
        120902 => "Frost Lance",
        121301 | 121302 => "Ice Arrow",
        121501 => "Crystal Veil",
        27009 => "Frost Shelter",

        // Wind Knight child/effect ids.
        140301 => "Vortex Strike (Final Strike)",
        140401 => "Drake Cannon (Backward Leap)",
        140501 => "Tornado",
        149901 => "Spiral Detonation (Wind Spiral)",
        149902 => "Spear Thrust",
        149904 => "Instant Edge Combo",
        149905 => "Falcon Toss (Spear Toss)",
        149906 => "Azure Sever (Spear Toss)",
        149907 => "Sharp Impact (Leap)",
        31901 => "Valor Cyclone",

        // Verdant Oracle / Smite / Lifebind.
        1518 => "Life Bloom",
        20301 => "Life Bloom",
        21402 => "Wild Bloom",
        21404 => "Nourish",
        21406 => "Grove Wish",
        21414 => "Divine Circle Bloom",
        21418 => "Stag Charge",
        21423 => "Symbiotic Mark",
        21424 => "Thorns",
        21427 => "Smite Healing Missiles A1",
        21428 => "Smite Healing Missiles A2",
        21429 => "Smite Healing Missiles A3",
        21430 => "Smite Healing Missiles A4",

        // Twin Striker secondary/talent ids.
        160102 => "Hellfire Meteor",
        160103 => "Falling Hellfire - Crimson Earth",
        35104 => "Frenzied Slash",
        35105 => "Flowing Splendor Slash",
        35106 => "Life Steal",
        35107 => "Formless Flame Slash - Stage 1",
        35108 => "Formless Flame Slash - Stage 2",
        35109 => "Formless Flame Slash - Stage 3",

        // Stormblade. These replace the generic v1.8 "Signature" labels.
        1701 => "Judgment Cut - Stage 1",
        1702 => "Judgment Cut - Stage 2",
        1703 => "Judgment Cut - Stage 3",
        1704 => "Judgment Cut - Stage 4",
        1714 | 1734 => "Iaido Slash",
        1715 => "Moonstrike",
        1724 => "Thundercut - Stage 1",
        1725 => "Thundercut - Stage 2",
        1726 => "Thundercut - Stage 3",
        1727 | 1739 => "Piercing Slash",
        1728 => "Ultimate Slash",
        1731 | 1732 => "Stormflash",
        1735 => "Dracoflash",
        1736 => "Phantom Slash",
        1738 => "Chaos Breaker",
        1740 => "Storm Scythe (Thundercleave)",
        1742 => "Thundercleave",
        43201 => "Stormflash",
        44701 => "Moonblade",
        179904 => "Phantom Slash - Final Attack",
        179906 => "Moonblade (Whirling)",
        179907 => "Bladewind Domain",
        179908 => "Thunderstrike",
        179910 => "Thunderburst",

        // Heavy Guardian / tank secondary ids.
        1901 => "Halberd's Edge - Stage 1 / Granite",
        1902 => "Halberd's Edge - Stage 2 / Granite",
        1903 => "Halberd's Edge - Stage 3",
        1904 => "Halberd's Edge - Stage 4",
        1909 => "Halberd's Edge (Shield Slam)",
        1912 => "Halberd's Edge - Lethal Skill Counter",
        1932 => "Shield Combo",
        1935 => "Rageblow",
        199902 => "Terra Sunder",
        199903 => "Stone Fist",
        50024 => "Shield",
        50033 => "Sandgrip",
        50036 => "Weakness Strike",
        50037 => "Countercrush (Counter Storm)",
        50042 => "Super Armor",
        50049 => "Sandshroud",
        50050 => "Countercrush",
        50052 => "Magic Shield",
        50057 => "Stoneform",
        50058 => "Brave Bastion",
        50067 => "Block Counterattack - Medium",
        50068 => "Block Counterattack - Strong",

        // Marksman / Wildpack / Falconry.
        2201 | 220101 | 220103 | 221101 => "Bullseye",
        2240 => "Lumi Torrent",
        2288 => "Light Bomb",
        2289 => "Arrow Rain",
        2290 => "Luminary Bolt",
        2291 => "Photon Explosion",
        2292 => "Phantom Direwolf",
        2294 => "Quadraflare (Second Arrow)",
        2295 => "Luminary Bolt (Gravity Orb)",
        2296 | 220113 => "Phantom Falcon",
        220102 => "Torrent Volley",
        220104 => "Storm Arrow",
        220105 => "Lightseeker Arrow",
        220106 => "Double Arrow",
        220107 => "Magic Arrow",
        220108 => "Explosive Arrow",
        220109 => "Deter Shot",
        220110 => "Blast Shot",
        220111 => "Photon Reforge - Quadraflare",
        220112 => "Radiance Rift",
        220203 => "Quadraflare (First Arrow)",
        220301 => "Luminary Bolt (Arrow Damage)",
        55221 => "Photon Regorge",
        55223 => "Focus",
        55231 => "Blast Shot (Buff Explosion)",
        55235 => "Photon Reforge - Quadraflare (AOE)",
        55240 => "Radiance Barrage",

        // Beat Performer. v1.8 previously labelled several of these as a
        // generic Concerto/Dissonance signature rather than their real names.
        2301 => "Resonant Strings - Stage 1",
        2302 => "Resonant Strings - Stage 2",
        2303 => "Resonant Strings - Stage 3",
        2304 => "Resonant Strings - Stage 4",
        2305 | 230501 | 230401 => "Encore Damage",
        2306 => "Amplified Beat",
        2309 | 230801 | 230901 | 231001 | 55301 => "Rhapsody of Flame",
        2310 | 55341 => "Heroic Melody",
        2311 | 55342 => "Healing Melody",
        2312 | 55304 => "Fivefold Crescendo",
        2313 => "Passion Burst",
        2314 | 55344 => "Rock the Stage",
        2315 | 55314 => "Encore",
        2316 => "Center Stage",
        2317 => "Fierce Strike",
        2318 => "Finale! Healing Movement",
        2319 => "Sound Wave Surge",
        2320 => "Flame Shock",
        2321 => "String Strike - Stage 1",
        2322 => "String Strike - Stage 2",
        2323 => "String Strike - Stage 3",
        2324 => "String Strike - Stage 4",
        2329 => "Flame Pillar Blast",
        2330 => "Flame Pillar Blast (Enhanced)",
        2331 => "Sound Blaze - Scorching Impact",
        2332 => "Passion Fury",
        2335 => "Infinite Rhapsody",
        2336 | 55339 => "Concert Circuit",
        2361 => "Healing Beat copy",
        2362 => "Fivefold Crescendo (Close Range)",
        2363 => "Healing Beat (Speaker)",
        2364 => "Rock the Stage (Speaker)",
        2365 => "Infinite Rhapsody (Speaker)",
        2366 => "Concert Circuit (Speaker)",
        55302 => "Healing Beat",
        55311 => "Peaceful Tune",
        553313 => "Amplified Harmonic Anthem",
        55328 => "Surge Quintet - Double Play",
        55335 => "Passionate Stage 3",
        55355 => "Healing of Stillness - Stage 1/2",
        55356 => "Healing of Stillness - Stage 3",
        55357 => "Healing of Stillness - Stage 4",

        // Sword & Shield / Recovery / Shield.
        2401 => "Blade of Justice - Stage 1",
        2402 => "Blade of Justice - Stage 2",
        2403 => "Blade of Justice - Stage 3",
        2404 => "Blade of Justice - Stage 4",
        2405 => "Valor Bash",
        2406 => "Vanguard Strike / Vanguard Hunt",
        2407 => "Radiant Infusion",
        2408 | 240101 | 2425 => "Shield Toss",
        2409 | 55404 => "Divine Circle",
        2410 | 2451 | 55421 => "Judgment",
        2411 => "Scorching Judgment",
        2452 => "Scorching Judgment Sacred Blade",
        2412 => "Reckoning",
        2413 => "Inferno Reckoning",
        2414 => "Holy Barrier",
        2415 | 55405 => "Aegis Ward",
        2416 => "Condemn",
        2417 => "Enhanced Condemn",
        2419 | 55412 | 55416 | 55417 | 55431 | 55432 => "Zeal Crusade",
        2420 | 240102 | 55413 | 55414 | 55415 => "Radiance",
        2421 => "Sacred Blade",
        2450 => "Radiant Impact",
        2206401 => "Divine Strike",

        // Collaboration player-form skills that can legitimately appear as a
        // player's outgoing damage rather than a monster ability.
        2501 => "Lucy - Basic - Stage 1",
        2502 => "Lucy - Basic - Stage 2",
        2503 => "Lucy - Basic - Stage 3",
        2504 => "Lucy - Fleuve d'Etoiles",
        2505 => "Lucy - Celestial Guardian",
        2506 | 250101 => "Lucy - Aqua Metria",
        2601 => "Natsu - Basic - Stage 1",
        2602 => "Natsu - Basic - Stage 2",
        2603 => "Natsu - Basic - Stage 3",
        2604 => "Natsu - Fire Dragon's Iron Fist",
        2605 => "Natsu - Fire Dragon's Wing Attack",
        2606 => "Natsu - Fire Dragon's Roar",

        _ => return None,
    })
}

/// Common Battle Imagine names sourced from CN Resonance's fantasy summon map
/// plus the English BPSR-ZDPS override descriptions. Unknown/new ids remain
/// visible using the telemetry core fallback instead of being guessed.
fn imagine_metadata(id: i32) -> Option<(&'static str, &'static str)> {
    Some(match id {
        3903 => ("Brigand Leader", "BL"),
        3904 => ("Muku Warrior", "MW"),
        3905 => ("Boarrier Tyrant", "BT"),
        3910 => ("Void Ogre", "VO"),
        3915 => ("Goblin Warrior", "GW"),
        3920 => ("Airona", "AI"),
        3921 => ("Tina", "TN"),
        3922 => ("Olvera", "OL"),
        3923 => ("Muku Chief", "MC"),
        3924 => ("Goblin Wind Mage", "GM"),
        3925 => ("Mighty Colossus", "CO"),
        3926 => ("Storm Goblin King", "SG"),
        3941 => ("Void Bzzar", "VB"),
        3949 => ("Dorothy", "DO"),
        3968 => ("Denvel", "DE"),
        3969 => ("Igoreus", "IG"),
        3971 => ("Kartgriff", "KA"),
        3982 => ("Lucy", "LU"),
        3983 => ("Natsu", "NA"),
        _ => return None,
    })
}

fn infer_spec(row: &DpsRow) -> Option<(i32, i32, &'static str)> {
    for skill in &row.skills {
        let result = match skill.skill_id {
            1714 | 1734 => (1001, 1, "Iaido"),
            1715 | 1738 | 179906 => (1002, 1, "Moonstrike"),
            120901 | 120902 => (2001, 2, "Icicle"),
            1241 => (2002, 2, "Frostbeam"),
            160102 | 2208181 | 2208172 => (3001, 3, "Formless"),
            1606 | 1621 | 1622 | 35104 => (3002, 3, "Crimson"),
            1405 | 1418 => (4001, 4, "Vanguard"),
            1419 => (4002, 4, "Skyward"),
            1518 | 1541 | 21402 => (5001, 5, "Smite"),
            20301 => (5002, 5, "Lifebind"),
            1941 | 2201240 => (9001, 9, "Earthfort"),
            1930 | 1931 | 1934 | 1935 => (9002, 9, "Block"),
            2292 | 1700820 | 1700825 | 1700827 => (11001, 11, "Wildpack"),
            220112 | 2203622 | 220106 => (11002, 11, "Falconry"),
            2405 | 2411 | 2206401 => (12001, 12, "Recovery"),
            2406 | 55412 | 55417 => (12002, 12, "Shield"),
            2321 | 2335 => (13001, 13, "Dissonance"),
            2301 | 2336 | 2361 | 55302 => (13002, 13, "Concerto"),
            _ => continue,
        };
        return Some(result);
    }
    None
}

fn canonical_player_uuid(uid: i64) -> i64 {
    (uid << 16) | (10_i64 << 6)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SkillBreakdown;

    #[test]
    fn team_join_extracts_member_ids() {
        let body = [0x0a, 0x09, 0x12, 0x02, 0x08, 0x7b, 0x12, 0x03, 0x08, 0xc8, 0x03];
        let (tx, _rx) = mpsc::channel();
        let mut telemetry = TelemetryRuntime::new(tx);
        telemetry.update_team(0x03, &body);
        assert!(telemetry.team_uids.contains(&123));
        assert!(telemetry.team_uids.contains(&456));
    }

    #[test]
    fn smite_signature_sets_spec() {
        let row = DpsRow {
            skills: vec![SkillBreakdown { skill_id: 1518, ..Default::default() }],
            ..Default::default()
        };
        assert_eq!(infer_spec(&row), Some((5001, 5, "Smite")));
    }

    #[test]
    fn form_and_crimson_use_requested_names() {
        let formless = DpsRow {
            skills: vec![SkillBreakdown { skill_id: 160102, ..Default::default() }],
            ..Default::default()
        };
        let crimson = DpsRow {
            skills: vec![SkillBreakdown { skill_id: 1606, ..Default::default() }],
            ..Default::default()
        };
        assert_eq!(infer_spec(&formless).map(|x| x.2), Some("Formless"));
        assert_eq!(infer_spec(&crimson).map(|x| x.2), Some("Crimson"));
    }

    #[test]
    fn source_skill_ids_override_generic_signature_labels() {
        assert_eq!(known_skill_name(1714), Some("Iaido Slash"));
        assert_eq!(known_skill_name(1738), Some("Chaos Breaker"));
        assert_eq!(known_skill_name(2301), Some("Resonant Strings - Stage 1"));
        assert_eq!(known_skill_name(2405), Some("Valor Bash"));
        assert_eq!(known_skill_name(55302), Some("Healing Beat"));
    }

    #[test]
    fn child_damage_ids_resolve_to_player_facing_names() {
        assert_eq!(known_skill_name(120902), Some("Frost Lance"));
        assert_eq!(known_skill_name(179904), Some("Phantom Slash - Final Attack"));
        assert_eq!(known_skill_name(220110), Some("Blast Shot"));
        assert_eq!(known_skill_name(21414), Some("Divine Circle Bloom"));
    }

    #[test]
    fn imagine_metadata_supplies_hover_ready_names() {
        assert_eq!(imagine_metadata(3923), Some(("Muku Chief", "MC")));
        assert_eq!(imagine_metadata(3921), Some(("Tina", "TN")));
    }

    #[test]
    fn tina_badge_is_tn() {
        let mut snapshot = DpsSnapshot {
            rows: vec![DpsRow {
                imagines: vec![crate::model::ImagineBadge {
                    skill_id: 3921,
                    name: "Tina".into(),
                    icon_key: "TI".into(),
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        };
        enrich_snapshot(&mut snapshot, &HashSet::new());
        assert_eq!(snapshot.rows[0].imagines[0].icon_key, "TN");
        assert_eq!(snapshot.rows[0].imagines[0].name, "Tina");
    }
}
