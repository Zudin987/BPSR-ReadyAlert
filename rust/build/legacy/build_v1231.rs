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

    // A 20-player Raid can expose only a five-player subgroup through some Team
    // messages. Once the encounter has more than five player rows, include every
    // current encounter participant in wipe detection so one subgroup dying does
    // not arm a reset while another Raid member is still alive.
    replace_once(
        &mut source,
        r#"    fn party_is_wiped(&self) -> bool {
        let mut roster: Vec<i64> = self.team.iter().copied().collect();
        if self.local_uid > 0 && !roster.contains(&self.local_uid) {
            roster.push(self.local_uid);
        }
        !roster.is_empty()
            && roster
                .iter()
                .all(|uid| self.combat.get(uid).is_some_and(|actor| actor.is_dead))
    }"#,
        r#"    fn party_is_wiped(&self) -> bool {
        let mut roster: Vec<i64> = self.team.iter().copied().collect();
        if self.local_uid > 0 && !roster.contains(&self.local_uid) {
            roster.push(self.local_uid);
        }
        // Normal parties keep their authoritative Team roster. In Raid-sized
        // encounters, combat rows also contain members outside the local subgroup.
        // Treat those rows as part of the wipe roster and fail closed when any
        // participant is still alive or has not produced a dead ActorState.
        if self.combat.len() > 5 {
            for uid in self.combat.keys().copied() {
                if !roster.contains(&uid) {
                    roster.push(uid);
                }
            }
        }
        !roster.is_empty()
            && roster
                .iter()
                .all(|uid| self.combat.get(uid).is_some_and(|actor| actor.is_dead))
    }"#,
        "Raid-safe wipe roster",
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
    fn raid_wipe_waits_for_members_outside_local_subgroup() {
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
