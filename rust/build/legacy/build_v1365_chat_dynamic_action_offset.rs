use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1364_chat_icon_tiles.rs");
    pub fn run() { main(); }
}

fn main() {
    previous::run();
    let path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"))
        .join("overlay_v150_v1181.rs");
    let mut source = fs::read_to_string(&path).expect("generated chat overlay");
    // The TTS rect already uses its calculated width; subtract that same width
    // before placing + Tab so visual and hit rectangles never overlap.
    let old = "let tts = RECT { left: r - tts_width, top: 2, right: r, bottom: TOOLBAR_HEIGHT }; r -= TTS_WIDTH;";
    let new = "let tts = RECT { left: r - tts_width, top: 2, right: r, bottom: TOOLBAR_HEIGHT }; r -= tts_width;";
    assert_eq!(source.matches(old).count(),1,"expected one chat TTS layout offset");
    source=source.replacen(old,new,1);
    fs::write(path,source).expect("write corrected responsive chat layout");
    println!("cargo:rerun-if-changed=build/legacy/build_v1365_chat_dynamic_action_offset.rs");
}
