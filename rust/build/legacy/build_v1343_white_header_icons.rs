// Final visual-only pass: earlier stages own all button actions and hit boxes.
use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1342_dungeon_flow_diag.rs");
    pub fn run() { main(); }
}

// Preserve the exact-one-anchor guard and support generated CRLF on Windows.
fn once(source: &mut String, from: &str, to: &str, label: &str) {
    let crlf = source.contains("\r\n");
    let from = if crlf { from.replace('\n', "\r\n") } else { from.to_owned() };
    let to = if crlf { to.replace('\n', "\r\n") } else { to.to_owned() };
    assert_eq!(source.matches(&from).count(), 1, "white header icons: {label} anchor changed");
    *source = source.replacen(&from, &to, 1);
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let feature_path = out.join("feature_overlays_v170_fixed.rs");
    let mut feature = fs::read_to_string(&feature_path).expect("feature renderer");

    // Mechanics already paints hover backgrounds. Replace only the three
    // glyphs with matching rounded tiles; leave original geometry intact.
    once(&mut feature,
        "draw_toolbar_symbol(hdc,\"⚙\",gear,icon_scale);draw_toolbar_symbol(hdc,collapse_glyph(state),collapse,icon_scale);draw_toolbar_symbol(hdc,\"×\",hide,hide_scale);",
        r#"white_header_icons::paint_tile(hdc,gear,white_header_icons::Icon::Settings,state.hover_y<TOOLBAR_H&&state.hover_x>=gear.left&&state.hover_x<gear.right);white_header_icons::paint_tile(hdc,collapse,white_header_icons::collapse_icon(&layout_side(state)),state.hover_y<TOOLBAR_H&&state.hover_x>=collapse.left&&state.hover_x<collapse.right);white_header_icons::paint_tile(hdc,hide,white_header_icons::Icon::Close,state.hover_y<TOOLBAR_H&&state.hover_x>=hide.left&&state.hover_x<hide.right);"#,
        "Mechanics settings/collapse/close");

    // DPS: clear the old hand-drawn symbols and render the actual approved
    // antialiased icon masks instead. Keep every original hitbox and action.
    once(&mut feature,
        "    }\n    for index in 0..3 {\n        let r = reference_system_rect(rc.right, index);",
        r#"        if matches!(item.action, ToolbarAction::Copy | ToolbarAction::Reset) {
            let icon = if item.action == ToolbarAction::Copy { white_header_icons::Icon::Copy } else { white_header_icons::Icon::Reset };
            white_header_icons::paint_button(hdc, r, icon, bg, hovered);
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
        white_header_icons::paint_button(hdc,r,icon,bg,reference_contains(r,state.hover_x,state.hover_y));
    }
    SelectObject(hdc, old);
}
unsafe fn reference_set_layout("#,
        "DPS settings/collapse/close");
    feature.push_str("\nmod white_header_icons { include!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/src/white_header_icons.rs\")); }\n");
    fs::write(&feature_path, feature).expect("write DPS and Mechanics icons");

    // Chat uses an older single-line native toolbar. Replace its icon draw
    // calls directly, avoiding opaque repaints and keeping the tab/TTS layout.
    let chat_path = out.join("overlay_v150_v1181.rs");
    let mut chat = fs::read_to_string(&chat_path).expect("chat renderer");
    once(&mut chat,
        "draw_toolbar_button(hdc,actions.gear,\"⚙\",crate::ui_modern::DARK_SURFACE,crate::ui_modern::BPSR_TEXT);",
        "white_header_icons::paint_tile(hdc,actions.gear,white_header_icons::Icon::Settings,false);",
        "chat settings");
    once(&mut chat,
        "let collapse=match snapshot.chat.collapse_side.to_ascii_lowercase().as_str(){\"left\"=>\"◀\",\"top\"=>\"▲\",\"bottom\"=>\"▼\",_=>\"▶\"};draw_toolbar_button(hdc,actions.collapse,collapse,crate::ui_modern::DARK_SURFACE,crate::ui_modern::BPSR_TEXT);",
        "white_header_icons::paint_tile(hdc,actions.collapse,white_header_icons::collapse_icon(&snapshot.chat.collapse_side),false);",
        "chat collapse");
    once(&mut chat,
        "draw_toolbar_button_sized(hdc,actions.hide,\"×\",crate::ui_modern::DARK_SURFACE,crate::ui_modern::BPSR_TEXT,15);",
        "white_header_icons::paint_tile(hdc,actions.hide,white_header_icons::Icon::Close,false);",
        "chat close");
    chat.push_str("\nmod white_header_icons { include!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/src/white_header_icons.rs\")); }\n");
    fs::write(&chat_path, chat).expect("write Chat icons");

    println!("cargo:rerun-if-changed=build/legacy/build_v1343_white_header_icons.rs");
    println!("cargo:rerun-if-changed=src/white_header_icons.rs");
    println!("cargo:rerun-if-changed=assets/header-icons");
}
