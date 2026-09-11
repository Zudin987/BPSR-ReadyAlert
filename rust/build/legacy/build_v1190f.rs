use std::{env,fs,path::{Path,PathBuf}};

mod prior { include!("build_v1190e.rs"); pub fn run(){main();} }

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){let count=source.matches(from).count();assert_eq!(count,1,"build_v1190f patch {label} expected one match, found {count}");*source=source.replacen(from,to,1);}

fn patch_feature_overlay(out:&Path){
    let path=out.join("feature_overlays_v170_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read responsive overlay source").replace("\r\n","\n");
    replace_once(&mut source,
        r###"fn dps_toolbar_button_w(right:i32,scale:i32)->i32{let tier=dps_layout_tier(right.max(1),scale);match tier{DpsLayoutTier::Comfortable=>BUTTON_W,DpsLayoutTier::Compact=>44,DpsLayoutTier::Dense=>36,DpsLayoutTier::Minimum=>30}}"###,
        r###"fn dps_toolbar_button_w(right:i32,scale:i32)->i32{let tier=dps_layout_tier(right.max(1),scale);match tier{DpsLayoutTier::Comfortable=>BUTTON_W,DpsLayoutTier::Compact=>44,DpsLayoutTier::Dense=>36,DpsLayoutTier::Minimum=>30}}
fn dps_username_gap(tier:DpsLayoutTier)->i32{match tier{DpsLayoutTier::Comfortable=>8,DpsLayoutTier::Compact=>6,DpsLayoutTier::Dense=>4,DpsLayoutTier::Minimum=>3}}
fn dps_min_identity_width(tier:DpsLayoutTier)->i32{match tier{DpsLayoutTier::Comfortable=>90,DpsLayoutTier::Compact=>76,DpsLayoutTier::Dense=>62,DpsLayoutTier::Minimum=>48}}"###,
        "responsive identity constants");
    replace_once(&mut source,
        r###"    let name_left=r.left+25;let badge_w=dps_badge_w(scale);let badge_gap=dps_badge_gap(scale);let build_gap=dps_build_badge_gap(scale);let min_identity=match tier{DpsLayoutTier::Comfortable=>90,DpsLayoutTier::Compact=>76,DpsLayoutTier::Dense=>62,DpsLayoutTier::Minimum=>48};let mut badge_count=if show_imagines{row.imagines.len().min(2)}else{0};"###,
        r###"    let name_left=r.left+25;let badge_w=dps_badge_w(scale);let badge_gap=dps_badge_gap(scale);let build_gap=dps_build_badge_gap(scale);let min_identity=dps_min_identity_width(tier);let mut badge_count=if show_imagines{row.imagines.len().min(2)}else{0};"###,
        "identity minimum helper");
    replace_once(&mut source,
        r###"    let old=SelectObject(hdc,name_font);let measured_name=dps_text_width(hdc,&row.name,identity_text_px(&row.name,true));SelectObject(hdc,old);let name_gap=match tier{DpsLayoutTier::Comfortable=>8,DpsLayoutTier::Compact=>6,DpsLayoutTier::Dense=>4,DpsLayoutTier::Minimum=>3};let name_right=(name_left+measured_name+2).min(identity_right).max(name_left);let spec_left=(name_right+name_gap).min(identity_right);let spec_right=identity_right;"###,
        r###"    let old=SelectObject(hdc,name_font);let measured_name=dps_text_width(hdc,&row.name,identity_text_px(&row.name,true));SelectObject(hdc,old);let name_gap=dps_username_gap(tier);let name_right=(name_left+measured_name+2).min(identity_right).max(name_left);let spec_left=(name_right+name_gap).min(identity_right);let spec_right=identity_right;"###,
        "username build gap helper");
    source.push_str(r###"

#[cfg(test)]
mod v1190_empty_space_priority_tests{
    use super::*;
    #[test]
    fn username_to_build_gap_collapses_before_information(){
        assert!(dps_username_gap(DpsLayoutTier::Compact)<dps_username_gap(DpsLayoutTier::Comfortable));
        assert!(dps_username_gap(DpsLayoutTier::Dense)<dps_username_gap(DpsLayoutTier::Compact));
        assert!(dps_username_gap(DpsLayoutTier::Minimum)<dps_username_gap(DpsLayoutTier::Dense));
    }
    #[test]
    fn reserved_identity_space_also_shrinks_progressively(){
        assert!(dps_min_identity_width(DpsLayoutTier::Compact)<dps_min_identity_width(DpsLayoutTier::Comfortable));
        assert!(dps_min_identity_width(DpsLayoutTier::Dense)<dps_min_identity_width(DpsLayoutTier::Compact));
        assert!(dps_min_identity_width(DpsLayoutTier::Minimum)<dps_min_identity_width(DpsLayoutTier::Dense));
    }
}
"###);
    fs::write(path,source).expect("write responsive identity helpers");
}

fn main(){prior::run();let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));patch_feature_overlay(&out);println!("cargo:rerun-if-changed=build/legacy/build_v1190f.rs");}
