use std::{env, fs, path::{Path, PathBuf}};

mod prior {
    include!("build_v1250_chat_archive.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.26 archive patch {label} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn patch_win(out: &Path) {
    let path = out.join("win_v182_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read v1.25 Win UI").replace("\r\n", "\n");
    replace_once(
        &mut source,
        "crate::chat_archive::generate(&state.paths.chat_logs)",
        "crate::archive_hub::generate(&state.paths.chat_logs)",
        "Open Chat Logs builds local archive hub",
    );
    fs::write(path, source).expect("write v1.26 Win UI");
}

fn main() {
    prior::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_win(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1260_encounter_archive.rs");
    println!("cargo:rerun-if-changed=src/archive_hub.rs");
    println!("cargo:rerun-if-changed=src/encounter_archive.rs");
    println!("cargo:rerun-if-changed=src/encounter_context.rs");
    println!("cargo:rerun-if-changed=src/encounter_store.rs");
    println!("cargo:rerun-if-changed=src/game_data_v1260.rs");
    println!("cargo:rerun-if-changed=src/telemetry_v1260.rs");
}
