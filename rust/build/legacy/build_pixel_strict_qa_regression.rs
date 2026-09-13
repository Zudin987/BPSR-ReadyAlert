use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_pixel_strict_qa.rs");
    pub fn run() { main(); }
}

fn require_all(source: &str, label: &str, needles: &[&str]) {
    for needle in needles {
        assert!(
            source.contains(needle),
            "strict QA {label} contract missing `{needle}`",
        );
    }
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let path = out.join("overlay_v150_v1181.rs");
    let mut source = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read chat overlay for strict QA regression expectation: {e}"))
        .replace("\r\n", "\n");
    let old = "#[test]fn tts_enabled_is_teal_and_disabled_is_neutral(){let(disabled,_)=tts_visual(false,true);let(enabled,_)=tts_visual(true,true);let(warning,_)=tts_visual(true,false);assert_eq!(disabled,crate::ui_modern::DARK_SURFACE);assert_eq!(enabled,dim_chat_color(crate::ui_modern::BPSR_ACCENT,62));assert_ne!(warning,crate::ui_modern::BPSR_DANGER);}";
    let new = "#[test]fn tts_states_follow_pixel_tonal_palette(){let(disabled,_)=tts_visual(false,true);let(enabled,_)=tts_visual(true,true);let(warning,_)=tts_visual(true,false);assert_eq!(disabled,crate::ui_modern::DARK_SURFACE);assert_eq!(enabled,crate::ui_modern::mix_color(crate::ui_modern::DARK_RAISED,crate::ui_modern::BPSR_ACCENT,28));assert_eq!(warning,crate::ui_modern::mix_color(crate::ui_modern::DARK_RAISED,crate::ui_modern::BPSR_WARNING,26));assert_ne!(warning,crate::ui_modern::BPSR_DANGER);}";
    let count = source.matches(old).count();
    assert_eq!(count, 1, "strict QA TTS visual regression target expected once, found {count}");
    source = source.replacen(old, new, 1);

    // Empty states are part of the visual contract, not optional copy. Keep the
    // structured Chat states from silently regressing back to a blank canvas.
    require_all(
        &source,
        "chat empty state",
        &["draw_empty_state(hdc, state", "Waiting for chat", "No messages in this tab"],
    );
    fs::write(path, source).unwrap_or_else(|e| panic!("write strict QA TTS regression expectation: {e}"));

    let feature_path = out.join("feature_overlays_v170_fixed.rs");
    let feature = fs::read_to_string(&feature_path)
        .unwrap_or_else(|e| panic!("read feature overlays for strict QA empty-state contract: {e}"))
        .replace("\r\n", "\n");
    require_all(
        &feature,
        "meter/mechanics empty state",
        &[
            "Waiting for combat data",
            "Waiting for raid data",
            "No active mechanics",
            "qa_draw_empty_state",
        ],
    );

    let settings_path = out.join("settings_ui_v1160_fixed.rs");
    let settings = fs::read_to_string(&settings_path)
        .unwrap_or_else(|e| panic!("read settings UI for strict QA empty-state contract: {e}"))
        .replace("\r\n", "\n");
    require_all(
        &settings,
        "settings empty list",
        &["qa_theme_empty_list(c,empty_kind)", "ID_BLOCKED_LIST", "ID_TAB_LIST"],
    );

    let tracker_path = out.join("event_tracker_ui_v1160_fixed.rs");
    let tracker = fs::read_to_string(&tracker_path)
        .unwrap_or_else(|e| panic!("read Event Tracker UI for strict QA empty-state contract: {e}"))
        .replace("\r\n", "\n");
    require_all(&tracker, "event tracker empty list", &["qa_theme_empty_list(c,3)"]);

    println!("cargo:rerun-if-changed=build/legacy/build_pixel_strict_qa_regression.rs");
}
