use std::{env,fs,path::{Path,PathBuf}};

mod prior{
    include!("build_v1200.rs");
    pub fn run(){main();}
}

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){
    let count=source.matches(from).count();
    assert_eq!(count,1,"v1.21.0 patch {label} expected one match, found {count}");
    *source=source.replacen(from,to,1);
}
fn replace_between(source:&mut String,start:&str,end:&str,replacement:&str,label:&str){
    let count=source.matches(start).count();
    assert_eq!(count,1,"v1.21.0 patch {label} start expected one match, found {count}");
    let begin=source.find(start).expect("v1.21.0 start anchor");
    let rel_end=source[begin..].find(end).unwrap_or_else(||panic!("v1.21.0 patch {label} end anchor missing"));
    source.replace_range(begin..begin+rel_end,replacement);
}

include!("v1210/chat.rs");
include!("v1210/settings.rs");
include!("v1210/features.rs");

fn main(){
    prior::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_chat(&out);
    patch_settings(&out);
    patch_features(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1210.rs");
    println!("cargo:rerun-if-changed=build/legacy/v1210");
}
