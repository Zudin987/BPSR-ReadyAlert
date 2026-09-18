use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1362_enrage_header.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, old: &str, new: &str, label: &str) {
    let count = source.matches(old).count();
    assert_eq!(count, 1, "chat header {label}: expected one anchor, found {count}");
    *source = source.replacen(old, new, 1);
}

fn replace_in_fn(source: &mut String, start: &str, end: &str, old: &str, new: &str, label: &str) {
    let begin = source.find(start).expect("chat header function start");
    let finish = begin + source[begin..].find(end).expect("chat header function end");
    let mut fragment = source[begin..finish].to_owned();
    replace_once(&mut fragment, old, new, label);
    source.replace_range(begin..finish, &fragment);
}

fn main() {
    previous::run();
    let path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"))
        .join("overlay_v150_v1181.rs");
    let mut source = fs::read_to_string(&path).expect("generated current chat overlay");

    // Deliberately local to Chat: the approved DPS/Tracker visual theme is unchanged.
    replace_once(&mut source,
        "const ROW_GAP: i32 = 4;",
        "const ROW_GAP: i32 = 4;\nconst CHAT_HEADER_RADIUS: i32 = 3;",
        "small rectangular chat header radius");

    // The toolbar previously switched back to DEFAULT_GUI_FONT and ignored the
    // user's chat font size and family. The message font is already selected here.
    replace_once(&mut source,
        "        let before_toolbar = SelectObject(hdc, stock_font);\n        draw_toolbar(hdc, &snapshot, client.right);\n        SelectObject(hdc, before_toolbar);",
        "        draw_toolbar(hdc, &snapshot, client.right);",
        "inherit the configured chat font in toolbar");

    // Keep paint and hit testing on the same size-aware action rectangles.
    replace_once(&mut source,
        "let actions = action_rects(client.right);",
        "let actions = action_rects(client.right, &snapshot);",
        "action hit targets scale with chat text");
    replace_once(&mut source,
        "let actions=action_rects(width);let drag=",
        "let actions=action_rects(width,snapshot);let drag=",
        "action paint rectangles scale with chat text");
    replace_once(&mut source,
        "fn action_rects(width: i32) -> ActionRects {",
        "fn action_rects(width: i32, settings: &AppSettings) -> ActionRects {\n    let add_width = chat_header_label_width(settings, 5, ADD_WIDTH, 20);\n    let tts_width = chat_header_label_width(settings, 3, TTS_WIDTH, 22);",
        "responsive action widths");
    replace_in_fn(&mut source, "fn action_rects(", "unsafe fn draw_tabs(",
        "r - TTS_WIDTH", "r - tts_width", "responsive TTS width");
    replace_in_fn(&mut source, "fn action_rects(", "unsafe fn draw_tabs(",
        "r - ADD_WIDTH", "r - add_width", "responsive add-tab width");
    replace_once(&mut source,
        "for tab in settings.chat.tabs.iter().take(MAX_MENU_TABS){let guessed=28+tab.name.chars().count()as i32*8;let w=guessed.clamp(68,150);",
        "for tab in settings.chat.tabs.iter().take(MAX_MENU_TABS){let w=chat_header_tab_width(settings,&tab.name);",
        "tab width follows configured chat font");

    // The tab and text-button surfaces become almost rectangular, with a small
    // 3px radius. No global theme constants or other overlays are modified.
    replace_in_fn(&mut source, "unsafe fn draw_toolbar_button(", "unsafe fn draw_toolbar_button_sized(",
        "crate::ui_modern::RADIUS_SMALL", "CHAT_HEADER_RADIUS", "add/TTS corner radius");
    replace_in_fn(&mut source, "unsafe fn draw_toolbar_button_sized(", "fn action_rects(",
        "crate::ui_modern::RADIUS_SMALL", "CHAT_HEADER_RADIUS", "fallback text-button corner radius");
    replace_in_fn(&mut source, "unsafe fn draw_tabs(", "fn tab_rects_with_limit(",
        "crate::ui_modern::SELECTED_SURFACE,crate::ui_modern::RADIUS_SMALL",
        "crate::ui_modern::SELECTED_SURFACE,CHAT_HEADER_RADIUS", "selected tab corner radius");
    replace_in_fn(&mut source, "unsafe fn draw_tabs(", "fn tab_rects_with_limit(",
        "crate::ui_modern::DARK_SURFACE,crate::ui_modern::RADIUS_SMALL",
        "crate::ui_modern::DARK_SURFACE,CHAT_HEADER_RADIUS", "overflow corner radius");
    replace_once(&mut source, "fn action_rects(width: i32, settings: &AppSettings) -> ActionRects {",
        "fn chat_header_font_px(settings: &AppSettings) -> i32 {\n    ((settings.chat.font_size.clamp(8.0,24.0)*96.0/72.0).round() as i32).max(10)\n}\nfn chat_header_label_width(settings: &AppSettings, letters: i32, minimum: i32, padding: i32) -> i32 {\n    (chat_header_font_px(settings)*letters*56/100+padding).max(minimum)\n}\nfn chat_header_tab_width(settings: &AppSettings, name: &str) -> i32 {\n    (28+name.chars().count().min(128) as i32*chat_header_font_px(settings)*56/100).clamp(68,220)\n}\nfn action_rects(width: i32, settings: &AppSettings) -> ActionRects {",
        "shared font-aware size helpers");

    source.push_str(r#"
#[cfg(test)]
mod chat_header_style_regressions {
    use super::*;
    #[test]
    fn chat_font_size_resizes_labels_and_click_targets_together() {
        let mut settings = AppSettings::default();
        settings.chat.font_size = 8.0;
        let small_tab = chat_header_tab_width(&settings,"Guild&Team");
        let small = action_rects(620,&settings);
        settings.chat.font_size = 24.0;
        let large_tab = chat_header_tab_width(&settings,"Guild&Team");
        let large = action_rects(620,&settings);
        assert!(large_tab > small_tab);
        assert!(large.add.right-large.add.left > small.add.right-small.add.left);
        assert!(large.tts.right-large.tts.left > small.tts.right-small.tts.left);
        assert_eq!(large.add.right,small.add.right);
        assert_eq!(large.tts.right,large.add.left);
        assert_eq!(CHAT_HEADER_RADIUS,3);
    }
    #[test]
    fn tab_hit_rects_use_the_same_font_dependent_layout_as_paint() {
        let mut settings=AppSettings::default();
        settings.chat.font_size=8.0;
        let small=tab_rects(&settings,300);
        settings.chat.font_size=24.0;
        let large=tab_rects(&settings,300);
        assert!(!small.is_empty());
        assert!(!large.is_empty());
        assert!(large[0].right-large[0].left>=small[0].right-small[0].left);
    }
}
"#);
    fs::write(path, source).expect("write chat header styling and typography");
    println!("cargo:rerun-if-changed=build/legacy/build_v1363_chat_header_style.rs");
}
