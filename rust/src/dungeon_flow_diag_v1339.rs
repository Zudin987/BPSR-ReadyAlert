// Diagnostic-only dungeon flow decoding for release validation.
//
// This module MUST NOT reset, archive, or otherwise mutate encounter state. It
// exists only to record sanitized flow-state observations so real packet
// sequences can be validated before any automatic encounter-finalization logic
// is reconsidered.

fn dungeon_flow_state_name(state: i32) -> &'static str {
    match state {
        0 => "None",
        1 => "Active",
        2 => "Ready",
        3 => "Playing",
        4 => "End",
        5 => "Settlement",
        6 => "Vote",
        _ => "Unknown",
    }
}

fn dungeon_full_flow_state(body: &[u8]) -> Option<(i64, i32)> {
    let data = proto::get_len_field(body, 1)?;
    let scene_uuid = i64::try_from(proto::get_varint_field(data, 1)?).ok()?;
    if scene_uuid == 0 { return None; }
    let flow = proto::get_len_field(data, 2)?;
    let state = i32::try_from(proto::get_varint_field(flow, 1)?).ok()?;
    (0..=6).contains(&state).then_some((scene_uuid, state))
}

fn dungeon_dirty_i32(blob: &[u8], pos: &mut usize) -> Option<i32> {
    let end = (*pos).checked_add(4)?;
    let value = i32::from_le_bytes(blob.get(*pos..end)?.try_into().ok()?);
    *pos = end;
    Some(value)
}

fn dungeon_dirty_i64(blob: &[u8], pos: &mut usize) -> Option<i64> {
    let end = (*pos).checked_add(8)?;
    let value = i64::from_le_bytes(blob.get(*pos..end)?.try_into().ok()?);
    *pos = end;
    Some(value)
}

fn dungeon_dirty_container_header(blob: &[u8], pos: &mut usize) -> Option<Option<usize>> {
    if dungeon_dirty_i32(blob, pos)? != -2 { return None; }
    let length = dungeon_dirty_i32(blob, pos)?;
    if length == -3 { return Some(None); }
    let length = usize::try_from(length).ok()?;
    let end = (*pos).checked_add(length)?;
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
        if !(1..=8).contains(&field) { return None; }
        let value = dungeon_dirty_i32(blob, pos)?;
        if field == 1 { state = (0..=6).contains(&value).then_some(value); }
    }
    dungeon_dirty_finish(blob, pos, end)?;
    Some(state)
}

fn dungeon_dirty_flow_state(body: &[u8]) -> Option<(Option<i64>, i32)> {
    // SyncDungeonDirtyData { BufferStream v_data = 1 } -> { bytes buffer = 1 }.
    let stream = proto::get_len_field(body, 1)?;
    let blob = proto::get_len_field(stream, 1)?;
    if blob.len() > 1024 * 1024 { return None; }

    let mut pos = 0;
    let end = dungeon_dirty_container_header(blob, &mut pos)??;
    let mut scene_uuid = None;
    let mut state = None;

    while pos < end {
        let field = dungeon_dirty_i32(blob, &mut pos)?;
        match field {
            1 => {
                let value = dungeon_dirty_i64(blob, &mut pos)?;
                if value != 0 { scene_uuid = Some(value); }
            }
            2 => state = dungeon_dirty_read_flow(blob, &mut pos)?,
            3..=26 => dungeon_dirty_skip_container(blob, &mut pos)?,
            27 => { dungeon_dirty_i32(blob, &mut pos)?; }
            _ => return None,
        }
        if pos > end { return None; }
    }

    dungeon_dirty_finish(blob, &mut pos, end)?;
    if pos != blob.len() { return None; }
    state.map(|flow| (scene_uuid, flow))
}

#[cfg(test)]
mod dungeon_flow_diag_v1339_tests {
    use super::*;

    fn protobuf_field_one(bytes: &[u8]) -> Vec<u8> {
        assert!(bytes.len() < 128, "synthetic fixture must fit one-byte length");
        let mut out = vec![0x0a, bytes.len() as u8];
        out.extend_from_slice(bytes);
        out
    }

    fn full_state(scene_uuid: u8, state: u8) -> Vec<u8> {
        protobuf_field_one(&[0x08, scene_uuid, 0x12, 0x02, 0x08, state])
    }

    fn container(entries: &[u8]) -> Vec<u8> {
        let mut out = (-2i32).to_le_bytes().to_vec();
        out.extend_from_slice(&(entries.len() as i32).to_le_bytes());
        out.extend_from_slice(entries);
        out.extend_from_slice(&(-3i32).to_le_bytes());
        out
    }

    fn dirty_state(scene_uuid: Option<i64>, state: i32) -> Vec<u8> {
        let mut flow_entry = 1i32.to_le_bytes().to_vec();
        flow_entry.extend_from_slice(&state.to_le_bytes());
        let flow = container(&flow_entry);

        let mut entries = Vec::new();
        if let Some(scene_uuid) = scene_uuid {
            entries.extend_from_slice(&1i32.to_le_bytes());
            entries.extend_from_slice(&scene_uuid.to_le_bytes());
        }
        entries.extend_from_slice(&2i32.to_le_bytes());
        entries.extend_from_slice(&flow);

        let blob = container(&entries);
        protobuf_field_one(&protobuf_field_one(&blob))
    }

    #[test]
    fn diagnostic_full_sync_requires_nonzero_scene_uuid() {
        assert_eq!(dungeon_full_flow_state(&full_state(42, 3)), Some((42, 3)));
        assert_eq!(dungeon_full_flow_state(&full_state(0, 3)), None);
        assert_eq!(dungeon_full_flow_state(&full_state(42, 7)), None);
    }

    #[test]
    fn diagnostic_dirty_sync_accepts_optional_scene_uuid() {
        assert_eq!(dungeon_dirty_flow_state(&dirty_state(Some(42), 4)), Some((Some(42), 4)));
        assert_eq!(dungeon_dirty_flow_state(&dirty_state(None, 5)), Some((None, 5)));
        assert_eq!(dungeon_dirty_flow_state(&dirty_state(Some(42), 7)), None);
    }

    #[test]
    fn malformed_dirty_sync_fails_closed() {
        let mut packet = dirty_state(Some(42), 4);
        packet.pop();
        assert_eq!(dungeon_dirty_flow_state(&packet), None);
    }

    #[test]
    fn state_labels_are_stable_for_diagnostics() {
        assert_eq!(dungeon_flow_state_name(3), "Playing");
        assert_eq!(dungeon_flow_state_name(4), "End");
        assert_eq!(dungeon_flow_state_name(5), "Settlement");
        assert_eq!(dungeon_flow_state_name(99), "Unknown");
    }
}
