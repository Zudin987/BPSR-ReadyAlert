//! Final presentation adapter for the historical native source generator.
//! Runtime design code belongs in ordinary source files; this bridge keeps the
//! old assertion-guarded stages intact and makes all active windows use it.
use std::{env, fs, path::PathBuf};

fn remove_import(source: &mut String, name: &str) {
    let mut cursor = 0;
    while let Some(relative) = source[cursor..].find("use windows_sys::") {
        let start = cursor + relative;
        let end = start + source[start..].find(';').expect("native import terminator") + 1;
        let mut import = source[start..end].to_string();
        if import.ends_with(&format!("::{name};")) {
            import.clear();
        } else {
            while let Some(at) = import.find(name) {
                let before = import[..at].chars().last();
                let after = import[at + name.len()..].chars().next();
                if before.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
                    || after.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
                {
                    break;
                }
                let mut finish = at + name.len();
                while import
                    .as_bytes()
                    .get(finish)
                    .is_some_and(u8::is_ascii_whitespace)
                {
                    finish += 1;
                }
                if import.as_bytes().get(finish) == Some(&b',') {
                    finish += 1;
                }
                import.replace_range(at..finish, "");
            }
        }
        source.replace_range(start..end, &import);
        cursor = start + import.len();
    }
}

fn bind_palette(source: &mut String, inherit_aliases: bool, preserve_chat_colors: bool) {
    let mut aliases = Vec::new();
    for (native, themed) in [
        ("FillRect", "fill_rect"),
        ("SetTextColor", "set_text_color"),
        ("SetBkColor", "set_bk_color"),
    ] {
        if preserve_chat_colors && native == "SetTextColor" {
            continue;
        }
        let qualified = format!("windows_sys::Win32::Graphics::Gdi::{native}(");
        *source = source.replace(&qualified, &format!("crate::appearance::{themed}("));
        if source.contains(&format!("{native}(")) {
            assert!(
                !source.contains(&format!("fn {native}(")),
                "appearance adapter must not replace an FFI declaration"
            );
            remove_import(source, native);
            aliases.push(format!("{themed} as {native}"));
        }
    }
    if !aliases.is_empty() && !inherit_aliases {
        source.push_str(&format!(
            "\nuse crate::appearance::{{{}}};\n",
            aliases.join(", ")
        ));
    }
}

fn once(source: &mut String, old: &str, new: &str, label: &str) {
    assert_eq!(
        source.matches(old).count(),
        1,
        "design bridge {label}: expected one anchor"
    );
    *source = source.replacen(old, new, 1);
}

fn function(source: &mut String, name: &str, edit: impl FnOnce(&mut String)) {
    let marker = format!("fn {name}(");
    assert_eq!(
        source.matches(&marker).count(),
        1,
        "design bridge {name}: expected one function"
    );
    let start = source.find(&marker).expect("function marker");
    let brace = start + source[start..].find('{').expect("function body");
    let mut depth = 1;
    let mut end = brace + 1;
    while depth > 0 {
        match source.as_bytes()[end] {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            _ => {}
        }
        end += 1;
    }
    let mut part = source[start..end].to_string();
    edit(&mut part);
    source.replace_range(start..end, &part);
}

fn meter_design(source: &mut String) {
    for name in [
        "paint_reference_normal_rows",
        "paint_compact_player",
        "paint_raid_player",
    ] {
        function(source, name, |part| {
            if name == "paint_compact_player" {
                let start = part
                    .find("let leader = meter_rows(state)")
                    .expect("compact leader");
                let end = start + part[start..].find(';').expect("compact leader terminator") + 1;
                let leader = part[start..end].to_string();
                part.replace_range(start..end, "");
                let top = part.find('{').expect("compact body") + 1;
                part.insert_str(top, &format!("\n    {leader}\n"));
            }
            let call = part
                .find("crate::ui_modern::fill_round_rect(hdc, r,")
                .expect("meter row surface");
            let after = call + part[call..].find(';').expect("meter surface terminator") + 1;
            part.insert_str(
                after,
                "\n    paint_notmeter_gauge(hdc, state, row, r, leader);",
            );
        });
    }
    once(
        source,
        "unsafe fn build_feature_form(",
        "unsafe fn build_classic_feature_form(",
        "settings builder delegation",
    );
    once(
        source,
        "unsafe fn audit_place_feature_footer(",
        "unsafe fn classic_place_feature_footer(",
        "settings footer delegation",
    );
    function(source, "open_feature_settings", |part| {
        let start = part.find("let (w,h)=").expect("settings size");
        let end = start + part[start..].find(';').expect("settings size terminator") + 1;
        part.replace_range(start..end, "let (w,h)=(1000,870);");
        once(
            part,
            "WS_POPUP|0x00c00000|0x00080000|0x02000000",
            "WS_POPUP|0x00c00000|0x00080000|0x02000000|WS_THICKFRAME|0x00300000",
            "scrollable settings frame",
        );
    });
    function(source, "refresh_feature_form", |part| {
        let body = part.find('{').expect("settings refresh body") + 1;
        part.insert_str(body, "\n    refresh_design_theme(hwnd,state);");
    });
    function(source, "settings_wnd_proc", |part| {
        once(
            part,
            "crate::ui_modern::qa_draw_dark_button(item,false,false,false,false)",
            "paint_design_settings_button(item,state)",
            "selected settings navigation and layouts",
        );
        once(part, "let id=wparam as u16 as i32;let code=(wparam>>16) as u16;", "let id=wparam as u16 as i32;let code=(wparam>>16) as u16;\n            if design_settings_command(hwnd,state,id,code){return 0;}", "settings navigation commands");
        once(
            part,
            "FillRect(hdc,&rc,state.background);",
            "FillRect(hdc,&rc,state.background);paint_design_settings(hdc,rc);",
            "settings sidebar surface",
        );
        once(part, "WM_SIZE=>{crate::ui_modern::apply_overlay_frame_region(hwnd);audit_place_feature_footer(hwnd);0}", "WM_SIZE=>{crate::ui_modern::apply_overlay_frame_region(hwnd);state.form.layout(hwnd);audit_place_feature_footer(hwnd);0}", "settings resize");
        once(part, "crate::ui::WM_REVEAL_FOCUS=>0,", "crate::ui::WM_REVEAL_FOCUS=>{state.form.reveal_focus(hwnd);place_design_footer(hwnd);0},\n        0x020A|0x0115=>{state.form.scroll(hwnd,msg,wparam);place_design_footer(hwnd);0},", "settings keyboard and vertical scroll");
        once(part, "return 0;}DefWindowProcW(hwnd,msg,wparam,lparam)},", "return 0;}if lparam==0{state.form.scroll(hwnd,msg,wparam);place_design_footer(hwnd);0}else{DefWindowProcW(hwnd,msg,wparam,lparam)}},", "settings horizontal scroll");
    });
    function(source, "open_detail", |part| {
        once(
            part,
            "ReadyAlert Entity Inspector",
            "ReadyAlert Combat Details",
            "combat-details window title",
        );
    });
    function(source, "paint_detail", |part| {
        once(
            part,
            "Player details - {}",
            "Combat details · {}",
            "combat-details toolbar title",
        );
    });
    source.push_str(
        "\ninclude!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/src/notmeter_overlay.rs\"));\n",
    );
}

pub fn apply() {
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    for name in [
        "feature_overlays_v170_fixed.rs",
        "settings_ui_v1160_fixed.rs",
        "event_tracker_ui_v1160_fixed.rs",
        "overlay_v150_v1181.rs",
        "win_v182_fixed.rs",
        "ui_pixel_core_shape_v1394.rs",
        "ui_pixel_controls_v1316.rs",
        "ui_pixel_dialog_shape_v1394.rs",
        "ui_pixel_qa_v1316.rs",
    ] {
        let path = out.join(name);
        let mut source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("appearance input {name}: {err}"));
        if name == "feature_overlays_v170_fixed.rs" {
            meter_design(&mut source);
        }
        if name == "ui_pixel_core_shape_v1394.rs" {
            source = source.replace(
                "let brush = CreateSolidBrush(color);",
                "let brush = CreateSolidBrush(crate::appearance::color(color));",
            );
            source = source.replace(
                "let pen = CreatePen(PS_SOLID_, width.max(1), color);",
                "let pen = CreatePen(PS_SOLID_, width.max(1), crate::appearance::color(color));",
            );
            for token in ["MIST_BG", "MIST_INPUT", "DARK_BG"] {
                let original =
                    format!("*BRUSH.get_or_init(|| CreateSolidBrush({token}) as usize) as HBRUSH");
                source = source.replace(
                    &original,
                    &format!("crate::appearance::themed_brush({original})"),
                );
            }
        }
        for field in ["background", "input_background"] {
            for state in ["state", "(*state_ptr)", "(*ptr)", "s"] {
                let original = format!("{state}.{field} as isize");
                source = source.replace(
                    &original,
                    &format!("crate::appearance::themed_brush({state}.{field}) as isize"),
                );
            }
        }
        bind_palette(
            &mut source,
            matches!(
                name,
                "ui_pixel_controls_v1316.rs"
                    | "ui_pixel_dialog_shape_v1394.rs"
                    | "ui_pixel_qa_v1316.rs"
            ),
            name == "overlay_v150_v1181.rs",
        );
        fs::write(path, source).expect("write themed native source");
    }
    println!("cargo:rerun-if-changed=build/design.rs");
    println!("cargo:rerun-if-changed=src/notmeter_overlay.rs");
}
