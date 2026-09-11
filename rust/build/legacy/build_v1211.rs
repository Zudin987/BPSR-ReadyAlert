use std::{env,fs,path::{Path,PathBuf}};

mod prior {
    include!("build_v1210.rs");
    pub fn run(){main();}
}

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){
    let count=source.matches(from).count();
    assert_eq!(count,1,"v1.21.1 patch {label} expected one match, found {count}");
    *source=source.replacen(from,to,1);
}

fn replace_exact_count(source:&mut String,from:&str,to:&str,expected:usize,label:&str){
    let count=source.matches(from).count();
    assert_eq!(count,expected,"v1.21.1 patch {label} expected {expected} matches, found {count}");
    *source=source.replace(from,to);
}

fn patch_feature_overlay(out:&Path){
    let path=out.join("feature_overlays_v170_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read v1.21.0 generated overlays").replace("\r\n","\n");

    replace_once(
        &mut source,
        "fn overlay_min_width(kind:Kind,scale:i32)->i32{scale_px(if kind==Kind::Dps{500}else{340},clamp_kind_scale(kind,scale))}",
        "fn overlay_min_width(kind:Kind,scale:i32)->i32{scale_px(if kind==Kind::Dps{300}else{340},clamp_kind_scale(kind,scale))}",
        "DPS minimum width base",
    );

    replace_once(&mut source,"dps_minimum_is_500_by_200_at_100_percent","dps_minimum_is_300_by_200_at_100_percent","minimum-size test name");
    replace_once(&mut source,"assert_eq!(overlay_min_width(Kind::Dps,30),250);","assert_eq!(overlay_min_width(Kind::Dps,30),150);","legacy scale test width");
    replace_exact_count(&mut source,"assert_eq!(overlay_min_width(Kind::Dps,100),500);","assert_eq!(overlay_min_width(Kind::Dps,100),300);",2,"100 percent width assertions");
    replace_once(&mut source,"assert_eq!(overlay_min_width(Kind::Dps,300),1500);","assert_eq!(overlay_min_width(Kind::Dps,300),900);","300 percent width assertion");
    replace_exact_count(&mut source,"(overlay_min_width(Kind::Dps,30),overlay_min_height(Kind::Dps,30)),(250,100)","(overlay_min_width(Kind::Dps,30),overlay_min_height(Kind::Dps,30)),(150,100)",2,"30 percent tuple assertions");
    replace_once(&mut source,"(overlay_min_width(Kind::Dps,50),overlay_min_height(Kind::Dps,50)),(250,100)","(overlay_min_width(Kind::Dps,50),overlay_min_height(Kind::Dps,50)),(150,100)","50 percent tuple assertion");
    replace_once(&mut source,"(overlay_min_width(Kind::Dps,100),overlay_min_height(Kind::Dps,100)),(500,200)","(overlay_min_width(Kind::Dps,100),overlay_min_height(Kind::Dps,100)),(300,200)","100 percent tuple assertion");
    replace_exact_count(&mut source,"(overlay_min_width(Kind::Dps,300),overlay_min_height(Kind::Dps,300)),(1500,600)","(overlay_min_width(Kind::Dps,300),overlay_min_height(Kind::Dps,300)),(900,600)",2,"300 percent tuple assertions");

    source.push_str(r#"

#[cfg(test)]
mod v1211_dps_min_width_tests{
    use super::*;

    #[test]
    fn dps_minimum_width_is_300_at_100_percent(){
        assert_eq!(overlay_min_width(Kind::Dps,100),300);
        assert_eq!(overlay_min_height(Kind::Dps,100),200);
    }

    #[test]
    fn dps_minimum_width_tracks_scale_floor_and_zoom(){
        assert_eq!(overlay_min_width(Kind::Dps,50),150);
        assert_eq!(overlay_min_width(Kind::Dps,300),900);
    }

    #[test]
    fn mechanics_minimum_width_is_unchanged(){
        assert_eq!(overlay_min_width(Kind::Mechanics,100),340);
    }
}
"#);

    fs::write(path,source).expect("write v1.21.1 feature overlay");
}

fn main(){
    prior::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_feature_overlay(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1211.rs");
}
