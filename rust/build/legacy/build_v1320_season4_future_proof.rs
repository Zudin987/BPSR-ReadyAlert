use std::{env, fs, path::{Path, PathBuf}};

mod previous {
    include!("build_v1317_meter_retention_chat_buffer.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.32.0 future-proof patch {label:?} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn patch_telemetry(out: &Path) {
    let path = out.join("telemetry_v170_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read telemetry for v1.32.0 future-proofing: {e}"))
        .replace("\r\n", "\n");

    replace_once(
        &mut source,
        "const SYNC_TO_ME_DELTA_INFO: u32 = 0x2e;",
        "const SYNC_TO_ME_DELTA_INFO: u32 = 0x2e;\n// Generic seasonal stat family. Keep the serialized DpsRow field name for old\n// history compatibility, but treat its meaning as Season Strength from here on.\nconst ATTR_SEASON_STRENGTH_TOTAL: i32 = 0x2cb1;",
        "season-strength total constant",
    );

    replace_once(
        &mut source,
        "fn subprofession_name(id:i32)->&'static str{match id{1001=>\"Iaido\",1002=>\"Moonstrike\",2001=>\"Icicle\",2002=>\"Frostbeam\",3001=>\"Formless\",3002=>\"Crimson\",4001=>\"Vanguard\",4002=>\"Skyward\",5001=>\"Smite\",5002=>\"Lifebind\",9001=>\"Earthfort\",9002=>\"Block\",11001=>\"Wildpack\",11002=>\"Falconry\",12001=>\"Recovery\",12002=>\"Shield\",13001=>\"Dissonance\",13002=>\"Concerto\",_=>\"\"}}",
        "fn subprofession_name(id:i32)->&'static str{if let Some(name)=crate::telemetry::game_names_v1300::override_spec_name(id){return name;}match id{1001=>\"Iaido\",1002=>\"Moonstrike\",2001=>\"Icicle\",2002=>\"Frostbeam\",3001=>\"Formless\",3002=>\"Crimson\",4001=>\"Vanguard\",4002=>\"Skyward\",5001=>\"Smite\",5002=>\"Lifebind\",9001=>\"Earthfort\",9002=>\"Block\",11001=>\"Wildpack\",11002=>\"Falconry\",12001=>\"Recovery\",12002=>\"Shield\",13001=>\"Dissonance\",13002=>\"Concerto\",_=>crate::telemetry::game_names_v1300::spec_name(id).unwrap_or(\"\")}}",
        "future spec naming",
    );

    replace_once(
        &mut source,
        "                        x if x == ATTR_SEASON_STRENGTH as u64 => {\n                            if let Some(value) = value {\n                                player.illusion_break = value;\n                                player.attrs.insert(ATTR_SEASON_STRENGTH, value);\n                            }\n                        }",
        "                        x if x == ATTR_SEASON_STRENGTH as u64 => {\n                            if let Some(value) = value {\n                                // 11440 is the protocol's final Season Strength and always wins.\n                                player.illusion_break = value;\n                                player.attrs.insert(ATTR_SEASON_STRENGTH, value);\n                            }\n                        }\n                        x if x == ATTR_SEASON_STRENGTH_TOTAL as u64 => {\n                            if let Some(value) = value {\n                                // Some future builds may synchronize only the generic total (11441).\n                                // Use it as a fallback, but never overwrite an observed final 11440.\n                                player.attrs.insert(ATTR_SEASON_STRENGTH_TOTAL, value);\n                                if !player.attrs.contains_key(&ATTR_SEASON_STRENGTH) {\n                                    player.illusion_break = value;\n                                }\n                            }\n                        }",
        "season-strength 11440/11441 fallback",
    );

    replace_once(
        &mut source,
        "            name: exact_monster_name(meta.monster_id)\n                .map(str::to_string)\n                .or_else(|| (!meta.name.trim().is_empty()).then(|| meta.name.clone()))\n                .or_else(|| {\n                    crate::telemetry::game_names_v1300::monster_name(meta.monster_id)\n                        .map(str::to_owned)\n                })\n                .unwrap_or_else(|| \"Unknown Target\".into()),",
        "            name: crate::telemetry::game_names_v1300::override_monster_name(meta.monster_id)\n                .map(str::to_owned)\n                .or_else(|| exact_monster_name(meta.monster_id).map(str::to_string))\n                .or_else(|| (!meta.name.trim().is_empty()).then(|| meta.name.clone()))\n                .or_else(|| {\n                    crate::telemetry::game_names_v1300::monster_name(meta.monster_id)\n                        .map(str::to_owned)\n                })\n                .unwrap_or_else(|| if meta.monster_id!=0{format!(\"Unknown Target ({})\",meta.monster_id)}else{\"Unknown Target\".into()}),",
        "target override and numeric fallback",
    );

    replace_once(
        &mut source,
        "fn skill_name(id: i32) -> String {\n    match id {",
        "fn skill_name(id: i32) -> String {\n    if let Some(name)=crate::telemetry::game_names_v1300::override_skill_name(id){return name.to_owned();}\n    match id {",
        "skill override precedence",
    );

    replace_once(
        &mut source,
        "_ => crate::telemetry::game_names_v1300::skill_name(id)\n    .map(str::to_owned)\n    .unwrap_or_else(|| format!(\"Skill {id}\")),",
        "_ => crate::telemetry::game_names_v1300::skill_name(id)\n    .map(str::to_owned)\n    .unwrap_or_else(|| format!(\"Unknown Skill ({id})\")),",
        "unknown skill numeric fallback",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1320_future_season_tests {
    use super::*;

    #[test]
    fn season_strength_final_and_total_ids_are_adjacent_generic_family() {
        assert_eq!(ATTR_SEASON_STRENGTH, 11_440);
        assert_eq!(ATTR_SEASON_STRENGTH_TOTAL, 11_441);
    }

    #[test]
    fn unknown_skill_keeps_numeric_identity() {
        let id=i32::MAX;
        assert_eq!(skill_name(id),format!("Unknown Skill ({id})"));
    }
}
"#);

    fs::write(path, source).unwrap_or_else(|e| panic!("write telemetry v1.32.0 future-proofing: {e}"));
}

fn patch_analysis_names(out: &Path) {
    let path = out.join("analysis_names_v1100.rs");
    let mut source = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read analysis names for v1.32.0: {e}"))
        .replace("\r\n", "\n");

    replace_once(
        &mut source,
        "pub fn skill_name(id:i32)->String{match id{",
        "pub fn skill_name(id:i32)->String{if let Some(name)=crate::telemetry::game_names_v1300::override_skill_name(id){return name.to_owned();}match id{",
        "analysis skill override precedence",
    );
    replace_once(
        &mut source,
        "_=>crate::telemetry::game_names_v1300::skill_name(id).map(str::to_owned).unwrap_or_else(||format!(\"Skill {id}\")),}}",
        "_=>crate::telemetry::game_names_v1300::skill_name(id).map(str::to_owned).unwrap_or_else(||format!(\"Unknown Skill ({id})\")),}}",
        "analysis unknown skill numeric fallback",
    );
    replace_once(
        &mut source,
        "pub fn buff_name(id:i32)->Option<&'static str>{match id{",
        "pub fn buff_name(id:i32)->Option<&'static str>{if let Some(name)=crate::telemetry::game_names_v1300::override_buff_name(id){return Some(name);}match id{",
        "analysis buff override precedence",
    );

    fs::write(path, source).unwrap_or_else(|e| panic!("write analysis names v1.32.0: {e}"));
}

fn patch_feature_overlay(out: &Path) {
    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read feature overlay for v1.32.0: {e}"))
        .replace("\r\n", "\n");

    replace_once(
        &mut source,
        "fn dps_identity(row:&DpsRow)->String{let spec=if !row.subprofession_name.trim().is_empty(){row.subprofession_name.as_str()}else{profession_name(row.profession_id)};if spec.trim().is_empty(){row.name.clone()}else{format!(\"{} · {}\",row.name,spec)}}\nfn dps_spec(row:&DpsRow)->String{if !row.subprofession_name.trim().is_empty(){row.subprofession_name.clone()}else{profession_name(row.profession_id).to_string()}}",
        "fn dps_identity(row:&DpsRow)->String{let spec=dps_spec(row);if spec.trim().is_empty(){row.name.clone()}else{format!(\"{} · {}\",row.name,spec)}}\nfn dps_spec(row:&DpsRow)->String{if row.subprofession_id>0{if let Some(name)=crate::telemetry::game_names_v1300::override_spec_name(row.subprofession_id){return name.to_string();}}if !row.subprofession_name.trim().is_empty(){return row.subprofession_name.clone();}if row.subprofession_id>0{if let Some(name)=crate::telemetry::game_names_v1300::spec_name(row.subprofession_id){return name.to_string();}}if row.profession_id>0{if let Some(name)=crate::telemetry::game_names_v1300::override_class_name(row.profession_id){return name.to_string();}}let known=profession_name(row.profession_id);if !known.is_empty(){return known.to_string();}if row.profession_id>0{if let Some(name)=crate::telemetry::game_names_v1300::class_name(row.profession_id){return name.to_string();}return format!(\"Class {}\",row.profession_id);}String::new()}",
        "future class/spec display fallback",
    );

    replace_once(
        &mut source,
        "let row=&state.row;paint_popup_toolbar(hdc,rc,&format!(\"READYALERT // ENTITY   {}\",row.name));let hp_pct=if row.max_hp>0{format!(\"{:.1}%\",(row.hp.max(0)as f64*100.0/row.max_hp as f64).clamp(0.0,100.0))}else{\"?%\".into()};let spec=if row.subprofession_name.trim().is_empty(){profession_name(row.profession_id).to_string()}else{row.subprofession_name.clone()};",
        "let row=&state.row;paint_popup_toolbar(hdc,rc,&format!(\"READYALERT // ENTITY   {}\",row.name));let hp_pct=if row.max_hp>0{format!(\"{:.1}%\",(row.hp.max(0)as f64*100.0/row.max_hp as f64).clamp(0.0,100.0))}else{\"?%\".into()};let spec=dps_spec(row);",
        "detail popup future spec",
    );

    replace_once(
        &mut source,
        "format!(\"{}   •   UID {}   •   AS {} +{}\",spec,row.uid,row.ability_score,row.illusion_break)",
        "format!(\"{}   •   UID {}   •   AS {}   •   Season Strength {}\",spec,row.uid,row.ability_score,row.illusion_break)",
        "generic season-strength label",
    );

    replace_once(
        &mut source,
        "fn identity_tail(row:&DpsRow)->String{let spec=if !row.subprofession_name.trim().is_empty(){row.subprofession_name.as_str()}else{profession_name(row.profession_id)};if row.ability_score>0||row.illusion_break>0{format!(\"{} + {}  {}\",score(row.ability_score),score(row.illusion_break),spec)}else{spec.to_string()}}",
        "fn identity_tail(row:&DpsRow)->String{let spec=dps_spec(row);if row.ability_score>0||row.illusion_break>0{format!(\"{} + {}  {}\",score(row.ability_score),score(row.illusion_break),spec)}else{spec}}",
        "identity tail future spec",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1320_future_display_tests {
    use super::*;

    #[test]
    fn unknown_future_profession_remains_identifiable() {
        let row=DpsRow{profession_id:999,subprofession_id:0,..DpsRow::default()};
        assert_eq!(dps_spec(&row),"Class 999");
    }

    #[test]
    fn generic_season_strength_keeps_legacy_storage_field() {
        let row=DpsRow{illusion_break:4_321,..DpsRow::default()};
        assert_eq!(row.illusion_break,4_321);
    }
}
"#);

    fs::write(path, source).unwrap_or_else(|e| panic!("write feature overlay v1.32.0: {e}"));
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_telemetry(&out);
    patch_analysis_names(&out);
    patch_feature_overlay(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1320_season4_future_proof.rs");
}
