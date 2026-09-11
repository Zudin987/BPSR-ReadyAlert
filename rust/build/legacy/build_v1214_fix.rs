use std::{env,fs,path::PathBuf};

mod prior {
    include!("build_v1214.rs");
    pub fn run(){main();}
}

fn main(){
    prior::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let path=out.join("feature_overlays_v170_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read v1.21.4 generated overlays").replace("\r\n","\n");
    let from="let not_yet=status(30_001,now);";
    let count=source.matches(from).count();
    assert_eq!(count,1,"v1.21.4 test fix expected one threshold fixture, found {count}");
    source=source.replacen(from,"let not_yet=status(31_000,now);",1);
    fs::write(path,source).expect("write v1.21.4 threshold test fix");
    println!("cargo:rerun-if-changed=build/legacy/build_v1214_fix.rs");
}
