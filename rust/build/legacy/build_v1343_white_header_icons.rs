// Final visual-only pass: earlier stages own the behavior and hit boxes.
use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1342_dungeon_flow_diag.rs");
    pub fn run() { main(); }
}

fn once(source: &mut String, from: &str, to: &str, label: &str) {
    assert_eq!(source.matches(from).count(), 1, "white header icons: {label} anchor changed");
    *source = source.replacen(from, to, 1);
}

// Insert at the end of a function's body, respecting quoted braces.
fn append_to_function(source: &mut String, name: &str, insertion: &str) {
    let signature = format!("unsafe fn {name}(");
    assert_eq!(source.matches(&signature).count(), 1, "white header icons: {name} function");
    let start = source.find(&signature).unwrap();
    let opening = start + source[start..].find('{').expect("function body");
    let mut depth = 0i32;
    let mut quoted = false;
    let mut escaped = false;
    for (offset, ch) in source[opening..].char_indices() {
        if quoted {
            if escaped { escaped = false; }
            else if ch == '\\' { escaped = true; }
            else if ch == '"' { quoted = false; }
            continue;
        }
        if ch == '"' { quoted = true; }
        if ch == '{' { depth += 1; }
        if ch == '}' {
            depth -= 1;
            if depth == 0 {
                source.insert_str(opening + offset, insertion);
                return;
            }
        }
    }
    panic!("white header icons: unterminated {name}");
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let feature_path = out.join("feature_overlays_v170_fixed.rs");
    let mut feature = fs::read_to_string(&feature_path).expect("feature renderer");

    // Mechanics: overpaint just the old three glyphs; retain their hit boxes.
    append_to_function(&mut feature, "reference_legacy_paint_toolbar", r#"
if state.kind==Kind::Mechanics {
    use white_header_icons::Icon;
    let background=rgb(23,28,35);
    let buttons=[
        (RECT{left:rc.right-BUTTON_W*3,top:0,right:rc.right-BUTTON_W*2,bottom:TOOLBAR_H-2},Icon::Settings),
        (RECT{left:rc.right-BUTTON_W*2,top:0,right:rc.right-BUTTON_W,bottom:TOOLBAR_H-2},white_header_icons::collapse_icon(&layout_side(state))),
        (RECT{left:rc.right-BUTTON_W,top:0,right:rc.right,bottom:TOOLBAR_H-2},Icon::Close),
    ];
    for (rect,icon) in buttons { white_header_icons::paint_button(hdc,rect,icon,background); }
}
"#);

    // DPS reference header owns the current rendering. Replace visuals only.
    once(&mut feature,
        "    }\n    for index in 0..3 {\n        let r = reference_system_rect(rc.right, index);",
        r#"        if matches!(item.action, ToolbarAction::Copy | ToolbarAction::Reset) {
            let icon = if item.action == ToolbarAction::Copy { white_header_icons::Icon::Copy } else { white_header_icons::Icon::Reset };
            white_header_icons::paint_button(hdc, r, icon, surface);
        }
    }
    for index in 0..3 {
        let r = reference_system_rect(rc.right, index);"#,
        "DPS copy and reset");
    once(&mut feature,
        "        }\n    }\n    SelectObject(hdc, old);\n}\nunsafe fn reference_set_layout(",
        r#"        }
        let icon=match index {
            0 => white_header_icons::Icon::Settings,
            1 => white_header_icons::collapse_icon(&layout_side(state)),
            _ => white_header_icons::Icon::Close,
        };
        let background=if reference_contains(r,state.hover_x,state.hover_y) { rgb(44,50,59) } else { bg };
        white_header_icons::paint_button(hdc,r,icon,background);
    }
    SelectObject(hdc, old);
}
unsafe fn reference_set_layout("#,
        "DPS settings/collapse/close");
    feature.push_str("\nmod white_header_icons { include!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/src/white_header_icons.rs\")); }\n");
    fs::write(&feature_path, feature).expect("write feature icons");

    let chat_path = out.join("overlay_v150_v1181.rs");
    let mut chat = fs::read_to_string(&chat_path).expect("chat renderer");
    once(&mut chat,
        "    draw_toolbar_button(hdc, actions.hide, \"×\", rgb(26,30,36), rgb(239,243,247));",
        r#"    draw_toolbar_button(hdc, actions.hide, "×", rgb(26,30,36), rgb(239,243,247));
    let background=rgb(26,30,36);
    white_header_icons::paint_button(hdc,actions.gear,white_header_icons::Icon::Settings,background);
    white_header_icons::paint_button(hdc,actions.collapse,white_header_icons::collapse_icon(&snapshot.chat.collapse_side),background);
    white_header_icons::paint_button(hdc,actions.hide,white_header_icons::Icon::Close,background);"#,
        "chat settings/collapse/close");
    chat.push_str("\nmod white_header_icons { include!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/src/white_header_icons.rs\")); }\n");
    fs::write(&chat_path, chat).expect("write chat icons");

    println!("cargo:rerun-if-changed=build/legacy/build_v1343_white_header_icons.rs");
    println!("cargo:rerun-if-changed=src/white_header_icons.rs");
    println!("cargo:rerun-if-changed=assets/header-icons");
}
