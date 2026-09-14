mod previous {
    include!("encounter_context_v1300.rs");
}

pub use previous::EncounterContextSnapshot;

pub fn observe_notify(service: u64, method: u32, body: &[u8]) {
    previous::observe_notify(service, method, body);
}

pub fn snapshot() -> EncounterContextSnapshot {
    let mut snapshot = previous::snapshot();
    finalize_scene_name(&mut snapshot);
    snapshot
}

fn finalize_scene_name(snapshot: &mut EncounterContextSnapshot) {
    if snapshot.scene_id == 0 {
        return;
    }

    // Runtime overrides are intentionally strongest. This lets a frozen EXE be
    // corrected for a future Global scene name without rebuilding ReadyAlert,
    // even when an older embedded dungeon table already supplied another name.
    if let Some(name) = crate::telemetry::game_names_v1300::override_scene_name(snapshot.scene_id) {
        snapshot.scene_name = name.to_owned();
        return;
    }

    if snapshot.scene_name.trim().is_empty() {
        snapshot.scene_name = crate::telemetry::game_names_v1300::scene_name(snapshot.scene_id)
            .map(str::to_owned)
            .unwrap_or_else(|| format!("Unknown Scene ({})", snapshot.scene_id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_future_scene_keeps_numeric_identity() {
        let mut snapshot = EncounterContextSnapshot {
            scene_id: i32::MAX,
            ..EncounterContextSnapshot::default()
        };
        finalize_scene_name(&mut snapshot);
        assert_eq!(snapshot.scene_name, format!("Unknown Scene ({})", i32::MAX));
    }

    #[test]
    fn known_scene_name_is_not_replaced_by_unknown_fallback() {
        let mut snapshot = EncounterContextSnapshot {
            scene_id: 6615,
            scene_name: "Master - Desolate Court".into(),
            ..EncounterContextSnapshot::default()
        };
        finalize_scene_name(&mut snapshot);
        assert!(!snapshot.scene_name.starts_with("Unknown Scene"));
    }
}
