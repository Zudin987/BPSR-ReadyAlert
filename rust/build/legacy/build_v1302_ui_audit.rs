use std::{env, fs, path::{Path, PathBuf}};

mod previous {
    include!("build_v1300_idmap.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.30.2 UI audit patch `{label}` expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn replace_all(source: &mut String, from: &str, to: &str, expected: usize, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, expected, "v1.30.2 UI audit patch `{label}` expected {expected} matches, found {count}");
    *source = source.replace(from, to);
}

fn patch_generated_dialog_calls(path: &Path, expected_boxes: usize, expected_mist: usize, expected_dark_modern: usize, expected_dark_legacy: usize) {
    let mut source = fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    if expected_boxes > 0 {
        replace_all(&mut source,"crate::ui_modern::message_box_w(","crate::telemetry::ui_audit_v1302::message_box_w(",expected_boxes,&format!("{} message boxes",path.display()));
    }
    if expected_mist > 0 {
        replace_all(&mut source,"crate::ui_modern::mist_titlebar(","crate::telemetry::ui_audit_v1302::mist_titlebar(",expected_mist,&format!("{} mist titlebars",path.display()));
    }
    if expected_dark_modern > 0 {
        replace_all(&mut source,"crate::ui_modern::dark_titlebar(","crate::telemetry::ui_audit_v1302::dark_titlebar(",expected_dark_modern,&format!("{} modern dark titlebars",path.display()));
    }
    if expected_dark_legacy > 0 {
        replace_all(&mut source,"crate::ui_theme::dark_titlebar(","crate::telemetry::ui_audit_v1302::dark_titlebar(",expected_dark_legacy,&format!("{} legacy dark titlebars",path.display()));
    }
    fs::write(path, source).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
}

fn patch_feature_toolbar(out: &Path) {
    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read generated feature overlay for UI audit");
    replace_once(
        &mut source,
        r#"DpsLayoutTier::Compact=>(24,24,24,30,28,28,28,28,"R","L","Cp","B","R"),DpsLayoutTier::Dense=>(22,22,20,28,26,26,26,26,"R","L","Cp","B","R"),DpsLayoutTier::Minimum=>(22,22,18,26,24,24,24,24,"R","L","Cp","B","R")"#,
        r#"DpsLayoutTier::Compact=>(24,24,24,30,28,28,28,28,"Ra","L","Cp","B","Rs"),DpsLayoutTier::Dense=>(22,22,20,28,26,26,26,26,"Ra","L","Cp","B","Rs"),DpsLayoutTier::Minimum=>(22,22,18,26,24,24,24,24,"Ra","L","Cp","B","Rs")"#,
        "distinct narrow Raid/Reset labels",
    );
    replace_once(
        &mut source,
        r#"    else{SetTextColor(hdc,crate::ui_theme::TEXT);draw(hdc,&toolbar_title(state),RECT{left:10,top:0,right:action_left.max(48),bottom:TOOLBAR_H},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}"#,
        r#"    else{let title_right=action_left-4;if title_right>64{SetTextColor(hdc,crate::ui_theme::TEXT);draw(hdc,&toolbar_title(state),RECT{left:10,top:0,right:title_right,bottom:TOOLBAR_H},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}}"#,
        "hide normal toolbar title instead of painting beneath actions",
    );
    fs::write(path, source).expect("write generated feature overlay UI audit");
}

fn generate_benchmark(out: &Path) {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let source_path = manifest.join("src/benchmark_ui.rs");
    let source = fs::read_to_string(&source_path).expect("read benchmark UI source").replace("\r\n", "\n");
    assert_eq!(
        source.matches("crate::telemetry::ui_audit_v1302::mist_titlebar(hwnd);").count(),
        1,
        "v1.30.2 Benchmark source must use the audited small-rounded titlebar exactly once",
    );
    assert_eq!(
        source.matches("crate::telemetry::ui_audit_v1302::message_box_w(hwnd, body.as_ptr(), title.as_ptr(), MB_OK | MB_ICONWARNING);").count(),
        1,
        "v1.30.2 Benchmark source must use the audited warning dialog exactly once",
    );
    assert!(
        !source.contains("run_modal_window"),
        "v1.30.2 Benchmark must remain modeless; keyboard routing belongs in ui::dialog_message",
    );
    fs::write(out.join("benchmark_ui_v1302.rs"), source).expect("write generated benchmark UI audit");
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_generated_dialog_calls(&out.join("settings_ui_v1160_fixed.rs"),9,1,0,0);
    patch_generated_dialog_calls(&out.join("event_tracker_ui_v1160_fixed.rs"),2,1,0,0);
    patch_generated_dialog_calls(&out.join("feature_overlays_v170_fixed.rs"),0,0,0,2);
    patch_generated_dialog_calls(&out.join("overlay_v150_v1181.rs"),0,0,2,0);
    patch_generated_dialog_calls(&out.join("updater_v1241.rs"),2,0,0,0);
    patch_generated_dialog_calls(&out.join("win_v182_fixed.rs"),2,0,0,0);
    patch_feature_toolbar(&out);
    generate_benchmark(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1302_ui_audit.rs");
    println!("cargo:rerun-if-changed=src/ui_audit_v1302.rs");
    println!("cargo:rerun-if-changed=src/benchmark_ui.rs");
}
