use std::{env, fs, path::{Path, PathBuf}};

mod base {
    include!("build_v1320_season4_future_proof_fix.rs");
    pub fn run() { main(); }
}

mod tracker_patch {
    include!("build_v1321_freeze_hardening.rs");
    pub fn run_event_tracker(manifest: &std::path::Path, out: &std::path::Path) {
        patch_event_tracker(manifest, out);
    }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.32.1 freeze hardening fix {label:?} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn patch_tracker_ui(out: &Path) {
    let path = out.join("event_tracker_ui_v1160_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .expect("read generated Event Tracker UI for v1.32.1 fix")
        .replace("\r\n", "\n");

    replace_once(
        &mut source,
        "let rules=create_static(hwnd,\"Rules\",18,112,270,18);",
        "let rules=create_static(hwnd,\"Rules (up to 64)\",18,112,270,18);",
        "tracker UI rule label",
    );
    replace_once(&mut source, "state.working.rules.len()>=24", "state.working.rules.len()>=64", "tracker UI add limit");
    replace_once(&mut source, "state.working.rules.len()<24", "state.working.rules.len()<64", "tracker UI add enable limit");
    replace_once(
        &mut source,
        "set_combo_items(hwnd,ID_KIND,&[\"Buff ID\",\"Skill ID\"],0);",
        "set_combo_items(hwnd,ID_KIND,&[\"Buff ID\",\"Skill ID\",\"Attribute ID\"],0);",
        "tracker UI kind list",
    );
    replace_once(
        &mut source,
        "seconds (skills only)",
        "seconds (skill / attribute)",
        "tracker UI hold label",
    );
    replace_once(
        &mut source,
        "Buffs use the game timer. Skill rules stay visible for the configured display time.",
        "Buffs use the game timer. Skill / attribute rules stay visible for the configured display time.",
        "tracker UI help text",
    );
    replace_once(
        &mut source,
        "ID_KIND => { EnableWindow(GetDlgItem(hwnd,ID_HOLD),(combo_sel(hwnd,ID_KIND)==1) as i32); }",
        "ID_KIND => { EnableWindow(GetDlgItem(hwnd,ID_HOLD),(combo_sel(hwnd,ID_KIND)!=0) as i32); }",
        "tracker UI hold enable",
    );
    replace_once(
        &mut source,
        "(combo_sel(hwnd,ID_KIND)==1)!=(r.kind==TrackerKind::Skill)",
        "combo_sel(hwnd,ID_KIND)!=tracker_kind_index(r.kind)",
        "tracker UI dirty kind handling",
    );
    replace_once(
        &mut source,
        "(r.kind==TrackerKind::Skill && get_text(hwnd,ID_HOLD)!=r.hold_seconds.to_string())",
        "(r.kind!=TrackerKind::Buff && get_text(hwnd,ID_HOLD)!=r.hold_seconds.to_string())",
        "tracker UI dirty hold handling",
    );
    replace_once(
        &mut source,
        "unsafe fn load_selected(hwnd: HWND, state: &UiState) {",
        "fn tracker_kind_index(kind: TrackerKind) -> isize { match kind { TrackerKind::Buff=>0, TrackerKind::Skill=>1, TrackerKind::Attribute=>2 } }\n\nunsafe fn load_selected(hwnd: HWND, state: &UiState) {",
        "tracker UI kind helper",
    );
    replace_once(
        &mut source,
        "    SendMessageW(GetDlgItem(hwnd, ID_KIND), CB_SETCURSEL, if rule.kind == TrackerKind::Buff { 0 } else { 1 }, 0);",
        "    SendMessageW(GetDlgItem(hwnd, ID_KIND), CB_SETCURSEL, tracker_kind_index(rule.kind) as usize, 0);",
        "tracker UI load kind",
    );
    replace_once(
        &mut source,
        "    EnableWindow(GetDlgItem(hwnd,ID_HOLD),(rule.kind==TrackerKind::Skill) as i32);",
        "    EnableWindow(GetDlgItem(hwnd,ID_HOLD),(rule.kind!=TrackerKind::Buff) as i32);",
        "tracker UI load hold",
    );
    replace_once(
        &mut source,
        "    let Some(id)=positive_number(hwnd,ID_EVENT_ID,i32::MAX,\"Enter a positive numeric Buff or Skill ID.\") else{return false;};\n    let kind=if combo_sel(hwnd,ID_KIND)==1 {TrackerKind::Skill}else{TrackerKind::Buff};\n    let hold=if kind==TrackerKind::Skill {\n        let Some(n)=positive_number(hwnd,ID_HOLD,30,\"Skill display seconds must be from 1 to 30.\") else{return false;};n as u8\n    }else{state.working.rules[index].hold_seconds};",
        "    let Some(id)=positive_number(hwnd,ID_EVENT_ID,i32::MAX,\"Enter a positive numeric Buff, Skill or Attribute ID.\") else{return false;};\n    let kind=match combo_sel(hwnd,ID_KIND){1=>TrackerKind::Skill,2=>TrackerKind::Attribute,_=>TrackerKind::Buff};\n    let hold=if kind!=TrackerKind::Buff {\n        let Some(n)=positive_number(hwnd,ID_HOLD,30,\"Skill / attribute display seconds must be from 1 to 30.\") else{return false;};n as u8\n    }else{state.working.rules[index].hold_seconds};",
        "tracker UI save attribute kind",
    );

    fs::write(path, source).expect("write v1.32.1 Event Tracker UI fix");
}

fn main() {
    base::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    tracker_patch::run_event_tracker(&manifest, &out);
    patch_tracker_ui(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1321_freeze_hardening_fix.rs");
}
