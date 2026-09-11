use std::{env,fs,path::PathBuf};
mod prior { include!("build_v1190j.rs"); pub fn run(){main();} }
fn main(){
    prior::run();
    let path=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("feature_overlays_v170_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read v1.19.0 overlay").replace("\r\n","\n");
    replace_once(&mut source,r######"const OVERLAY_SCALE_MIN:i32=50;"######,r######"// Shared coordinate math retains the Mechanics 30% range.
const OVERLAY_SCALE_MIN:i32=30;
const DPS_SCALE_MIN:i32=50;
fn overlay_scale_min(kind:Kind)->i32{if kind==Kind::Dps{DPS_SCALE_MIN}else{OVERLAY_SCALE_MIN}}
fn clamp_kind_scale(kind:Kind,value:i32)->i32{value.clamp(overlay_scale_min(kind),OVERLAY_SCALE_MAX)}"######,"scope floor to DPS");
    replace_once(&mut source,r######"fn overlay_min_width(kind:Kind,scale:i32)->i32{scale_px(if kind==Kind::Dps{500}else{340},scale)}"######,r######"fn overlay_min_width(kind:Kind,scale:i32)->i32{scale_px(if kind==Kind::Dps{500}else{340},clamp_kind_scale(kind,scale))}"######,"overlay_min_width");
    replace_once(&mut source,r######"fn overlay_min_height(kind:Kind,scale:i32)->i32{scale_px(if kind==Kind::Dps{200}else{260},scale)}"######,r######"fn overlay_min_height(kind:Kind,scale:i32)->i32{scale_px(if kind==Kind::Dps{200}else{260},clamp_kind_scale(kind,scale))}"######,"overlay_min_height");
    replace_once(&mut source,r######"fn dps_readability_percent(scale:i32)->i32{let s=clamp_overlay_scale(scale);if s<100{(s+100)/2}else{s}}"######,r######"fn dps_readability_percent(scale:i32)->i32{let s=clamp_kind_scale(Kind::Dps,scale);if s<100{(s+100)/2}else{s}}"######,"dps_readability_percent");
    replace_once(&mut source,r######"fn dps_adaptive_logical_px(value:i32,scale:i32)->i32{let s=clamp_overlay_scale(scale).max(1)as i64;let r=dps_readability_percent(scale)as i64;(((value.max(1)as i64)*r+s/2)/s).clamp(1,i32::MAX as i64)as i32}"######,r######"fn dps_adaptive_logical_px(value:i32,scale:i32)->i32{let s=clamp_kind_scale(Kind::Dps,scale).max(1)as i64;let r=dps_readability_percent(scale)as i64;(((value.max(1)as i64)*r+s/2)/s).clamp(1,i32::MAX as i64)as i32}"######,"dps_adaptive_logical_px");
    replace_once(&mut source,r######"fn dps_effective_width(logical:i32,scale:i32)->i32{let s=clamp_overlay_scale(scale).max(1)as i64;let r=dps_readability_percent(scale).max(1)as i64;(((logical.max(1)as i64)*s+r/2)/r).clamp(1,i32::MAX as i64)as i32}"######,r######"fn dps_effective_width(logical:i32,scale:i32)->i32{let s=clamp_kind_scale(Kind::Dps,scale).max(1)as i64;let r=dps_readability_percent(scale).max(1)as i64;(((logical.max(1)as i64)*s+r/2)/r).clamp(1,i32::MAX as i64)as i32}"######,"dps_effective_width");
    replace_once(&mut source,r######"let key=(clamp_overlay_scale(scale),bold);"######,r######"let key=(clamp_kind_scale(Kind::Dps,scale),bold);"######,"DPS font cache clamp");
    replace_once(&mut source,r######"fn normalize_scale_file(value:&mut OverlayScaleFile){normalize_scale_slot(&mut value.dps);normalize_scale_slot(&mut value.mechanics);}"######,r######"fn normalize_kind_scale_slot(kind:Kind,slot:&mut OverlayScaleSlot){
    normalize_scale_slot(slot);let old=slot.percent;let new=clamp_kind_scale(kind,old);
    if old!=new&&slot.has_bounds{slot.width=rescale_px(slot.width,old,new);slot.height=rescale_px(slot.height,old,new);}
    slot.percent=new;
}
fn normalize_scale_file(value:&mut OverlayScaleFile){normalize_kind_scale_slot(Kind::Dps,&mut value.dps);normalize_kind_scale_slot(Kind::Mechanics,&mut value.mechanics);}"######,"persisted scale migration");
    replace_once(&mut source,r######">{normalize_scale_slot(&mut slot);let mut file=load_scale_file(paths);"######,r######">{normalize_kind_scale_slot(kind,&mut slot);let mut file=load_scale_file(paths);"######,"save per-kind scale");
    replace_once(&mut source,r######"let new=snap_overlay_scale(requested);"######,r######"let new=clamp_kind_scale(overlay.kind,snap_overlay_scale(requested));"######,"settings clamp");
    replace_once(&mut source,r######"feature_scale_slider(hwnd,7007,184,72,256);"######,r######"feature_scale_slider(hwnd,7007,184,72,256);SendMessageW(fc(hwnd,7007),0x0407,1,overlay_scale_min(state.kind)as isize);"######,"slider minimum");
    replace_once(&mut source,r######"(scale>OVERLAY_SCALE_MIN)as i32"######,r######"(scale>overlay_scale_min(state.kind))as i32"######,"decrement bound");
    replace_once(&mut source,r######"assert_eq!(overlay_min_width(Kind::Dps,30),150);assert_eq!(overlay_min_height(Kind::Dps,30),60);"######,r######"assert_eq!(overlay_min_width(Kind::Dps,30),250);assert_eq!(overlay_min_height(Kind::Dps,30),100);"######,"historical DPS minimum assertion");
    replace_once(&mut source,r######"        assert_eq!(OVERLAY_SCALE_MIN,50);"######,r######"        assert_eq!(DPS_SCALE_MIN,50);"######,"DPS contract assertion");
    replace_once(&mut source,r######"    fn minimum_scale_is_practical_and_max_is_preserved(){assert_eq!(OVERLAY_SCALE_MIN,50);assert_eq!(OVERLAY_SCALE_MAX,300);}"######,r######"    fn minimum_scale_is_practical_and_max_is_preserved(){assert_eq!(DPS_SCALE_MIN,50);assert_eq!(OVERLAY_SCALE_MAX,300);}"######,"DPS contract assertion");
    replace_once(&mut source,r######"fn scale_floor_is_fifty_percent(){
        assert_eq!(DPS_SCALE_MIN,50);
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
}"######,r######"fn scale_floor_is_fifty_percent(){
        assert_eq!(DPS_SCALE_MIN,50);
        assert_eq!(clamp_kind_scale(Kind::Dps,30),50);
        assert_eq!(clamp_kind_scale(Kind::Dps,50),50);
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
}"######,"DPS contract assertion");
    replace_once(&mut source,r######"        assert_eq!(clamp_overlay_scale(30),50);"######,r######"        assert_eq!(clamp_kind_scale(Kind::Dps,30),50);"######,"DPS contract assertion");
    replace_once(&mut source,r######"spec.clone(),primary.clone()];"######,r######"spec.clone(),if spec.is_empty(){primary.clone()}else{String::new()}];"######,"never replace known spec with ability");
    replace_once(&mut source,r######"    let old=SelectObject(hdc,name_font);let measured_name=dps_text_width(hdc,&row.name,identity_text_px(&row.name,true));SelectObject(hdc,old);let name_gap=dps_username_gap(tier);let name_right=(name_left+measured_name+2).min(identity_right).max(name_left);let spec_left=(name_right+name_gap).min(identity_right);let spec_right=identity_right;"######,r######"    let old=SelectObject(hdc,name_font);let measured_name=dps_text_width(hdc,&row.name,identity_text_px(&row.name,true));
    SelectObject(hdc,secondary_font);let spec=dps_spec(row);let spec_width=dps_text_width(hdc,&spec,identity_text_px(&spec,false));SelectObject(hdc,old);
    let name_gap=dps_username_gap(tier);let available=(identity_right-name_left).max(0);
    // Reserve the highest-priority metadata before a long username consumes its space.
    let name_min=dps_adaptive_logical_px(36,scale).min(available);
    let spec_reserve=if spec.is_empty(){0}else{spec_width.min((available-name_min-name_gap).max(0))};
    let name_limit=if spec_reserve>0{identity_right-spec_reserve-name_gap}else{identity_right};
    let name_right=(name_left+measured_name+2).min(name_limit).max(name_left);let spec_left=(name_right+name_gap).min(identity_right);let spec_right=identity_right;"######,"reserve measured spec alongside badges");
    replace_once(&mut source,r######"let _=secondary_font;DpsRowLayout{"######,r######"DpsRowLayout{"######,"use measured metadata font");
    replace_once(&mut source,r######"let tier_size=(w.min(h)*11/23).clamp(7,11);"######,r######"let tier_size=(w.min(h)*11/23).max(7);"######,"badge tier follows readable badge size");
    replace_once(&mut source,r######"    String::new()
}
fn dps_take_column"######,r######"    spec
}
fn dps_take_column"######,"ellipsis preserves spec priority");
    source.push_str(include_str!("v1190_release_tests.inc"));
    fs::write(path,source).expect("write v1.19.0 release fixes");
}
fn replace_once(source:&mut String,from:&str,to:&str,label:&str){assert_eq!(source.matches(from).count(),1,"v1190k: {label}");*source=source.replacen(from,to,1);}
