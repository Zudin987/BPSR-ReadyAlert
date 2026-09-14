use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1314_audit_hardening.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.31.5 patch {label} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn patch_telemetry(out: &PathBuf) {
    let path = out.join("telemetry_v170_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read v1.31.5 telemetry: {e}"))
        .replace("\r\n", "\n");

    let old = r#"        if method == TEAM_LEAVE {
            let before = self.team.clone();
            self.scan_team_members(body);
            if self.team == before {
                self.team.retain(|uid| *uid == self.local_uid);
            }
            crate::event_tracker::set_party(&self.team);
            self.emit_dps();
            return;
        }"#;
    let new = r#"        if method == TEAM_LEAVE {
            // GrpcTeamNtf.NotifyLeaveTeam wraps NotifyLeaveTeamRequest in field 1;
            // NotifyLeaveTeamRequest.charId is field 1. A leave notification names
            // exactly the character that left, not a replacement roster snapshot.
            // The old fallback rescanned this packet as member data and, when that
            // found nothing, collapsed the entire party to the local player.
            let leaving_uid = team_leave_uid(body);
            if leaving_uid > 0 {
                if leaving_uid == self.local_uid {
                    self.team.clear();
                    if self.local_uid > 0 { self.team.insert(self.local_uid); }
                } else {
                    self.team.remove(&leaving_uid);
                }
            }
            crate::event_tracker::set_party(&self.team);
            self.emit_dps();
            return;
        }"#;
    replace_once(&mut source, old, new, "single-member team leave handling");

    let marker = "fn proto_string(data: &[u8], field: u32) -> Option<String> {";
    let helper = r#"fn team_leave_uid(data:&[u8])->i64{
    proto::get_len_field(data,1)
        .and_then(|request|proto::get_varint_field(request,1))
        .map(signed)
        .filter(|uid|*uid>0)
        .unwrap_or(0)
}

fn proto_string(data: &[u8], field: u32) -> Option<String> {"#;
    replace_once(&mut source, marker, helper, "team leave protobuf parser");

    source.push_str(r#"
#[cfg(test)]
mod v1315_team_leave_tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn leave_packet_extracts_only_departing_char_id() {
        // outer field 1 (VRequest), inner field 1 (charId = 99)
        assert_eq!(team_leave_uid(&[0x0a, 0x02, 0x08, 0x63]), 99);
        assert_eq!(team_leave_uid(&[]), 0);
    }

    #[test]
    fn one_remote_member_leaving_does_not_delete_the_rest_of_party() {
        let (tx, _rx) = mpsc::channel();
        let mut runtime = TelemetryRuntime::new(tx);
        runtime.local_uid = 7;
        runtime.team.extend([7, 8, 9, 10]);
        runtime.handle_team(TEAM_LEAVE, &[0x0a, 0x02, 0x08, 0x09]);
        assert!(runtime.team.contains(&7));
        assert!(runtime.team.contains(&8));
        assert!(!runtime.team.contains(&9));
        assert!(runtime.team.contains(&10));
        assert_eq!(runtime.team.len(), 3);
    }

    #[test]
    fn local_player_leaving_collapses_party_to_self_only() {
        let (tx, _rx) = mpsc::channel();
        let mut runtime = TelemetryRuntime::new(tx);
        runtime.local_uid = 7;
        runtime.team.extend([7, 8, 9, 10]);
        runtime.handle_team(TEAM_LEAVE, &[0x0a, 0x02, 0x08, 0x07]);
        assert_eq!(runtime.team.len(), 1);
        assert!(runtime.team.contains(&7));
    }
}
"#);

    for required in [
        "let leaving_uid = team_leave_uid(body);",
        "self.team.remove(&leaving_uid);",
        "fn team_leave_uid(data:&[u8])->i64",
    ] {
        assert!(source.contains(required), "v1.31.5 team leave contract missing {required}");
    }

    fs::write(path, source).unwrap_or_else(|e| panic!("write v1.31.5 telemetry: {e}"));
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_telemetry(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1315_audit_hardening.rs");
}
