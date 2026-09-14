use std::{env, fs, path::{Path, PathBuf}};

mod previous {
    include!("build_v1315_audit_hardening.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.31.6 UI polish patch {label:?} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn patch_pixel_controls(out: &Path, manifest: &Path) {
    let input = manifest.join("src/ui_pixel_controls.rs");
    let mut source = fs::read_to_string(&input)
        .unwrap_or_else(|e| panic!("read ui_pixel_controls.rs for v1.31.6 UI polish: {e}"))
        .replace("\r\n", "\n");

    replace_once(
        &mut source,
        "const LB_GETITEMHEIGHT_: u32 = 0x01a1;",
        r#"const LB_GETITEMHEIGHT_: u32 = 0x01a1;
const CB_SETITEMHEIGHT_: u32 = 0x0153;
const ODS_COMBOBOXEDIT_: u32 = 0x1000;
const COMBO_LIST_ROW_H_: i32 = 26;
const SWP_NOZORDER_: u32 = 0x0004;
const SWP_NOACTIVATE_: u32 = 0x0010;

#[repr(C)]
struct ComboPoint { x: i32, y: i32 }

#[link(name = "user32")]
extern "system" {
    #[link_name = "GetParent"]
    fn combo_get_parent(hwnd: HWND) -> HWND;
    #[link_name = "GetWindowRect"]
    fn combo_get_window_rect(hwnd: HWND, rect: *mut RECT) -> i32;
    #[link_name = "ClientToScreen"]
    fn combo_client_to_screen(hwnd: HWND, point: *mut ComboPoint) -> i32;
}

#[link(name = "gdi32")]
extern "system" {
    #[link_name = "MoveToEx"]
    fn combo_move_to_ex(hdc: HDC, x: i32, y: i32, old: *mut c_void) -> i32;
    #[link_name = "LineTo"]
    fn combo_line_to(hdc: HDC, x: i32, y: i32) -> i32;
}

/// Vector check mark used by both native and hand-painted settings checkboxes.
/// It is rasterized directly at the final control size, so DPI scaling does not
/// stretch a tiny glyph/bitmap into the jagged mark visible in the old UI.
pub unsafe fn draw_vector_checkmark(hdc: HDC, rect: RECT, color: u32) {
    let width = (rect.right - rect.left).max(1);
    let height = (rect.bottom - rect.top).max(1);
    let stroke = (width.min(height) / 8).clamp(2, 3);
    let pen = CreatePen(PS_SOLID_, stroke, color);
    if pen.is_null() { return; }
    let old = SelectObject(hdc, pen);
    let x1 = rect.left + width * 3 / 14;
    let y1 = rect.top + height * 7 / 13;
    let x2 = rect.left + width * 6 / 14;
    let y2 = rect.top + height * 10 / 13;
    let x3 = rect.left + width * 11 / 14;
    let y3 = rect.top + height * 4 / 13;
    combo_move_to_ex(hdc, x1, y1, null_mut());
    combo_line_to(hdc, x2, y2);
    combo_line_to(hdc, x3, y3);
    SelectObject(hdc, old);
    DeleteObject(pen);
}

/// Keep every ReadyAlert combo popup on-screen and inside its settings surface.
/// Native Win32 dropdowns choose a direction on their own, but the owner-drawn
/// list could still overlap/crop at a short fixed dialog edge. Prefer opening up
/// whenever the full list will not fit below the combo.
pub unsafe fn position_combo_dropdown(hwnd: HWND) {
    if hwnd.is_null() { return; }
    let mut info: windows_sys::Win32::UI::Controls::COMBOBOXINFO = std::mem::zeroed();
    info.cbSize = std::mem::size_of_val(&info) as u32;
    if windows_sys::Win32::UI::Controls::GetComboBoxInfo(hwnd, &mut info) == 0 || info.hwndList.is_null() { return; }
    let parent = combo_get_parent(hwnd);
    if parent.is_null() { return; }

    let mut combo: RECT = std::mem::zeroed();
    let mut list: RECT = std::mem::zeroed();
    let mut client: RECT = std::mem::zeroed();
    if combo_get_window_rect(hwnd, &mut combo) == 0
        || combo_get_window_rect(info.hwndList, &mut list) == 0
        || GetClientRect(parent, &mut client) == 0
    { return; }

    let mut client_top = ComboPoint { x: client.left, y: client.top };
    let mut client_bottom = ComboPoint { x: client.right, y: client.bottom };
    if combo_client_to_screen(parent, &mut client_top) == 0
        || combo_client_to_screen(parent, &mut client_bottom) == 0
    { return; }

    let list_w = (list.right - list.left).max(combo.right - combo.left).max(1);
    let list_h = (list.bottom - list.top).max(COMBO_LIST_ROW_H_).max(1);
    let room_above = combo.top - client_top.y;
    let room_below = client_bottom.y - combo.bottom;
    let needs_reposition = list.bottom > client_bottom.y || list.top < client_top.y;
    if !needs_reposition { return; }

    let y = if room_above >= list_h || room_above > room_below {
        (combo.top - list_h).max(client_top.y)
    } else {
        combo.bottom.min((client_bottom.y - list_h).max(client_top.y))
    };
    SetWindowPos(
        info.hwndList,
        null_mut(),
        list.left,
        y,
        list_w,
        list_h,
        SWP_NOZORDER_ | SWP_NOACTIVATE_,
    );
}"#,
        "shared combo metrics, popup placement and vector checkbox primitive",
    );

    replace_once(
        &mut source,
        r#"pub unsafe fn theme_combo(hwnd: HWND) {
    if hwnd.is_null() { return; }
    set_control_font(hwnd, body_font());
    let cfd = wide("DarkMode_CFD");
    if SetWindowTheme(hwnd, cfd.as_ptr(), null()) != 0 { dark_common_control(hwnd); }
}"#,
        r#"pub unsafe fn theme_combo(hwnd: HWND) {
    if hwnd.is_null() { return; }
    set_control_font(hwnd, body_font());
    // One item metric for every dropdown in Settings, Event Tracker and the
    // DPS/Mechanics settings. This prevents the cramped 18px popup rows.
    SendMessageW(hwnd, CB_SETITEMHEIGHT_, 0, COMBO_LIST_ROW_H_ as isize);
    let cfd = wide("DarkMode_CFD");
    if SetWindowTheme(hwnd, cfd.as_ptr(), null()) != 0 { dark_common_control(hwnd); }
}"#,
        "standardize combo item height",
    );

    replace_once(
        &mut source,
        r#"unsafe fn draw_combo_family(item: *const DRAWITEMSTRUCT, family: SurfaceFamily) -> isize {
    if item.is_null() { return 0; }
    let item = &*item;
    let disabled = item.itemState & 0x0004 != 0;
    let selected = item.itemState & 0x0001 != 0;
    let focused = item.itemState & 0x0010 != 0;
    let mut rect = item.rcItem;
    rect.left += 1; rect.top += 1; rect.right -= 1; rect.bottom -= 1;
    let back = if disabled { family_surface(family) } else if selected { family_selected(family) } else { family_input(family) };
    fill_round_rect(item.hDC, rect, back, RADIUS_SMALL);
    if focused { stroke_round_rect(item.hDC, rect, BPSR_ACCENT, RADIUS_SMALL, 2); }

    let index = if item.itemID == u32::MAX { SendMessageW(item.hwndItem, 0x0147, 0, 0) } else { item.itemID as isize };
    if index >= 0 {
        let len = SendMessageW(item.hwndItem, 0x0149, index as usize, 0).max(0) as usize;
        let mut buf = vec![0u16; len + 1];
        let got = SendMessageW(item.hwndItem, 0x0148, index as usize, buf.as_mut_ptr() as isize).max(0) as usize;
        SetBkMode(item.hDC, TRANSPARENT as i32);
        SetTextColor(item.hDC, if disabled { BPSR_DISABLED } else { BPSR_TEXT });
        SelectObject(item.hDC, body_font());
        let mut text = item.rcItem; text.left += 11; text.right -= 10;
        DrawTextW(item.hDC, buf.as_ptr(), got as i32, &mut text, 0x0004 | 0x0020 | 0x0800 | 0x8000);
    }
    1
}"#,
        r#"unsafe fn draw_combo_family(item: *const DRAWITEMSTRUCT, family: SurfaceFamily) -> isize {
    if item.is_null() { return 0; }
    let item = &*item;
    let disabled = item.itemState & 0x0004 != 0;
    let selected = item.itemState & 0x0001 != 0;
    let focused = item.itemState & 0x0010 != 0;
    let closed_face = item.itemState & ODS_COMBOBOXEDIT_ != 0 || item.itemID == u32::MAX;
    let back = if disabled { family_surface(family) } else if selected { family_selected(family) } else { family_input(family) };

    if closed_face {
        let mut rect = item.rcItem;
        rect.left += 1; rect.top += 1; rect.right -= 1; rect.bottom -= 1;
        fill_round_rect(item.hDC, rect, back, RADIUS_SMALL);
        if focused { stroke_round_rect(item.hDC, rect, BPSR_ACCENT, RADIUS_SMALL, 2); }
    } else {
        // Popup rows are a single continuous list. The old renderer rounded every
        // row independently, producing the white/rounded caps visible in screenshots.
        let brush = CreateSolidBrush(back);
        if !brush.is_null() {
            FillRect(item.hDC, &item.rcItem, brush);
            DeleteObject(brush);
        }
    }

    let index = if item.itemID == u32::MAX { SendMessageW(item.hwndItem, 0x0147, 0, 0) } else { item.itemID as isize };
    if index >= 0 {
        let len = SendMessageW(item.hwndItem, 0x0149, index as usize, 0).max(0) as usize;
        let mut buf = vec![0u16; len + 1];
        let got = SendMessageW(item.hwndItem, 0x0148, index as usize, buf.as_mut_ptr() as isize).max(0) as usize;
        SetBkMode(item.hDC, TRANSPARENT as i32);
        SetTextColor(item.hDC, if disabled { BPSR_DISABLED } else { BPSR_TEXT });
        SelectObject(item.hDC, if selected && !closed_face { medium_font() } else { body_font() });
        let mut text = item.rcItem;
        text.left += 11; text.right -= 10;
        DrawTextW(item.hDC, buf.as_ptr(), got as i32, &mut text, 0x0004 | 0x0020 | 0x0800 | 0x8000);
    }
    1
}"#,
        "render dropdown popup as continuous rows",
    );

    replace_once(
        &mut source,
        "fill_round_rect(hdc, glyph, fill, if radio { side / 2 } else { 5 });",
        "fill_round_rect(hdc, glyph, fill, if radio { side / 2 } else { 4 });",
        "native checkbox corner radius",
    );
    replace_once(
        &mut source,
        "stroke_round_rect(hdc, glyph, if focused { BPSR_ACCENT } else { MIST_BORDER_STRONG }, if radio { side / 2 } else { 5 }, if focused { 2 } else { 1 });",
        "stroke_round_rect(hdc, glyph, if focused { BPSR_ACCENT } else { MIST_BORDER_STRONG }, if radio { side / 2 } else { 4 }, if focused { 2 } else { 1 });",
        "native checkbox outline radius",
    );
    replace_once(
        &mut source,
        "if enabled && focused && active { stroke_round_rect(hdc, glyph, BPSR_ACCENT_HOVER, if radio { side / 2 } else { 5 }, 2); }",
        "if enabled && focused && active { stroke_round_rect(hdc, glyph, BPSR_ACCENT_HOVER, if radio { side / 2 } else { 4 }, 2); }",
        "native checkbox focus radius",
    );
    replace_once(
        &mut source,
        r#"        } else {
            let mark = wide("✓");
            let mut r = glyph;
            SetBkMode(hdc, TRANSPARENT as i32); SetTextColor(hdc, BPSR_ACCENT_TEXT); SelectObject(hdc, medium_font());
            DrawTextW(hdc, mark.as_ptr(), 1, &mut r, 0x0001 | 0x0004 | 0x0020 | 0x0800);
        }"#,
        r#"        } else {
            draw_vector_checkmark(hdc, glyph, BPSR_ACCENT_TEXT);
        }"#,
        "replace scaled font checkmark with vector geometry",
    );

    fs::write(out.join("ui_pixel_controls_v1316.rs"), source)
        .unwrap_or_else(|e| panic!("write ui_pixel_controls_v1316.rs: {e}"));
}

fn patch_pixel_qa(out: &Path, manifest: &Path) {
    let input = manifest.join("src/ui_pixel_qa.rs");
    let mut source = fs::read_to_string(&input)
        .unwrap_or_else(|e| panic!("read ui_pixel_qa.rs for v1.31.6 UI polish: {e}"))
        .replace("\r\n", "\n");

    replace_once(
        &mut source,
        r#"        WM_SETFOCUS_ | WM_KILLFOCUS_ | WM_ENABLE_ | 0x014e | 0x014f => {
            let result = DefSubclassProc(hwnd, msg, wparam, lparam);
            InvalidateRect(hwnd, null(), 0);
            return result;
        }"#,
        r#"        0x014f => { // CB_SHOWDROPDOWN
            let result = DefSubclassProc(hwnd, msg, wparam, lparam);
            if wparam != 0 { position_combo_dropdown(hwnd); }
            InvalidateRect(hwnd, null(), 0);
            return result;
        }
        WM_SETFOCUS_ | WM_KILLFOCUS_ | WM_ENABLE_ | 0x014e => {
            let result = DefSubclassProc(hwnd, msg, wparam, lparam);
            InvalidateRect(hwnd, null(), 0);
            return result;
        }"#,
        "automatic combo popup direction",
    );

    fs::write(out.join("ui_pixel_qa_v1316.rs"), source)
        .unwrap_or_else(|e| panic!("write ui_pixel_qa_v1316.rs: {e}"));
}

fn patch_feature_overlays(out: &Path) {
    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read feature overlay for v1.31.6 UI polish: {e}"))
        .replace("\r\n", "\n");

    replace_once(
        &mut source,
        "let (w,h)=if state.kind==Kind::Dps{(crate::ui_theme::DPS_SETTINGS_W,crate::ui_theme::DPS_SETTINGS_H)}else{(crate::ui_theme::MECH_SETTINGS_W,crate::ui_theme::MECH_SETTINGS_H)};",
        "let (w,h)=if state.kind==Kind::Dps{(crate::ui_theme::DPS_SETTINGS_W,crate::ui_theme::DPS_SETTINGS_H+30)}else{(crate::ui_theme::MECH_SETTINGS_W,crate::ui_theme::MECH_SETTINGS_H)};",
        "DPS settings bottom breathing room",
    );

    replace_once(
        &mut source,
        "unsafe fn feature_combo(hwnd:HWND,id:i32,x:i32,y:i32,items:&[&str]) {",
        r#"const COLLAPSE_EDGE_ITEMS:[&str;4]=["Left","Right","Top","Bottom"];
fn collapse_edge_index(side:&str)->isize{match side.to_ascii_lowercase().as_str(){"right"=>1,"top"=>2,"bottom"=>3,_=>0}}
fn collapse_edge_value(index:isize)->&'static str{match index{1=>"Right",2=>"Top",3=>"Bottom",_=>"Left"}}

unsafe fn feature_combo(hwnd:HWND,id:i32,x:i32,y:i32,items:&[&str]) {"#,
        "canonical collapse-edge options",
    );
    replace_once(
        &mut source,
        "feature_label(hwnd,0,\"Collapse\",458,50,60,22);feature_combo_sized(hwnd,7003,520,44,84,&[\"Right\",\"Bottom\",\"Left\",\"Top\"]);",
        "feature_label(hwnd,0,\"Collapse\",458,50,60,22);feature_combo_sized(hwnd,7003,520,44,84,&COLLAPSE_EDGE_ITEMS);",
        "DPS and Mechanics collapse dropdown order",
    );
    replace_once(
        &mut source,
        "let side=match layout.collapse_side.to_ascii_lowercase().as_str(){\"bottom\"=>1,\"left\"=>2,\"top\"=>3,_=>0};SendMessageW(fc(hwnd,7003),0x014e,side,0);",
        "let side=collapse_edge_index(&layout.collapse_side);SendMessageW(fc(hwnd,7003),0x014e,side,0);",
        "collapse selection mapping",
    );
    replace_once(
        &mut source,
        "7003=>layout.collapse_side=match selected{1=>\"Bottom\",2=>\"Left\",3=>\"Top\",_=>\"Right\"}.into(),",
        "7003=>layout.collapse_side=collapse_edge_value(selected).into(),",
        "collapse save mapping",
    );

    replace_once(
        &mut source,
        r#"unsafe fn paint_mode_tab(hdc:HDC,x:i32,y:i32,width:i32,text:&str,mode:SortMode,active:bool){let r=RECT{left:x,top:y,right:x+width,bottom:y+23};if active{crate::ui_modern::fill_round_rect(hdc,r,crate::ui_modern::mix_color(crate::ui_modern::DARK_RAISED,mode_color(mode),18),crate::ui_modern::RADIUS_SMALL);let marker_w=20.min(width-12).max(8);let cx=(r.left+r.right)/2;crate::ui_modern::fill_round_rect(hdc,RECT{left:cx-marker_w/2,top:r.bottom-3,right:cx+(marker_w+1)/2,bottom:r.bottom-1},mode_color(mode),2);}SetTextColor(hdc,if active{crate::ui_modern::BPSR_TEXT}else{crate::ui_modern::BPSR_TEXT_SECONDARY});draw(hdc,text,r,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}"#,
        r#"unsafe fn paint_mode_tab(hdc:HDC,x:i32,y:i32,width:i32,text:&str,mode:SortMode,active:bool){let r=RECT{left:x,top:y,right:x+width,bottom:y+23};if active{crate::ui_modern::fill_round_rect(hdc,r,crate::ui_modern::mix_color(crate::ui_modern::DARK_RAISED,mode_color(mode),18),4);}SetTextColor(hdc,if active{crate::ui_modern::BPSR_TEXT}else{crate::ui_modern::BPSR_TEXT_SECONDARY});draw(hdc,text,r,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}"#,
        "single-signal DPS mode tab selection",
    );

    replace_once(
        &mut source,
        r#"unsafe fn paint_check(hdc:HDC,x:i32,y:i32,label:&str,checked:bool,right:i32){let box_r=RECT{left:x,top:y+3,right:x+18,bottom:y+21};crate::ui_modern::fill_round_rect(hdc,box_r,if checked{crate::ui_modern::BPSR_ACCENT}else{crate::ui_modern::DARK_RAISED},5);if !checked{crate::ui_modern::stroke_round_rect(hdc,box_r,crate::ui_modern::DARK_BORDER_STRONG,5,1);}if checked{SetTextColor(hdc,crate::ui_modern::BPSR_ACCENT_TEXT);draw(hdc,"✓",box_r,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}SetTextColor(hdc,crate::ui_modern::BPSR_TEXT_SECONDARY);draw(hdc,label,RECT{left:x+26,top:y,right:right-10,bottom:y+25},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}"#,
        r#"unsafe fn paint_check(hdc:HDC,x:i32,y:i32,label:&str,checked:bool,right:i32){let box_r=RECT{left:x,top:y+3,right:x+18,bottom:y+21};crate::ui_modern::fill_round_rect(hdc,box_r,if checked{crate::ui_modern::BPSR_ACCENT}else{crate::ui_modern::DARK_RAISED},4);if !checked{crate::ui_modern::stroke_round_rect(hdc,box_r,crate::ui_modern::DARK_BORDER_STRONG,4,1);}if checked{crate::ui_modern::draw_vector_checkmark(hdc,box_r,crate::ui_modern::BPSR_ACCENT_TEXT);}SetTextColor(hdc,crate::ui_modern::BPSR_TEXT_SECONDARY);draw(hdc,label,RECT{left:x+26,top:y,right:right-10,bottom:y+25},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}"#,
        "DPS and Mechanics vector checkbox rendering",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1316_ui_controls_polish_tests {
    use super::*;

    #[test]
    fn collapse_edge_order_is_canonical_and_round_trips() {
        assert_eq!(COLLAPSE_EDGE_ITEMS,["Left","Right","Top","Bottom"]);
        for (index, edge) in COLLAPSE_EDGE_ITEMS.iter().enumerate() {
            assert_eq!(collapse_edge_index(edge),index as isize);
            assert_eq!(collapse_edge_value(index as isize),*edge);
        }
    }

    #[test]
    fn unknown_collapse_edge_falls_back_to_left_consistently() {
        assert_eq!(collapse_edge_index("unknown"),0);
        assert_eq!(collapse_edge_value(99),"Left");
    }
}
"#);

    fs::write(path, source)
        .unwrap_or_else(|e| panic!("write feature overlay v1.31.6 UI polish: {e}"));
}

fn verify_all_combo_entry_points(out: &Path) {
    let checks = [
        ("settings_ui_v1160_fixed.rs", "qa_theme_mist_combo(c)"),
        ("event_tracker_ui_v1160_fixed.rs", "qa_theme_mist_combo(c)"),
        ("feature_overlays_v170_fixed.rs", "qa_theme_dark_combo(c)"),
    ];
    for (name, theme) in checks {
        let source = fs::read_to_string(out.join(name))
            .unwrap_or_else(|e| panic!("read {name} while verifying dropdown coverage: {e}"));
        let combo_count = source.matches("\"COMBOBOX\"").count();
        let theme_count = source.matches(theme).count();
        assert!(combo_count > 0, "v1.31.6 expected at least one COMBOBOX in {name}");
        assert_eq!(combo_count, theme_count, "v1.31.6 every dropdown in {name} must use the shared QA combo renderer");
    }
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    patch_pixel_controls(&out, &manifest);
    patch_pixel_qa(&out, &manifest);
    patch_feature_overlays(&out);
    verify_all_combo_entry_points(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1316_ui_controls_polish.rs");
    println!("cargo:rerun-if-changed=src/ui_pixel_controls.rs");
    println!("cargo:rerun-if-changed=src/ui_pixel_qa.rs");
}
