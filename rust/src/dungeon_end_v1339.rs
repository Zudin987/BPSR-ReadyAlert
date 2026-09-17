// WorldNtf dungeon protocol reference:
// StellarProtocol/StellarResonanceModSystem, WorldNtfMethodIds.cs,
// DungeonSyncReader.cs and DungeonDirtyDataReader.cs (2026-09-17).
// Method 23 is a protobuf full sync, including deliveries in town; method 24
// wraps a LITTLE-ENDIAN container-merge dirty delta, NOT protobuf flow_info.
// No game capture is stored in this repository; tests below cover structural
// fixtures and the state transition, not a live game session.

fn dungeon_full_flow_state(body: &[u8]) -> Option<i32> {
    let data = proto::get_len_field(body, 1)?;
    // Full dungeon syncs must carry a nonzero run scene UUID. A flow-like
    // submessage alone is insufficient evidence of an actual dungeon packet.
    if proto::get_varint_field(data, 1)? == 0 { return None; }
    let flow = proto::get_len_field(data, 2)?;
    let state = i32::try_from(proto::get_varint_field(flow, 1)?).ok()?;
    (0..=6).contains(&state).then_some(state)
}

fn dungeon_dirty_i32(blob: &[u8], pos: &mut usize) -> Option<i32> {
    let end = (*pos).checked_add(4)?;
    let value = i32::from_le_bytes(blob.get(*pos..end)?.try_into().ok()?);
    *pos = end;
    Some(value)
}

fn dungeon_dirty_skip(blob: &[u8], pos: &mut usize, length: usize) -> Option<()> {
    let end = (*pos).checked_add(length)?;
    blob.get(*pos..end)?;
    *pos = end;
    Some(())
}

fn dungeon_dirty_container_header(blob: &[u8], pos: &mut usize) -> Option<Option<usize>> {
    if dungeon_dirty_i32(blob, pos)? != -2 { return None; }
    let length = dungeon_dirty_i32(blob, pos)?;
    if length == -3 { return Some(None); } // An empty nested container.
    let length = usize::try_from(length).ok()?;
    let end = (*pos).checked_add(length)?;
    // End tag is outside the declared entry size.
    blob.get(end..end.checked_add(4)?)?;
    Some(Some(end))
}

fn dungeon_dirty_finish(blob: &[u8], pos: &mut usize, entries_end: usize) -> Option<()> {
    if *pos != entries_end || dungeon_dirty_i32(blob, pos)? != -3 { return None; }
    Some(())
}

fn dungeon_dirty_skip_container(blob: &[u8], pos: &mut usize) -> Option<()> {
    let Some(end) = dungeon_dirty_container_header(blob, pos)? else { return Some(()); };
    *pos = end;
    dungeon_dirty_finish(blob, pos, end)
}

fn dungeon_dirty_read_flow(blob: &[u8], pos: &mut usize) -> Option<Option<i32>> {
    let Some(end) = dungeon_dirty_container_header(blob, pos)? else { return Some(None); };
    let mut state = None;
    while *pos < end {
        let field = dungeon_dirty_i32(blob, pos)?;
        // DungeonFlowInfo has eight int32 scalar fields; an unexpected field
        // layout invalidates this delta rather than guessing its byte length.
        if !(1..=8).contains(&field) { return None; }
        let value = dungeon_dirty_i32(blob, pos)?;
        if field == 1 { state = (0..=6).contains(&value).then_some(value); }
    }
    dungeon_dirty_finish(blob, pos, end)?;
    Some(state)
}

fn dungeon_dirty_flow_state(body: &[u8]) -> Option<i32> {
    // SyncDungeonDirtyData { BufferStream v_data = 1 } -> { bytes buffer = 1 }.
    let stream = proto::get_len_field(body, 1)?;
    let blob = proto::get_len_field(stream, 1)?;
    if blob.len() > 1024 * 1024 { return None; }
    let mut pos = 0;
    let end = dungeon_dirty_container_header(blob, &mut pos)??;
    let mut state = None;
    while pos < end {
        let field = dungeon_dirty_i32(blob, &mut pos)?;
        match field {
            1 => dungeon_dirty_skip(blob, &mut pos, 8)?, // scene_uuid: int64
            2 => state = dungeon_dirty_read_flow(blob, &mut pos)?,
            3..=26 => dungeon_dirty_skip_container(blob, &mut pos)?,
            27 => dungeon_dirty_skip(blob, &mut pos, 4)?, // err_code: int32
            _ => return None, // unknown future field: fail closed
        }
        if pos > end { return None; }
    }
    dungeon_dirty_finish(blob, &mut pos, end)?;
    (pos == blob.len()).then_some(state).flatten()
}

#[cfg(test)]
mod dungeon_end_v1339_tests {
    use super::*;
    use std::sync::mpsc;

    fn protobuf_field_one(bytes: &[u8]) -> Vec<u8> {
        assert!(bytes.len() < 128, "synthetic fixture must fit one-byte length");
        let mut out = vec![0x0a, bytes.len() as u8];
        out.extend_from_slice(bytes);
        out
    }

    fn full_state(state: u8) -> Vec<u8> {
        // DungeonSyncData.scene_uuid(1)=42, flow_info(2).state(1)=state.
        protobuf_field_one(&[0x08, 42, 0x12, 0x02, 0x08, state])
    }

    fn container(entries: &[u8]) -> Vec<u8> {
        let mut out = (-2i32).to_le_bytes().to_vec();
        out.extend_from_slice(&(entries.len() as i32).to_le_bytes());
        out.extend_from_slice(entries);
        out.extend_from_slice(&(-3i32).to_le_bytes());
        out
    }

    fn dirty_state(state: i32) -> Vec<u8> {
        let mut field = 1i32.to_le_bytes().to_vec();
        field.extend_from_slice(&state.to_le_bytes());
        let flow = container(&field);
        let mut entries = 2i32.to_le_bytes().to_vec();
        entries.extend_from_slice(&flow);
        let blob = container(&entries);
        protobuf_field_one(&protobuf_field_one(&blob))
    }

    fn active() -> (TelemetryRuntime, mpsc::Receiver<AppEvent>) {
        let (tx, rx) = mpsc::channel();
        let mut runtime = TelemetryRuntime::new(tx);
        runtime.current_scene_id = 13023;
        runtime.local_uid = 42;
        // History rejects a snapshot with encounter_ms == 0. Give the
        // synthetic combat a deterministic elapsed duration, rather than
        // relying on scheduler timing between creation and finalization.
        runtime.encounter_started = Some(Instant::now() - std::time::Duration::from_secs(5));
        runtime.combat.entry(42).or_default().damage = 1234;
        (runtime, rx)
    }

    #[test]
    fn confirmed_protocol_parses_full_and_le_dirty_not_as_protobuf() {
        for state in 0..=6 {
            assert_eq!(dungeon_full_flow_state(&full_state(state as u8)), Some(state));
            assert_eq!(dungeon_dirty_flow_state(&dirty_state(state)), Some(state));
        }
        assert_eq!(dungeon_full_flow_state(&full_state(255)), None);
        assert_eq!(dungeon_dirty_flow_state(&dirty_state(255)), None);
        // A full-sync envelope must actually contain a nonzero scene UUID;
        // a stand-alone state field must never count as an authoritative run.
        assert_eq!(dungeon_full_flow_state(&protobuf_field_one(&[0x12, 0x02, 0x08, 4])), None);
        assert_eq!(dungeon_full_flow_state(&protobuf_field_one(&[0x08, 0, 0x12, 0x02, 0x08, 4])), None);
    }

    #[test]
    fn unrelated_world_messages_and_bad_dirty_data_cannot_reset() {
        let (mut runtime, rx) = active();
        runtime.handle_notify(proto::WORLD_SERVICE, SYNC_DUNGEON_DATA, &full_state(3));
        let mut truncated = dirty_state(4);
        truncated.pop();
        runtime.handle_notify(proto::WORLD_SERVICE, SYNC_DUNGEON_DIRTY, &truncated);
        runtime.handle_notify(proto::TEAM_SERVICE, SYNC_DUNGEON_DIRTY, &dirty_state(4));
        runtime.handle_notify(proto::WORLD_SERVICE, SYNC_DUNGEON_DIRTY, &protobuf_field_one(&protobuf_field_one(&container(&[]))));
        assert_eq!(runtime.combat.get(&42).map(|row| row.damage), Some(1234));
        assert!(runtime.encounter_started.is_some());
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn dungeon_dirty_end_archives_once_without_waiting_for_new_hit() {
        let (mut runtime, rx) = active();
        runtime.emit_dps();
        let before = match rx.try_recv().unwrap() { AppEvent::Dps(snapshot) => snapshot, _ => panic!("Dps expected") };
        assert!(before.encounter_ms > 0, "synthetic combat must have positive duration for history");
        runtime.handle_notify(proto::WORLD_SERVICE, SYNC_DUNGEON_DATA, &full_state(3));
        assert!(rx.try_recv().is_err(), "Playing must not end encounter");
        runtime.handle_notify(proto::WORLD_SERVICE, SYNC_DUNGEON_DIRTY, &dirty_state(4));
        let final_snapshot = match rx.try_recv().unwrap() { AppEvent::Dps(snapshot) => snapshot, _ => panic!("final Dps expected") };
        let after = match rx.try_recv().unwrap() { AppEvent::Dps(snapshot) => snapshot, _ => panic!("empty Dps expected") };
        assert_eq!(final_snapshot.total_damage, before.total_damage);
        assert_eq!(after.total_damage, 0);
        assert!(crate::history::encounter_rolled(&final_snapshot, &after), "UI should archive the final snapshot");
        assert!(runtime.encounter_started.is_none());
        runtime.handle_notify(proto::WORLD_SERVICE, SYNC_DUNGEON_DIRTY, &dirty_state(4));
        runtime.handle_notify(proto::WORLD_SERVICE, SYNC_DUNGEON_DATA, &full_state(5));
        assert!(rx.try_recv().is_err(), "replayed end/settlement cannot duplicate history");
    }

    #[test]
    fn town_end_without_playing_and_intermediate_states_are_safe() {
        let (mut runtime, rx) = active();
        runtime.handle_notify(proto::WORLD_SERVICE, SYNC_DUNGEON_DATA, &full_state(4));
        runtime.handle_notify(proto::WORLD_SERVICE, SYNC_DUNGEON_DIRTY, &dirty_state(5));
        assert!(runtime.encounter_started.is_some());
        runtime.handle_notify(proto::WORLD_SERVICE, SYNC_DUNGEON_DIRTY, &dirty_state(3));
        runtime.handle_notify(proto::WORLD_SERVICE, SYNC_DUNGEON_DATA, &full_state(2));
        runtime.handle_notify(proto::WORLD_SERVICE, SYNC_DUNGEON_DIRTY, &dirty_state(4));
        assert_eq!(runtime.combat.get(&42).map(|row| row.damage), Some(1234));
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn full_sync_end_and_settlement_fallback_both_finalize() {
        for final_state in [4u8, 5u8] {
            let (mut runtime, rx) = active();
            runtime.handle_notify(proto::WORLD_SERVICE, SYNC_DUNGEON_DIRTY, &dirty_state(3));
            runtime.handle_notify(proto::WORLD_SERVICE, SYNC_DUNGEON_DATA, &full_state(final_state));
            assert_eq!(runtime.combat.get(&42).map(|row| row.damage), Some(0));
            assert!(matches!(rx.try_recv(), Ok(AppEvent::Dps(snapshot)) if snapshot.total_damage == 1234));
            assert!(matches!(rx.try_recv(), Ok(AppEvent::Dps(snapshot)) if snapshot.total_damage == 0));
            assert!(rx.try_recv().is_err());
        }
    }
}
