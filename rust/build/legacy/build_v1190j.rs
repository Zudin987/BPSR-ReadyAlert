use std::{env,fs,path::{Path,PathBuf}};

mod prior { include!("build_v1190i.rs"); pub fn run(){main();} }

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){let count=source.matches(from).count();assert_eq!(count,1,"build_v1190j patch {label} expected one match, found {count}");*source=source.replacen(from,to,1);}

fn patch_feature_overlay(out:&Path){
    let path=out.join("feature_overlays_v170_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read responsive overlay source").replace("\r\n","\n");

    // v1.19.0 deliberately raises the practical scale floor from 30% to 50%.
    // Keep the historical v1.18.7 regression meaningful by asserting that a
    // legacy/requested 30% value is clamped to the new 50% minimum rather than
    // restoring the unreadable 30% product behavior.
    replace_once(&mut source,
        "assert_eq!((overlay_min_width(Kind::Dps,30),overlay_min_height(Kind::Dps,30)),(150,60));",
        "assert_eq!((overlay_min_width(Kind::Dps,30),overlay_min_height(Kind::Dps,30)),(250,100));",
        "v1.18.7 legacy 30 percent clamp expectation");

    source.push_str(r###"

#[cfg(test)]
mod v1190_legacy_scale_compat_tests{
    use super::*;
    #[test]
    fn legacy_thirty_percent_requests_clamp_to_new_fifty_percent_floor(){
        assert_eq!(clamp_overlay_scale(30),50);
        assert_eq!((overlay_min_width(Kind::Dps,30),overlay_min_height(Kind::Dps,30)),(250,100));
    }
}
"###);

    fs::write(path,source).expect("write v1.19.0 legacy-scale compatibility source");
}

fn main(){
    prior::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_feature_overlay(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1190j.rs");
}
