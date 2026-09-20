// Native UI audit implementation against checked-in build generation chain.
// These anchors are validated against the Windows generated-source CI artifact;
// assert instead of silently dropping a promised change on a new build revision.
use std::{env, fs, path::PathBuf};
mod previous {
    include!("build_v1375_ui_ux_audit.rs");
    pub fn run() { main(); }
}

fn required(src: &mut String, from: &str, to: &str, expected: usize, id: &str) {
    let count = src.matches(from).count();
    assert_eq!(count, expected, "UI audit {id}: expected {expected} source anchor(s), found {count}");
    *src = src.replace(from, to);
    println!("cargo:warning=UI audit {id}: applied {count} checked source replacement(s)");
}
fn edit_output(out: &std::path::Path, name: &str, f: impl FnOnce(&mut String)) {
    let path = out.join(name);
    let mut src = fs::read_to_string(&path).unwrap_or_else(|e| panic!("UI audit {name}: {e}"));
    f(&mut src);
    fs::write(path, src).unwrap_or_else(|e| panic!("UI audit {name}: {e}"));
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));

    // H03: edit the ACTUAL chat rendering unit (rather than meter overlays).
    edit_output(&out, "overlay_v150_v1181.rs", |src| {
        required(src,
            "Recent chat exists, but none matches this tab's channels, level rule or filters.",
            "No recent messages match this tab.", 1, "H03 chat empty state");
    });

    // G01/G02: the primary settings use Apply, not immediate persistence.
    // The modal's documented Yes/No/Cancel IDs are 6/7/2, respectively.
    // Apply performs all current validations and leaves dirty=true on failure.
    edit_output(&out, "settings_ui_v1160_fixed.rs", |src| {
        required(src,
            "    create_button(hwnd, ID_CLOSE, \"Close\", 690, 405, 80, 30);",
            "    create_button(hwnd, ID_CLOSE, \"Close\", 690, 405, 80, 30);\n    let policy=create_control(hwnd,\"STATIC\",\"Apply to save changes; Close asks about unsaved edits.\",0,174,410,420,20,WS_CHILD|WS_VISIBLE|0x80,0);\n    crate::ui_theme::set_font(policy,crate::ui_theme::FontRole::Secondary);",
            1, "G01 explicit settings save policy");
        required(src,
            "unsafe fn close_settings(hwnd:HWND,state:&mut SettingsState){if state.dirty{let result=crate::ui_modern::qa_message_box_w(hwnd,wide(\"Discard unsaved settings changes?\").as_ptr(),wide(\"ReadyAlert Settings\").as_ptr(),0x00000004|0x00000030);if result!=6{return;}}DestroyWindow(hwnd);}",
            "unsafe fn close_settings(hwnd:HWND,state:&mut SettingsState){\n    if state.dirty {\n        let result=crate::ui_modern::qa_message_box_w(hwnd,wide(\"Save changes before closing?\").as_ptr(),wide(\"Unsaved settings\").as_ptr(),0x00000003|0x00000030);\n        match result {\n            6 => { apply(hwnd,state); if state.dirty { return; } },\n            7 => {}, // Explicitly discard unsaved work.\n            _ => return, // Escape, X, or Keep Editing always preserves work.\n        }\n    }\n    DestroyWindow(hwnd);\n}",
            1, "G02 save discard keep editing");
    });
    edit_output(&out, "ui_pixel_qa_v1316.rs", |src| {
        required(src,
            "QaDialogButton { id: QA_IDYES, label: \"Save\".into(), primary: false, danger: false }",
            "QaDialogButton { id: QA_IDYES, label: \"Save & close\".into(), primary: false, danger: false }",
            1, "G02 explicit save and close button");
    });

    // M01/S04/T01/T02/T04/I04 in their actual native meter/mechanics code.
    edit_output(&out, "feature_overlays_v170_fixed.rs", |src| {
        required(src,"feature_label(hwnd,0,\"Visible rows\",20,398,90,22)",
            "feature_label(hwnd,0,\"Maximum rows\",20,398,90,22)",1,"S02 maximum rows wording");
        required(src,"\"Scale: {}%\"","\"UI size: {}%\"",1,"S04 truthful UI-size label");
        required(src,"feature_label(hwnd,0,\"Text and controls keep minimum readable sizes.\",296,136,390,22);",
            "feature_label(hwnd,0,\"UI size affects text and controls; readable minimums apply.\",20,116,670,18);",
            1,"S04 scale helper beside setting");
        required(src,"\"No active mechanics\"","\"Waiting for mechanics\"",1,"T01 idle tracker state");
        required(src,"\"Tracker & Mech\"","\"Tracker & Mechanics\"",2,"T04 overlay title and associated test");
        required(src,"\"Dungeon Mechanics\"","\"Tracker & Mechanics\"",1,"T04 settings title");
        required(src,"format!(\"Tracked attributes  {} / {}\",f.mechanic_attributes.tracked.len(),feature_settings::MAX_TRACKED_ATTRIBUTES)",
            "format!(\"Tracked attributes  {} / {} · Choose up to 8{}\",f.mechanic_attributes.tracked.len(),feature_settings::MAX_TRACKED_ATTRIBUTES,if f.mechanic_attributes.tracked.len()>=feature_settings::MAX_TRACKED_ATTRIBUTES{\" · Limit reached; deselect one\"}else{\"\"})",
            1,"T02 inline selection-limit feedback");
        required(src,"\"Choose up to 8 stats. Food, Serum and Event Tracker rows appear automatically. Changes save immediately.\"",
            "\"Food, Serum and Event Tracker appear automatically. Changes save immediately.\"",1,"T02 remove repeated limit footer");
        // A disabled toolbar button continues to offer Return to live as its help,
        // but historical browsing explicitly labels the central button's state.
        required(src,"label: \"Live\"","label: if state.history_index.is_some() { \"↩ Live\" } else { \"Live\" }",2,"M01 history return control");
    });

    // T03/T04: event rules are a subfeature of Tracker & Mechanics. Explain
    // numeric IDs and game-timer semantics without claiming an ID catalogue.
    edit_output(&out, "event_tracker_ui_v1160_fixed.rs", |src| {
        required(src,"Show custom buff or skill events inside Dungeon Mechanics.",
            "Show custom buff or skill events inside Tracker & Mechanics.",1,"T04 event-tracker context");
        required(src,"Buffs use the game timer. Skill and attribute rules use Display time.",
            "Enter a numeric buff/skill ID from the game. Buffs use game time; skills and attributes use Display time.",1,"T03 inline ID and duration help");
    });

    println!("cargo:rerun-if-changed=build/legacy/build_v1376_ui_audit_copy.rs");
}
