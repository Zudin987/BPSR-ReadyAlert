use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1365_chat_dynamic_action_offset.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, old: &str, new: &str, label: &str) {
    let matches = source.matches(old).count();
    assert_eq!(matches, 1, "chat font offset {label}: expected one anchor, got {matches}");
    *source = source.replacen(old, new, 1);
}

fn main() {
    previous::run();
    let path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"))
        .join("overlay_v150_v1181.rs");
    let mut source = fs::read_to_string(&path).expect("generated chat overlay");

    // Keep the message font exactly as configured, but use a separate cached
    // font two points smaller for header tab labels and text buttons.
    replace_once(&mut source,
        "    bold_font: HFONT,\n    font_family_key:",
        "    bold_font: HFONT,\n    header_font: HFONT,\n    font_family_key:",
        "cached header font field");
    replace_once(&mut source,
        "        bold_font: null_mut(),\n        font_family_key:",
        "        bold_font: null_mut(),\n        header_font: null_mut(),\n        font_family_key:",
        "cached header font init");
    replace_once(&mut source,
        "                if !(*ptr).bold_font.is_null() { DeleteObject((*ptr).bold_font); }",
        "                if !(*ptr).bold_font.is_null() { DeleteObject((*ptr).bold_font); }\n                if !(*ptr).header_font.is_null() { DeleteObject((*ptr).header_font); }",
        "destroy cached header font");
    replace_once(&mut source,
        "        && !state.bold_font.is_null()\n    {",
        "        && !state.bold_font.is_null()\n        && !state.header_font.is_null()\n    {",
        "header font cache validity");
    replace_once(&mut source,
        "    if !state.bold_font.is_null() { DeleteObject(state.bold_font); state.bold_font = null_mut(); }",
        "    if !state.bold_font.is_null() { DeleteObject(state.bold_font); state.bold_font = null_mut(); }\n    if !state.header_font.is_null() { DeleteObject(state.header_font); state.header_font = null_mut(); }",
        "invalidate header font on setting changes");
    replace_once(&mut source,
        "    state.bold_font = CreateFontW(height, 0, 0, 0, 500, 0, 0, 0, 1, 0, 0, 5, 0, face.as_ptr());",
        "    state.bold_font = CreateFontW(height, 0, 0, 0, 500, 0, 0, 0, 1, 0, 0, 5, 0, face.as_ptr());\n    state.header_font = CreateFontW(-chat_header_font_px(settings), 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 0, face.as_ptr());",
        "create smaller header font with configured family");
    replace_once(&mut source,
        "        draw_toolbar(hdc, &snapshot, client.right);",
        "        let before_toolbar = SelectObject(hdc, if state.header_font.is_null() { regular_font } else { state.header_font });\n        draw_toolbar(hdc, &snapshot, client.right);\n        SelectObject(hdc, before_toolbar);",
        "select header font for toolbar only");
    replace_once(&mut source,
        "fn chat_header_font_px(settings: &AppSettings) -> i32 {\n    ((settings.chat.font_size.clamp(8.0,24.0)*96.0/72.0).round() as i32).max(10)\n}",
        "fn chat_header_font_pt(settings: &AppSettings) -> f32 {\n    (settings.chat.font_size.clamp(8.0,24.0)-2.0).max(6.0)\n}\nfn chat_header_font_px(settings: &AppSettings) -> i32 {\n    ((chat_header_font_pt(settings)*96.0/72.0).round() as i32).max(8)\n}",
        "header points and label hitbox sizing use 2 point offset");

    source.push_str(r#"
#[cfg(test)]
mod chat_header_font_offset_regressions {
    use super::*;
    #[test]
    fn header_uses_two_points_less_than_messages() {
        let mut settings = AppSettings::default();
        settings.chat.font_size = 12.0;
        assert_eq!(chat_header_font_pt(&settings), 10.0);
        assert_eq!(chat_header_font_px(&settings), 13);
        settings.chat.font_size = 14.0;
        assert_eq!(chat_header_font_pt(&settings), 12.0);
        assert_eq!(chat_header_font_px(&settings), 16);
        settings.chat.font_size = 8.0;
        assert_eq!(chat_header_font_pt(&settings), 6.0);
    }
    #[test]
    fn smaller_header_labels_keep_paint_and_hitbox_layout_in_sync() {
        let mut settings = AppSettings::default();
        settings.chat.font_size = 12.0;
        let small = action_rects(620, &settings);
        let small_tab = chat_header_tab_width(&settings,"Guild&Team");
        settings.chat.font_size = 14.0;
        let large = action_rects(620, &settings);
        let large_tab = chat_header_tab_width(&settings,"Guild&Team");
        assert!(large_tab > small_tab);
        assert_eq!(small.add.right,small.tts.left);
        assert_eq!(large.add.right,large.tts.left);
        assert!(large.add.right-large.add.left>=small.add.right-small.add.left);
    }
}
"#);
    fs::write(path, source).expect("write smaller Chat header font");
    println!("cargo:rerun-if-changed=build/legacy/build_v1366_chat_header_font_offset.rs");
}
