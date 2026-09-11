use std::{env,fs,path::{Path,PathBuf}};

mod prior { include!("build_v1190f.rs"); pub fn run(){main();} }

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){let count=source.matches(from).count();assert_eq!(count,1,"build_v1190g patch {label} expected one match, found {count}");*source=source.replacen(from,to,1);}

fn patch_feature_overlay(out:&Path){
    let path=out.join("feature_overlays_v170_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read responsive overlay source").replace("\r\n","\n");
    replace_once(&mut source,
        r###"fn dps_column_profile(tier:DpsLayoutTier,width:i32)->(i32,i32,i32,i32,i32){match tier{DpsLayoutTier::Comfortable=>(3,80,74,50,if width>=760{24}else{20}),DpsLayoutTier::Compact=>(2,70,62,46,20),DpsLayoutTier::Dense=>(2,60,54,42,18),DpsLayoutTier::Minimum=>(1,54,46,38,0)}}"###,
        r###"fn dps_column_profile(tier:DpsLayoutTier,width:i32)->(i32,i32,i32,i32,i32){match tier{DpsLayoutTier::Comfortable=>(3,80,74,50,if width>=760{24}else{20}),DpsLayoutTier::Compact=>(2,68,60,44,18),DpsLayoutTier::Dense=>(1,58,50,40,16),DpsLayoutTier::Minimum=>(1,52,44,36,0)}}
fn dps_can_keep_secondary(width:i32)->bool{width>=28}"###,
        "tighter responsive columns");
    source.push_str(r###"

#[cfg(test)]
mod v1190_data_retention_priority_tests{
    use super::*;
    #[test]
    fn compact_columns_reclaim_more_space_for_identity(){
        let c=dps_column_profile(DpsLayoutTier::Comfortable,700);
        let k=dps_column_profile(DpsLayoutTier::Compact,520);
        let d=dps_column_profile(DpsLayoutTier::Dense,400);
        let c_sum=c.1+c.2+c.3+c.4;let k_sum=k.1+k.2+k.3+k.4;let d_sum=d.1+d.2+d.3+d.4;
        assert!(k_sum<c_sum);assert!(d_sum<k_sum);
    }
    #[test]
    fn secondary_text_is_not_considered_hideable_until_space_is_genuinely_tiny(){
        assert!(dps_can_keep_secondary(28));assert!(!dps_can_keep_secondary(27));
    }
}
"###);
    fs::write(path,source).expect("write tighter responsive columns");
}

fn main(){prior::run();let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));patch_feature_overlay(&out);println!("cargo:rerun-if-changed=build/legacy/build_v1190g.rs");}
