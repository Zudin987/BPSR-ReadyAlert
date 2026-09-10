use std::{env, fs, path::{Path, PathBuf}};

mod prior {
    include!("build_v1165.rs");
    pub fn run() { main(); }
}
mod telemetry_patch { include!("build_v1166_telemetry.rs"); }
mod overlay_patch { include!("build_v1166_overlay.rs"); }

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){
    let count=source.matches(from).count();
    assert_eq!(count,1,"v1.16.6 patch {label} expected one match, found {count}");
    *source=source.replacen(from,to,1);
}
fn replace_all_checked(source:&mut String,from:&str,to:&str,expected:usize,label:&str){
    let count=source.matches(from).count();
    assert_eq!(count,expected,"v1.16.6 patch {label} expected {expected} matches, found {count}");
    *source=source.replace(from,to);
}
fn replace_between(source:&mut String,start:&str,end:&str,replacement:&str,label:&str){
    let start_count=source.matches(start).count();let end_count=source.matches(end).count();
    assert_eq!(start_count,1,"v1.16.6 patch {label} start expected one match, found {start_count}");
    assert_eq!(end_count,1,"v1.16.6 patch {label} end expected one match, found {end_count}");
    let a=source.find(start).expect("v1.16.6 start anchor");
    let b=source[a..].find(end).map(|i|a+i).expect("v1.16.6 end anchor");
    source.replace_range(a..b,replacement);
}
fn rust_string(value:&str)->String{value.replace('\\',"\\\\").replace('"',"\\\"")}
fn exact_consumable_catalog()->String{
    let manifest=PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let path=manifest.join("data/consumables_v1166.csv");
    let csv=fs::read_to_string(&path).expect("read data/consumables_v1166.csv");
    let mut arms=String::new();let mut food=0usize;let mut serum=0usize;let mut seen=std::collections::HashSet::new();
    for(line_no,raw)in csv.lines().enumerate().skip(1){
        let raw=raw.trim_end_matches('\r');if raw.trim().is_empty(){continue;}
        let(raw_name,raw_id)=raw.rsplit_once(',').unwrap_or_else(||panic!("invalid consumables CSV line {}",line_no+1));
        let id:i32=raw_id.trim().parse().unwrap_or_else(|_|panic!("invalid buff id on line {}",line_no+1));
        assert!(seen.insert(id),"duplicate consumable id {id}");
        let mut name=raw_name.trim().to_string();if name.starts_with('"')&&name.ends_with('"')&&name.len()>=2{name=name[1..name.len()-1].replace("\"\"","\"");}
        let(kind,label)=if let Some(label)=name.strip_prefix("[Food] "){food+=1;("Food",label)}else if let Some(label)=name.strip_prefix("[Serum] "){serum+=1;("Serum",label)}else{panic!("unknown consumable kind on CSV line {}: {name}",line_no+1)};
        arms.push_str(&format!("        {id} => ({kind}, \"{}\"),\n",rust_string(label)));
    }
    assert_eq!(food,160,"uploaded catalog Food count changed");assert_eq!(serum,210,"uploaded catalog Serum count changed");assert_eq!(seen.len(),370,"uploaded catalog total changed");
    format!("fn consumable_info(id: i32) -> Option<(ConsumableKind, &'static str)> {{\n    use ConsumableKind::{{Food, Serum}};\n    Some(match id {{\n{arms}        _ => return None,\n    }})\n}}\n\n")
}
fn main(){
    prior::run();let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    telemetry_patch::patch(&out);overlay_patch::patch(&out);
    println!("cargo:rerun-if-changed=build_v1166.rs");
    println!("cargo:rerun-if-changed=build_v1166_telemetry.rs");
    println!("cargo:rerun-if-changed=build_v1166_overlay.rs");
    println!("cargo:rerun-if-changed=build_v1166_popup.txt");
    println!("cargo:rerun-if-changed=build_v1166_ring_helpers.txt");
    println!("cargo:rerun-if-changed=data/consumables_v1166.csv");
}
