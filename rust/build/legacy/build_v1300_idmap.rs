use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1290_compact_meter.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.30 id-map patch `{label}` expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));

    let telemetry_path = out.join("telemetry_v170_fixed.rs");
    let mut telemetry = fs::read_to_string(&telemetry_path)
        .expect("read generated telemetry for supplemental id mapping");
    replace_once(
        &mut telemetry,
        "        _ => format!(\"Skill {id}\"),\n",
        "        _ => crate::telemetry::game_names_v1300::skill_name(id)\n            .map(str::to_owned)\n            .unwrap_or_else(|| format!(\"Skill {id}\")),\n",
        "base telemetry skill fallback",
    );
    fs::write(&telemetry_path, telemetry)
        .expect("write generated telemetry with supplemental id mapping");

    let analysis_path = out.join("analysis_names_v1100.rs");
    let mut analysis = fs::read_to_string(&analysis_path)
        .expect("read generated analysis names for supplemental id mapping");
    replace_once(
        &mut analysis,
        "_=>format!(\"Skill {id}\"),}}\n\npub fn buff_name",
        "_=>crate::telemetry::game_names_v1300::skill_name(id).map(str::to_owned).unwrap_or_else(||format!(\"Skill {id}\")),}}\n\npub fn buff_name",
        "analysis skill fallback",
    );
    replace_once(
        &mut analysis,
        "_=>None,}}\n",
        "_=>crate::telemetry::game_names_v1300::buff_name(id),}}\n",
        "analysis buff fallback",
    );
    fs::write(&analysis_path, analysis)
        .expect("write generated analysis names with supplemental id mapping");

    println!("cargo:rerun-if-changed=build/legacy/build_v1300_idmap.rs");
    println!("cargo:rerun-if-changed=data/game_names_v1300.tsv.zst");
    println!("cargo:rerun-if-changed=src/game_names_v1300.rs");
}
