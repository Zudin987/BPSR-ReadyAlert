use std::{env,fs,path::{Path,PathBuf}};

mod prior { include!("build_v1190d.rs"); pub fn run(){main();} }

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){let count=source.matches(from).count();assert_eq!(count,1,"build_v1190e patch {label} expected one match, found {count}");*source=source.replacen(from,to,1);}

fn patch_feature_overlay(out:&Path){
    let path=out.join("feature_overlays_v170_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read responsive overlay source").replace("\r\n","\n");
    replace_once(&mut source,
        r###"fn dps_layout_tier(width:i32,scale:i32)->DpsLayoutTier{let effective=dps_effective_width(width,scale);if effective>=600{DpsLayoutTier::Comfortable}else if effective>=480{DpsLayoutTier::Compact}else if effective>=340{DpsLayoutTier::Dense}else{DpsLayoutTier::Minimum}}"###,
        r###"fn dps_layout_tier(width:i32,scale:i32)->DpsLayoutTier{let effective=dps_effective_width(width,scale);if effective>=600{DpsLayoutTier::Comfortable}else if effective>=480{DpsLayoutTier::Compact}else if effective>=340{DpsLayoutTier::Dense}else{DpsLayoutTier::Minimum}}
fn dps_column_profile(tier:DpsLayoutTier,width:i32)->(i32,i32,i32,i32,i32){match tier{DpsLayoutTier::Comfortable=>(3,80,74,50,if width>=760{24}else{20}),DpsLayoutTier::Compact=>(2,70,62,46,20),DpsLayoutTier::Dense=>(2,60,54,42,18),DpsLayoutTier::Minimum=>(1,54,46,38,0)}}"###,
        "column profile helper");
    replace_once(&mut source,
        r###"    let tier=dps_layout_tier((r.right-r.left).max(1),scale);let(gap,total_w,active_w,share_w,death_w)=match tier{DpsLayoutTier::Comfortable=>(3,80,74,50,if r.right-r.left>=760{24}else{20}),DpsLayoutTier::Compact=>(2,70,62,46,20),DpsLayoutTier::Dense=>(2,60,54,46,18),DpsLayoutTier::Minimum=>(1,56,50,44,0)};"###,
        r###"    let width=(r.right-r.left).max(1);let tier=dps_layout_tier(width,scale);let(gap,total_w,active_w,share_w,death_w)=dps_column_profile(tier,width);"###,
        "use responsive column profile");
    source.push_str(r###"

#[cfg(test)]
mod v1190_space_efficiency_tests{
    use super::*;
    #[test]
    fn numeric_columns_shrink_before_layout_reaches_minimum(){
        let comfortable=dps_column_profile(DpsLayoutTier::Comfortable,700);
        let compact=dps_column_profile(DpsLayoutTier::Compact,520);
        let dense=dps_column_profile(DpsLayoutTier::Dense,400);
        assert!(compact.1<comfortable.1&&compact.2<comfortable.2&&compact.3<comfortable.3);
        assert!(dense.1<compact.1&&dense.2<compact.2&&dense.3<compact.3&&dense.4<=compact.4);
    }
    #[test]
    fn minimum_tier_reclaims_death_space_first(){
        let minimum=dps_column_profile(DpsLayoutTier::Minimum,300);
        assert_eq!(minimum.4,0);
        assert!(minimum.1>0&&minimum.2>0&&minimum.3>0);
    }
    #[test]
    fn badge_and_build_gap_remain_small_under_downscale(){
        assert!(dps_build_badge_gap(50)<=dps_build_badge_gap(100)*2);
        assert!(dps_badge_gap(50)<=dps_badge_gap(100)*2);
    }
}
"###);
    fs::write(path,source).expect("write responsive column profile");
}

fn main(){prior::run();let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));patch_feature_overlay(&out);println!("cargo:rerun-if-changed=build/legacy/build_v1190e.rs");}
