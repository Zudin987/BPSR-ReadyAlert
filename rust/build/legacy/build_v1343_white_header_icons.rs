// Final visual-only pass: the existing build stages own behavior and hit boxes.
use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1342_dungeon_flow_diag.rs");
    pub fn run() { main(); }
}

// The checked-in presentation is partly CRLF on Windows. Match each renderer's
// actual line endings without weakening the exactly-one-anchor assertion.
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

    // Replace the real Mechanics glyph calls in place, preserving the existing
    // rounded hover paint and original button rectangles and click handlers.
    once(&mut feature,
        "draw_toolbar_symbol(hdc,\"⚙\",gear,icon_scale);draw_toolbar_symbol(hdc,collapse_glyph(state),collapse,icon_scale);draw_toolbar_symbol(hdc,\"×\",hide,hide_scale);",
        r#"white_header_icons::paint(hdc,gear,white_header_icons::Icon::Settings,if state.hover_y<TOOLBAR_H&&state.hover_x>=gear.left&&state.hover_x<gear.right{crate::ui_modern::DARK_HOVER}else{crate::ui_modern::DARK_RAISED});white_header_icons::paint(hdc,collapse,white_header_icons::collapse_icon(&layout_side(state)),crate::ui_modern::DARK_RAISED);white_header_icons::paint(hdc,hide,white_header_icons::Icon::Close,crate::ui_modern::DARK_RAISED);"#,
        "Mechanics settings/collapse/close");

    // DPS: leave the reference renderer's positioning and dispatch untouched.
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

    // Chat's toolbar is a single-line function in the actual generated output.
    // Swap its three draw calls for rounded backgrounds and native vector icons.
    let chat_path = out.join("overlay_v150_v1181.rs");
    let mut chat = fs::read_to_string(&chat_path).expect("chat renderer");
    once(&mut chat,
        "draw_toolbar_button(hdc,actions.gear,\"⚙\",crate::ui_modern::DARK_SURFACE,crate::ui_modern::BPSR_TEXT);",
        "crate::ui_modern::fill_round_rect(hdc,actions.gear,crate::ui_modern::DARK_SURFACE,crate::ui_modern::RADIUS_SMALL);white_header_icons::paint(hdc,actions.gear,white_header_icons::Icon::Settings,crate::ui_modern::DARK_SURFACE);",
        "chat settings");
    once(&mut chat,
        "let collapse=match snapshot.chat.collapse_side.to_ascii_lowercase().as_str(){\"left\"=>\"◀\",\"top\"=>\"▲\",\"bottom\"=>\"▼\",_=>\"▶\"};draw_toolbar_button(hdc,actions.collapse,collapse,crate::ui_modern::DARK_SURFACE,crate::ui_modern::BPSR_TEXT);",
        "crate::ui_modern::fill_round_rect(hdc,actions.collapse,crate::ui_modern::DARK_SURFACE,crate::ui_modern::RADIUS_SMALL);white_header_icons::paint(hdc,actions.collapse,white_header_icons::collapse_icon(&snapshot.chat.collapse_side),crate::ui_modern::DARK_SURFACE);",
        "chat collapse");
    once(&mut chat,
        "draw_toolbar_button_sized(hdc,actions.hide,\"×\",crate::ui_modern::DARK_SURFACE,crate::ui_modern::BPSR_TEXT,15);",
        "crate::ui_modern::fill_round_rect(hdc,actions.hide,crate::ui_modern::DARK_SURFACE,crate::ui_modern::RADIUS_SMALL);white_header_icons::paint(hdc,actions.hide,white_header_icons::Icon::Close,crate::ui_modern::DARK_SURFACE);",
        "chat close");
    chat.push_str("\nmod white_header_icons { include!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/src/white_header_icons.rs\")); }\n");
    fs::write(&chat_path, chat).expect("write chat icons");

    println!("cargo:rerun-if-changed=build/legacy/build_v1343_white_header_icons.rs");
    println!("cargo:rerun-if-changed=src/white_header_icons.rs");
    println!("cargo:rerun-if-changed=assets/header-icons");
}
