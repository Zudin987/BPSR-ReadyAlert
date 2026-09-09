use std::{env, fs, path::{Path, PathBuf}};

mod prior {
    include!("build_v1160_release.rs");
    pub fn run() { main(); }
}

fn remove_second_function(source: &mut String, marker: &str, label: &str) {
    let positions: Vec<usize> = source.match_indices(marker).map(|(i, _)| i).collect();
    assert_eq!(positions.len(), 2, "v1.16 compile fix {label} expected two definitions, found {}", positions.len());
    let start = positions[1];
    let open_rel = source[start..].find('{').expect("function opening brace");
    let open = start + open_rel;
    let bytes = source.as_bytes();
    let mut depth = 0i32;
    let mut end = None;
    for i in open..bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(i + 1);
                    break;
                }
            }
            _ => {}
        }
    }
    let end = end.expect("function closing brace");
    source.replace_range(start..end, "");
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.16 compile fix {label} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn patch_overlay(out: &Path) {
    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read generated v1.16 overlay");

    // The v1.15 chain intentionally retained its original click helper. v1.16
    // adds a history-aware replacement, so keep the newer first definition and
    // remove the stale later copy.
    remove_second_function(&mut source, "unsafe fn dps_row_at(", "dps_row_at");

    // Inspector/detail tables still use encounter-rate calculations even though
    // Encounter/s was removed from the compact meter columns.
    if !source.contains("fn rate(value:i64,ms:u64)->f64{") {
        let marker = "fn primary_metric(row:&DpsRow,mode:SortMode)->String{";
        let pos = source.find(marker).expect("primary_metric anchor for rate helper");
        source.insert_str(pos, "fn rate(value:i64,ms:u64)->f64{if ms==0{0.0}else{value as f64/(ms as f64/1000.0)}}\n");
    }

    // Keep the existing Target / HP summary preference meaningful. The compact
    // encounter strip stays visible for time/context, but target identity and HP
    // disappear when the user disables that setting.
    replace_once(
        &mut source,
        "draw(hdc,&target_title(snapshot),RECT{",
        "draw(hdc,&if settings.meter.show_target{target_title(snapshot)}else{\"Encounter\".into()},RECT{",
        "target title preference",
    );
    replace_once(
        &mut source,
        "let status=format!(\"{}   TIME {}\",target_hp(snapshot),format_time(snapshot.encounter_ms));",
        "let status=if settings.meter.show_target{format!(\"{}   TIME {}\",target_hp(snapshot),format_time(snapshot.encounter_ms))}else{format!(\"TIME {}\",format_time(snapshot.encounter_ms))};",
        "target HP preference",
    );

    assert_eq!(source.matches("unsafe fn dps_row_at(").count(), 1, "v1.16 compile fix must leave one dps_row_at");
    fs::write(path, source).expect("write compile-fixed v1.16 overlay");
}

fn patch_owner_draw_types(out: &Path) {
    for name in ["settings_ui_v1160_fixed.rs", "event_tracker_ui_v1160_fixed.rs"] {
        let path = out.join(name);
        let mut source = fs::read_to_string(&path).unwrap_or_else(|_| panic!("read {name}"));
        let old = "windows_sys::Win32::UI::WindowsAndMessaging::DRAWITEMSTRUCT";
        let count = source.matches(old).count();
        assert!(count >= 1, "v1.16 compile fix expected DRAWITEMSTRUCT references in {name}");
        source = source.replace(old, "windows_sys::Win32::UI::Controls::DRAWITEMSTRUCT");
        fs::write(path, source).unwrap_or_else(|_| panic!("write {name}"));
    }
}

fn main() {
    prior::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_overlay(&out);
    patch_owner_draw_types(&out);
    println!("cargo:rerun-if-changed=build_v1160_compilefix.rs");
}
