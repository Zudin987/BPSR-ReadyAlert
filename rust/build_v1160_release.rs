use std::{env, fs, path::{Path, PathBuf}};

mod prior {
    include!("build_v1160_final.rs");

    pub fn run_without_tracker() {
        v1160_core::run_core();
        let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
        let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
        patch_settings_source(&manifest, &out);
    }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.16 release patch {label} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn replace_between(source: &mut String, start: &str, end: &str, replacement: &str, label: &str) {
    let start_count = source.matches(start).count();
    let end_count = source.matches(end).count();
    assert_eq!(start_count, 1, "v1.16 release patch {label} start expected one match, found {start_count}");
    assert_eq!(end_count, 1, "v1.16 release patch {label} end expected one match, found {end_count}");
    let a = source.find(start).expect("v1.16 release start anchor");
    let b = source[a..].find(end).map(|i| a + i).expect("v1.16 release end anchor");
    source.replace_range(a..b, replacement);
}

fn patch_tracker_source(manifest: &Path, out: &Path) {
    let input = manifest.join("src/event_tracker_ui.rs");
    let mut source = fs::read_to_string(&input).expect("read event_tracker_ui.rs");
    source = source.replace("BPSRReadyAlertEventTrackerV114", "BPSRReadyAlertEventTrackerV160");

    replace_once(
        &mut source,
        "    brush: HBRUSH,",
        "    brush: HBRUSH,\n    input_brush: HBRUSH,",
        "tracker input brush field",
    );
    replace_once(
        &mut source,
        "        brush: CreateSolidBrush(rgb(22, 25, 30)),",
        "        brush: CreateSolidBrush(rgb(19, 24, 29)),\n        input_brush: CreateSolidBrush(rgb(34, 43, 52)),",
        "tracker input brush init",
    );

    let cleanup = "if !state.brush.is_null() { DeleteObject(state.brush); }";
    let cleanup_count = source.matches(cleanup).count();
    assert_eq!(cleanup_count, 2, "v1.16 release patch tracker brush cleanup expected two matches, found {cleanup_count}");
    source = source.replace(
        cleanup,
        "if !state.brush.is_null() { DeleteObject(state.brush); } if !state.input_brush.is_null() { DeleteObject(state.input_brush); }",
    );

    source = source.replace(
        "WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON, 0",
        "WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON | 0x0000_000B, 0",
    );
    source = source.replace(", WS_EX_CLIENTEDGE)", ", 0)");
    source = source.replace("\"Custom Event Tracker\"", "\"READYALERT // EVENT TRACKER\"");
    source = source.replace("\"BPSR ReadyAlert - Custom Event Tracker\"", "\"BPSR ReadyAlert // Event Tracker\"");

    let colors = r#"        WM_CTLCOLORSTATIC | WM_CTLCOLORBTN => {
            if ptr.is_null() { return DefWindowProcW(hwnd, msg, wparam, lparam); }
            let hdc=wparam as HDC;
            SetTextColor(hdc,rgb(230,237,243));
            SetBkColor(hdc,rgb(19,24,29));
            (*ptr).brush as LRESULT
        }
        WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX => {
            if ptr.is_null() { return DefWindowProcW(hwnd, msg, wparam, lparam); }
            let hdc=wparam as HDC;
            SetTextColor(hdc,rgb(235,240,244));
            SetBkColor(hdc,rgb(34,43,52));
            (*ptr).input_brush as LRESULT
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
    if item.is_null(){return 0;}
    let item=&*item;
    let apply=item.CtlID as i32==ID_APPLY;
    let bg=if apply{rgb(38,112,103)}else{rgb(30,38,46)};
    let border=if apply{rgb(66,211,190)}else{rgb(49,61,72)};
    let brush=CreateSolidBrush(bg);
    windows_sys::Win32::Graphics::Gdi::FillRect(item.hDC,&item.rcItem,brush);
    DeleteObject(brush);
    let top=RECT{left:item.rcItem.left,top:item.rcItem.top,right:item.rcItem.right,bottom:item.rcItem.top+1};
    let bottom=RECT{left:item.rcItem.left,top:item.rcItem.bottom-1,right:item.rcItem.right,bottom:item.rcItem.bottom};
    let left=RECT{left:item.rcItem.left,top:item.rcItem.top,right:item.rcItem.left+1,bottom:item.rcItem.bottom};
    let right=RECT{left:item.rcItem.right-1,top:item.rcItem.top,right:item.rcItem.right,bottom:item.rcItem.bottom};
    let edge=CreateSolidBrush(border);
    for r in [&top,&bottom,&left,&right]{windows_sys::Win32::Graphics::Gdi::FillRect(item.hDC,r,edge);}
    DeleteObject(edge);
    let len=GetWindowTextLengthW(item.hwndItem).max(0)as usize;
    let mut buf=vec![0u16;len+1];
    let got=GetWindowTextW(item.hwndItem,buf.as_mut_ptr(),buf.len()as i32).max(0)as usize;
    windows_sys::Win32::Graphics::Gdi::SetBkMode(item.hDC,1);
    SetTextColor(item.hDC,if apply{rgb(238,247,246)}else{rgb(211,220,227)});
    let mut rect=item.rcItem;
    windows_sys::Win32::Graphics::Gdi::DrawTextW(item.hDC,buf.as_ptr(),got as i32,&mut rect,0x0001|0x0004|0x0020|0x0800);
    1
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
    prior::run_without_tracker();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    patch_tracker_source(&manifest, &out);
    println!("cargo:rerun-if-changed=build_v1160_release.rs");
    println!("cargo:rerun-if-changed=overlay_v1160_meter_patch.txt");
}
