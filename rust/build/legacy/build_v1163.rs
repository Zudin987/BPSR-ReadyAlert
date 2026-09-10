use std::{env, fs, path::{Path, PathBuf}};

mod prior {
    include!("build_v1162.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.16.3 patch {label} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn replace_between(source: &mut String, start: &str, end: &str, replacement: &str, label: &str) {
    let start_count = source.matches(start).count();
    let end_count = source.matches(end).count();
    assert_eq!(start_count, 1, "v1.16.3 patch {label} start expected one match, found {start_count}");
    assert_eq!(end_count, 1, "v1.16.3 patch {label} end expected one match, found {end_count}");
    let a = source.find(start).expect("v1.16.3 start anchor");
    let b = source[a..].find(end).map(|i| a + i).expect("v1.16.3 end anchor");
    source.replace_range(a..b, replacement);
}

fn patch_overlay(out: &Path) {
    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read generated v1.16.3 overlay");

    replace_once(&mut source, "const DPS_ROW_H: i32 = 36;", "const DPS_ROW_H: i32 = 33;", "compact row height");
    replace_once(&mut source,
        "if x>=bx&&x<bx+BADGE_W&&y>=r.top+4&&y<r.top+27",
        "if x>=bx&&x<bx+BADGE_W&&y>=r.top+2&&y<r.top+25",
        "compact Imagine hover bounds");
    replace_once(&mut source,
        "paint_badge(hdc,bx,r.top+4,badge);",
        "paint_badge(hdc,bx,r.top+2,badge);",
        "compact Imagine vertical alignment");

    let colors = r#"fn profession_color(id:i32)->u32{match id{1=>hex_color(0x433558),2=>hex_color(0x315f5c),3=>hex_color(0x64382c),4=>hex_color(0x315862),5=>hex_color(0x435d36),8=>hex_color(0x4d4a3b),9=>hex_color(0x5e5237),11=>hex_color(0x625d3c),12=>hex_color(0x45565a),13=>hex_color(0x5e3740),14=>hex_color(0x36556a),15=>hex_color(0x6a4031),_=>rgb(40,47,56)}}
fn spec_color(row:&DpsRow)->u32{let spec=row.subprofession_name.trim().to_ascii_lowercase();match spec.as_str(){"iaido"|"iai"=>hex_color(0x513c72),"moonstrike"=>hex_color(0x332944),"icicle"|"ice spear"=>hex_color(0x3f7773),"frostbeam"|"crystal"=>hex_color(0x315e5b),"formless"|"voidflame"=>hex_color(0x7b412f),"crimson"|"blazecrimson"=>hex_color(0x5b2b22),"vanguard"=>hex_color(0x376f76),"skyward"=>hex_color(0x254653),"smite"=>hex_color(0x526c3f),"lifebind"=>hex_color(0x33482b),"earthfort"=>hex_color(0x6d5f3d),"block"=>hex_color(0x453e29),"wildpack"|"taming"=>hex_color(0x756f4c),"falconry"=>hex_color(0x625b2e),"recovery"=>hex_color(0x536568),"shield"=>hex_color(0x374649),"dissonance"=>hex_color(0x70434d),"concerto"=>hex_color(0x4f292e),_=>profession_color(row.profession_id)}}
"#;
    replace_between(&mut source, "fn spec_color(row:&DpsRow)->u32{", "fn text_on(color:u32)->u32{", colors, "muted spec and class fallback palette");

    let tests = r#"#[cfg(test)]mod tests{use super::*;#[test]fn requested_spec_colors_are_dark_and_exact(){let row=|name:&str|DpsRow{subprofession_name:name.into(),..DpsRow::default()};assert_eq!(spec_color(&row("Iaido")),hex_color(0x513c72));assert_eq!(spec_color(&row("Formless")),hex_color(0x7b412f));assert_eq!(spec_color(&row("Crimson")),hex_color(0x5b2b22));assert_eq!(spec_color(&row("Concerto")),hex_color(0x4f292e));}#[test]fn heavy_guardian_has_class_color_before_spec_is_known(){let row=DpsRow{profession_id:9,..DpsRow::default()};assert_eq!(spec_color(&row),hex_color(0x5e5237));assert_ne!(spec_color(&row),rgb(40,47,56));}#[test]fn sort_modes_order_expected_metric(){let snapshot=DpsSnapshot{rows:vec![DpsRow{name:"A".into(),damage:5,healing:50,damage_taken:1,..DpsRow::default()},DpsRow{name:"B".into(),damage:50,healing:5,damage_taken:100,..DpsRow::default()}],..DpsSnapshot::default()};assert_eq!(sorted_rows(&snapshot,SortMode::Damage)[0].name,"B");assert_eq!(sorted_rows(&snapshot,SortMode::Heal)[0].name,"A");assert_eq!(sorted_rows(&snapshot,SortMode::Tank)[0].name,"B");}}
"#;
    replace_between(&mut source, "#[cfg(test)]mod tests{use super::*;", "\nfn refresh_view_detail", tests, "palette regression tests");

    fs::write(path, source).expect("write v1.16.3 overlay");
}

fn patch_telemetry(out: &Path) {
    let path = out.join("telemetry_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read generated v1.16.3 telemetry");

    let skill_helpers = r#"fn decode_skill_level_entries(raw:&[u8])->Vec<(i32,i32,i32)>{
    proto::len_fields(raw,1).into_iter().filter_map(|entry|{
        let skill=i32::try_from(proto::get_varint_field(entry,1)?).ok()?;
        let current=proto::get_varint_field(entry,2).unwrap_or(0).min(i32::MAX as u64)as i32;
        let remodel=proto::get_varint_field(entry,3).unwrap_or(0).min(i32::MAX as u64)as i32;
        Some((skill,current,remodel))
    }).collect()
}
fn decode_imagine_skill_tiers(raw:&[u8])->Vec<(i32,i32)>{Self::decode_skill_level_entries(raw).into_iter().filter_map(|(skill,_,tier)|is_fantasy_skill(skill).then_some((skill,tier))).collect()}
fn spec_from_skill_id(skill:i32)->Option<(i32,i32,&'static str)>{Some(match skill{
    1714|1734=>(1001,1,"Iaido"),1737|1715|1738|179906=>(1002,1,"Moonstrike"),
    120902|120901=>(2001,2,"Icicle"),1241=>(2002,2,"Frostbeam"),
    1605|160102|2208181|2208172=>(3001,3,"Formless"),1606|1621|1622|35104=>(3002,3,"Crimson"),
    1405|1418=>(4001,4,"Vanguard"),1419=>(4002,4,"Skyward"),
    1518|1541|21402=>(5001,5,"Smite"),20301=>(5002,5,"Lifebind"),
    1922|1941|2201240=>(9001,9,"Earthfort"),1930|1931|1934|1935=>(9002,9,"Block"),
    2292|1700820|1700825|1700827=>(11001,11,"Wildpack"),220112|2203622|220106=>(11002,11,"Falconry"),
    2405|2411|2206401=>(12001,12,"Recovery"),2406|55412|55417=>(12002,12,"Shield"),
    2306|2321|2335=>(13001,13,"Dissonance"),2307|2301|2336|2361|55302=>(13002,13,"Concerto"),
    _=>return None,
})}
fn subprofession_name(id:i32)->&'static str{match id{1001=>"Iaido",1002=>"Moonstrike",2001=>"Icicle",2002=>"Frostbeam",3001=>"Formless",3002=>"Crimson",4001=>"Vanguard",4002=>"Skyward",5001=>"Smite",5002=>"Lifebind",9001=>"Earthfort",9002=>"Block",11001=>"Wildpack",11002=>"Falconry",12001=>"Recovery",12002=>"Shield",13001=>"Dissonance",13002=>"Concerto",_=>""}}
fn infer_spec_from_entries(entries:&[(i32,i32,i32)],profession:i32)->Option<(i32,i32,&'static str)>{
    fn unique(entries:&[(i32,i32,i32)],profession:i32,active_only:bool)->Option<(i32,i32,&'static str)>{
        let mut found:Option<(i32,i32,&'static str)>=None;
        for &(skill,current,_) in entries{
            if active_only&&current<=0{continue;}
            let Some(spec)=TelemetryRuntime::spec_from_skill_id(skill)else{continue;};
            if profession>0&&spec.1!=profession{continue;}
            match found{None=>found=Some(spec),Some(previous)if previous.0==spec.0=>{},Some(_)=>return None}
        }
        found
    }
    unique(entries,profession,true).or_else(||unique(entries,profession,false))
}
fn apply_player_skill_list(&mut self,uid:i64,entries:&[(i32,i32,i32)]){
    let profession=self.players.get(&uid).map(|p|p.profession_id).unwrap_or(0);
    if let Some((sub_id,profession_id,_))=Self::infer_spec_from_entries(entries,profession){let player=self.players.entry(uid).or_default();player.subprofession_id=sub_id;if player.profession_id==0{player.profession_id=profession_id;}}
    let imagines:Vec<(i32,i32)>=entries.iter().filter_map(|&(skill,_,tier)|is_fantasy_skill(skill).then_some((skill,tier.max(0)))).collect();
    if entries.len()>=10{
        self.trusted_imagine_tiers.retain(|(owner,_),_|*owner!=uid);
        let combat=self.combat.entry(uid).or_default();combat.imagines.clear();
        for(skill,tier)in imagines.into_iter().take(2){self.trusted_imagine_tiers.insert((uid,skill),tier);combat.imagines.insert(skill,tier);}
    }else{
        for(skill,tier)in imagines{
            let exists=self.trusted_imagine_tiers.contains_key(&(uid,skill));
            let count=self.trusted_imagine_tiers.keys().filter(|(owner,_)|*owner==uid).count();
            if exists||count<2{self.trusted_imagine_tiers.insert((uid,skill),tier);self.combat.entry(uid).or_default().imagines.insert(skill,tier);}
        }
    }
}

"#;
    replace_between(&mut source, "fn decode_imagine_skill_tiers(raw:&[u8])->Vec<(i32,i32)>{", "    fn update_entity_from_attrs", skill_helpers, "SkillLevel list spec and Imagine helpers");

    replace_once(&mut source,
        "            let mut actor_state_seen = false;\n            let mut player_imagine_tiers: Vec<(i32,i32)> = Vec::new();",
        "            let mut actor_state_seen = false;\n            let mut player_skill_entries: Option<Vec<(i32,i32,i32)>> = None;",
        "stage player SkillLevel list");
    replace_once(&mut source,
        "                        ATTR_SKILL_LEVEL_ID_LIST => {\n                            player_imagine_tiers.extend(Self::decode_imagine_skill_tiers(raw));\n                        }",
        "                        ATTR_SKILL_LEVEL_ID_LIST => {\n                            player_skill_entries=Some(Self::decode_skill_level_entries(raw));\n                        }",
        "capture player SkillLevel list");
    replace_once(&mut source,
        "            for (skill,tier) in player_imagine_tiers {\n                let tier=tier.max(0);\n                self.trusted_imagine_tiers.insert((uid,skill),tier);\n                if let Some(combat)=self.combat.get_mut(&uid){combat.imagines.insert(skill,tier);}\n            }",
        "            if let Some(entries)=player_skill_entries.as_deref(){self.apply_player_skill_list(uid,entries);}",
        "apply player SkillLevel metadata");

    replace_once(&mut source,
        "        let skill = proto::get_varint_field(damage, 12).unwrap_or(0) as i32;\n        crate::event_tracker::observe_skill",
        "        let skill = proto::get_varint_field(damage, 12).unwrap_or(0) as i32;\n        if attacker_uid>0{if let Some((sub_id,profession_id,_))=Self::spec_from_skill_id(skill){let player=self.players.entry(attacker_uid).or_default();player.subprofession_id=sub_id;if player.profession_id==0{player.profession_id=profession_id;}}}\n        crate::event_tracker::observe_skill",
        "CN canonical combat spec fallback");

    replace_once(&mut source,
        "                // Adapter infers the active specialization from observed profession skills.\n                subprofession_name: String::new(),",
        "                subprofession_name: Self::subprofession_name(meta.subprofession_id).to_string(),",
        "emit pre-combat specialization name");

    source.push_str(r#"

#[cfg(test)]
mod v1163_roster_metadata_tests {
    use super::*;
    use std::sync::mpsc;

    fn full_entries(extra:&[(i32,i32,i32)])->Vec<(i32,i32,i32)>{let mut out=vec![(100,1,0),(101,1,0),(102,1,0),(103,1,0),(104,1,0),(105,1,0),(106,1,0),(107,1,0),(108,1,0),(109,1,0)];out.extend_from_slice(extra);out}

    #[test]
    fn instant_spec_skill_list_detects_heavy_guardian_branches(){
        let earth=full_entries(&[(1922,1,0)]);let block=full_entries(&[(1930,1,0)]);
        assert_eq!(TelemetryRuntime::infer_spec_from_entries(&earth,9).map(|x|x.2),Some("Earthfort"));
        assert_eq!(TelemetryRuntime::infer_spec_from_entries(&block,9).map(|x|x.2),Some("Block"));
    }

    #[test]
    fn instant_spec_does_not_guess_ambiguous_skill_list(){
        let mixed=full_entries(&[(1922,1,0),(1930,1,0)]);
        assert!(TelemetryRuntime::infer_spec_from_entries(&mixed,9).is_none());
    }

    #[test]
    fn imagine_tier_full_skill_list_replaces_changed_loadout(){
        let (tx,_rx)=mpsc::channel();let mut runtime=TelemetryRuntime::new(tx);runtime.players.entry(42).or_default().profession_id=5;runtime.combat.entry(42).or_default();
        runtime.apply_player_skill_list(42,&full_entries(&[(1518,1,0),(3921,1,0),(3923,1,5)]));
        assert_eq!(runtime.combat[&42].imagines.len(),2);assert_eq!(runtime.combat[&42].imagines.get(&3921),Some(&0));
        runtime.apply_player_skill_list(42,&full_entries(&[(1518,1,0),(3949,1,2),(3968,1,4)]));
        assert_eq!(runtime.combat[&42].imagines.len(),2);assert!(!runtime.combat[&42].imagines.contains_key(&3921));assert_eq!(runtime.combat[&42].imagines.get(&3949),Some(&2));assert_eq!(runtime.combat[&42].imagines.get(&3968),Some(&4));
    }

    #[test]
    fn imagine_tier_partial_list_updates_known_slot_without_adding_third(){
        let (tx,_rx)=mpsc::channel();let mut runtime=TelemetryRuntime::new(tx);runtime.combat.entry(42).or_default();runtime.apply_player_skill_list(42,&full_entries(&[(3921,1,0),(3923,1,5)]));
        runtime.apply_player_skill_list(42,&[(3921,1,2)]);assert_eq!(runtime.combat[&42].imagines.get(&3921),Some(&2));
        runtime.apply_player_skill_list(42,&[(3949,1,1)]);assert_eq!(runtime.combat[&42].imagines.len(),2);assert!(!runtime.combat[&42].imagines.contains_key(&3949));
    }
}
"#);

    fs::write(path, source).expect("write v1.16.3 telemetry");
}

fn main() {
    prior::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_overlay(&out);
    patch_telemetry(&out);
    println!("cargo:rerun-if-changed=build_v1163.rs");
}
