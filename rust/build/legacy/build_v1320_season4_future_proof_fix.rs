use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1320_season4_future_proof.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.32.0 future-proof follow-up {label:?} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let path = out.join("telemetry_v170_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .expect("read telemetry for v1.32.0 Imagine future-proof follow-up")
        .replace("\r\n", "\n");

    replace_once(
        &mut source,
        "fn imagine_info(id:i32)->(String,String){match id{",
        "fn imagine_info(id:i32)->(String,String){if let Some(name)=crate::telemetry::game_names_v1300::override_skill_name(id){return(name.to_owned(),\"BI\".into());}match id{",
        "Imagine override precedence",
    );
    replace_once(
        &mut source,
        "3998=>(\"Darkriver Fin (AI)\".into(),\"DF\".into()),3999=>(\"Valley Ranger (AI)\".into(),\"VR\".into()),_=>(format!(\"Battle Imagine {id}\"),\"BI\".into()),}}",
        "3998=>(\"Darkriver Fin (AI)\".into(),\"DF\".into()),3999=>(\"Valley Ranger (AI)\".into(),\"VR\".into()),_=>{if let Some(name)=crate::telemetry::game_names_v1300::skill_name(id){(name.to_owned(),\"BI\".into())}else{(format!(\"Unknown Imagine ({id})\"),\"BI\".into())}},}}",
        "unknown Imagine numeric fallback",
    );

    fs::write(path, source).expect("write telemetry v1.32.0 Imagine future-proof follow-up");
    println!("cargo:rerun-if-changed=build/legacy/build_v1320_season4_future_proof_fix.rs");
}
