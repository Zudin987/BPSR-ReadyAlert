mod previous {
    include!("encounter_context_v1270.rs");
}

pub use previous::EncounterContextSnapshot;

pub fn observe_notify(service: u64, method: u32, body: &[u8]) {
    previous::observe_notify(service, method, body);
}

pub fn snapshot() -> EncounterContextSnapshot {
    let mut snapshot = previous::snapshot();
    add_scene_name_fallback(&mut snapshot);
    snapshot
}

fn add_scene_name_fallback(snapshot: &mut EncounterContextSnapshot) {
    if snapshot.scene_id == 0 || !snapshot.scene_name.trim().is_empty() {
        return;
    }
    if let Some(name) = crate::telemetry::game_names_v1300::scene_name(snapshot.scene_id) {
        snapshot.scene_name = name.to_owned();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ambiguous_scene_gets_exact_scene_table_name_without_guessing_dungeon() {
        let mut snapshot = EncounterContextSnapshot {
            scene_id: 1031,
            ..EncounterContextSnapshot::default()
        };
        add_scene_name_fallback(&mut snapshot);
        assert_eq!(snapshot.dungeon_id, 0);
        assert_eq!(snapshot.scene_name, "Chaotic - Tina's Mindrealm");
    }
}
