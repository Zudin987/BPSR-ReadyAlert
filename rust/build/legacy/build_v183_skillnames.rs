use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

mod previous {
    include!("build_v183_followup.rs");
    pub fn run() { main(); }
}

const ZDPS_COMMIT: &str = "cfeb58c0acc85bc17181b413b9e50b0b26c15c5d";
const ZDPS_SKILL_OVERRIDES_SIZE: u64 = 157_049;

fn ps_quote(path: &Path) -> String {
    path.to_string_lossy().replace('\'', "''")
}

fn download_zdps_skill_overrides(out: &Path) -> PathBuf {
    let path = out.join("zdps_SkillOverrides.en.json");
    if !cfg!(windows) {
        // The app is Windows-only, but leave a deterministic empty catalog for
        // tooling that evaluates the build script on another host.
        fs::write(&path, "{}").expect("write non-Windows ZDPS skill placeholder");
        return path;
    }

    let url = format!(
        "https://raw.githubusercontent.com/Blue-Protocol-Source/BPSR-ZDPS/{ZDPS_COMMIT}/BPSR-ZDPS/Data/SkillOverrides.en.json"
    );
    let script = format!(
        "$ErrorActionPreference='Stop'; Invoke-WebRequest -UseBasicParsing -Uri '{url}' -OutFile '{}'; $n=(Get-Item '{}').Length; if ($n -ne {ZDPS_SKILL_OVERRIDES_SIZE}) {{ throw \"ZDPS SkillOverrides size mismatch: $n\" }}",
        ps_quote(&path),
        ps_quote(&path),
    );
    let status = Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &script])
        .status()
        .unwrap_or_else(|err| panic!("failed to launch PowerShell for ZDPS skill catalog: {err}"));
    assert!(status.success(), "failed to download pinned ZDPS skill catalog");
    path
}

fn generated_skill_name_function(path: &Path) -> String {
    let text = fs::read_to_string(path).expect("read ZDPS SkillOverrides.en.json");
    let value: serde_json::Value = serde_json::from_str(&text).expect("parse ZDPS SkillOverrides.en.json");
    let object = value.as_object().expect("ZDPS SkillOverrides top-level object");

    let mut entries: Vec<(i32, String)> = object
        .iter()
        .filter_map(|(id, data)| {
            let id = id.parse::<i32>().ok()?;
            if id == 0 { return None; }
            let name = data.get("Name")?.as_str()?.trim();
            if name.is_empty() { return None; }
            Some((id, name.to_owned()))
        })
        .collect();
    entries.sort_by_key(|entry| entry.0);

    if cfg!(windows) {
        assert!(entries.len() >= 400, "unexpectedly small ZDPS English skill catalog: {}", entries.len());
    }

    let mut out = String::from(
        "/// Complete player-facing English skill-id catalog generated at build time from\n\
         /// BPSR-ZDPS Data/SkillOverrides.en.json at a pinned MIT-licensed commit.\n\
         /// Unknown/future ids remain visible instead of being hidden.\n\
         fn skill_name(id: i32) -> String {\n    match id {\n        0 => \"Unknown\".into(),\n"
    );
    for (id, name) in entries {
        // serde_json's string serializer gives us a quoted/escaped UTF-8 literal
        // suitable for these ordinary game display names.
        let quoted = serde_json::to_string(&name).expect("serialize skill name");
        out.push_str(&format!("        {id} => {quoted}.into(),\n"));
    }
    out.push_str(
        "        _ if (3_898..=3_999).contains(&id) => imagine_info(id).0,\n\
         _ => format!(\"Skill {id}\"),\n\
         }\n}\n\n"
    );
    out
}

fn replace_skill_name_function(source: &mut String, replacement: &str) {
    let start = "/// Frequently observed player skill names sourced from BPSR-ZDPS's MIT-licensed";
    let end = "fn consumable_info(id: i32) -> Option<(ConsumableKind, &'static str)> {";
    let start_count = source.matches(start).count();
    assert_eq!(start_count, 1, "full skill catalog start anchor expected once, found {start_count}");
    let begin = source.find(start).expect("skill catalog start checked");
    let rel_end = source[begin..].find(end).expect("skill catalog end anchor");
    source.replace_range(begin..begin + rel_end, replacement);
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let catalog = download_zdps_skill_overrides(&out);
    let replacement = generated_skill_name_function(&catalog);

    let telemetry_path = out.join("telemetry_v170_fixed.rs");
    let mut telemetry = fs::read_to_string(&telemetry_path).expect("read generated telemetry for full skill catalog");
    replace_skill_name_function(&mut telemetry, &replacement);
    fs::write(&telemetry_path, telemetry).expect("write generated telemetry with full ZDPS skill catalog");

    println!("cargo:rerun-if-changed=build_v183_skillnames.rs");
}
