use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1100.rs");
    pub fn run() { main(); }
}

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){
    let count=source.matches(from).count();
    assert_eq!(count,1,"v1.11.0 patch `{label}` expected one match, found {count}");
    *source=source.replacen(from,to,1);
}

fn replace_between(source:&mut String,start:&str,end:&str,replacement:&str,label:&str){
    let count=source.matches(start).count();
    assert_eq!(count,1,"v1.11.0 patch `{label}` start expected one match, found {count}");
    let begin=source.find(start).expect("v1.11 start checked");
    let rel_end=source[begin..].find(end).unwrap_or_else(||panic!("v1.11.0 patch `{label}` end anchor missing"));
    source.replace_range(begin..begin+rel_end,replacement);
}

fn main(){
    previous::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));

    // v1.11 model additions remain serde-defaulted for old history, while the
    // generated v1.8 telemetry core uses explicit literals and needs zero values.
    let telemetry_path=out.join("telemetry_v170_fixed.rs");
    let mut telemetry=fs::read_to_string(&telemetry_path).expect("read generated v1.10 telemetry");
    replace_once(
        &mut telemetry,
        "                    damage: value.damage,\n                    healing: value.healing,",
        "                    damage: value.damage,\n                    boss_damage: 0,\n                    healing: value.healing,\n                    effective_healing: 0,\n                    overhealing: 0,",
        "skill analysis defaults",
    );
    replace_once(
        &mut telemetry,
        "                is_party: false,\n                boss_damage: 0,",
        "                is_party: false,\n                absorbed_damage: 0,\n                boss_damage: 0,",
        "absorbed row total default",
    );
    replace_once(
        &mut telemetry,
        "                taken_sources: Vec::new(),\n                attributes,",
        "                taken_sources: Vec::new(),\n                absorbed_sources: Vec::new(),\n                attributes,",
        "absorbed row sources default",
    );
    replace_once(
        &mut telemetry,
        "            total_damage_taken: total_taken,\n            total_boss_damage: 0,",
        "            total_damage_taken: total_taken,\n            total_absorbed_damage: 0,\n            total_boss_damage: 0,",
        "absorbed snapshot default",
    );
    fs::write(&telemetry_path,telemetry).expect("write v1.11 generated telemetry defaults");

    let overlay_path=out.join("feature_overlays_v170_fixed.rs");
    let mut overlay=fs::read_to_string(&overlay_path).expect("read generated v1.10 overlay");
    replace_once(
        &mut overlay,
        "DetailMode::Tank=>state.row.taken_sources.len(),",
        "DetailMode::Tank=>state.row.taken_sources.len().saturating_add(state.row.absorbed_sources.len()),",
        "combined tank scroll count",
    );
    replace_between(
        &mut overlay,
        "fn detail_skills",
        "unsafe fn open_feature_settings",
        include_str!("overlay_v1110_analysis_patch.txt"),
        "skill analysis inspector",
    );
    fs::write(&overlay_path,overlay).expect("write v1.11 generated overlay");

    println!("cargo:rerun-if-changed=build_v1110.rs");
    println!("cargo:rerun-if-changed=overlay_v1110_analysis_patch.txt");
    println!("cargo:rerun-if-changed=src/telemetry_v1110.rs");
}
