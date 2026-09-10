use std::{env,fs,path::{Path,PathBuf}};

mod prior {
    include!("build_v1186.rs");
    pub fn run() { main(); }
}

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){
    let count=source.matches(from).count();
    assert_eq!(count,1,"v1.18.7 patch {label} expected one match, found {count}");
    *source=source.replacen(from,to,1);
}

fn patch_feature_overlay(out:&Path){
    let path=out.join("feature_overlays_v170_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read v1.18.6 generated overlays").replace("\r\n","\n");

    replace_once(
        &mut source,
        "fn overlay_min_width(kind:Kind,scale:i32)->i32{scale_px(if kind==Kind::Dps{620}else{340},scale)}",
        "fn overlay_min_width(kind:Kind,scale:i32)->i32{scale_px(if kind==Kind::Dps{500}else{340},scale)}",
        "DPS minimum width base",
    );
    replace_once(
        &mut source,
        "fn overlay_min_height(scale:i32)->i32{scale_px(260,scale)}",
        "fn overlay_min_height(kind:Kind,scale:i32)->i32{scale_px(if kind==Kind::Dps{200}else{260},scale)}",
        "per-overlay minimum height base",
    );

    replace_once(&mut source,"overlay_min_height(scale_percent)","overlay_min_height(kind,scale_percent)","creation minimum height");
    replace_once(&mut source,"overlay_min_height((*ptr).scale_percent)","overlay_min_height((*ptr).kind,(*ptr).scale_percent)","resize minimum height");
    replace_once(&mut source,"overlay_min_height(new)","overlay_min_height(overlay.kind,new)","scale-change minimum height");
    replace_once(&mut source,"overlay_min_height(state.scale_percent)","overlay_min_height(state.kind,state.scale_percent)","expand minimum height");

    replace_once(
        &mut source,
        "#[test]fn minimum_window_size_tracks_scale(){assert_eq!(overlay_min_width(Kind::Dps,30),186);assert_eq!(overlay_min_height(30),78);assert_eq!(overlay_min_width(Kind::Dps,100),620);assert_eq!(overlay_min_height(100),260);assert_eq!(overlay_min_width(Kind::Dps,300),1860);assert_eq!(overlay_min_height(300),780);assert_eq!(overlay_min_width(Kind::Mechanics,30),102);}",
        "#[test]fn minimum_window_size_tracks_scale(){assert_eq!(overlay_min_width(Kind::Dps,30),150);assert_eq!(overlay_min_height(Kind::Dps,30),60);assert_eq!(overlay_min_width(Kind::Dps,100),500);assert_eq!(overlay_min_height(Kind::Dps,100),200);assert_eq!(overlay_min_width(Kind::Dps,300),1500);assert_eq!(overlay_min_height(Kind::Dps,300),600);assert_eq!(overlay_min_width(Kind::Mechanics,30),102);assert_eq!(overlay_min_height(Kind::Mechanics,30),78);}",
        "updated v1.18.6 scale regression expectations",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1187_dps_min_size_tests{
    use super::*;

    #[test]
    fn dps_minimum_is_500_by_200_at_100_percent(){
        assert_eq!(overlay_min_width(Kind::Dps,100),500);
        assert_eq!(overlay_min_height(Kind::Dps,100),200);
    }

    #[test]
    fn dps_minimum_continues_to_follow_scale_percent(){
        assert_eq!((overlay_min_width(Kind::Dps,30),overlay_min_height(Kind::Dps,30)),(150,60));
        assert_eq!((overlay_min_width(Kind::Dps,300),overlay_min_height(Kind::Dps,300)),(1500,600));
    }

    #[test]
    fn mechanics_minimum_is_unchanged(){
        assert_eq!((overlay_min_width(Kind::Mechanics,100),overlay_min_height(Kind::Mechanics,100)),(340,260));
    }
}
"#);

    fs::write(path,source).expect("write v1.18.7 feature overlay");
}

fn main(){
    prior::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_feature_overlay(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1187.rs");
}
