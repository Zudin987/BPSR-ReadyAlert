use std::{env, fs, path::{Path, PathBuf}};

mod prior {
    include!("build_v1260_encounter_archive.rs");
    pub fn run() { main(); }
}

fn replace_between(source: &mut String, start: &str, end: &str, replacement: &str, label: &str) {
    let start_count = source.matches(start).count();
    assert_eq!(start_count, 1, "v1.27 patch {label} start expected one match, found {start_count}");
    let begin = source.find(start).expect("v1.27 start anchor");
    let rel_end = source[begin..].find(end)
        .unwrap_or_else(|| panic!("v1.27 patch {label} end anchor missing"));
    source.replace_range(begin..begin + rel_end, replacement);
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.27 patch {label} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn patch_overlay(out: &Path) {
    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read v1.26 generated overlay").replace("\r\n", "\n");

    // Keep the mature v1.26 adaptive toolbar/layout machinery. Only extend its
    // action strip with Benchmark and use compact one-letter labels once the
    // overlay leaves the Comfortable tier.
    replace_between(
        &mut source,
        "fn toolbar_action_rects_responsive(right:i32,scale:i32)->[(RECT,&'static str);6]{",
        "fn dps_identity",
        r#"fn toolbar_action_rects_responsive(right:i32,scale:i32)->[(RECT,&'static str);7]{
    let tier=dps_layout_tier(right.max(1),scale);let button=dps_toolbar_button_w(right,scale);
    let(arrow,live,raid,copy,benchmark,reset,gap,label_raid,label_live,label_copy,label_benchmark,label_reset)=match tier{
        DpsLayoutTier::Comfortable=>(30,46,44,48,76,52,4,"Raid","Live","Copy","Benchmark","Reset"),
        DpsLayoutTier::Compact=>(24,30,28,28,28,28,3,"R","L","C","B","R"),
        DpsLayoutTier::Dense=>(20,28,26,26,26,26,2,"R","L","C","B","R"),
        DpsLayoutTier::Minimum=>(18,26,24,24,24,24,1,"R","L","C","B","R")
    };
    let mut x=right-button*3;
    let mut take=|w:i32,label:&'static str|{let r=RECT{left:x-w,top:5,right:x,bottom:29};x-=w+gap;(r,label)};
    let reset_r=take(reset,label_reset);let benchmark_r=take(benchmark,label_benchmark);let copy_r=take(copy,label_copy);
    let newer=take(arrow,">");let live_r=take(live,label_live);let older=take(arrow,"<");let raid_r=take(raid,label_raid);
    [raid_r,older,live_r,newer,copy_r,benchmark_r,reset_r]
}
"#,
        "responsive meter actions",
    );

    replace_once(
        &mut source,
        "fn overlay_header(kind:Kind)->&'static str{if kind==Kind::Mechanics{\"Tracker & Mech\"}else{\"DPSMETER\"}}",
        "fn overlay_header(kind:Kind)->&'static str{if kind==Kind::Mechanics{\"Tracker & Mech\"}else{\"Meter\"}}",
        "Meter header label",
    );

    replace_once(
        &mut source,
        "match index{0=>toggle_raid_mode(hwnd,state),1=>history_older(state),2=>{state.history_index=None;state.scroll=0;},3=>history_newer(state),4=>copy_view_image(hwnd,state),5=>{if state.history_index.is_none(){archive_live_snapshot(state);crate::telemetry::request_manual_reset();state.dps=DpsSnapshot::default();state.scroll=0;}else{state.history_index=None;state.scroll=0;}},_=>{}}",
        "match index{0=>toggle_raid_mode(hwnd,state),1=>history_older(state),2=>{state.history_index=None;state.scroll=0;},3=>history_newer(state),4=>copy_view_image(hwnd,state),5=>crate::telemetry::benchmark_ui::show(hwnd),6=>{if state.history_index.is_none(){archive_live_snapshot(state);crate::telemetry::request_manual_reset();state.dps=DpsSnapshot::default();state.scroll=0;}else{state.history_index=None;state.scroll=0;}},_=>{}}",
        "Benchmark toolbar routing",
    );

    replace_once(
        &mut source,
        "match index{0=>\"Toggle 20-player Raid Mode\",1=>\"Older encounter\",2=>\"Return to live encounter\",3=>\"Newer encounter\",4=>\"Copy the full DPS Meter as an image\",5=>\"Reset the live encounter\",_=>\"Combat action\"}",
        "match index{0=>\"Toggle 20-player Raid Mode\",1=>\"Older encounter\",2=>\"Return to live encounter\",3=>\"Newer encounter\",4=>\"Copy the full DPS Meter as an image\",5=>\"Set up a timed benchmark\",6=>\"Reset the live encounter\",_=>\"Combat action\"}",
        "toolbar help text",
    );

    replace_once(
        &mut source,
        "assert_eq!(overlay_header(Kind::Dps), \"DPSMETER\");",
        "assert_eq!(overlay_header(Kind::Dps), \"Meter\");",
        "historical header regression expectation",
    );

    source.push_str(r#"
#[cfg(test)]
mod v1270_toolbar_tests{
    use super::*;
    #[test]
    fn benchmark_action_is_between_copy_and_reset(){
        let actions=toolbar_action_rects_responsive(900,100);
        let labels:Vec<_>=actions.iter().map(|(_,label)|*label).collect();
        assert_eq!(labels,vec!["Raid","<","Live",">","Copy","Benchmark","Reset"]);
    }
    #[test]
    fn narrow_toolbar_uses_requested_short_labels(){
        let actions=toolbar_action_rects_responsive(500,100);
        let labels:Vec<_>=actions.iter().map(|(_,label)|*label).collect();
        assert_eq!(labels,vec!["R","<","L",">","C","B","R"]);
        for pair in actions.windows(2){assert!(pair[0].0.right<=pair[1].0.left);}
        assert!(actions[0].0.left>=0);
    }
}
"#);

    fs::write(path, source).expect("write v1.27 generated overlay");
}

fn main() {
    prior::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_overlay(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1270_benchmark_history.rs");
    println!("cargo:rerun-if-changed=src/benchmark_ui.rs");
    println!("cargo:rerun-if-changed=src/encounter_archive_v1270.rs");
    println!("cargo:rerun-if-changed=src/encounter_context_v1270.rs");
    println!("cargo:rerun-if-changed=src/telemetry_v1270.rs");
}
