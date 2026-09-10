use std::{env, fs, path::{Path, PathBuf}};

mod v1170 {
    include!("build_v1170.rs");

    pub fn run_prior() {
        prior::run();
    }

    pub fn apply_patches(out: &Path) {
        patch_settings(out);
        patch_event_tracker(out);
        patch_feature_settings(out);
    }
}

fn normalize_generated(path: &Path) {
    let source = fs::read_to_string(path).expect("read generated v1.17 source for line-ending normalization");
    if source.contains("\r\n") {
        fs::write(path, source.replace("\r\n", "\n")).expect("normalize generated v1.17 source line endings");
    }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.17 final polish {label} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn final_polish(out: &Path) {
    let event_path = out.join("event_tracker_ui_v1160_fixed.rs");
    let mut event = fs::read_to_string(&event_path).expect("read v1.17 Event Tracker for final polish");
    replace_once(
        &mut event,
        "BPSR ReadyAlert // Event Tracker",
        "BPSR ReadyAlert - Event Tracker",
        "Event Tracker window title",
    );
    replace_once(
        &mut event,
        "BPSR ReadyAlert - Custom Event Tracker (saved)",
        "BPSR ReadyAlert - Event Tracker",
        "stable Event Tracker title after save",
    );
    fs::write(event_path, event).expect("write v1.17 Event Tracker final polish");

    let overlay_path = out.join("feature_overlays_v170_fixed.rs");
    let mut overlay = fs::read_to_string(&overlay_path).expect("read v1.17 feature overlays for final polish");
    replace_once(
        &mut overlay,
        "fill(hdc,&rc,rgb(18,22,27));",
        "fill(hdc,&rc,crate::ui_theme::BG);",
        "overlay background token",
    );
    replace_once(
        &mut overlay,
        "if state.collapsed{SetTextColor(hdc,rgb(66,211,190));",
        "if state.collapsed{SetTextColor(hdc,crate::ui_theme::ACCENT);",
        "collapsed overlay accent",
    );
    replace_once(
        &mut overlay,
        "unsafe fn paint_toolbar(hdc:HDC,rc:RECT,state:&State){let toolbar=RECT{left:0,top:0,right:rc.right,bottom:TOOLBAR_H};fill(hdc,&toolbar,rgb(20,27,33));fill(hdc,&RECT{left:0,top:TOOLBAR_H-2,right:rc.right,bottom:TOOLBAR_H},rgb(66,211,190));let title_right=if state.kind==Kind::Dps{toolbar_action_rects(rc.right)[0].0.left-5}else{rc.right-BUTTON_W*3-5};SetTextColor(hdc,rgb(235,242,245));",
        "unsafe fn paint_toolbar(hdc:HDC,rc:RECT,state:&State){let toolbar=RECT{left:0,top:0,right:rc.right,bottom:TOOLBAR_H};fill(hdc,&toolbar,crate::ui_theme::SIDEBAR);fill(hdc,&RECT{left:0,top:TOOLBAR_H-2,right:rc.right,bottom:TOOLBAR_H},crate::ui_theme::ACCENT);let title_right=if state.kind==Kind::Dps{toolbar_action_rects(rc.right)[0].0.left-5}else{rc.right-BUTTON_W*3-5};SetTextColor(hdc,crate::ui_theme::TEXT);",
        "combat toolbar design tokens",
    );
    replace_once(
        &mut overlay,
        "SetTextColor(hdc,rgb(170,187,198));draw(hdc,\"Settings\"",
        "SetTextColor(hdc,crate::ui_theme::TEXT_SECONDARY);draw(hdc,\"Settings\"",
        "combat toolbar secondary text",
    );
    fs::write(overlay_path, overlay).expect("write v1.17 feature overlay final polish");
}

fn main() {
    // Build the proven v1.16.7 generated sources first, then normalize CRLF
    // before applying v1.17's exact structural patches. GitHub's Windows runner
    // writes generated sources with CRLF while local artifact inspection often
    // transparently normalizes them; keeping this boundary explicit makes the
    // generator deterministic across Windows/local validation.
    v1170::run_prior();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    for name in [
        "settings_ui_v1160_fixed.rs",
        "event_tracker_ui_v1160_fixed.rs",
        "feature_overlays_v170_fixed.rs",
    ] {
        normalize_generated(&out.join(name));
    }
    v1170::apply_patches(&out);
    final_polish(&out);

    println!("cargo:rerun-if-changed=build_v1170_winfix.rs");
    println!("cargo:rerun-if-changed=build_v1170.rs");
    println!("cargo:rerun-if-changed=ui_v1170/settings_layout.txt");
    println!("cargo:rerun-if-changed=ui_v1170/event_tracker_layout.txt");
    println!("cargo:rerun-if-changed=ui_v1170/feature_settings_layout.txt");
    println!("cargo:rerun-if-changed=src/ui_theme.rs");
}
