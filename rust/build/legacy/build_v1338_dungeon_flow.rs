use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1338_encounter_boundary.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.33.8 {label}: expected one generated-code match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let path = out.join("telemetry_v170_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .expect("read generated telemetry for explicit dungeon-end detection")
        .replace("\r\n", "\n");

    // WorldNtf SyncDungeonData (0x17): v_data(1) -> flow_info(2) -> state(1).
    // DungeonStateEnd = 4. Unlike death or target disappearance, the game's
    // dungeon state explicitly marks the run over. Do not wait for another hit
    // and do not mistake the intermediate Playing(3) for an encounter end.
    replace_once(&mut source,
        "const SYNC_CONTAINER_DATA: u32 = 0x15;\nconst SYNC_NEAR_DELTA_INFO: u32 = 0x2d;",
        "const SYNC_CONTAINER_DATA: u32 = 0x15;\nconst SYNC_DUNGEON_DATA: u32 = 0x17;\nconst SYNC_NEAR_DELTA_INFO: u32 = 0x2d;",
        "dungeon method constant");
    replace_once(&mut source,
        "            SYNC_CONTAINER_DATA => self.handle_container_snapshot(body),\n            SYNC_NEAR_DELTA_INFO => {",
        "            SYNC_CONTAINER_DATA => self.handle_container_snapshot(body),\n            SYNC_DUNGEON_DATA => self.handle_dungeon_flow(body),\n            SYNC_NEAR_DELTA_INFO => {",
        "dungeon dispatcher");
    replace_once(&mut source,
        "    /// CN protocol: SyncContainerData.v_data = field 1 (CharSerialize),",
        r#"    /// SyncDungeonData.v_data(1).flow_info(2).state(1) is an explicit
    /// encounter boundary. Ending emits a zeroed DPS snapshot, so the existing
    /// encounter-history wrapper archives the previous nonempty snapshot once
    /// immediately, even if no further combat packet arrives.
    fn handle_dungeon_flow(&mut self, body: &[u8]) {
        let Some(data) = proto::get_len_field(body, 1) else { return; };
        let Some(flow) = proto::get_len_field(data, 2) else { return; };
        let Some(state) = proto::get_varint_field(flow, 1) else { return; };
        if state == 4 && self.encounter_started.is_some() {
            crate::logging::write("encounter-boundary: dungeon flow ended; finalizing active encounter");
            self.reset_encounter_keep_roster();
            self.emit_dps();
        }
    }

    /// CN protocol: SyncContainerData.v_data = field 1 (CharSerialize),"#,
        "explicit dungeon-end handler");

    source.push_str(r#"

#[cfg(test)]
mod v1338_dungeon_flow_tests {
    use super::*;
    use std::sync::mpsc;

    // SyncDungeonData.v_data(1).flow_info(2).state(1) = DungeonStateEnd(4).
    const END_PACKET: &[u8] = &[0x0a, 0x04, 0x12, 0x02, 0x08, 0x04];
    const PLAYING_PACKET: &[u8] = &[0x0a, 0x04, 0x12, 0x02, 0x08, 0x03];

    #[test]
    fn dungeon_end_finalizes_without_a_followup_attack() {
        let (tx, rx) = mpsc::channel();
        let mut runtime = TelemetryRuntime::new(tx);
        runtime.local_uid = 42;
        runtime.encounter_started = Some(Instant::now());
        runtime.combat.entry(42).or_default().damage = 1234;
        runtime.handle_notify(proto::WORLD_SERVICE, SYNC_DUNGEON_DATA, END_PACKET);
        assert!(runtime.encounter_started.is_none());
        assert_eq!(runtime.combat.get(&42).map(|actor| actor.damage), Some(0));
        let event = rx.try_recv().expect("dungeon end must emit immediately");
        assert!(matches!(event, AppEvent::Dps(snapshot) if snapshot.total_damage == 0));
        assert!(rx.try_recv().is_err(), "duplicate end must not be emitted");
        runtime.handle_notify(proto::WORLD_SERVICE, SYNC_DUNGEON_DATA, END_PACKET);
        assert!(rx.try_recv().is_err(), "replayed end cannot reset twice");
    }

    #[test]
    fn dungeon_playing_and_malformed_packets_do_not_split_raid_phases() {
        let (tx, rx) = mpsc::channel();
        let mut runtime = TelemetryRuntime::new(tx);
        runtime.local_uid = 42;
        runtime.encounter_started = Some(Instant::now());
        runtime.combat.entry(42).or_default().damage = 1234;
        runtime.handle_notify(proto::WORLD_SERVICE, SYNC_DUNGEON_DATA, PLAYING_PACKET);
        runtime.handle_notify(proto::WORLD_SERVICE, SYNC_DUNGEON_DATA, &[0x0a, 0xff]);
        assert!(runtime.encounter_started.is_some());
        assert_eq!(runtime.combat.get(&42).map(|actor| actor.damage), Some(1234));
        assert!(rx.try_recv().is_err());
    }
}
"#);
    fs::write(path, source).expect("write v1.33.8 explicit dungeon-end telemetry");
    println!("cargo:rerun-if-changed=build/legacy/build_v1338_dungeon_flow.rs");
}
