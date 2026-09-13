use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_pixel_phase3.rs");
    pub fn run() { main(); }
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let path = out.join("overlay_v150_v1181.rs");
    let mut source = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read chat overlay for Pixel regression expectation: {e}"))
        .replace("\r\n", "\n");
    let old = "#[test]fn tts_enabled_is_teal_and_disabled_is_neutral(){let(disabled,_)=tts_visual(false,true);let(enabled,_)=tts_visual(true,true);let(warning,_)=tts_visual(true,false);assert_eq!(disabled,crate::ui_modern::DARK_SURFACE);assert_eq!(enabled,dim_chat_color(crate::ui_modern::BPSR_ACCENT,62));assert_ne!(warning,crate::ui_modern::BPSR_DANGER);}";
    let new = "#[test]fn tts_states_follow_pixel_tonal_palette(){let(disabled,_)=tts_visual(false,true);let(enabled,_)=tts_visual(true,true);let(warning,_)=tts_visual(true,false);assert_eq!(disabled,crate::ui_modern::DARK_SURFACE);assert_eq!(enabled,crate::ui_modern::mix_color(crate::ui_modern::DARK_RAISED,crate::ui_modern::BPSR_ACCENT,28));assert_eq!(warning,crate::ui_modern::mix_color(crate::ui_modern::DARK_RAISED,crate::ui_modern::BPSR_WARNING,26));assert_ne!(warning,crate::ui_modern::BPSR_DANGER);}";
    let count = source.matches(old).count();
    assert_eq!(count, 1, "Pixel TTS visual regression target expected once, found {count}");
    source = source.replacen(old, new, 1);
    fs::write(path, source).unwrap_or_else(|e| panic!("write Pixel TTS regression expectation: {e}"));
    println!("cargo:rerun-if-changed=build/legacy/build_pixel_phase3_regression.rs");
}
