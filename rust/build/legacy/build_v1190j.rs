use std::{env,fs,path::{Path,PathBuf}};

mod prior { include!("build_v1190i.rs"); pub fn run(){main();} }

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){let count=source.matches(from).count();assert_eq!(count,1,"build_v1190j patch {label} expected one match, found {count}");*source=source.replacen(from,to,1);}

fn patch_feature_overlay(out:&Path){
    let path=out.join("feature_overlays_v170_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read responsive overlay source").replace("\r\n","\n");
    replace_once(&mut source,
        "fn dps_can_keep_secondary(width:i32)->bool{width>=28}",
        "fn dps_can_keep_secondary(width:i32)->bool{width>=28}",
        "final responsive helper presence");
    fs::write(path,source).expect("write final responsive overlay");
}

fn main(){prior::run();let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));patch_feature_overlay(&out);println!("cargo:rerun-if-changed=build/legacy/build_v1190j.rs");}
