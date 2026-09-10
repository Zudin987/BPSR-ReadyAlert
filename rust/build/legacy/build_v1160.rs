use std::{env, fs, path::{Path, PathBuf}};

mod previous {
    include!("build_v1150.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.16 patch {label} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn replace_between(source: &mut String, start: &str, end: &str, replacement: &str, label: &str) {
    let start_count = source.matches(start).count();
    let end_count = source.matches(end).count();
    assert_eq!(start_count, 1, "v1.16 patch {label} start expected one match, found {start_count}");
    assert_eq!(end_count, 1, "v1.16 patch {label} end expected one match, found {end_count}");
    let a = source.find(start).expect("v1.16 start anchor");
    let b = source[a..].find(end).map(|i| a + i).expect("v1.16 end anchor");
    source.replace_range(a..b, replacement);
}

fn patch_overlay(out: &Path) {
    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read v1.15 generated overlay");
    let meter = include_str!("overlay_v1160_meter_patch.txt");
    replace_between(
        &mut source,
        "#[derive(Clone,Copy)]struct DpsRowLayout",
        "unsafe fn draw_percent(",
        meter,
        "combat meter layout",
    );

    let muted_spec = r#"fn spec_color(row:&DpsRow)->u32{let spec=row.subprofession_name.trim().to_ascii_lowercase();match spec.as_str(){"iaido"|"iai"=>hex_color(0x7655a8),"moonstrike"=>hex_color(0x46365f),"icicle"|"ice spear"=>hex_color(0x69b8b2),"frostbeam"|"crystal"=>hex_color(0x3f817c),"formless"|"voidflame"=>hex_color(0xb85c3e),"crimson"|"blazecrimson"=>hex_color(0x7f3325),"vanguard"=>hex_color(0x4d9eaa),"skyward"=>hex_color(0x275c6d),"smite"=>hex_color(0x799d55),"lifebind"=>hex_color(0x405e30),"earthfort"=>hex_color(0x9c864c),"block"=>hex_color(0x5e522e),"wildpack"|"taming"=>hex_color(0xafa66c),"falconry"=>hex_color(0x8d822c),"recovery"=>hex_color(0x7d9295),"shield"=>hex_color(0x46575a),"dissonance"=>hex_color(0xa45c6b),"concerto"=>hex_color(0x6f3138),_=>rgb(46,53,63)}}
"#;
    replace_between(&mut source, "fn spec_color(row:&DpsRow)->u32{", "fn text_on(color:u32)->u32{", muted_spec, "muted class colors");

    let compact = r#"fn compact(value:f64)->String{let abs=value.abs();if abs>=1_000_000_000.0{format!("{:.2} B",value/1_000_000_000.0)}else if abs>=1_000_000.0{format!("{:.2} M",value/1_000_000.0)}else if abs>=1_000.0{format!("{:.1} K",value/1_000.0)}else{format!("{value:.0}")}}
"#;
    replace_between(&mut source, "fn compact(value:f64)->String{", "fn compact_attr(value:i64)->String{", compact, "spaced metric suffixes");

    source = source.replace("Active + encounter rates", "Active rate");
    source = source.replace("Total / Active / Encounter / Share", "Total / Active / Share");
    source = source.replace("Total, active and encounter rates", "Total and active rates");
    source = source.replace("rgb(22,25,30)", "rgb(19,24,29)");
    source = source.replace("rgb(31,36,43)", "rgb(34,43,52)");
    fs::write(path, source).expect("write v1.16 overlay");
}

fn patch_telemetry(out: &Path) {
    let path = out.join("telemetry_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read generated telemetry");

    replace_once(
        &mut source,
        "    owner_uuid: i64,\n    remodel_level: i32,",
        "    owner_uuid: i64,\n    remodel_level: i32,\n    remodel_seen: bool,",
        "Imagine remodel freshness field",
    );
    replace_once(
        &mut source,
        "                                meta.remodel_level = value.clamp(0, i32::MAX as i64) as i32;",
        "                                meta.remodel_level = value.clamp(0, i32::MAX as i64) as i32;\n                                meta.remodel_seen = true;",
        "Imagine remodel freshness observation",
    );

    let imagine_logic = r#"    fn record_imagine_from_damage(&mut self, uid: i64, raw_attacker: i64, skill: i32, owner_level: i32) {
        if let Some((fantasy_skill, observed_tier)) = self.entities.get(&raw_attacker).and_then(|meta| {
            fantasy_skill_for_monster(meta.monster_id).map(|fantasy_skill| {
                (fantasy_skill, meta.remodel_seen.then_some(meta.remodel_level))
            })
        }) {
            if let Some(tier) = observed_tier {
                self.record_imagine_observed(uid, fantasy_skill, tier);
            } else {
                self.record_imagine_fallback(uid, fantasy_skill, owner_level);
            }
            return;
        }
        // Some packets identify the Fantasy parent skill directly but do not carry
        // the authoritative ATTR_SKILL_REMODEL_LEVEL. This is fallback-only data:
        // it may seed a badge, but it must never overwrite a tier observed on the
        // summoned entity (notably a legitimate T0 with an unrelated field-13 = 1).
        if is_fantasy_skill(skill) {
            self.record_imagine_fallback(uid, skill, owner_level);
        }
    }

    fn register_fantasy_summon(&mut self, summon_uuid: i64, marker_source: Option<i32>) {
        let Some(meta) = self.entities.get(&summon_uuid).cloned() else { return; };
        if !(3_000_000..=3_009_999).contains(&meta.monster_id) || meta.owner_uuid == 0 {
            return;
        }
        let owner_uid = if entity_kind(meta.owner_uuid) == ENTITY_PLAYER {
            meta.owner_uuid >> 16
        } else {
            return;
        };
        let skill = marker_source
            .and_then(normalize_fantasy_source)
            .or_else(|| fantasy_skill_for_monster(meta.monster_id));
        if let Some(skill) = skill {
            if meta.remodel_seen {
                self.record_imagine_observed(owner_uid, skill, meta.remodel_level);
            }
        }
    }

    fn record_imagine_observed(&mut self, uid: i64, skill: i32, tier: i32) {
        self.combat.entry(uid).or_default().imagines.insert(skill, tier.max(0));
    }

    fn record_imagine_fallback(&mut self, uid: i64, skill: i32, tier: i32) {
        self.combat
            .entry(uid)
            .or_default()
            .imagines
            .entry(skill)
            .or_insert(tier.max(0));
    }

"#;
    replace_between(
        &mut source,
        "    fn record_imagine_from_damage(&mut self, uid: i64, raw_attacker: i64, skill: i32, owner_level: i32) {",
        "    fn reset_encounter_keep_roster(&mut self) {",
        imagine_logic,
        "Imagine trusted tier merge",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1160_imagine_tier_tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn imagine_tier_trusted_t0_survives_later_fallback_t1() {
        let (tx, _rx) = mpsc::channel();
        let mut runtime = TelemetryRuntime::new(tx);
        runtime.record_imagine_observed(42, 3921, 0);
        runtime.record_imagine_fallback(42, 3921, 1);
        assert_eq!(runtime.combat.get(&42).and_then(|a| a.imagines.get(&3921)).copied(), Some(0));
    }

    #[test]
    fn imagine_tier_trusted_observation_replaces_earlier_fallback() {
        let (tx, _rx) = mpsc::channel();
        let mut runtime = TelemetryRuntime::new(tx);
        runtime.record_imagine_fallback(42, 3921, 1);
        runtime.record_imagine_observed(42, 3921, 0);
        assert_eq!(runtime.combat.get(&42).and_then(|a| a.imagines.get(&3921)).copied(), Some(0));
    }

    #[test]
    fn imagine_tier_t5_is_not_degraded_by_fallback() {
        let (tx, _rx) = mpsc::channel();
        let mut runtime = TelemetryRuntime::new(tx);
        runtime.record_imagine_observed(42, 3921, 5);
        runtime.record_imagine_fallback(42, 3921, 1);
        assert_eq!(runtime.combat.get(&42).and_then(|a| a.imagines.get(&3921)).copied(), Some(5));
    }
}
"#);
    fs::write(path, source).expect("write v1.16 telemetry");
}

fn patch_settings_source(manifest: &Path, out: &Path) {
    let input = manifest.join("src/settings_ui.rs");
    let mut source = fs::read_to_string(&input).expect("read settings_ui.rs");
    source = source.replace("BPSRReadyAlertRustSettingsV151", "BPSRReadyAlertRustSettingsV160");
    source = source.replace("CreateSolidBrush(rgb(22, 25, 30))", "CreateSolidBrush(rgb(19, 24, 29))");
    source = source.replace("CreateSolidBrush(rgb(31, 36, 43))", "CreateSolidBrush(rgb(34, 43, 52))");
    source = source.replace("SetTextColor(hdc, rgb(225, 231, 238));\n                SetBkColor(hdc, rgb(22, 25, 30));", "SetTextColor(hdc, rgb(230, 237, 243));\n                SetBkColor(hdc, rgb(19, 24, 29));");
    source = source.replace("SetTextColor(hdc, rgb(235, 239, 244));\n                SetBkColor(hdc, rgb(31, 36, 43));", "SetTextColor(hdc, rgb(235, 240, 244));\n                SetBkColor(hdc, rgb(34, 43, 52));");
    source = source.replace("WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON, 0)", "WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON | 0x0000_000B, 0)");
    source = source.replace(", WS_EX_CLIENTEDGE);", ", 0);");
    source = source.replace("InvalidateRect(hwnd,null(),1);", "for i in 0..PAGE_COUNT { InvalidateRect(GetDlgItem(hwnd,NAV_BASE+i as i32),null(),0); } InvalidateRect(hwnd,null(),0);");

    let erase_old = r#"WM_ERASEBKGND => {
            if !state_ptr.is_null() { let mut r: RECT=std::mem::zeroed(); GetClientRect(hwnd,&mut r); FillRect(wparam as HDC,&r,(*state_ptr).background); return 1; }
            DefWindowProcW(hwnd,msg,wparam,lparam)
        }"#;
    let erase_new = r#"WM_ERASEBKGND => {
            if !state_ptr.is_null() {
                let mut r: RECT=std::mem::zeroed(); GetClientRect(hwnd,&mut r); let hdc=wparam as HDC;
                FillRect(hdc,&r,(*state_ptr).background);
                let side=RECT{left:0,top:0,right:166,bottom:r.bottom}; let side_br=CreateSolidBrush(rgb(23,29,35)); FillRect(hdc,&side,side_br); DeleteObject(side_br);
                let rail=RECT{left:164,top:0,right:166,bottom:r.bottom}; let rail_br=CreateSolidBrush(rgb(45,89,85)); FillRect(hdc,&rail,rail_br); DeleteObject(rail_br);
                return 1;
            }
            DefWindowProcW(hwnd,msg,wparam,lparam)
        }"#;
    replace_once(&mut source, erase_old, erase_new, "settings layered background");

    let close_arm = "        WM_CLOSE => { DestroyWindow(hwnd); 0 }";
    let draw_arm = r#"        0x002B => {
            if !state_ptr.is_null() { draw_settings_button(&*state_ptr, lparam as *const windows_sys::Win32::UI::WindowsAndMessaging::DRAWITEMSTRUCT) } else { 0 }
        }
        WM_CLOSE => { DestroyWindow(hwnd); 0 }"#;
    replace_once(&mut source, close_arm, draw_arm, "settings owner-draw arm");

    let helper = r#"unsafe fn draw_settings_button(state: &SettingsState, item: *const windows_sys::Win32::UI::WindowsAndMessaging::DRAWITEMSTRUCT) -> LRESULT {
    if item.is_null() { return 0; }
    let item=&*item; let id=item.CtlID as i32;
    let selected=id>=NAV_BASE&&id<NAV_BASE+PAGE_COUNT as i32&&(id-NAV_BASE)as usize==state.current_page;
    let apply=id==ID_APPLY;
    let bg=if selected{rgb(30,48,51)}else if apply{rgb(38,112,103)}else{rgb(30,38,46)};
    let border=if selected||apply{rgb(66,211,190)}else{rgb(49,61,72)};
    let brush=CreateSolidBrush(bg); FillRect(item.hDC,&item.rcItem,brush); DeleteObject(brush);
    let top=RECT{left:item.rcItem.left,top:item.rcItem.top,right:item.rcItem.right,bottom:item.rcItem.top+1};
    let bottom=RECT{left:item.rcItem.left,top:item.rcItem.bottom-1,right:item.rcItem.right,bottom:item.rcItem.bottom};
    let left=RECT{left:item.rcItem.left,top:item.rcItem.top,right:item.rcItem.left+if selected{3}else{1},bottom:item.rcItem.bottom};
    let right=RECT{left:item.rcItem.right-1,top:item.rcItem.top,right:item.rcItem.right,bottom:item.rcItem.bottom};
    let edge=CreateSolidBrush(border); for r in [&top,&bottom,&left,&right] { FillRect(item.hDC,r,edge); } DeleteObject(edge);
    let len=GetWindowTextLengthW(item.hwndItem).max(0)as usize; let mut buf=vec![0u16;len+1]; let got=GetWindowTextW(item.hwndItem,buf.as_mut_ptr(),buf.len()as i32).max(0)as usize;
    windows_sys::Win32::Graphics::Gdi::SetBkMode(item.hDC,1); SetTextColor(item.hDC,if selected||apply{rgb(238,247,246)}else{rgb(211,220,227)});
    let mut text_rect=item.rcItem; windows_sys::Win32::Graphics::Gdi::DrawTextW(item.hDC,buf.as_ptr(),got as i32,&mut text_rect,0x0001|0x0004|0x0020|0x0800);
    1
}

"#;
    replace_once(&mut source, "unsafe fn try_dark_titlebar(hwnd: HWND) {", &format!("{helper}unsafe fn try_dark_titlebar(hwnd: HWND) {{"), "settings owner-draw helper");

    source = source.replace("\"General alerts\"", "\"READYALERT // GENERAL\"");
    source = source.replace("\"Chat overlay\"", "\"READYALERT // CHAT OVERLAY\"");
    source = source.replace("\"Overlay colors & visual highlight\"", "\"READYALERT // CHAT COLORS\"");
    source = source.replace("\"Speech & translation\"", "\"READYALERT // SPEECH\"");
    source = source.replace("\"Tabs & filters\"", "\"READYALERT // TABS & FILTERS\"");
    source = source.replace("\"Highlights, sounds & logs\"", "\"READYALERT // SOUNDS & LOGS\"");
    source = source.replace("\"Network & integration\"", "\"READYALERT // NETWORK\"");
    source = source.replace("\"Blocked users\"", "\"READYALERT // BLOCKED USERS\"");
    fs::write(out.join("settings_ui_v1160_fixed.rs"), source).expect("write v1.16 settings ui");
    println!("cargo:rerun-if-changed={}", input.display());
}

fn patch_tracker_source(manifest: &Path, out: &Path) {
    let input = manifest.join("src/event_tracker_ui.rs");
    let mut source = fs::read_to_string(&input).expect("read event_tracker_ui.rs");
    source = source.replace("BPSRReadyAlertEventTrackerV114", "BPSRReadyAlertEventTrackerV160");
    source = source.replace("brush: HBRUSH,\n    form:", "brush: HBRUSH,\n    input_brush: HBRUSH,\n    form:");
    source = source.replace("brush: CreateSolidBrush(rgb(22, 25, 30)),\n        form:", "brush: CreateSolidBrush(rgb(19, 24, 29)),\n        input_brush: CreateSolidBrush(rgb(34, 43, 52)),\n        form:");
    source = source.replace("if !state.brush.is_null() { DeleteObject(state.brush); }\n        message_error", "if !state.brush.is_null() { DeleteObject(state.brush); } if !state.input_brush.is_null() { DeleteObject(state.input_brush); }\n        message_error");
    source = source.replace("if !state.brush.is_null() { DeleteObject(state.brush); }\n            }", "if !state.brush.is_null() { DeleteObject(state.brush); } if !state.input_brush.is_null() { DeleteObject(state.input_brush); }\n            }");
    source = source.replace("WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON, 0)", "WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON | 0x0000_000B, 0)");
    source = source.replace(", WS_EX_CLIENTEDGE)", ", 0)");
    source = source.replace("\"Custom Event Tracker\"", "\"READYALERT // EVENT TRACKER\"");
    source = source.replace("\"BPSR ReadyAlert - Custom Event Tracker\"", "\"BPSR ReadyAlert // Event Tracker\"");

    let color_old = r#"WM_CTLCOLORSTATIC | WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX | WM_CTLCOLORBTN => {
            if ptr.is_null() { return DefWindowProcW(hwnd, msg, wparam, lparam); }
            let hdc = wparam as HDC;
            SetTextColor(hdc, rgb(228, 232, 238));
            SetBkColor(hdc, rgb(22, 25, 30));
            (*ptr).brush as LRESULT
        }"#;
    let color_new = r#"WM_CTLCOLORSTATIC | WM_CTLCOLORBTN => {
            if ptr.is_null() { return DefWindowProcW(hwnd, msg, wparam, lparam); }
            let hdc=wparam as HDC; SetTextColor(hdc,rgb(230,237,243)); SetBkColor(hdc,rgb(19,24,29)); (*ptr).brush as LRESULT
        }
        WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX => {
            if ptr.is_null() { return DefWindowProcW(hwnd, msg, wparam, lparam); }
            let hdc=wparam as HDC; SetTextColor(hdc,rgb(235,240,244)); SetBkColor(hdc,rgb(34,43,52)); (*ptr).input_brush as LRESULT
        }"#;
    replace_once(&mut source, color_old, color_new, "tracker control palette");

    replace_once(&mut source, "        WM_CLOSE => { if !ptr.is_null() { close_form(hwnd,&mut *ptr); } 0 }", r#"        0x002B => { if !ptr.is_null() { draw_tracker_button(lparam as *const windows_sys::Win32::UI::WindowsAndMessaging::DRAWITEMSTRUCT) } else { 0 } }
        WM_CLOSE => { if !ptr.is_null() { close_form(hwnd,&mut *ptr); } 0 }"#, "tracker owner-draw arm");

    let helper = r#"unsafe fn draw_tracker_button(item: *const windows_sys::Win32::UI::WindowsAndMessaging::DRAWITEMSTRUCT) -> LRESULT {
    if item.is_null(){return 0;} let item=&*item; let apply=item.CtlID as i32==ID_APPLY;
    let bg=if apply{rgb(38,112,103)}else{rgb(30,38,46)}; let border=if apply{rgb(66,211,190)}else{rgb(49,61,72)};
    let brush=CreateSolidBrush(bg); windows_sys::Win32::Graphics::Gdi::FillRect(item.hDC,&item.rcItem,brush); DeleteObject(brush);
    let top=RECT{left:item.rcItem.left,top:item.rcItem.top,right:item.rcItem.right,bottom:item.rcItem.top+1}; let bottom=RECT{left:item.rcItem.left,top:item.rcItem.bottom-1,right:item.rcItem.right,bottom:item.rcItem.bottom}; let left=RECT{left:item.rcItem.left,top:item.rcItem.top,right:item.rcItem.left+1,bottom:item.rcItem.bottom}; let right=RECT{left:item.rcItem.right-1,top:item.rcItem.top,right:item.rcItem.right,bottom:item.rcItem.bottom};
    let edge=CreateSolidBrush(border); for r in [&top,&bottom,&left,&right]{windows_sys::Win32::Graphics::Gdi::FillRect(item.hDC,r,edge);} DeleteObject(edge);
    let len=GetWindowTextLengthW(item.hwndItem).max(0)as usize; let mut buf=vec![0u16;len+1]; let got=GetWindowTextW(item.hwndItem,buf.as_mut_ptr(),buf.len()as i32).max(0)as usize;
    windows_sys::Win32::Graphics::Gdi::SetBkMode(item.hDC,1); SetTextColor(item.hDC,if apply{rgb(238,247,246)}else{rgb(211,220,227)}); let mut rect=item.rcItem; windows_sys::Win32::Graphics::Gdi::DrawTextW(item.hDC,buf.as_ptr(),got as i32,&mut rect,0x0001|0x0004|0x0020|0x0800); 1
}

"#;
    replace_once(&mut source, "unsafe fn message_error(hwnd: HWND, text: &str) {", &format!("{helper}unsafe fn message_error(hwnd: HWND, text: &str) {{"), "tracker owner-draw helper");
    fs::write(out.join("event_tracker_ui_v1160_fixed.rs"), source).expect("write v1.16 tracker ui");
    println!("cargo:rerun-if-changed={}", input.display());
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    patch_overlay(&out);
    patch_telemetry(&out);
    patch_settings_source(&manifest, &out);
    patch_tracker_source(&manifest, &out);
    println!("cargo:rerun-if-changed=overlay_v1160_meter_patch.txt");
}
