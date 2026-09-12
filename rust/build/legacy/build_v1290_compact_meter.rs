use std::{env, path::PathBuf};
mod prior { include!("build_ui_modernization.rs"); pub fn run() { main(); } }
mod stage1 { include!("build_v1290_compact_stage1.rs"); }
mod stage2 { include!("build_v1290_compact_stage2.rs"); }
mod stage3 { include!("build_v1290_compact_stage3.rs"); }
mod stage4 { include!("build_v1290_compact_stage4.rs"); }
mod stage5 { include!("build_v1290_compact_stage5.rs"); }
mod stage6 { include!("build_v1290_compact_stage6.rs"); }
mod stage7 { include!("build_v1290_compact_stage7.rs"); }
mod finish { include!("build_v1290_native_finish.rs"); }
fn main(){
    // Ordering is deliberate:
    // 1) the existing modernization patches establish shared semantic surfaces,
    // 2) Compact/Raid adds its mode/state/layout behavior,
    // 3) the final native pass only restyles the resulting generated source.
    // This prevents either visual pass from replacing Compact behavior.
    prior::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    stage1::run(&out);stage2::run(&out);stage3::run(&out);stage4::run(&out);stage5::run(&out);stage6::run(&out);stage7::run(&out);
    finish::run(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1290_compact_meter.rs");
    for stage in 1..=7{println!("cargo:rerun-if-changed=build/legacy/build_v1290_compact_stage{stage}.rs");}
    println!("cargo:rerun-if-changed=build/legacy/build_v1290_native_finish.rs");
    println!("cargo:rerun-if-changed=src/ui_modern.rs");
}
