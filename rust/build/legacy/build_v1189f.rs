use std::{env,fs,path::{Path,PathBuf}};

mod prior { include!("build_v1189e.rs"); pub fn run(){main();} }

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){let count=source.matches(from).count();assert_eq!(count,1,"v1.18.9f patch {label} expected one match, found {count}");*source=source.replacen(from,to,1);}

fn patch_tracker_party_sync(out:&Path){
    let path=out.join("telemetry_v170_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read v1.18.9e telemetry source").replace("\r\n","\n");

    replace_once(&mut source,
        r###"    fn reset_scene(&mut self) {
        crate::event_tracker::clear_scene();
        self.entities.clear();"###,
        r###"    fn reset_scene(&mut self) {
        crate::event_tracker::clear_scene();
        // Telemetry intentionally preserves the known local identity and party
        // roster across scene transitions. Keep Event Tracker's Self/Party scope
        // in the same state instead of temporarily forgetting both.
        crate::event_tracker::set_local_uid(self.local_uid);
        crate::event_tracker::set_party(&self.team);
        self.entities.clear();"###,
        "preserve tracker identity across scene reset");

    replace_once(&mut source,
        r###"                    self.team.insert(self.local_uid);
                    self.combat.entry(self.local_uid).or_default();"###,
        r###"                    self.team.insert(self.local_uid);
                    crate::event_tracker::set_party(&self.team);
                    self.combat.entry(self.local_uid).or_default();"###,
        "sync party on enter scene");

    replace_once(&mut source,
        r###"            self.team.insert(char_id);
            self.combat.entry(char_id).or_default();"###,
        r###"            self.team.insert(char_id);
            crate::event_tracker::set_party(&self.team);
            self.combat.entry(char_id).or_default();"###,
        "sync party on container identity");

    replace_once(&mut source,
        r###"            self.retain_team_rows();
            self.emit_dps();
            return;
        }
        if method == TEAM_LEAVE {"###,
        r###"            self.retain_team_rows();
            crate::event_tracker::set_party(&self.team);
            self.emit_dps();
            return;
        }
        if method == TEAM_LEAVE {"###,
        "sync party on dissolve");

    replace_once(&mut source,
        r###"            self.retain_team_rows();
            self.emit_dps();
            return;
        }
        if matches!(method, TEAM_INFO | TEAM_MEMBER_INFO | TEAM_JOIN) {"###,
        r###"            self.retain_team_rows();
            crate::event_tracker::set_party(&self.team);
            self.emit_dps();
            return;
        }
        if matches!(method, TEAM_INFO | TEAM_MEMBER_INFO | TEAM_JOIN) {"###,
        "sync party on leave");

    replace_once(&mut source,
        r###"        if matches!(method, TEAM_INFO | TEAM_MEMBER_INFO | TEAM_JOIN) {
            self.scan_team_members(body);
            for uid in self.team.clone() {"###,
        r###"        if matches!(method, TEAM_INFO | TEAM_MEMBER_INFO | TEAM_JOIN) {
            self.scan_team_members(body);
            crate::event_tracker::set_party(&self.team);
            for uid in self.team.clone() {"###,
        "sync party on team update");

    source.push_str(r###"

#[cfg(test)]
mod v1189_repeat_tracker_party_sync_tests {
    use super::*;
    #[test]
    fn scene_reset_keeps_telemetry_party_roster() {
        let (tx,_rx)=std::sync::mpsc::channel();
        let mut runtime=TelemetryRuntime::new(tx);
        runtime.local_uid=42;
        runtime.team.insert(42);
        runtime.team.insert(77);
        runtime.reset_scene();
        assert_eq!(runtime.local_uid,42);
        assert!(runtime.team.contains(&42));
        assert!(runtime.team.contains(&77));
    }
}
"###);

    fs::write(path,source).expect("write v1.18.9f telemetry source");
}

fn main(){
    prior::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_tracker_party_sync(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1189f.rs");
}
