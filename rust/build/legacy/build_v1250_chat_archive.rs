use std::{env, fs, path::{Path, PathBuf}};

mod prior {
    include!("build_v1241_audit_hardening.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.25 chat archive patch {label} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn patch_win(out: &Path) {
    let path = out.join("win_v182_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read v1.24.1 Win UI").replace("\r\n", "\n");
    replace_once(
        &mut source,
        "if command==CMD_OPEN_LOGS{let _=std::fs::create_dir_all(&state.paths.chat_logs);open_path(&state.paths.chat_logs);return;}",
        r###"if command==CMD_OPEN_LOGS{let _=std::fs::create_dir_all(&state.paths.chat_logs);match crate::chat_archive::generate(&state.paths.chat_logs){Ok(path)=>open_path(&path),Err(err)=>{logging::write(format!("chat archive: {err}"));message_box("BPSR ReadyAlert - Chat Archive",&format!("Could not build the offline chat archive.\n\n{err}\n\nThe raw chat-log folder will open instead."),true);open_path(&state.paths.chat_logs);}}return;}"###,
        "open chat logs as local HTML archive",
    );
    fs::write(path, source).expect("write chat archive Win UI");
}

fn main() {
    prior::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_win(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1250_chat_archive.rs");
    println!("cargo:rerun-if-changed=src/chat_archive.rs");
}
