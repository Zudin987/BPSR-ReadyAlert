// Coordinate meter header density with existing minimum-readable row policy;
// reserve the real chat speech-state label width before laying out tabs.
use std::{env, fs, path::{Path, PathBuf}};
mod previous {
    include!("build_v1380_ui_audit_regressions.rs");
    pub fn run() { main(); }
}
fn replace_exact(out:&Path,name:&str,old:&str,new:&str,id:&str) {
    let path=out.join(name);
    let mut src=fs::read_to_string(&path).unwrap_or_else(|e|panic!("UI audit {id}: read {name}: {e}"));
    assert_eq!(src.matches(old).count(),1,"UI audit {id}: generated-source anchor missing or ambiguous");
    src=src.replacen(old,new,1);
    assert!(src.contains(new),"UI audit {id}: compiled replacement absent");
    fs::write(path,src).unwrap_or_else(|e|panic!("UI audit {id}: write {name}: {e}"));
    println!("cargo:warning=UI audit {id}: exact compiled-source replacement verified");
}
fn main(){
    previous::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    // S01/I03: the old 34-logical-unit header fell to approximately 20
    // physical pixels at 60%, while readable normal rows retained about 35.
    // Give toolbar, tabs and hit rectangles the same logical geometry budget.
    // This source owns both DPS and mechanics overlays, so deliberately keep
    // the change modest and test its full native geometry rather than applying
    // a separate paint-only scale that would desynchronise hit testing.
    replace_exact(&out,"feature_overlays_v170_fixed.rs",
        "const TOOLBAR_H: i32 = 34;",
        "const TOOLBAR_H: i32 = 40;",
        "S01 coordinated native header hit geometry");
    let meter=out.join("feature_overlays_v170_fixed.rs");
    let mut src=fs::read_to_string(&meter).expect("final meter source");
    src.push_str(r#"
#[cfg(test)]
mod september_header_scale_tests {
    use super::*;
    #[test]
    fn toolbar_hit_regions_are_never_shorter_than_target_or_disproportionate() {
        for scale in [60,70,80,90,100] {
            assert!(TOOLBAR_H-6 >= 24, "toolbar pointer target below 24 logical units");
            let normal=dps_row_h(scale).max(1);
            let compact=dps_compact_row_h(scale).max(1);
            // Both are in the logical coordinate system that Win32 scales.
            assert!(TOOLBAR_H * 2 >= normal && TOOLBAR_H * 2 >= compact,
                "scale={scale} header too small relative to row");
            for index in 0..3 {
                let target=reference_system_rect(420,index);
                assert!(target.bottom-target.top>=24 && target.right-target.left>=24);
            }
        }
    }
}
"#);
    fs::write(meter,src).expect("write S01 native regression");
    // H02/H04: the previous dynamic width estimated a three-character "TTS"
    // while painting "TTS OFF" or "TTS ON". Reserve the longest actual label;
    // existing tab-overflow logic receives the reduced remaining tab budget.
    replace_exact(&out,"overlay_v150_v1181.rs",
        "let tts_width = chat_header_label_width(settings, 3, TTS_WIDTH, 22);",
        "let tts_width = chat_header_label_width(settings, 7, TTS_WIDTH, 22);",
        "H02/H04 full speech label avoids clipping and action overlap");
    println!("cargo:rerun-if-changed=build/legacy/build_v1381_ui_audit_header_chat.rs");
}
