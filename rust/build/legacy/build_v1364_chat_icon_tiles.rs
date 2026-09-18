use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1363_chat_header_style.rs");
    pub fn run() { main(); }
}

fn main() {
    previous::run();
    let path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"))
        .join("overlay_v150_v1181.rs");
    let mut source = fs::read_to_string(&path).expect("generated chat overlay");

    // Keep approved DPS/Tracker icon tiles unchanged: only Chat gets sharper
    // 3px corners. Reuse the approved icon raster without changing click areas.
    let call = "white_header_icons::paint_tile(hdc,actions.";
    assert_eq!(source.matches(call).count(),3,"expected three chat header icon tiles");
    source=source.replace(call,"chat_header_icon_tile(hdc,actions.");
    let marker="unsafe fn draw_toolbar(hdc:HDC,";
    assert_eq!(source.matches(marker).count(),1,"expected chat toolbar renderer");
    source=source.replacen(marker,r#"unsafe fn chat_header_icon_tile(hdc:HDC,rect:RECT,icon:white_header_icons::Icon,hovered:bool){
    let width=rect.right-rect.left;
    let height=rect.bottom-rect.top;
    if width<16||height<16{return;}
    let tile_width=(width-2).min(30);
    let tile_height=(height-2).min(28);
    let left=rect.left+(width-tile_width)/2;
    let top=rect.top+(height-tile_height)/2;
    let tile=RECT{left,top,right:left+tile_width,bottom:top+tile_height};
    let background=if hovered{rgb(52,58,68)}else{rgb(35,39,47)};
    crate::ui_modern::fill_round_rect(hdc,tile,background,CHAT_HEADER_RADIUS);
    white_header_icons::paint(hdc,tile,icon,background);
}
unsafe fn draw_toolbar(hdc:HDC,"#,1);

    // Enlarging TTS shifts + Tab left; both edges must still meet. The initial
    // stage's check accidentally compared absolute positions at different sizes.
    let incorrect="        assert_eq!(large.add.right,small.add.right);\n        assert_eq!(large.tts.right,large.add.left);";
    let correct="        assert_eq!(large.add.right,large.tts.left);\n        assert_eq!(small.add.right,small.tts.left);\n        assert_eq!(large.hide.right,small.hide.right);";
    assert_eq!(source.matches(incorrect).count(),1,"expected initial layout assertions");
    source=source.replacen(incorrect,correct,1);
    fs::write(path,source).expect("write sharp chat icon tiles");
    println!("cargo:rerun-if-changed=build/legacy/build_v1364_chat_icon_tiles.rs");
}
