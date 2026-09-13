use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1312_tracker_owner.rs");
    pub fn run() { main(); }
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read v1.31.3 feature overlay: {e}"))
        .replace("\r\n", "\n");

    source.push_str(r#"
#[cfg(test)]
mod v1313_single_buff_row_tests{
    use super::*;

    fn tracked(label:&str,id:i32,expiry:i64,created:i64)->crate::model::MechanicRow{
        crate::model::MechanicRow{
            key:format!("trackedbuff:655370:{id}"),label:label.into(),target:None,
            created_unix_ms:created,expires_unix_ms:expiry,persistent:false,priority:250,
        }
    }

    #[test]
    fn builtin_tracked_buffs_share_one_physical_row(){
        let now=1_000_i64;
        let rows=compose_mechanic_rows(vec![
            tracked("BL",2_110_065,5_000,10),
            tracked("TINA",2_110_056,21_000,11),
            tracked("BASIL",2_110_050,11_000,12),
            tracked("KARTGRIFF",2_110_049,6_000,13),
        ],Vec::new(),now);
        assert_eq!(rows.len(),1,"four active built-in buffs must consume one overlay row");
        match &rows[0]{
            MechanicDisplayRow::TrackerGroup(items)=>{
                assert_eq!(items.len(),4);
                assert_eq!(items.iter().map(|item|item.0.as_str()).collect::<Vec<_>>(),vec!["KARTGRIFF","BASIL","TINA","BL"]);
                assert_eq!(items.iter().find(|item|item.0=="BL").and_then(|item|item.1.remaining_ms),Some(4_000));
                assert_eq!(items.iter().find(|item|item.0=="TINA").and_then(|item|item.1.remaining_ms),Some(20_000));
            }
            _=>panic!("built-in tracked buffs must render through TrackerGroup"),
        }
    }

    #[test]
    fn builtin_buff_wins_over_duplicate_user_tracker_slot(){
        let tracker=crate::event_tracker::TrackerDisplayRow{label:"TINA".into(),remaining_ms:Some(99_000),last_seen_unix_ms:99,..Default::default()};
        let rows=compose_mechanic_rows(vec![tracked("TINA",2_110_056,21_000,1)],vec![tracker],1_000);
        assert_eq!(rows.len(),1);
        match &rows[0]{
            MechanicDisplayRow::TrackerGroup(items)=>{
                assert_eq!(items.len(),1);
                assert_eq!(items[0].0,"TINA");
                assert_eq!(items[0].1.remaining_ms,Some(20_000));
            }
            _=>panic!("duplicate TINA sources must collapse into one slot"),
        }
    }

    #[test]
    fn expired_builtin_buff_does_not_leave_a_group_row(){
        let rows=compose_mechanic_rows(vec![tracked("BL",2_110_065,900,1)],Vec::new(),1_000);
        assert!(rows.is_empty());
    }
}
"#);

    for required in [
        "special_tracked_mechanic",
        "special_tracker_from_mechanic",
        "authoritative||row.last_seen_unix_ms",
        "items.len()>=5",
        "\"KARTGRIFF\"=>\"KART\"",
        "draw(hdc,\"|\"",
    ] {
        assert!(source.contains(required), "v1.31.3 single-buff-row contract missing {required}");
    }

    fs::write(path, source).unwrap_or_else(|e| panic!("write v1.31.3 feature overlay: {e}"));
    println!("cargo:rerun-if-changed=build/legacy/build_v1313_single_buff_row.rs");
}
