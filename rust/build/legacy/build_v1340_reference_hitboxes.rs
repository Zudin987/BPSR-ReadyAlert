use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1339_reference_layout.rs");
    pub fn run() { main(); }
}

fn replace_exact(source: &mut String, from: &str, to: &str, expected: usize, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, expected, "reference hitbox {label}: expected {expected}, found {count}");
    *source = source.replace(from, to);
}

fn main() {
    previous::run();
    let path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"))
        .join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read reference meter with expanded header");
    // Both WM_LBUTTONDOWN mode selection and hover guidance must follow the
    // same extra 16 px as the rendered 43 px target strip. Keep keyboard and
    // compact actions untouched; this applies to the normal-mode tabs only.
    replace_exact(&mut source, "let tabs_top=TOOLBAR_H+35;", "let tabs_top=TOOLBAR_H+51;", 2, "mode tabs");
    replace_exact(&mut source, "if y>=target_top&&y<target_top+27", "if y>=target_top&&y<target_top+43", 1, "target hover");
    source.push_str(r#"

#[cfg(test)]
mod reference_meter_hitbox_tests {
    use super::*;
    #[test]
    fn single_target_strip_keeps_mode_tabs_in_control_area() {
        let target_bottom = TOOLBAR_H + REFERENCE_ENCOUNTER_H;
        assert_eq!(reference_tabs_top(), target_bottom);
        assert_eq!(reference_tab_rect(0).top, target_bottom + 2);
        assert!(reference_tab_rect(2).bottom < dps_rows_top());
    }
}
"#);
    fs::write(path, source).expect("write matching header hitboxes");
    println!("cargo:rerun-if-changed=build/legacy/build_v1340_reference_hitboxes.rs");
}
