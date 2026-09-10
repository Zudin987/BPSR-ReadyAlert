use std::{env, fs, path::{Path, PathBuf}};

mod v1160_core {
    include!("build_v1160.rs");

    // Reuse the validated v1.15 chain plus the v1.16 meter/telemetry patches,
    // but let this final layer generate the settings shells with structural
    // anchors instead of whitespace-sensitive literals.
    pub fn run_core() {
        previous::run();
        let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
        patch_overlay(&out);
        patch_telemetry(&out);
    }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.16 final patch {label} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn replace_between(source: &mut String, start: &str, end: &str, replacement: &str, label: &str) {
    let start_count = source.matches(start).count();
    let end_count = source.matches(end).count();
    assert_eq!(start_count, 1, "v1.16 final patch {label} start expected one match, found {start_count}");
    assert_eq!(end_count, 1, "v1.16 final patch {label} end expected one match, found {end_count}");
    let a = source.find(start).expect("v1.16 final start anchor");
    let b = source[a..].find(end).map(|i| a + i).expect("v1.16 final end anchor");
    source.replace_range(a..b, replacement);
}

fn patch_settings_source(manifest: &Path, out: &Path) {
    let input = manifest.join("src/settings_ui.rs");
    let mut source = fs::read_to_string(&input).expect("read settings_ui.rs");
    source = source.replace("BPSRReadyAlertRustSettingsV151", "BPSRReadyAlertRustSettingsV160");
    source = source.replace("CreateSolidBrush(rgb(22, 25, 30))", "CreateSolidBrush(rgb(19, 24, 29))");
    source = source.replace("CreateSolidBrush(rgb(31, 36, 43))", "CreateSolidBrush(rgb(34, 43, 52))");
    source = source.replace("SetTextColor(hdc, rgb(225, 231, 238));", "SetTextColor(hdc, rgb(230, 237, 243));");
    source = source.replace("SetBkColor(hdc, rgb(22, 25, 30));", "SetBkColor(hdc, rgb(19, 24, 29));");
    source = source.replace("SetTextColor(hdc, rgb(235, 239, 244));", "SetTextColor(hdc, rgb(235, 240, 244));");
    source = source.replace("SetBkColor(hdc, rgb(31, 36, 43));", "SetBkColor(hdc, rgb(34, 43, 52));");
    source = source.replace("WM_CTLCOLOREDIT => {", "WM_CTLCOLOREDIT | 0x0134 => {");
    source = source.replace("WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON, 0", "WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON | 0x0000_000B, 0");
    source = source.replace(", WS_EX_CLIENTEDGE);", ", 0);");
    source = source.replace(
        "state.form.reset(hwnd); InvalidateRect(hwnd,null(),1);",
        "state.form.reset(hwnd); for i in 0..PAGE_COUNT { InvalidateRect(GetDlgItem(hwnd,NAV_BASE+i as i32),null(),0); } InvalidateRect(hwnd,null(),0);",
    );

    let erase = r#"        WM_ERASEBKGND => {
            if !state_ptr.is_null() {
                let mut r: RECT=std::mem::zeroed(); GetClientRect(hwnd,&mut r); let hdc=wparam as HDC;
                FillRect(hdc,&r,(*state_ptr).background);
                let side=RECT{left:0,top:0,right:166,bottom:r.bottom};
                let side_br=CreateSolidBrush(rgb(23,29,35)); FillRect(hdc,&side,side_br); DeleteObject(side_br);
                let rail=RECT{left:164,top:0,right:166,bottom:r.bottom};
                let rail_br=CreateSolidBrush(rgb(45,89,85)); FillRect(hdc,&rail,rail_br); DeleteObject(rail_br);
                return 1;
            }
            DefWindowProcW(hwnd,msg,wparam,lparam)
        }
"#;
    replace_between(
        &mut source,
        "        WM_ERASEBKGND => {",
        "        WM_CTLCOLORSTATIC | WM_CTLCOLORBTN => {",
        erase,
        "settings layered background",
    );

    replace_once(
        &mut source,
        "        WM_CLOSE => { DestroyWindow(hwnd); 0 }",
        r#"        0x002B => {
            if !state_ptr.is_null() { draw_settings_button(&*state_ptr, lparam as *const windows_sys::Win32::UI::WindowsAndMessaging::DRAWITEMSTRUCT) } else { 0 }
        }
        WM_CLOSE => { DestroyWindow(hwnd); 0 }"#,
        "settings owner-draw arm",
    );

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
    let len=GetWindowTextLengthW(item.hwndItem).max(0)as usize; let mut buf=vec![0u16;len+1];
    let got=GetWindowTextW(item.hwndItem,buf.as_mut_ptr(),buf.len()as i32).max(0)as usize;
    windows_sys::Win32::Graphics::Gdi::SetBkMode(item.hDC,1);
    SetTextColor(item.hDC,if selected||apply{rgb(238,247,246)}else{rgb(211,220,227)});
    let mut text_rect=item.rcItem;
    windows_sys::Win32::Graphics::Gdi::DrawTextW(item.hDC,buf.as_ptr(),got as i32,&mut text_rect,0x0001|0x0004|0x0020|0x0800);
    1
}

"#;
    replace_once(
        &mut source,
        "unsafe fn try_dark_titlebar(hwnd: HWND) {",
        &format!("{helper}unsafe fn try_dark_titlebar(hwnd: HWND) {{"),
        "settings owner-draw helper",
    );

    let headings = [
        ("heading(hwnd, state, PAGE_GENERAL, \"General alerts\", 180, 18);", "heading(hwnd, state, PAGE_GENERAL, \"READYALERT // GENERAL\", 180, 18);"),
        ("heading(hwnd, state, PAGE_OVERLAY, \"Chat overlay\", 180, 18);", "heading(hwnd, state, PAGE_OVERLAY, \"READYALERT // CHAT OVERLAY\", 180, 18);"),
        ("heading(hwnd, state, PAGE_COLORS, \"Overlay colors & visual highlight\", 180, 18);", "heading(hwnd, state, PAGE_COLORS, \"READYALERT // CHAT COLORS\", 180, 18);"),
        ("heading(hwnd, state, PAGE_SPEECH, \"Speech & translation\", 180, 18);", "heading(hwnd, state, PAGE_SPEECH, \"READYALERT // SPEECH\", 180, 18);"),
        ("heading(hwnd, state, PAGE_TABS, \"Tabs & filters\", 180, 18);", "heading(hwnd, state, PAGE_TABS, \"READYALERT // TABS & FILTERS\", 180, 18);"),
        ("heading(hwnd, state, PAGE_SOUNDS, \"Highlights, sounds & logs\", 180, 18);", "heading(hwnd, state, PAGE_SOUNDS, \"READYALERT // SOUNDS & LOGS\", 180, 18);"),
        ("heading(hwnd, state, PAGE_NETWORK, \"Network & integration\", 180, 18);", "heading(hwnd, state, PAGE_NETWORK, \"READYALERT // NETWORK\", 180, 18);"),
        ("heading(hwnd, state, PAGE_BLOCKED, \"Blocked users\", 180, 18);", "heading(hwnd, state, PAGE_BLOCKED, \"READYALERT // BLOCKED USERS\", 180, 18);"),
    ];
    for (from,to) in headings { source = source.replace(from,to); }

    fs::write(out.join("settings_ui_v1160_fixed.rs"), source).expect("write v1.16 settings ui");
    println!("cargo:rerun-if-changed={}", input.display());
}

fn patch_tracker_source(manifest: &Path, out: &Path) {
    let input = manifest.join("src/event_tracker_ui.rs");
    let mut source = fs::read_to_string(&input).expect("read event_tracker_ui.rs");
    source = source.replace("BPSRReadyAlertEventTrackerV114", "BPSRReadyAlertEventTrackerV160");
    replace_once(&mut source, "    brush: HBRUSH,\n    form: crate::ui::ScrollForm,", "    brush: HBRUSH,\n    input_brush: HBRUSH,\n    form: crate::ui::ScrollForm,", "tracker input brush field");
    replace_once(&mut source, "        brush: CreateSolidBrush(rgb(22, 25, 30)),\n        form: Default::default(),", "        brush: CreateSolidBrush(rgb(19, 24, 29)),\n        input_brush: CreateSolidBrush(rgb(34, 43, 52)),\n        form: Default::default(),", "tracker input brush init");
    replace_once(&mut source, "if !state.brush.is_null() { DeleteObject(state.brush); }\n        message_error", "if !state.brush.is_null() { DeleteObject(state.brush); } if !state.input_brush.is_null() { DeleteObject(state.input_brush); }\n        message_error", "tracker failed-create cleanup");
    replace_once(&mut source, "                if !state.brush.is_null() { DeleteObject(state.brush); }\n            }", "                if !state.brush.is_null() { DeleteObject(state.brush); } if !state.input_brush.is_null() { DeleteObject(state.input_brush); }\n            }", "tracker destroy cleanup");
    source = source.replace("WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON, 0", "WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON | 0x0000_000B, 0");
    source = source.replace(", WS_EX_CLIENTEDGE)", ", 0)");
    source = source.replace("\"Custom Event Tracker\"", "\"READYALERT // EVENT TRACKER\"");
    source = source.replace("\"BPSR ReadyAlert - Custom Event Tracker\"", "\"BPSR ReadyAlert // Event Tracker\"");

    let colors = r#"        WM_CTLCOLORSTATIC | WM_CTLCOLORBTN => {
            if ptr.is_null() { return DefWindowProcW(hwnd, msg, wparam, lparam); }
            let hdc=wparam as HDC; SetTextColor(hdc,rgb(230,237,243)); SetBkColor(hdc,rgb(19,24,29)); (*ptr).brush as LRESULT
        }
        WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX => {
            if ptr.is_null() { return DefWindowProcW(hwnd, msg, wparam, lparam); }
            let hdc=wparam as HDC; SetTextColor(hdc,rgb(235,240,244)); SetBkColor(hdc,rgb(34,43,52)); (*ptr).input_brush as LRESULT
        }
"#;
    replace_between(
        &mut source,
        "        WM_CTLCOLORSTATIC | WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX | WM_CTLCOLORBTN => {",
        "        WM_CLOSE =>",
        colors,
        "tracker control palette",
    );
    replace_once(
        &mut source,
        "        WM_CLOSE => { if !ptr.is_null() { close_form(hwnd,&mut *ptr); } 0 }",
        r#"        0x002B => { if !ptr.is_null() { draw_tracker_button(lparam as *const windows_sys::Win32::UI::WindowsAndMessaging::DRAWITEMSTRUCT) } else { 0 } }
        WM_CLOSE => { if !ptr.is_null() { close_form(hwnd,&mut *ptr); } 0 }"#,
        "tracker owner-draw arm",
    );

    let helper = r#"unsafe fn draw_tracker_button(item: *const windows_sys::Win32::UI::WindowsAndMessaging::DRAWITEMSTRUCT) -> LRESULT {
    if item.is_null(){return 0;} let item=&*item; let apply=item.CtlID as i32==ID_APPLY;
    let bg=if apply{rgb(38,112,103)}else{rgb(30,38,46)}; let border=if apply{rgb(66,211,190)}else{rgb(49,61,72)};
    let brush=CreateSolidBrush(bg); windows_sys::Win32::Graphics::Gdi::FillRect(item.hDC,&item.rcItem,brush); DeleteObject(brush);
    let top=RECT{left:item.rcItem.left,top:item.rcItem.top,right:item.rcItem.right,bottom:item.rcItem.top+1};
    let bottom=RECT{left:item.rcItem.left,top:item.rcItem.bottom-1,right:item.rcItem.right,bottom:item.rcItem.bottom};
    let left=RECT{left:item.rcItem.left,top:item.rcItem.top,right:item.rcItem.left+1,bottom:item.rcItem.bottom};
    let right=RECT{left:item.rcItem.right-1,top:item.rcItem.top,right:item.rcItem.right,bottom:item.rcItem.bottom};
    let edge=CreateSolidBrush(border); for r in [&top,&bottom,&left,&right]{windows_sys::Win32::Graphics::Gdi::FillRect(item.hDC,r,edge);} DeleteObject(edge);
    let len=GetWindowTextLengthW(item.hwndItem).max(0)as usize; let mut buf=vec![0u16;len+1];
    let got=GetWindowTextW(item.hwndItem,buf.as_mut_ptr(),buf.len()as i32).max(0)as usize;
    windows_sys::Win32::Graphics::Gdi::SetBkMode(item.hDC,1); SetTextColor(item.hDC,if apply{rgb(238,247,246)}else{rgb(211,220,227)});
    let mut rect=item.rcItem; windows_sys::Win32::Graphics::Gdi::DrawTextW(item.hDC,buf.as_ptr(),got as i32,&mut rect,0x0001|0x0004|0x0020|0x0800); 1
}

"#;
    replace_once(
        &mut source,
        "unsafe fn message_error(hwnd: HWND, text: &str) {",
        &format!("{helper}unsafe fn message_error(hwnd: HWND, text: &str) {{"),
        "tracker owner-draw helper",
    );
    fs::write(out.join("event_tracker_ui_v1160_fixed.rs"), source).expect("write v1.16 tracker ui");
    println!("cargo:rerun-if-changed={}", input.display());
}

fn main() {
    v1160_core::run_core();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    patch_settings_source(&manifest, &out);
    patch_tracker_source(&manifest, &out);
    println!("cargo:rerun-if-changed=build_v1160_final.rs");
    println!("cargo:rerun-if-changed=overlay_v1160_meter_patch.txt");
}
