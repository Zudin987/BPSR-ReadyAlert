use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1334_imagine_hover_alignment.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.33.5 {label} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn main() {
    previous::run();

    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let path = out.join("telemetry_v170_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .expect("read generated telemetry for v1.33.5 wipe detection")
        .replace("\r\n", "\n");

    // Season 3 sometimes emits the wipe-reset buff in situations where it is not
    // an actual wipe. Match current ZDPS behavior by requiring death evidence
    // before trusting 510072, while still accepting the local actor-state as a
    // compatibility fallback if the death-marker buff packet was missed.
    replace_once(
        &mut source,
        r#"// CN Resonance emits WipeDetected when this buff is applied to the local player.
const WIPE_BUFF_BASE_ID: i32 = 510_072;"#,
        r#"// Automatic wipe detection markers used by current ZDPS. Season 3 can send
// 510072 in non-wipe reset paths, so it must be correlated with death evidence.
const WIPE_DEATH_MARKER_BUFF_ID: i32 = 500_111;
const WIPE_REVIVE_MARKER_BUFF_ID: i32 = 500_112;
const WIPE_BUFF_BASE_ID: i32 = 510_072;
const NON_WIPE_RESET_BUFF_BASE_ID: i32 = 900_122;"#,
        "wipe marker constants",
    );

    replace_once(
        &mut source,
        r#"    revive_block_instances: HashMap<(i64, i32), i64>,
    // host -> (buff_uuid, exact base_id, wall-clock expiry, total duration)."#,
        r#"    revive_block_instances: HashMap<(i64, i32), i64>,
    // Latest death-marker buff UUID per player host. A later 510072 on the same
    // host is accepted as a wipe; revive/non-wipe reset markers clear it.
    wipe_death_markers: HashMap<i64, i32>,
    // host -> (buff_uuid, exact base_id, wall-clock expiry, total duration)."#,
        "wipe marker runtime state",
    );

    replace_once(
        &mut source,
        r#"            revive_block_instances: HashMap::new(),
            enrage_instances: HashMap::new(),"#,
        r#"            revive_block_instances: HashMap::new(),
            wipe_death_markers: HashMap::new(),
            enrage_instances: HashMap::new(),"#,
        "wipe marker initialization",
    );

    replace_once(
        &mut source,
        r#"        self.revive_block_instances.clear();
        self.enrage_instances.clear();"#,
        r#"        self.revive_block_instances.clear();
        self.wipe_death_markers.clear();
        self.enrage_instances.clear();"#,
        "scene reset wipe marker cleanup",
    );

    replace_once(
        &mut source,
        r#"        self.encounter_started = None;
        self.pending_boundary = None;
        self.last_target = 0;"#,
        r#"        self.encounter_started = None;
        self.pending_boundary = None;
        self.wipe_death_markers.clear();
        self.last_target = 0;"#,
        "encounter reset wipe marker cleanup",
    );

    replace_once(
        &mut source,
        r#"    fn arm_boundary(&mut self) {
        // WIPE_BUFF_BASE_ID (510072) applied to the local player is the
        // authoritative wipe signal. Do not require a complete party/Raid roster.
        if self.encounter_started.is_some() && self.pending_boundary.is_none() {
            self.pending_boundary = Some(Instant::now());
        }
    }
"#,
        r#"    fn wipe_context_ready(&self) -> bool {
        self.encounter_started
            .is_some_and(|start| start.elapsed() >= Duration::from_secs(1))
    }

    fn observe_wipe_buff_event(&mut self, host: i64, buff_uuid: i32, base_id: i32) {
        if entity_kind(host) != ENTITY_PLAYER || buff_uuid <= 0 {
            return;
        }
        match base_id {
            WIPE_DEATH_MARKER_BUFF_ID => {
                if self.encounter_started.is_some() {
                    self.wipe_death_markers.insert(host, buff_uuid);
                }
            }
            WIPE_REVIVE_MARKER_BUFF_ID | NON_WIPE_RESET_BUFF_BASE_ID => {
                self.wipe_death_markers.remove(&host);
            }
            WIPE_BUFF_BASE_ID => {
                let death_before_wipe = self
                    .wipe_death_markers
                    .get(&host)
                    .is_some_and(|death_uuid| *death_uuid > 0 && *death_uuid < buff_uuid);
                let uid = host >> 16;
                let local_actor_down = uid == self.local_uid
                    && self.combat.get(&uid).is_some_and(|actor| actor.is_dead);
                // Team packets can be incomplete in raids, so party_is_wiped is
                // supporting evidence only, never a prerequisite.
                let party_down = self.party_is_wiped();
                if self.wipe_context_ready() && (death_before_wipe || local_actor_down || party_down) {
                    self.arm_boundary();
                }
            }
            _ => {}
        }
    }

    fn arm_boundary(&mut self) {
        if self.encounter_started.is_some() && self.pending_boundary.is_none() {
            self.pending_boundary = Some(Instant::now());
        }
    }
"#,
        "validated wipe detector",
    );

    replace_once(
        &mut source,
        r#"            if event_type == 1
                && base_id == WIPE_BUFF_BASE_ID
                && entity_kind(host) == ENTITY_PLAYER
                && host >> 16 == self.local_uid
            {
                self.arm_boundary();
            }"#,
        r#"            if event_type == 1 {
                self.observe_wipe_buff_event(host, buff_uuid, base_id);
            }"#,
        "all-player wipe buff observation",
    );

    // A normal living state is positive evidence that an old death marker is no
    // longer relevant. This protects against a dropped 500112 revive packet.
    replace_once(
        &mut source,
        r#"        combat.is_dead = match new_state {
            ACTOR_STATE_DEAD | ACTOR_STATE_RESURRECTION => true,
            ACTOR_STATE_TELEPORT if was_down || matches!(old_state, ACTOR_STATE_DEAD | ACTOR_STATE_RESURRECTION) => true,
            _ => false,
        };
        // Automatic wipe boundaries are intentionally not inferred here.
        // The server-provided local wipe buff is authoritative."#,
        r#"        combat.is_dead = match new_state {
            ACTOR_STATE_DEAD | ACTOR_STATE_RESURRECTION => true,
            ACTOR_STATE_TELEPORT if was_down || matches!(old_state, ACTOR_STATE_DEAD | ACTOR_STATE_RESURRECTION) => true,
            _ => false,
        };
        let is_down = combat.is_dead;
        if !is_down {
            let actor_uuid = self.players.get(&uid).map(|meta| meta.actor_uuid).unwrap_or(0);
            if actor_uuid != 0 {
                self.wipe_death_markers.remove(&actor_uuid);
            } else {
                self.wipe_death_markers.retain(|host, _| *host >> 16 != uid);
            }
        }"#,
        "stale death-marker cleanup",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1335_wipe_detection_tests {
    use super::*;
    use std::sync::mpsc;

    fn active_runtime() -> TelemetryRuntime {
        let (tx, _rx) = mpsc::channel();
        let mut runtime = TelemetryRuntime::new(tx);
        runtime.local_uid = 42;
        runtime.encounter_started = Instant::now().checked_sub(Duration::from_secs(2));
        runtime
    }

    #[test]
    fn remote_death_then_wipe_marker_arms_boundary() {
        let mut runtime = active_runtime();
        let host = canonical_player_uuid(77);
        runtime.observe_wipe_buff_event(host, 100, WIPE_DEATH_MARKER_BUFF_ID);
        assert!(runtime.pending_boundary.is_none());
        runtime.observe_wipe_buff_event(host, 101, WIPE_BUFF_BASE_ID);
        assert!(runtime.pending_boundary.is_some());
    }

    #[test]
    fn season3_bare_wipe_reset_is_not_trusted_for_alive_player() {
        let mut runtime = active_runtime();
        let host = canonical_player_uuid(77);
        runtime.observe_wipe_buff_event(host, 101, WIPE_BUFF_BASE_ID);
        assert!(runtime.pending_boundary.is_none());
    }

    #[test]
    fn revive_marker_cancels_prior_death_evidence() {
        let mut runtime = active_runtime();
        let host = canonical_player_uuid(77);
        runtime.observe_wipe_buff_event(host, 100, WIPE_DEATH_MARKER_BUFF_ID);
        runtime.observe_wipe_buff_event(host, 101, WIPE_REVIVE_MARKER_BUFF_ID);
        runtime.observe_wipe_buff_event(host, 102, WIPE_BUFF_BASE_ID);
        assert!(runtime.pending_boundary.is_none());
    }

    #[test]
    fn correct_non_wipe_reset_cancels_prior_death_evidence() {
        let mut runtime = active_runtime();
        let host = canonical_player_uuid(77);
        runtime.observe_wipe_buff_event(host, 100, WIPE_DEATH_MARKER_BUFF_ID);
        runtime.observe_wipe_buff_event(host, 101, NON_WIPE_RESET_BUFF_BASE_ID);
        runtime.observe_wipe_buff_event(host, 102, WIPE_BUFF_BASE_ID);
        assert!(runtime.pending_boundary.is_none());
    }

    #[test]
    fn local_dead_state_recovers_when_death_marker_packet_was_missed() {
        let mut runtime = active_runtime();
        runtime.combat.entry(42).or_default().is_dead = true;
        runtime.observe_wipe_buff_event(canonical_player_uuid(42), 101, WIPE_BUFF_BASE_ID);
        assert!(runtime.pending_boundary.is_some());
    }

    #[test]
    fn startup_reset_guard_prevents_false_boundary() {
        let (tx, _rx) = mpsc::channel();
        let mut runtime = TelemetryRuntime::new(tx);
        runtime.local_uid = 42;
        runtime.encounter_started = Some(Instant::now());
        let host = canonical_player_uuid(77);
        runtime.observe_wipe_buff_event(host, 100, WIPE_DEATH_MARKER_BUFF_ID);
        runtime.observe_wipe_buff_event(host, 101, WIPE_BUFF_BASE_ID);
        assert!(runtime.pending_boundary.is_none());
    }
}
"#);

    fs::write(path, source).expect("write v1.33.5 stronger wipe detection telemetry");
    println!("cargo:rerun-if-changed=build/legacy/build_v1335_stronger_wipe_detection.rs");
}
