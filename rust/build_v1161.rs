use std::{env, fs, path::{Path, PathBuf}};

mod prior {
    include!("build_v1160_compilefix.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.16.1 patch {label} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn replace_between(source: &mut String, start: &str, end: &str, replacement: &str, label: &str) {
    let start_count = source.matches(start).count();
    let end_count = source.matches(end).count();
    assert_eq!(start_count, 1, "v1.16.1 patch {label} start expected one match, found {start_count}");
    assert_eq!(end_count, 1, "v1.16.1 patch {label} end expected one match, found {end_count}");
    let a = source.find(start).expect("v1.16.1 start anchor");
    let b = source[a..].find(end).map(|i| a + i).expect("v1.16.1 end anchor");
    source.replace_range(a..b, replacement);
}

fn patch_overlay(out: &Path) {
    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read generated v1.16 overlay");

    let identity_layout = r#"#[derive(Clone,Copy)]struct DpsRowLayout{name_left:i32,name_right:i32,spec_left:i32,spec_right:i32,badge_left:i32,badge_count:usize,total_left:i32,total_right:i32,active_left:i32,active_right:i32,share_left:i32,share_right:i32,death_left:i32,death_right:i32}
fn identity_text_px(text:&str,bold:bool)->i32{text.chars().map(|ch|if ch.is_ascii(){if bold{8}else{7}}else{14}).sum::<i32>().max(0)}
fn dps_secondary(row:&DpsRow)->String{let spec=dps_spec(row);let score=dps_score_pair(row);if score.is_empty(){spec}else if spec.is_empty(){score}else{format!("{} · {}",spec,score)}}
fn dps_row_layout(r:RECT,show_imagines:bool,row:&DpsRow)->DpsRowLayout{let width=(r.right-r.left).max(1);let death_w=30;let share_w=56;let total_w=88;let active_w=90;let gap=5;let death_right=r.right-4;let death_left=death_right-death_w;let share_right=death_left-gap;let share_left=share_right-share_w;let active_right=share_left-gap;let active_left=active_right-active_w;let total_right=active_left-gap;let total_left=total_right-total_w;let name_left=r.left+25;let mut badge_count=if show_imagines&&!row.is_dead&&width>=690{row.imagines.len().min(2)}else{0};let mut badge_span=badge_count as i32*BADGE_W+(badge_count.saturating_sub(1)as i32)*BADGE_GAP;while badge_count>0&&(total_left-badge_span-14-name_left)<150{badge_count-=1;badge_span=badge_count as i32*BADGE_W+(badge_count.saturating_sub(1)as i32)*BADGE_GAP;}let identity_right=if badge_count>0{(total_left-badge_span-14).max(name_left+40)}else{(total_left-8).max(name_left+40)};let name_w=identity_text_px(&row.name,true).clamp(36,180);let name_right=(name_left+name_w).min(identity_right);let spec_left=(name_right+7).min(identity_right);let secondary=dps_secondary(row);let spec_w=identity_text_px(&secondary,false).clamp(0,220);let spec_right=(spec_left+spec_w).min(identity_right);let badge_left=if badge_count>0{(spec_right+8).min(total_left-badge_span-6)}else{total_left};DpsRowLayout{name_left,name_right,spec_left,spec_right,badge_left,badge_count,total_left,total_right,active_left,active_right,share_left,share_right,death_left,death_right}}
"#;
    replace_between(
        &mut source,
        "#[derive(Clone,Copy)]struct DpsRowLayout",
        "fn dps_identity(row:&DpsRow)->String{",
        identity_layout,
        "left anchored identity group",
    );

    replace_once(
        &mut source,
        "fn active_share(row:&DpsRow,mode:SortMode,settings:&FeatureSettings)->Option<(f64,u32)>{match mode{SortMode::Damage if settings.meter.show_damage_share=>Some((row.damage_share,rgb(218,102,102))),SortMode::Heal if settings.meter.show_healing_share=>Some((row.healing_share,rgb(91,190,126))),SortMode::Tank if settings.meter.show_tank_share=>Some((row.tank_share,rgb(91,151,205))),_=>None}}",
        "fn active_share(row:&DpsRow,mode:SortMode,settings:&FeatureSettings)->Option<(f64,u32)>{match mode{SortMode::Damage if settings.meter.show_damage_share=>Some((row.damage_share,rgb(255,0,0))),SortMode::Heal if settings.meter.show_healing_share=>Some((row.healing_share,rgb(0,255,0))),SortMode::Tank if settings.meter.show_tank_share=>Some((row.tank_share,rgb(0,0,255))),_=>None}}",
        "pure mode share colors",
    );

    let badge_loops = source.matches("row.imagines.iter().take(2)").count();
    assert_eq!(badge_loops, 2, "v1.16.1 patch Imagine loops expected two matches, found {badge_loops}");
    source = source.replace("row.imagines.iter().take(2)", "row.imagines.iter().take(layout.badge_count)");

    replace_once(
        &mut source,
        "let bar_bottom=if row.is_local{r.bottom-4}else{r.bottom};let bar_top=(bar_bottom-6).max(r.top+3);let content_bottom=bar_top-1;",
        "let bar_bottom=if row.is_local{r.bottom-2}else{r.bottom};let bar_top=(bar_bottom-3).max(r.top+3);let content_bottom=bar_top-1;",
        "thinner progress bar",
    );
    replace_once(
        &mut source,
        "let color=match state.sort_mode{SortMode::Damage=>rgb(190,82,82),SortMode::Heal=>rgb(70,166,104),SortMode::Tank=>rgb(71,128,181)};",
        "let color=match state.sort_mode{SortMode::Damage=>rgb(255,0,0),SortMode::Heal=>rgb(0,255,0),SortMode::Tank=>rgb(0,0,255)};",
        "pure mode progress colors",
    );
    replace_once(
        &mut source,
        "if row.is_local{outline(hdc,r,rgb(66,211,190),4);",
        "if row.is_local{outline(hdc,r,rgb(255,255,255),2);",
        "white local player frame",
    );

    fs::write(path, source).expect("write v1.16.1 overlay");
}

fn patch_telemetry(out: &Path) {
    let path = out.join("telemetry_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read generated v1.16 telemetry");

    replace_once(
        &mut source,
        "    combat: HashMap<i64, CombatActor>,\n    local_uid: i64,",
        "    combat: HashMap<i64, CombatActor>,\n    trusted_imagine_tiers: HashMap<(i64, i32), i32>,\n    local_uid: i64,",
        "trusted Imagine tier cache field",
    );
    replace_once(
        &mut source,
        "            combat: HashMap::new(),\n            local_uid: 0,",
        "            combat: HashMap::new(),\n            trusted_imagine_tiers: HashMap::new(),\n            local_uid: 0,",
        "trusted Imagine tier cache init",
    );
    replace_once(
        &mut source,
        "        self.entities.clear();\n        self.combat.clear();",
        "        self.entities.clear();\n        self.combat.clear();\n        self.trusted_imagine_tiers.clear();",
        "clear trusted Imagine tiers on scene reset",
    );

    let imagine_merge = r#"    fn record_imagine_observed(&mut self, uid: i64, skill: i32, tier: i32) {
        let tier=tier.max(0);
        // ATTR_SKILL_REMODEL_LEVEL is authoritative. A legitimate explicit T0
        // must stay sticky for the whole scene; a later summon/delta or damage
        // fallback must not turn it into T1. Likewise, keep the first trusted
        // non-zero observation stable unless a later explicit T0 corrects it.
        let trusted=match self.trusted_imagine_tiers.entry((uid,skill)){
            std::collections::hash_map::Entry::Vacant(entry)=>*entry.insert(tier),
            std::collections::hash_map::Entry::Occupied(mut entry)=>{
                if tier==0&&*entry.get()!=0{*entry.get_mut()=0;}
                *entry.get()
            }
        };
        self.combat.entry(uid).or_default().imagines.insert(skill,trusted);
    }

    fn record_imagine_fallback(&mut self, uid: i64, skill: i32, tier: i32) {
        if let Some(trusted)=self.trusted_imagine_tiers.get(&(uid,skill)).copied(){
            self.combat.entry(uid).or_default().imagines.insert(skill,trusted);
            return;
        }
        self.combat.entry(uid).or_default().imagines.entry(skill).or_insert(tier.max(0));
    }

"#;
    replace_between(
        &mut source,
        "    fn record_imagine_observed(&mut self, uid: i64, skill: i32, tier: i32) {",
        "    fn reset_encounter_keep_roster(&mut self) {",
        imagine_merge,
        "scene-sticky authoritative Imagine tiers",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1161_imagine_tier_tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn trusted_t0_survives_encounter_reset_then_fallback_t1() {
        let (tx, _rx) = mpsc::channel();
        let mut runtime = TelemetryRuntime::new(tx);
        runtime.record_imagine_observed(42, 3921, 0);
        runtime.reset_encounter_keep_roster();
        runtime.record_imagine_fallback(42, 3921, 1);
        assert_eq!(runtime.combat.get(&42).and_then(|a| a.imagines.get(&3921)).copied(), Some(0));
    }

    #[test]
    fn later_conflicting_observed_t1_cannot_promote_trusted_t0() {
        let (tx, _rx) = mpsc::channel();
        let mut runtime = TelemetryRuntime::new(tx);
        runtime.record_imagine_observed(42, 3921, 0);
        runtime.record_imagine_observed(42, 3921, 1);
        assert_eq!(runtime.combat.get(&42).and_then(|a| a.imagines.get(&3921)).copied(), Some(0));
    }

    #[test]
    fn explicit_t0_can_correct_an_earlier_trusted_nonzero_observation() {
        let (tx, _rx) = mpsc::channel();
        let mut runtime = TelemetryRuntime::new(tx);
        runtime.record_imagine_observed(42, 3921, 1);
        runtime.record_imagine_observed(42, 3921, 0);
        assert_eq!(runtime.combat.get(&42).and_then(|a| a.imagines.get(&3921)).copied(), Some(0));
    }

    #[test]
    fn trusted_t5_survives_encounter_reset_then_fallback_t1() {
        let (tx, _rx) = mpsc::channel();
        let mut runtime = TelemetryRuntime::new(tx);
        runtime.record_imagine_observed(42, 3921, 5);
        runtime.reset_encounter_keep_roster();
        runtime.record_imagine_fallback(42, 3921, 1);
        assert_eq!(runtime.combat.get(&42).and_then(|a| a.imagines.get(&3921)).copied(), Some(5));
    }
}
"#);

    fs::write(path, source).expect("write v1.16.1 telemetry");
}

fn main() {
    prior::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_overlay(&out);
    patch_telemetry(&out);
    println!("cargo:rerun-if-changed=build_v1161.rs");
}
