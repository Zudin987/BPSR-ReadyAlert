use std::{env, fs, path::{Path, PathBuf}};

mod prior {
    include!("build_v1231_text_mechanics.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.24.0 focused-mechanics patch {label} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn patch_telemetry(out: &Path) {
    let path = out.join("telemetry_v170_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .expect("read v1.23.1 focused telemetry")
        .replace("\r\n", "\n");

    // These scene ids are season-specific. The reused English dungeon names must
    // never be used for routing: S1 Tina/Towering and S3 Tina/Towering are separate
    // encounters. S4 names below are diagnostics only unless a rule explicitly
    // opts into the scene. In particular, keep CN's current 6615-only Wasteland
    // mechanic coverage rather than extrapolating the parser to 6613/6614.
    replace_once(
        &mut source,
        "        6513|6514|6515=>\"S3 Cursed Tomb\",\n        1150|1151|1152=>\"S3 Giant Tower\",\n        13021|13022|13023=>\"S3 Raid\",\n        6563|6564|6565=>\"S3 Sea-Ringed Reef\",\n        1631|1632|1633=>\"S3 Tina Mindrealm\",\n        6615=>\"S4 Wasteland Court\",",
        "        6513|6514|6515=>\"S3 Cursed Tomb\",\n        6523|6524|6525=>\"S3 Mechanized Processing Facility\",\n        6543|6544|6545=>\"S3 Mistveil Hunting Ground\",\n        1150|1151|1152=>\"S3 Giant Tower\",\n        13021|13022|13023=>\"S3 Raid\",\n        6563|6564|6565=>\"S3 Sea-Ringed Reef\",\n        1631|1632|1633=>\"S3 Tina Mindrealm\",\n        6593|6594=>\"S4 Judgment in the Mirror\",\n        6613|6614|6615=>\"S4 Wasteland Court\",",
        "season-specific scene diagnostics",
    );

    // Preserve every CN-derived rule exactly as v1.23.1 has it. Add only two
    // high-signal gaps that remain useful as text without a minimap:
    //   33501 = Steam Vent (Mech Facility)
    //   3380104 = Hidden/Hunting transition (Mistveil)
    // The short lifetime is an alert display window, not a fabricated mechanic
    // deadline. Priority 3 keeps these action calls ahead of informational rows.
    replace_once(
        &mut source,
        r#"fn text_monster_rule_for_scene(scene_id:i32,id:i32)->Option<(&'static str,u64,u8)>{
    if map_only_monster_mechanic(scene_id,id){None}else{monster_rule_for_scene(scene_id,id)}
}
fn text_skill_rule_for_scene(scene_id:i32,id:i32)->Option<(&'static str,u64,u8)>{
    if map_only_skill_mechanic(scene_id,id){None}else{skill_rule_for_scene(scene_id,id)}
}
fn text_buff_rule_for_scene(scene_id:i32,id:i32)->Option<(&'static str,u64,u8)>{
    if map_only_buff_mechanic(scene_id,id){None}else{buff_rule_for_scene(scene_id,id)}
}"#,
        r#"fn text_monster_rule_for_scene(scene_id:i32,id:i32)->Option<(&'static str,u64,u8)>{
    if map_only_monster_mechanic(scene_id,id){None}else{monster_rule_for_scene(scene_id,id)}
}
fn text_skill_rule_for_scene(scene_id:i32,id:i32)->Option<(&'static str,u64,u8)>{
    if map_only_skill_mechanic(scene_id,id){None}else{skill_rule_for_scene(scene_id,id)}
}
fn text_buff_rule_for_scene(scene_id:i32,id:i32)->Option<(&'static str,u64,u8)>{
    if map_only_buff_mechanic(scene_id,id){None}else{buff_rule_for_scene(scene_id,id)}
}

fn focused_monster_rule_for_scene(scene_id:i32,id:i32)->Option<(&'static str,u64,u8)>{
    if matches!(scene_id,6523|6524|6525)&&id==33501{
        return Some(("STEAM VENTS - BLOCK NOW",5_000,3));
    }
    text_monster_rule_for_scene(scene_id,id)
}
fn focused_skill_rule_for_scene(scene_id:i32,id:i32)->Option<(&'static str,u64,u8)>{
    if matches!(scene_id,6543|6544|6545)&&id==3380104{
        return Some(("HUNT - USE CHARGED TREE",5_000,3));
    }
    text_skill_rule_for_scene(scene_id,id)
}"#,
        "focused non-spatial dungeon rules",
    );

    replace_once(
        &mut source,
        "if let Some((label, duration, priority)) = text_monster_rule_for_scene(self.current_scene_id, monster_id) {",
        "if let Some((label, duration, priority)) = focused_monster_rule_for_scene(self.current_scene_id, monster_id) {",
        "route focused monster alerts",
    );
    replace_once(
        &mut source,
        "if let Some((label, duration, priority)) = text_skill_rule_for_scene(self.current_scene_id, skill) {",
        "if let Some((label, duration, priority)) = focused_skill_rule_for_scene(self.current_scene_id, skill) {",
        "route focused skill alerts",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1240_focused_mechanics_tests {
    use super::*;

    #[test]
    fn reused_s1_names_cannot_trigger_s3_tina_or_towering_rules(){
        for scene in [1001,1002,1011,1021,1031,1032,1033]{
            assert!(text_buff_rule_for_scene(scene,510571).is_none());
            assert!(text_buff_rule_for_scene(scene,841519).is_none());
            assert!(text_buff_rule_for_scene(scene,841509).is_none());
            assert!(text_monster_rule_for_scene(scene,300086).is_none());
            assert!(text_monster_rule_for_scene(scene,300089).is_none());
        }
        for scene in [1101,1102,1121,1122,1123]{
            assert!(text_buff_rule_for_scene(scene,821076).is_none());
            assert!(text_monster_rule_for_scene(scene,2106).is_none());
            assert!(text_skill_rule_for_scene(scene,111103).is_none());
        }
    }

    #[test]
    fn focused_gap_alerts_are_strictly_scene_gated(){
        assert_eq!(focused_monster_rule_for_scene(6523,33501),Some(("STEAM VENTS - BLOCK NOW",5_000,3)));
        assert_eq!(focused_monster_rule_for_scene(6524,33501),Some(("STEAM VENTS - BLOCK NOW",5_000,3)));
        assert_eq!(focused_monster_rule_for_scene(6525,33501),Some(("STEAM VENTS - BLOCK NOW",5_000,3)));
        assert!(focused_monster_rule_for_scene(6513,33501).is_none());
        assert!(focused_monster_rule_for_scene(1001,33501).is_none());

        assert_eq!(focused_skill_rule_for_scene(6543,3380104),Some(("HUNT - USE CHARGED TREE",5_000,3)));
        assert_eq!(focused_skill_rule_for_scene(6544,3380104),Some(("HUNT - USE CHARGED TREE",5_000,3)));
        assert_eq!(focused_skill_rule_for_scene(6545,3380104),Some(("HUNT - USE CHARGED TREE",5_000,3)));
        assert!(focused_skill_rule_for_scene(1631,3380104).is_none());
        assert!(focused_skill_rule_for_scene(6615,3380104).is_none());
    }

    #[test]
    fn s4_scene_names_do_not_expand_cn_mechanic_scope(){
        assert_eq!(mechanic_scene_name(6593),Some("S4 Judgment in the Mirror"));
        assert_eq!(mechanic_scene_name(6594),Some("S4 Judgment in the Mirror"));
        assert_eq!(mechanic_scene_name(6613),Some("S4 Wasteland Court"));
        assert_eq!(mechanic_scene_name(6614),Some("S4 Wasteland Court"));
        assert_eq!(mechanic_scene_name(6615),Some("S4 Wasteland Court"));
        // CN's current Wasteland mechanic parser is 6615-only. Do not guess that
        // the same ids are valid for Hard/Chaotic scenes.
        assert!(!scene_tracks_buff(6613,884659));
        assert!(!scene_tracks_buff(6614,884659));
        assert!(scene_tracks_buff(6615,884659));
    }
}
"#);

    fs::write(path, source).expect("write v1.24 focused telemetry");
}

fn patch_feature_overlay(out: &Path) {
    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .expect("read v1.23.1 feature overlays")
        .replace("\r\n", "\n");

    replace_once(
        &mut source,
        "const MECH_ROW_H: i32 = 36;",
        "const MECH_ROW_H: i32 = 36;\nconst MAX_MECHANIC_ROWS_VISIBLE: usize = 6;",
        "compact mechanic row budget",
    );

    // If a new priority-3 mechanic appears while the player has scrolled down,
    // jump back to the focused top rather than letting the important alert remain
    // below the fold. Normal row churn does not steal the player's scroll.
    replace_once(
        &mut source,
        "pub unsafe fn update_mechanics(hwnd:HWND,snapshot:MechanicSnapshot){with_state(hwnd,|state|state.mechanics=snapshot);}",
        "pub unsafe fn update_mechanics(hwnd:HWND,snapshot:MechanicSnapshot){with_state(hwnd,|state|{let new_urgent=snapshot.rows.iter().any(|next|next.priority>=3&&!state.mechanics.rows.iter().any(|old|old.priority>=3&&old.key==next.key));state.mechanics=snapshot;if new_urgent{state.scroll=0;}});InvalidateRect(hwnd,null(),0);}",
        "urgent mechanic returns viewport to top",
    );

    // Keep all active rows available for scrolling, but rank the mechanic rows by
    // importance while preserving the tracker rows that v1.23.1 renders first.
    // A hard six-row first-screen budget prevents a large raid state dump from
    // drowning the mechanic the player needs to act on now.
    replace_once(
        &mut source,
        "let now=now_ms();let tracker=crate::event_tracker::rows();let active:Vec<_>=state.mechanics.rows.iter().filter(|row|row.persistent||row.expires_unix_ms<=0||row.expires_unix_ms>now).collect();let total=tracker.len().saturating_add(active.len());let visible=((rc.bottom-top)/MECH_ROW_H).max(0)as usize;",
        "let now=now_ms();let tracker=crate::event_tracker::rows();let mut active:Vec<_>=state.mechanics.rows.iter().filter(|row|row.persistent||row.expires_unix_ms<=0||row.expires_unix_ms>now).collect();active.sort_by(|a,b|{let a_timed=!a.persistent&&a.expires_unix_ms>now;let b_timed=!b.persistent&&b.expires_unix_ms>now;let ae=if a_timed{a.expires_unix_ms}else{i64::MAX};let be=if b_timed{b.expires_unix_ms}else{i64::MAX};b.priority.cmp(&a.priority).then_with(||b_timed.cmp(&a_timed)).then_with(||ae.cmp(&be)).then_with(||b.created_unix_ms.cmp(&a.created_unix_ms)).then_with(||a.key.cmp(&b.key))});let total=tracker.len().saturating_add(active.len());let visible=(((rc.bottom-top)/MECH_ROW_H).max(0)as usize).min(MAX_MECHANIC_ROWS_VISIBLE);",
        "priority and urgency mechanic ordering",
    );

    replace_once(
        &mut source,
        "let visible=((bottom-top)/MECH_ROW_H).max(0)as usize;",
        "let visible=(((bottom-top)/MECH_ROW_H).max(0)as usize).min(MAX_MECHANIC_ROWS_VISIBLE);",
        "scroll clamp follows compact mechanic budget",
    );

    fs::write(path, source).expect("write v1.24 focused feature overlay");
}

fn main() {
    prior::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_telemetry(&out);
    patch_feature_overlay(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1240_focused_mechanics.rs");
}
