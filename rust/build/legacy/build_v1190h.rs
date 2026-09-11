use std::{env,fs,path::{Path,PathBuf}};

mod prior { include!("build_v1190g.rs"); pub fn run(){main();} }

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){let count=source.matches(from).count();assert_eq!(count,1,"build_v1190h patch {label} expected one match, found {count}");*source=source.replacen(from,to,1);}

fn patch_feature_overlay(out:&Path){
    let path=out.join("feature_overlays_v170_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read responsive overlay source").replace("\r\n","\n");
    replace_once(&mut source,
        r###"unsafe fn dps_secondary_compact_measured(hdc:HDC,row:&DpsRow,max_px:i32)->String{if max_px<=0{return String::new();}let spec=dps_spec(row);let full=dps_secondary(row);if dps_text_width(hdc,&full,identity_text_px(&full,false))<=max_px{return full;}let primary=if row.ability_score>0{score(row.ability_score)}else{String::new()};let medium=if spec.is_empty(){primary.clone()}else if primary.is_empty(){spec.clone()}else{format!("{} · {}",spec,primary)};if dps_text_width(hdc,&medium,identity_text_px(&medium,false))<=max_px{return medium;}if !spec.is_empty()&&dps_text_width(hdc,&spec,identity_text_px(&spec,false))<=max_px{return spec;}if spec.is_empty()&&dps_text_width(hdc,&primary,identity_text_px(&primary,false))<=max_px{return primary;}String::new()}"###,
        r###"unsafe fn dps_secondary_compact_measured(hdc:HDC,row:&DpsRow,max_px:i32)->String{
    if max_px<=0{return String::new();}
    let spec=dps_spec(row);let primary=if row.ability_score>0{score(row.ability_score)}else{String::new()};let break_score=if row.illusion_break>0{format!("+{}",score(row.illusion_break))}else{String::new()};
    let full=if spec.is_empty(){[primary.clone(),break_score.clone()].into_iter().filter(|s|!s.is_empty()).collect::<Vec<_>>().join(" ")}else{let score_pair=[primary.clone(),break_score.clone()].into_iter().filter(|s|!s.is_empty()).collect::<Vec<_>>().join(" ");if score_pair.is_empty(){spec.clone()}else{format!("{} · {}",spec,score_pair)}};
    let candidates=[full,if spec.is_empty(){primary.clone()}else if primary.is_empty(){spec.clone()}else{format!("{} · {}",spec,primary)},spec.clone(),primary.clone()];
    for candidate in candidates{if !candidate.is_empty()&&dps_text_width(hdc,&candidate,identity_text_px(&candidate,false))<=max_px{return candidate;}}
    String::new()
}"###,
        "measured secondary retention order");
    source.push_str(r###"

#[cfg(test)]
mod v1190_secondary_priority_tests{
    use super::*;
    #[test]
    fn score_format_keeps_primary_before_break(){
        let row=DpsRow{ability_score:58_000,illusion_break:3_210,..DpsRow::default()};
        assert_eq!(score(row.ability_score),"58k");
        assert_eq!(format!("+{}",score(row.illusion_break)),"+3210");
    }
}
"###);
    fs::write(path,source).expect("write secondary retention order");
}

fn main(){prior::run();let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));patch_feature_overlay(&out);println!("cargo:rerun-if-changed=build/legacy/build_v1190h.rs");}
