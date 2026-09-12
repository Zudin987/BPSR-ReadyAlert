use std::{env, path::PathBuf};
mod prior { include!("build_ui_modernization.rs"); pub fn run() { main(); } }
mod stage1 { include!("build_v1290_compact_stage1.rs"); }
mod stage2 { include!("build_v1290_compact_stage2.rs"); }
mod stage3 { include!("build_v1290_compact_stage3.rs"); }
mod stage4 { include!("build_v1290_compact_stage4.rs"); }
mod stage5 { include!("build_v1290_compact_stage5.rs"); }
mod stage6 { include!("build_v1290_compact_stage6.rs"); }
mod stage7 { include!("build_v1290_compact_stage7.rs"); }
fn main(){
    prior::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    stage1::run(&out);stage2::run(&out);stage3::run(&out);stage4::run(&out);stage5::run(&out);stage6::run(&out);stage7::run(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1290_compact_meter.rs");
    for stage in 1..=7{println!("cargo:rerun-if-changed=build/legacy/build_v1290_compact_stage{stage}.rs");}
}
