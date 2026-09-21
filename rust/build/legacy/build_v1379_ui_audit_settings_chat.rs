// Native UI audit: settings must open above the application's topmost overlays,
// and TTS must expose its effective state in text as well as by colour.
use std::{env, fs, path::{Path, PathBuf}};
mod previous {
    include!("build_v1378_ui_audit_scale.rs");
    pub fn run() { main(); }
}
fn required(out: &Path, name: &str, old: &str, new: &str, issue: &str) {
    let path = out.join(name);
    let mut src = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{name}: {e}"));
    assert_eq!(src.matches(old).count(), 1, "UI audit {issue}: missing or ambiguous native anchor");
    src = src.replacen(old, new, 1);
    fs::write(path, src).unwrap_or_else(|e| panic!("{name}: {e}"));
    println!("cargo:warning=UI audit {issue}: verified and applied");
}
fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let settings = "settings_ui_v1160_fixed.rs";
    // I01: settings and their owned dialogs are raised within the topmost z
    // group only while the settings window exists. Overlay topmost preferences
    // remain unchanged, and destroying settings restores normal app layering.
    required(&out, settings,
        "PostMessageW, RegisterClassW, SendMessageW, SetForegroundWindow, SetWindowLongPtrW,",
        "PostMessageW, RegisterClassW, SendMessageW, SetForegroundWindow, SetWindowPos, SetWindowLongPtrW,",
        "I01 Win32 z-order function import");
    required(&out, settings,
        "SetWindowTextW, ShowWindow, CREATESTRUCTW, CW_USEDEFAULT, GWLP_USERDATA, IDC_ARROW,",
        "SetWindowTextW, ShowWindow, CREATESTRUCTW, CW_USEDEFAULT, GWLP_USERDATA, IDC_ARROW, HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE, SWP_NOACTIVATE,",
        "I01 Win32 topmost constants import");
    required(&out, settings,
        "ShowWindow(existing, SW_SHOW);\n        SetForegroundWindow(existing);",
        "ShowWindow(existing, SW_SHOW);\n        SetWindowPos(existing, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);\n        SetForegroundWindow(existing);",
        "I01 raise reused settings above overlays");
    required(&out, settings,
        "ShowWindow(hwnd, SW_SHOW);\n    SetForegroundWindow(hwnd);",
        "ShowWindow(hwnd, SW_SHOW);\n    SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);\n    SetForegroundWindow(hwnd);",
        "I01 raise new settings above overlays");

    let chat = "overlay_v150_v1181.rs";
    // H02: use the existing effective-state calculation and tonal palette,
    // but do not force users to distinguish enabled/disabled by colour alone.
    required(&out, chat,
        "draw_toolbar_button(hdc,actions.tts,\"TTS\",tts_back,tts_text);",
        "draw_toolbar_button(hdc,actions.tts,audit_tts_label(speech.tts_enabled,usable),tts_back,tts_text);",
        "H02 visible chat speech-state label");
    let path = out.join(chat);
    let mut src = fs::read_to_string(&path).expect("chat source");
    src.push_str(r#"
// Keep the label tied to the settings snapshot used for the actual palette.
fn audit_tts_label(enabled: bool, usable: bool) -> &'static str {
    if !enabled { "TTS OFF" } else if !usable { "TTS !" } else { "TTS ON" }
}
#[cfg(test)]
mod september_tts_state_tests {
    use super::*;
    #[test]
    fn visible_text_distinguishes_off_configured_and_active() {
        assert_eq!(audit_tts_label(false, false), "TTS OFF");
        assert_eq!(audit_tts_label(false, true), "TTS OFF");
        assert_eq!(audit_tts_label(true, false), "TTS !");
        assert_eq!(audit_tts_label(true, true), "TTS ON");
    }
}
"#);
    fs::write(path, src).expect("write speech state tests");
    println!("cargo:rerun-if-changed=build/legacy/build_v1379_ui_audit_settings_chat.rs");
}
