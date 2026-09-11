use std::{env,fs,path::{Path,PathBuf}};

mod prior { include!("build_v1190h.rs"); pub fn run(){main();} }

fn patch_feature_overlay(out:&Path){
    let path=out.join("feature_overlays_v170_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read responsive overlay source").replace("\r\n","\n");
    source.push_str(r###"

#[cfg(test)]
mod v1190_final_invariants_tests{
    use super::*;
    #[test]
    fn minimum_scale_is_practical_and_max_is_preserved(){assert_eq!(OVERLAY_SCALE_MIN,50);assert_eq!(OVERLAY_SCALE_MAX,300);}
    #[test]
    fn build_badge_cluster_gap_stays_tight(){assert!(dps_build_badge_gap(50)<=8);assert!(dps_build_badge_gap(100)<=8);}
    #[test]
    fn responsive_numeric_columns_never_grow_when_space_tightens(){
        let a=dps_column_profile(DpsLayoutTier::Comfortable,700);let b=dps_column_profile(DpsLayoutTier::Compact,520);let c=dps_column_profile(DpsLayoutTier::Dense,400);let d=dps_column_profile(DpsLayoutTier::Minimum,300);
        assert!(a.1>=b.1&&b.1>=c.1&&c.1>=d.1);assert!(a.2>=b.2&&b.2>=c.2&&c.2>=d.2);assert!(a.3>=b.3&&b.3>=c.3&&c.3>=d.3);assert!(a.4>=b.4&&b.4>=c.4&&c.4>=d.4);
    }
}
"###);
    fs::write(path,source).expect("write v1.19.0 final responsive invariants");
}

fn main(){prior::run();let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));patch_feature_overlay(&out);println!("cargo:rerun-if-changed=build/legacy/build_v1190i.rs");}
