use std::{env,fs,path::{Path,PathBuf}};

mod prior { include!("build_v1190c.rs"); pub fn run(){main();} }

fn patch_feature_overlay(out:&Path){
    let path=out.join("feature_overlays_v170_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read v1.19.0 responsive overlay source").replace("\r\n","\n");
    source.push_str(r###"

#[cfg(test)]
mod v1190_responsive_dps_tests{
    use super::*;

    #[test]
    fn scale_floor_is_fifty_percent(){
        assert_eq!(OVERLAY_SCALE_MIN,50);
        assert_eq!(clamp_overlay_scale(30),50);
        assert_eq!(clamp_overlay_scale(50),50);
    }

    #[test]
    fn downscale_readability_is_softened_but_upscale_stays_linear(){
        assert_eq!(dps_readability_percent(50),75);
        assert_eq!(dps_readability_percent(60),80);
        assert_eq!(dps_readability_percent(70),85);
        assert_eq!(dps_readability_percent(80),90);
        assert_eq!(dps_readability_percent(90),95);
        assert_eq!(dps_readability_percent(100),100);
        assert_eq!(dps_readability_percent(150),150);
        assert_eq!(dps_readability_percent(300),300);
    }

    #[test]
    fn dps_minimum_still_tracks_selected_scale(){
        assert_eq!((overlay_min_width(Kind::Dps,50),overlay_min_height(Kind::Dps,50)),(250,100));
        assert_eq!((overlay_min_width(Kind::Dps,100),overlay_min_height(Kind::Dps,100)),(500,200));
        assert_eq!((overlay_min_width(Kind::Dps,300),overlay_min_height(Kind::Dps,300)),(1500,600));
    }

    #[test]
    fn width_tiers_progressively_compact_before_minimum(){
        assert_eq!(dps_layout_tier(700,100),DpsLayoutTier::Comfortable);
        assert_eq!(dps_layout_tier(520,100),DpsLayoutTier::Compact);
        assert_eq!(dps_layout_tier(400,100),DpsLayoutTier::Dense);
        assert_eq!(dps_layout_tier(300,100),DpsLayoutTier::Minimum);
    }

    #[test]
    fn low_scale_effective_width_accounts_for_readability_floor(){
        assert!(dps_effective_width(500,50)<500);
        assert!(dps_row_h(50)>DPS_ROW_H);
        assert!(dps_badge_w(50)>BADGE_W);
    }

    #[test]
    fn headers_and_toolbar_abbreviate_only_as_space_tightens(){
        assert_eq!(dps_header_labels(DpsLayoutTier::Comfortable),("TOTAL","ACTIVE/s","%"));
        assert_eq!(dps_header_labels(DpsLayoutTier::Dense),("TOT","ACT/s","%"));
        assert_eq!(dps_tab_damage_label(DpsLayoutTier::Minimum),"Dmg");
        assert!(dps_toolbar_button_w(300,100)<dps_toolbar_button_w(700,100));
    }
}
"###);
    fs::write(path,source).expect("write v1.19.0 responsive tests");
}

fn main(){
    prior::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_feature_overlay(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1190d.rs");
}
