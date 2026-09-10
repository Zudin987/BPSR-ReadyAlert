use std::{env, fs, path::{Path, PathBuf}, process::Command};

mod previous {
    include!("build_v190_compile.rs");
    pub fn run() { main(); }
}

const ZDPS_COMMIT: &str = "cfeb58c0acc85bc17181b413b9e50b0b26c15c5d";

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){let count=source.matches(from).count();assert_eq!(count,1,"v1.10.0 patch `{label}` expected one match, found {count}");*source=source.replacen(from,to,1);}
fn replace_between(source:&mut String,start:&str,end:&str,replacement:&str,label:&str){let count=source.matches(start).count();assert_eq!(count,1,"v1.10.0 patch `{label}` start expected one match, found {count}");let begin=source.find(start).expect("start checked");let rel_end=source[begin..].find(end).unwrap_or_else(||panic!("v1.10.0 patch `{label}` end anchor missing"));source.replace_range(begin..begin+rel_end,replacement);}
fn ps_quote(path:&Path)->String{path.to_string_lossy().replace('\'',"''")}

fn download_buff_catalog(out:&Path)->PathBuf{
    let path=out.join("zdps_BuffOverrides.en.json");
    if !cfg!(windows){fs::write(&path,"{}").expect("write non-Windows buff placeholder");return path;}
    let url=format!("https://raw.githubusercontent.com/Blue-Protocol-Source/BPSR-ZDPS/{ZDPS_COMMIT}/BPSR-ZDPS/Data/BuffOverrides.en.json");
    let script=format!("$ErrorActionPreference='Stop'; Invoke-WebRequest -UseBasicParsing -Uri '{url}' -OutFile '{}'; if ((Get-Item '{}').Length -lt 10000) {{ throw 'ZDPS BuffOverrides download unexpectedly small' }}",ps_quote(&path),ps_quote(&path));
    let status=Command::new("powershell").args(["-NoProfile","-ExecutionPolicy","Bypass","-Command",&script]).status().unwrap_or_else(|err|panic!("failed to launch PowerShell for ZDPS buff catalog: {err}"));
    assert!(status.success(),"failed to download pinned ZDPS buff catalog");
    path
}

fn read_named_entries(path:&Path,skip_bufftype_zero:bool)->Vec<(i32,String)>{
    let text=fs::read_to_string(path).unwrap_or_else(|err|panic!("read {}: {err}",path.display()));
    let value:serde_json::Value=serde_json::from_str(&text).unwrap_or_else(|err|panic!("parse {}: {err}",path.display()));
    let object=value.as_object().unwrap_or_else(||panic!("{} top-level object",path.display()));
    let mut entries:Vec<(i32,String)>=object.iter().filter_map(|(id,data)|{
        let id=id.parse::<i32>().ok()?;if id==0{return None;}
        if skip_bufftype_zero&&data.get("BuffType").and_then(|v|v.as_i64())==Some(0){return None;}
        let name=data.get("Name")?.as_str()?.trim();if name.is_empty(){return None;}
        Some((id,name.to_owned()))
    }).collect();
    entries.sort_by_key(|x|x.0);entries.dedup_by_key(|x|x.0);entries
}

fn generate_analysis_names(out:&Path){
    let skill_path=out.join("zdps_SkillOverrides.en.json");
    if !skill_path.exists(){panic!("v1.10 expected pinned ZDPS skill catalog from previous build layer");}
    let buff_path=download_buff_catalog(out);
    let skills=read_named_entries(&skill_path,false);
    let buffs=read_named_entries(&buff_path,true);
    if cfg!(windows){
        assert!(skills.len()>=400,"unexpectedly small skill catalog: {}",skills.len());
        assert!(buffs.len()>=100,"unexpectedly small buff catalog: {}",buffs.len());
        assert!(buffs.iter().any(|(id,name)|*id==55226&&!name.is_empty()),"expected known buff 55226");
    }
    let mut generated=String::from("pub fn skill_name(id:i32)->String{match id{\n0=>\"Unknown\".into(),\n");
    for(id,name)in skills{let q=serde_json::to_string(&name).expect("quote skill");generated.push_str(&format!("{id}=>{q}.into(),\n"));}
    generated.push_str("_=>format!(\"Skill {id}\"),}}\n\npub fn buff_name(id:i32)->Option<&'static str>{match id{\n");
    for(id,name)in buffs{let q=serde_json::to_string(&name).expect("quote buff");generated.push_str(&format!("{id}=>Some({q}),\n"));}
    generated.push_str("_=>None,}}\n");
    fs::write(out.join("analysis_names_v1100.rs"),generated).expect("write v1.10 analysis names");
}

fn main(){
    previous::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    generate_analysis_names(&out);

    // New v1.10 model fields are additive and serde-defaulted for old history,
    // but the compact v1.8 telemetry core uses explicit struct literals.
    let telemetry_path=out.join("telemetry_v170_fixed.rs");
    let mut telemetry=fs::read_to_string(&telemetry_path).expect("read generated v1.9 telemetry");
    replace_once(&mut telemetry,
        "                active_dtps: 0.0,\n                is_party: false,\n                attributes,",
        "                active_dtps: 0.0,\n                is_party: false,\n                boss_damage: 0,\n                effective_healing: 0,\n                overhealing: 0,\n                block_pct: 0,\n                buff_uptimes: Vec::new(),\n                death_recaps: Vec::new(),\n                taken_sources: Vec::new(),\n                attributes,",
        "v1.10 DpsRow defaults");
    replace_once(&mut telemetry,
        "            total_damage_taken: total_taken,\n            target,",
        "            total_damage_taken: total_taken,\n            total_boss_damage: 0,\n            target,",
        "v1.10 DpsSnapshot defaults");
    fs::write(&telemetry_path,telemetry).expect("write v1.10 telemetry defaults");

    let overlay_path=out.join("feature_overlays_v170_fixed.rs");
    let mut overlay=fs::read_to_string(&overlay_path).expect("read generated v1.9 overlay");
    replace_once(&mut overlay,
        "enum SortMode {\nDamage,\nHeal,\nTank,\n}",
        "enum SortMode {\nDamage,\nHeal,\nTank,\n}\n#[derive(Clone, Copy, Debug, Eq, PartialEq)]\nenum DetailMode { Damage, Heal, Tank, Buffs, Deaths }",
        "detail analysis mode");
    replace_once(&mut overlay,"mode: SortMode,\nencounter_ms: u64,","mode: DetailMode,\nencounter_ms: u64,","DetailState analysis mode");
    replace_between(&mut overlay,"unsafe fn open_detail","unsafe fn open_feature_settings",include_str!("overlay_v1100_detail_patch.txt"),"combat analysis inspector");
    fs::write(&overlay_path,overlay).expect("write v1.10 overlay");

    println!("cargo:rerun-if-changed=build_v1100.rs");
    println!("cargo:rerun-if-changed=overlay_v1100_detail_patch.txt");
    println!("cargo:rerun-if-changed=src/telemetry_v1100.rs");
}
