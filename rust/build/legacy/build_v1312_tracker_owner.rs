use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1311_1_grouped_tooltip_fix.rs");
    pub fn run() { main(); }
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read v1.31.2 feature overlay: {e}"))
        .replace("\r\n", "\n");

    let old = "pub unsafe fn update_dps(hwnd:HWND,snapshot:DpsSnapshot){\n    let mut repaint_target=false;";
    let new = "pub unsafe fn update_dps(hwnd:HWND,snapshot:DpsSnapshot){\n    remember_mechanic_owner(&snapshot);\n    let mut repaint_target=false;";
    let count = source.matches(old).count();
    assert_eq!(count, 1, "v1.31.2 local owner capture expected once, found {count}");
    source = source.replacen(old, new, 1);

    source.push_str(r#"
#[cfg(test)]
mod v1312_tracker_owner_tests{
    use super::*;

    #[test]
    fn special_tracker_labels_share_one_row_without_rule_lookup(){
        let tracker=vec![
            crate::event_tracker::TrackerDisplayRow{label:"TINA".into(),last_seen_unix_ms:10,..Default::default()},
            crate::event_tracker::TrackerDisplayRow{label:"BL".into(),last_seen_unix_ms:11,..Default::default()},
        ];
        let rows=compose_mechanic_rows(Vec::new(),tracker,1);
        assert_eq!(rows.len(),1);
        match &rows[0]{MechanicDisplayRow::TrackerGroup(items)=>{assert_eq!(items.len(),2);assert_eq!(items[0].0,"TINA");assert_eq!(items[1].0,"BL");},_=>panic!("TINA and BL must share TrackerGroup"),}
    }

    #[test]
    fn all_six_special_tracker_labels_have_stable_slots(){
        for (index,label) in ["DeterShot","KARTGRIFF","BASIL","TATTA","TINA","BL"].into_iter().enumerate(){
            let(_,order)=special_tracker_label(label).expect("special tracker label");
            assert_eq!(order,index);
        }
    }

    #[test]
    fn mechanic_group_requires_same_tracking_id_and_timer_wave(){
        let now=1_000_i64;
        let make=|key:&str,target:&str,expiry:i64|crate::model::MechanicRow{key:key.into(),label:"Hit order #1".into(),target:Some(target.into()),expires_unix_ms:expiry,priority:3,..Default::default()};
        let same=group_mechanics(vec![make("buff:1:10:829226","ValidStrike",3_000),make("buff:2:11:829226","AlterTI",3_300)],now);
        assert_eq!(same.len(),1);
        assert_eq!(same[0].targets.len(),2);
        let different=group_mechanics(vec![make("buff:1:10:829226","ValidStrike",3_000),make("buff:2:11:829227","AlterTI",3_080)],now);
        assert_eq!(different.len(),2);
    }

    #[test]
    fn local_mechanic_target_is_the_only_owner_identity(){
        let snapshot=DpsSnapshot{rows:vec![DpsRow{name:"ValidStrike".into(),is_local:true,..Default::default()}],..Default::default()};
        remember_mechanic_owner(&snapshot);
        assert!(mechanic_target_is_owner("ValidStrike"));
        assert!(!mechanic_target_is_owner("AlterTI"));
    }
}
"#);

    for required in ["remember_mechanic_owner(&snapshot)", "special_tracker_label", "mechanic_target_is_owner", "rgb(255,235,59)"] {
        assert!(source.contains(required), "v1.31.2 feature contract missing {required}");
    }

    fs::write(path, source).unwrap_or_else(|e| panic!("write v1.31.2 feature overlay: {e}"));
    println!("cargo:rerun-if-changed=build/legacy/build_v1312_tracker_owner.rs");
}
