use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1316_ui_controls_polish.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.31.6 UI polish follow-up {label:?} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .expect("read feature overlay for v1.31.6 type follow-up")
        .replace("\r\n", "\n");

    replace_once(
        &mut source,
        "let side=collapse_edge_index(&layout.collapse_side);SendMessageW(fc(hwnd,7003),0x014e,side,0);",
        "let side=collapse_edge_index(&layout.collapse_side);SendMessageW(fc(hwnd,7003),0x014e,side as usize,0);",
        "CB_SETCURSEL WPARAM type",
    );

    fs::write(path, source).expect("write feature overlay v1.31.6 type follow-up");
    println!("cargo:rerun-if-changed=build/legacy/build_v1316_ui_controls_polish_fix.rs");
}
