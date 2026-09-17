// These tests use synthetic runtime state, not observed current-game packets.
// They protect existing encounter-boundary behavior while live validation is pending.
#[cfg(test)]
mod dungeon_end_boundary_regression_tests {
    use super::*;
    use std::sync::mpsc;

    fn combat_runtime() -> (TelemetryRuntime, mpsc::Receiver<AppEvent>) {
        let (tx, rx) = mpsc::channel();
        let mut runtime = TelemetryRuntime::new(tx);
        runtime.current_scene_id = 13023;
        runtime.local_uid = 42;
        runtime.encounter_started = Some(Instant::now() - Duration::from_secs(5));
        runtime.combat.entry(42).or_default().damage = 1234;
        (runtime, rx)
    }

    #[test]
    fn manual_reset_disarms_stale_dungeon_end_for_new_combat() {
        let (mut runtime, rx) = combat_runtime();
        runtime.observe_dungeon_flow(3);
        runtime.manual_reset();
        assert!(matches!(rx.try_recv(), Ok(AppEvent::Dps(_))));
        runtime.encounter_started = Some(Instant::now() - Duration::from_secs(5));
        runtime.combat.entry(42).or_default().damage = 999;
        runtime.observe_dungeon_flow(4);
        assert_eq!(runtime.combat.get(&42).map(|actor| actor.damage), Some(999));
        assert!(runtime.encounter_started.is_some());
        assert!(rx.try_recv().is_err(), "stale End must not archive a new manual-reset segment");
    }

    #[test]
    fn pending_wipe_boundary_is_never_consumed_by_dungeon_end() {
        let (mut runtime, rx) = combat_runtime();
        runtime.observe_dungeon_flow(3);
        runtime.pending_boundary = Some(Instant::now());
        runtime.observe_dungeon_flow(4);
        assert_eq!(runtime.combat.get(&42).map(|actor| actor.damage), Some(1234));
        assert!(runtime.pending_boundary.is_some());
        assert!(runtime.encounter_started.is_some());
        assert!(rx.try_recv().is_err(), "wipe guard must not be bypassed by End");
    }

    #[test]
    fn reset_for_confirmed_wipe_disarms_replayed_end() {
        let (mut runtime, rx) = combat_runtime();
        runtime.observe_dungeon_flow(3);
        runtime.reset_encounter_keep_roster();
        runtime.encounter_started = Some(Instant::now() - Duration::from_secs(5));
        runtime.combat.entry(42).or_default().damage = 555;
        runtime.observe_dungeon_flow(5);
        assert_eq!(runtime.combat.get(&42).map(|actor| actor.damage), Some(555));
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn end_flushes_latest_hit_before_archiving_previous_encounter() {
        let (mut runtime, rx) = combat_runtime();
        runtime.emit_dps();
        let old = match rx.try_recv().unwrap() {
            AppEvent::Dps(snapshot) => snapshot,
            _ => panic!("initial Dps expected"),
        };
        assert_eq!(old.total_damage, 1234);
        // The latest hit has not yet passed the 50-ms UI emission throttle.
        runtime.combat.entry(42).or_default().damage = 2000;
        runtime.observe_dungeon_flow(3);
        runtime.observe_dungeon_flow(4);
        let final_snapshot = match rx.try_recv().unwrap() {
            AppEvent::Dps(snapshot) => snapshot,
            _ => panic!("final Dps expected"),
        };
        let empty = match rx.try_recv().unwrap() {
            AppEvent::Dps(snapshot) => snapshot,
            _ => panic!("empty Dps expected"),
        };
        assert_eq!(final_snapshot.total_damage, 2000);
        assert_eq!(empty.total_damage, 0);
        assert!(!crate::history::encounter_rolled(&old, &final_snapshot));
        assert!(crate::history::encounter_rolled(&final_snapshot, &empty));
        assert!(rx.try_recv().is_err());
    }
}
