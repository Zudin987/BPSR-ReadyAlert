// Follow-up R03/R04/R05/R08/R09/R14/R18. Apply to the final generated Rust,
// rather than altering HTML, exported names, telemetry or approved row colours.
use std::{env, fs, path::{Path, PathBuf}};
mod previous {
    include!("build_v1382_ui_audit_raid_navigation.rs");
    pub fn run() { main(); }
}
fn exact(src:&mut String, old:&str, new:&str, expected:usize, id:&str) {
    let found=src.matches(old).count();
    assert_eq!(found,expected,"follow-up {id}: expected {expected} native anchors, found {found}");
    *src=src.replace(old,new);
    assert_eq!(src.matches(new).count(),expected,"follow-up {id}: replacement missing or ambiguous");
}
fn write(out:&Path,name:&str, edit:impl FnOnce(&mut String)) {
    let path=out.join(name);
    let mut source=fs::read_to_string(&path).unwrap_or_else(|e|panic!("follow-up read {name}: {e}"));
    edit(&mut source);
    fs::write(path,source).unwrap_or_else(|e|panic!("follow-up write {name}: {e}"));
}
fn main(){
    previous::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    write(&out,"feature_overlays_v170_fixed.rs",|src|{
        exact(src,
            "&if row.is_local { format!(\"YOU · {}\", reference_player_display_name(row)) } else { reference_player_display_name(row) },",
            "&reference_player_display_name(row),",3,"R04 original name in every layout");
        exact(src,
            "label: if state.history_index.is_some() { \"↩ Live\" } else { \"Live\" }",
            "label: \"Live\"",2,"R03 single unadorned history control");
        exact(src,"ToolbarAction::More => \"Choose layout; history and settings are below the divider\",",
            "ToolbarAction::More => \"Layout and actions\",",1,"R13 short layout tooltip");
        exact(src,"(\"T#\", col.left + 4, col.left + 24, 0)",
            "(\"#\", col.left + 4, col.left + 24, 0)",1,"R09 rank header in Raid");
        exact(src,"(\"T#\", r.left + 3, r.left + rank_w, DT_RIGHT)",
            "(\"#\", r.left + 3, r.left + rank_w, DT_RIGHT)",1,"R09 rank header in Compact Raid");
        exact(src,"&format!(\"READYALERT // ENTITY   {}\",row.name)",
            "&format!(\"Player details — {}\",row.name)",1,"R18 player-first detail title");
        exact(src,"feature_label(hwnd,0,\"Maximum rows\",20,398,90,22)",
            "feature_label(hwnd,0,\"Max rows\",20,398,90,22)",1,"R08 complete concise row-limit label");
    });
    write(&out,"ui_meter_qa_tests_raid_v1345.rs",|src|{
        exact(src,"\"↩ Live\"","\"Live\"",1,"R03 historical navigation regression expectation");
    });
    write(&out,"overlay_v150_v1181.rs",|src|{
        exact(src,
            "ReadyAlert is listening for BPSR chat messages on the shared capture pipeline.",
            "New chat messages will appear here.",1,"R14 player-facing waiting copy");
        exact(src,"let tts_width = chat_header_label_width(settings, 7, TTS_WIDTH, 22);",
            "let tts_width = chat_header_label_width(settings, 3, TTS_WIDTH, 12);",1,"R05 constant three-character width and tab reclamation");
        exact(src,"if !enabled { \"TTS OFF\" } else if !usable { \"TTS !\" } else { \"TTS ON\" }",
            "let _ = (enabled, usable); \"TTS\"",1,"R05 stable compact toggle label");
        exact(src,"assert_eq!(audit_tts_label(false, false), \"TTS OFF\");",
            "assert_eq!(audit_tts_label(false, false), \"TTS\");",1,"R05 off-state regression");
        exact(src,"assert_eq!(audit_tts_label(false, true), \"TTS OFF\");",
            "assert_eq!(audit_tts_label(false, true), \"TTS\");",1,"R05 off-configured regression");
        exact(src,"assert_eq!(audit_tts_label(true, false), \"TTS !\");",
            "assert_eq!(audit_tts_label(true, false), \"TTS\");",1,"R05 unavailable-state regression");
        exact(src,"assert_eq!(audit_tts_label(true, true), \"TTS ON\");",
            "assert_eq!(audit_tts_label(true, true), \"TTS\");",1,"R05 on-state regression");
        exact(src,"draw_toolbar_button(hdc,actions.tts,audit_tts_label(speech.tts_enabled,usable),tts_back,tts_text);",
            "draw_followup_tts_toggle(hdc,actions.tts,speech.tts_enabled,usable,tts_back,tts_text);",1,"R05 strong visible pressed/unpressed treatment");
        src.push_str(r#"
// R05: the fixed-width TTS action has independent background, border and
// underline cues. Its text never changes width when speech settings change.
unsafe fn draw_followup_tts_toggle(hdc:HDC,rect:RECT,enabled:bool,usable:bool,back:u32,fore:u32){
    draw_toolbar_button(hdc,rect,"TTS",back,fore);
    if enabled {
        let edge=if usable {crate::ui_modern::BPSR_ACCENT_HOVER}else{crate::ui_modern::BPSR_WARNING};
        crate::ui_modern::stroke_round_rect(hdc,rect,edge,CHAT_HEADER_RADIUS,1);
        fill(hdc,&RECT{left:rect.left+7,top:rect.bottom-3,right:rect.right-7,bottom:rect.bottom-1},edge);
    }
}
#[cfg(test)]
mod september_followup_tts_tests {
    use super::*;
    #[test]fn toggle_label_and_width_are_independent_of_enabled_state(){
        assert_eq!(audit_tts_label(false,false),audit_tts_label(true,true));
        let s=AppSettings::default();
        let a=action_rects(640,&s);
        assert_eq!(a.tts.right-a.tts.left,chat_header_label_width(&s,3,TTS_WIDTH,12));
        assert!(a.tts.left>=a.add.right);
    }
}
"#);
    });
    println!("cargo:rerun-if-changed=build/legacy/build_v1383_followup_preferences.rs");
}
