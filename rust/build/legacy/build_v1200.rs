use std::{env, fs, path::PathBuf};

mod prior {
    include!("build_v1190k.rs");
    pub fn run() { main(); }
}

fn main() {
    prior::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_settings(&out);
    patch_win(&out);
}

fn patch_settings(out: &PathBuf) {
    let path = out.join("settings_ui_v1160_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read generated settings UI");

    replace_once(
        &mut source,
        "const ID_ALERT_VOLUME: i32 = 3206;",
        "const ID_ALERT_VOLUME: i32 = 3206;\nconst ID_UPDATE_AUTO_CHECK: i32 = 3230;\nconst ID_UPDATE_AUTO_DOWNLOAD: i32 = 3231;\nconst ID_UPDATE_CHECK_NOW: i32 = 3232;",
        "update control IDs",
    );

    replace_once(
        &mut source,
        "    section(hwnd,state,PAGE_GENERAL,\"SHORTCUTS\",174,270,580);\n    info(hwnd,state,PAGE_GENERAL,\"Ctrl+Shift+F10 toggles DPS + Mechanics. Chat click-through recovery defaults to Ctrl+Shift+F9. Hidden overlays can also be restored from the tray.\",174,292,580,42);",
        "    section(hwnd,state,PAGE_GENERAL,\"UPDATES\",174,270,580);\n    checkbox_at(hwnd,state,PAGE_GENERAL,ID_UPDATE_AUTO_CHECK,\"Automatically check for updates\",174,292,276);\n    checkbox_at(hwnd,state,PAGE_GENERAL,ID_UPDATE_AUTO_DOWNLOAD,\"Download updates automatically\",174,320,276);\n    button(hwnd,state,PAGE_GENERAL,ID_UPDATE_CHECK_NOW,\"Check now\",506,294,118,30);\n    info(hwnd,state,PAGE_GENERAL,&format!(\"Current version  v{}\",env!(\"CARGO_PKG_VERSION\")),506,328,180,20);",
        "General updates section",
    );

    replace_once(
        &mut source,
        "    set_text(hwnd, ID_ALERT_VOLUME, &s.alert_volume.to_string());",
        "    set_text(hwnd, ID_ALERT_VOLUME, &s.alert_volume.to_string());\n    let update_prefs=crate::updater::load_preferences(&state.paths.root);\n    set_check(hwnd,ID_UPDATE_AUTO_CHECK,update_prefs.auto_check);\n    set_check(hwnd,ID_UPDATE_AUTO_DOWNLOAD,update_prefs.auto_download);",
        "load update preferences",
    );

    replace_once(
        &mut source,
        "    s.alert_volume = read_i32(hwnd, ID_ALERT_VOLUME, s.alert_volume);",
        "    s.alert_volume = read_i32(hwnd, ID_ALERT_VOLUME, s.alert_volume);\n    let update_prefs=crate::updater::UpdatePreferences{auto_check:get_check(hwnd,ID_UPDATE_AUTO_CHECK),auto_download:get_check(hwnd,ID_UPDATE_AUTO_DOWNLOAD)};\n    if let Err(err)=crate::updater::save_preferences(&state.paths.root,&update_prefs){MessageBoxW(hwnd,wide(&format!(\"Could not save update settings.\\r\\n\\r\\n{err}\")).as_ptr(),wide(\"ReadyAlert Updates\").as_ptr(),MB_OK|MB_ICONERROR);return;}",
        "save update preferences",
    );

    replace_once(
        &mut source,
        "        3209 => crate::event_tracker_ui::show(hwnd),",
        "        3209 => crate::event_tracker_ui::show(hwnd),\n        ID_UPDATE_CHECK_NOW => crate::updater::check_interactive(state.paths.root.clone()),",
        "manual update command",
    );

    fs::write(path, source).expect("write v1.20 settings UI");
}

fn patch_win(out: &PathBuf) {
    let path = out.join("win_v182_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read generated win UI");
    replace_once(
        &mut source,
        "        add_tray(hwnd, &state.capture_status)?;",
        "        add_tray(hwnd, &state.capture_status)?;\n        crate::updater::spawn_startup(state.paths.root.clone());",
        "background updater startup",
    );
    fs::write(path, source).expect("write v1.20 win UI");
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.20.0 patch {label} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}