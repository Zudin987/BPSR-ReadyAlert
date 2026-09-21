mod previous {
    include!("encounter_context_v1320.rs");
}

pub use previous::EncounterContextSnapshot;

// Context updates must obey the same non-destructive scene guard as telemetry.
// A partial EnterScene is not evidence that the previous scene ended.
fn incoming_scene_id(body: &[u8]) -> Option<i32> {
    let info = crate::proto::get_len_field(body, 1)?;
    let attrs = crate::proto::get_len_field(info, 1)?;
    for attr in crate::proto::len_fields(attrs, 2) {
        if crate::proto::get_varint_field(attr, 1) != Some(0x155) { continue; }
        let raw = crate::proto::get_len_field(attr, 2)?;
        let mut offset = 0;
        let scene = crate::proto::read_varint(raw, &mut offset)?;
        return i32::try_from(scene).ok().filter(|scene| *scene > 0);
    }
    None
}

pub fn observe_notify(service: u64, method: u32, body: &[u8]) {
    if service == crate::proto::WORLD_SERVICE && method == crate::proto::ENTER_SCENE_METHOD {
        let Some(scene) = incoming_scene_id(body) else { return; };
        if previous::snapshot().scene_id == scene { return; }
    }
    previous::observe_notify(service, method, body);
}

pub fn snapshot() -> EncounterContextSnapshot {
    let mut snapshot = previous::snapshot();
    apply_season4_metadata(&mut snapshot);
    snapshot
}

fn apply_season4_metadata(snapshot: &mut EncounterContextSnapshot) {
    if snapshot.scene_id == 0 {
        return;
    }

    if snapshot.difficulty.trim().is_empty() {
        snapshot.difficulty = match snapshot.scene_id {
            6591 | 6592 => "Unstable",
            6593 => "Hard",
            6594 => "Master",
            6611 | 6612 => "Unstable",
            6613 => "Hard",
            6614 => "Extreme",
            6615 => "Master",
            1911 | 1912 => "Unstable",
            1931 => "Hard",
            1932 => "Master",
            13031 => "Hard",
            13032 => "Nightmare",
            13033 => "Challenge",
            _ => "",
        }
        .to_string();
    }

    // S3 raid entries in the authoritative table use PlayType 18. Current CN
    // data identifies 13031-13033 as the Season 4 raid variants, so preserve the
    // same generic Season Challenge/Raid classification when the old table has
    // no entry. Dungeon ids remain unset instead of being guessed.
    if snapshot.play_type == 0 && matches!(snapshot.scene_id, 13031..=13033) {
        snapshot.play_type = 18;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn season4_difficulty_fallbacks_cover_known_cn_variants() {
        for (scene, expected) in [
            (6591, "Unstable"),
            (6593, "Hard"),
            (6594, "Master"),
            (6613, "Hard"),
            (6614, "Extreme"),
            (6615, "Master"),
            (1931, "Hard"),
            (1932, "Master"),
            (13031, "Hard"),
            (13032, "Nightmare"),
            (13033, "Challenge"),
        ] {
            let mut snapshot = EncounterContextSnapshot { scene_id: scene, ..Default::default() };
            apply_season4_metadata(&mut snapshot);
            assert_eq!(snapshot.difficulty, expected);
        }
    }

    #[test]
    fn season4_raid_gets_generic_raid_play_type_without_fake_dungeon_id() {
        let mut snapshot = EncounterContextSnapshot { scene_id: 13031, ..Default::default() };
        apply_season4_metadata(&mut snapshot);
        assert_eq!(snapshot.play_type, 18);
        assert_eq!(snapshot.dungeon_id, 0);
    }

    #[test]
    fn known_existing_metadata_is_never_overwritten() {
        let mut snapshot = EncounterContextSnapshot {
            scene_id: 6615,
            difficulty: "Global Label".into(),
            play_type: 99,
            ..Default::default()
        };
        apply_season4_metadata(&mut snapshot);
        assert_eq!(snapshot.difficulty, "Global Label");
        assert_eq!(snapshot.play_type, 99);
    }
}
