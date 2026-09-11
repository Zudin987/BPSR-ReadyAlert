use std::{env, fs, path::{Path, PathBuf}};

mod prior {
    include!("build_v1230.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.23.1 patch {label} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn patch_telemetry(out: &Path) {
    let path = out.join("telemetry_v170_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .expect("read v1.23.0 telemetry")
        .replace("\r\n", "\n");

    // Keep roster inference conservative as a diagnostic/fallback helper, but do
    // not use it to decide automatic wipes. CN Resonance treats the dedicated
    // 510072 buff applied to the local player as the authoritative wipe signal.
    // In Raid-sized encounters include every current combat row, and only count
    // actor-state DEAD (9) as confirmed dead when authoritative player state exists.
    replace_once(
        &mut source,
        r#"    fn party_is_wiped(&self) -> bool {
        let mut roster: HashSet<i64> = self.team.iter().copied().collect();
        if self.local_uid > 0 { roster.insert(self.local_uid); }
        // Team packets can be partial. Include players that actually participated
        // in this encounter so one missing roster packet cannot turn a single
        // death into a false full-party wipe.
        for (uid, actor) in &self.combat {
            if actor.damage > 0 || actor.healing > 0 || actor.damage_taken > 0 || actor.hits > 0 || actor.deaths > 0 {
                roster.insert(*uid);
            }
        }
        // Be conservative when only one player is known. In party content this
        // avoids the v1.8.1 false reset when the roster sync is incomplete; solo
        // users can still use manual Reset or scene re-entry.
        if roster.len() < 2 { return false; }
        roster.iter().all(|uid| {
            self.combat.get(uid).is_some_and(|actor| actor.is_dead)
                || self.players.get(uid).is_some_and(|player| {
                    matches!(player.actor_state, ACTOR_STATE_DEAD | ACTOR_STATE_RESURRECTION)
                        || (player.actor_state == ACTOR_STATE_TELEPORT && player.hp <= 0)
                })
        })
    }"#,
        r#"    fn party_is_wiped(&self) -> bool {
        let mut roster: HashSet<i64> = self.team.iter().copied().collect();
        if self.local_uid > 0 { roster.insert(self.local_uid); }
        // Team packets can be partial. Include players that actually participated
        // in this encounter so one missing roster packet cannot turn a single
        // death into a false full-party wipe.
        for (uid, actor) in &self.combat {
            if actor.damage > 0 || actor.healing > 0 || actor.damage_taken > 0 || actor.hits > 0 || actor.deaths > 0 {
                roster.insert(*uid);
            }
        }
        // Raid Team messages may expose only the local five-player subgroup.
        // Once Raid-sized combat state is visible, include every known row so a
        // quiet/alive member outside the subgroup blocks inferred all-dead state.
        if self.combat.len() > 5 {
            roster.extend(self.combat.keys().copied());
        }
        if roster.len() < 2 { return false; }
        roster.iter().all(|uid| {
            // Match current CN semantics: actor state 9 is the confirmed dead
            // state. Resurrection/Teleport are transitions, not proof of death.
            if let Some(player) = self.players.get(uid) {
                player.actor_state == ACTOR_STATE_DEAD
            } else {
                self.combat.get(uid).is_some_and(|actor| actor.is_dead)
            }
        })
    }"#,
        "conservative Raid roster diagnostics",
    );

    // Do not infer an encounter reset from player ActorState transitions. A Raid
    // can expose only the local five-player subgroup, so five dead rows are not
    // proof that the full Raid wiped. The authoritative local 510072 wipe buff
    // below is what calls arm_boundary(). ActorState still drives death UI/counts.
    replace_once(
        &mut source,
        r#"        if combat.is_dead && self.party_is_wiped() {
            self.arm_boundary();
        }"#,
        r#"        // Automatic wipe boundaries are intentionally not inferred here.
        // The server-provided local wipe buff is authoritative."#,
        "stop ActorState inferred wipe resets",
    );

    // v1.8.2 guarded the CN wipe marker with an inferred all-dead roster check.
    // Current CN Resonance does the opposite: local application of wipe buff
    // 510072 is itself WipeDetected. Trust that signal, then keep the existing
    // 3-second pending boundary so trailing packets stay in the old pull and the
    // reset happens on the next eligible attack.
    replace_once(
        &mut source,
        r#"    fn arm_boundary(&mut self) {
        // The CN 510072 marker is useful evidence, but it also appears around
        // individual deaths. Never arm an automatic reset unless the observed
        // encounter roster is actually down.
        if !self.party_is_wiped() { return; }
        if self.encounter_started.is_some() && self.pending_boundary.is_none() {
            self.pending_boundary = Some(Instant::now());
        }
    }"#,
        r#"    fn arm_boundary(&mut self) {
        // WIPE_BUFF_BASE_ID (510072) applied to the local player is the
        // authoritative wipe signal. Do not require a complete party/Raid roster.
        if self.encounter_started.is_some() && self.pending_boundary.is_none() {
            self.pending_boundary = Some(Instant::now());
        }
    }"#,
        "trust authoritative CN wipe marker",
    );

    // Keep individual mechanic instances internally so despawn/buff removal still
    // works correctly, but collapse simultaneous equivalent rows at presentation
    // time. Same-label mechanics on different players remain separate.
    replace_once(
        &mut source,
        r#"        rows.sort_by(|a, b| b.priority.cmp(&a.priority).then_with(|| a.expires_unix_ms.cmp(&b.expires_unix_ms)));
        rows.truncate(20);

        let tracked_attributes = self"#,
        r#"        rows.sort_by(|a, b| b.priority.cmp(&a.priority).then_with(|| a.expires_unix_ms.cmp(&b.expires_unix_ms)));
        rows = dedupe_mechanic_rows(rows, 20);

        let tracked_attributes = self"#,
        "simultaneous mechanic display dedupe",
    );

    replace_once(
        &mut source,
        "}\n\n#[derive(Default)]\nstruct TeamCandidate {",
        r#"}

const MECHANIC_WAVE_DEDUPE_MS: u64 = 350;

fn same_mechanic_wave(a: &MechanicRow, b: &MechanicRow) -> bool {
    if a.label != b.label || a.target != b.target || a.persistent != b.persistent || a.priority != b.priority {
        return false;
    }
    if a.expires_unix_ms > 0 || b.expires_unix_ms > 0 {
        return a.expires_unix_ms > 0
            && b.expires_unix_ms > 0
            && a.expires_unix_ms.abs_diff(b.expires_unix_ms) <= MECHANIC_WAVE_DEDUPE_MS;
    }
    a.created_unix_ms.abs_diff(b.created_unix_ms) <= MECHANIC_WAVE_DEDUPE_MS
}

fn dedupe_mechanic_rows(rows: Vec<MechanicRow>, limit: usize) -> Vec<MechanicRow> {
    let mut kept = Vec::with_capacity(rows.len().min(limit));
    for row in rows {
        if kept.iter().any(|existing| same_mechanic_wave(existing, &row)) {
            continue;
        }
        kept.push(row);
        if kept.len() >= limit {
            break;
        }
    }
    kept
}

#[derive(Default)]
struct TeamCandidate {"#,
        "mechanic wave dedupe helpers",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1231_mechanics_and_raid_tests {
    use super::*;

    #[test]
    fn simultaneous_identical_mechanics_collapse_but_targets_stay_distinct() {
        let now = now_ms();
        let make = |key: &str, target: Option<&str>, expiry_offset: i64| MechanicRow {
            key: key.into(),
            label: "Tower activating".into(),
            target: target.map(str::to_string),
            created_unix_ms: now,
            expires_unix_ms: now + 40_000 + expiry_offset,
            persistent: false,
            priority: 2,
        };
        let rows = dedupe_mechanic_rows(vec![
            make("tower-a", None, 0),
            make("tower-b", None, 40),
            make("tower-player", Some("Player 1"), 20),
        ], 20);
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().any(|row| row.target.is_none()));
        assert!(rows.iter().any(|row| row.target.as_deref() == Some("Player 1")));
    }

    #[test]
    fn raid_roster_diagnostic_waits_for_members_outside_local_subgroup() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let mut runtime = TelemetryRuntime::new(tx);
        runtime.local_uid = 1;
        for uid in 1..=5 {
            runtime.team.insert(uid);
            runtime.combat.entry(uid).or_default().is_dead = true;
        }
        assert!(runtime.party_is_wiped());

        runtime.combat.entry(6).or_default().is_dead = false;
        assert!(!runtime.party_is_wiped());
        runtime.combat.get_mut(&6).unwrap().is_dead = true;
        assert!(runtime.party_is_wiped());
    }

    #[test]
    fn resurrection_transition_is_not_confirmed_dead_for_wipe_diagnostic() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let mut runtime = TelemetryRuntime::new(tx);
        runtime.local_uid = 1;
        for uid in 1..=5 {
            runtime.team.insert(uid);
            runtime.combat.entry(uid).or_default().is_dead = true;
        }
        runtime.players.entry(1).or_default().actor_state = ACTOR_STATE_RESURRECTION;
        assert!(!runtime.party_is_wiped());
        runtime.players.get_mut(&1).unwrap().actor_state = ACTOR_STATE_DEAD;
        assert!(runtime.party_is_wiped());
    }

    #[test]
    fn actorstate_deaths_do_not_arm_an_automatic_wipe_boundary() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let mut runtime = TelemetryRuntime::new(tx);
        runtime.local_uid = 1;
        runtime.encounter_started = Some(Instant::now());
        for uid in 1..=5 {
            runtime.team.insert(uid);
            runtime.apply_player_actor_state(uid, 0, ACTOR_STATE_DEAD);
        }
        assert!(runtime.pending_boundary.is_none());
    }

    #[test]
    fn authoritative_wipe_signal_does_not_require_complete_raid_roster() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let mut runtime = TelemetryRuntime::new(tx);
        runtime.local_uid = 1;
        runtime.encounter_started = Some(Instant::now());
        for uid in 1..=5 {
            runtime.team.insert(uid);
            runtime.combat.entry(uid).or_default().is_dead = false;
        }
        runtime.arm_boundary();
        assert!(runtime.pending_boundary.is_some());
    }
}
"#);

    fs::write(path, source).expect("write v1.23.1 telemetry");
}

fn main() {
    prior::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_telemetry(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1231.rs");
}
