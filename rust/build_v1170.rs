use std::{env, fs, path::{Path, PathBuf}};

mod prior {
    include!("build_v1167.rs");
    pub fn run() { main(); }
}

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){
    let count=source.matches(from).count();
    assert_eq!(count,1,"v1.17.0 patch {label} expected one match, found {count}");
    *source=source.replacen(from,to,1);
}
fn replace_between(source:&mut String,start:&str,end:&str,replacement:&str,label:&str){
    let start_count=source.matches(start).count();let end_count=source.matches(end).count();
    assert_eq!(start_count,1,"v1.17.0 patch {label} start expected one match, found {start_count}");
    assert_eq!(end_count,1,"v1.17.0 patch {label} end expected one match, found {end_count}");
    let a=source.find(start).expect("v1.17.0 start anchor");
    let b=source[a..].find(end).map(|i|a+i).expect("v1.17.0 end anchor");
    source.replace_range(a..b,replacement);
}

fn patch_settings(out:&Path){
    let path=out.join("settings_ui_v1160_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read generated settings UI");
    replace_once(&mut source,"    form: crate::ui::ScrollForm,\n","","remove Settings scroll form");
    replace_once(&mut source,
        "        background: CreateSolidBrush(rgb(19, 24, 29)),\n        input_background: CreateSolidBrush(rgb(34, 43, 52)),\n        form: Default::default(),",
        "        background: CreateSolidBrush(crate::ui_theme::BG),\n        input_background: CreateSolidBrush(crate::ui_theme::INPUT),",
        "Settings design tokens");
    replace_once(&mut source,
        "        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_THICKFRAME,\n        CW_USEDEFAULT, CW_USEDEFAULT, 900, 680,",
        "        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,\n        CW_USEDEFAULT, CW_USEDEFAULT, crate::ui_theme::SETTINGS_W, crate::ui_theme::SETTINGS_H,",
        "fixed Settings window");
    replace_once(&mut source,"    try_dark_titlebar(hwnd);","    crate::ui_theme::dark_titlebar(hwnd);","Settings title bar");
    replace_once(&mut source,"                (*state_ptr).form.capture(hwnd, 878, 640);\n","","remove Settings capture");
    replace_once(&mut source,
        "        WM_SIZE => { if !state_ptr.is_null() { (*state_ptr).form.layout(hwnd); } 0 }\n\n        crate::ui::WM_REVEAL_FOCUS => { if !state_ptr.is_null() { (*state_ptr).form.reveal_focus(hwnd); } 0 }",
        "        WM_SIZE => 0,\n\n        crate::ui::WM_REVEAL_FOCUS => 0,",
        "remove Settings resize scrolling");
    replace_once(&mut source,
        "                FillRect(hdc,&r,(*state_ptr).background);\n                let side=RECT{left:0,top:0,right:166,bottom:r.bottom};\n                let side_br=CreateSolidBrush(rgb(23,29,35)); FillRect(hdc,&side,side_br); DeleteObject(side_br);\n                let rail=RECT{left:164,top:0,right:166,bottom:r.bottom};\n                let rail_br=CreateSolidBrush(rgb(45,89,85)); FillRect(hdc,&rail,rail_br); DeleteObject(rail_br);",
        "                FillRect(hdc,&r,(*state_ptr).background);\n                let side=RECT{left:0,top:0,right:156,bottom:r.bottom};\n                let side_br=CreateSolidBrush(crate::ui_theme::SIDEBAR); FillRect(hdc,&side,side_br); DeleteObject(side_br);\n                let rail=RECT{left:155,top:0,right:156,bottom:r.bottom};\n                let rail_br=CreateSolidBrush(crate::ui_theme::BORDER); FillRect(hdc,&rail,rail_br); DeleteObject(rail_br);",
        "Settings sidebar");
    replace_once(&mut source,
        "                SetTextColor(hdc, rgb(230, 237, 243));\n                SetBkColor(hdc, rgb(19, 24, 29));",
        "                SetTextColor(hdc, crate::ui_theme::TEXT);\n                SetBkColor(hdc, crate::ui_theme::BG);",
        "Settings static colors");
    replace_once(&mut source,
        "                SetTextColor(hdc, rgb(235, 240, 244));\n                SetBkColor(hdc, rgb(34, 43, 52));",
        "                SetTextColor(hdc, crate::ui_theme::TEXT);\n                SetBkColor(hdc, crate::ui_theme::INPUT);",
        "Settings input colors");
    replace_once(&mut source,
        "        0x002B => {\n            if !state_ptr.is_null() { draw_settings_button(&*state_ptr, lparam as *const windows_sys::Win32::UI::Controls::DRAWITEMSTRUCT) } else { 0 }\n        }",
        "        0x002B => {\n            let item=lparam as *const windows_sys::Win32::UI::Controls::DRAWITEMSTRUCT;\n            if item.is_null(){0}else if (*item).CtlType==3{crate::ui_theme::draw_combo(item)}else if !state_ptr.is_null(){draw_settings_button(&*state_ptr,item)}else{0}\n        }",
        "Settings owner draw routing");
    replace_between(&mut source,
        "unsafe fn build_ui(hwnd: HWND, state: &mut SettingsState) {",
        "unsafe fn load_all(hwnd: HWND, state: &mut SettingsState) {",
        include_str!("ui_v1170/settings_layout.txt"),
        "Settings layouts");
    replace_once(&mut source,
        "    if let Some((control,_))=state.page_controls.first() { let hwnd=windows_sys::Win32::UI::WindowsAndMessaging::GetParent(*control); state.form.reset(hwnd); for i in 0..PAGE_COUNT { InvalidateRect(GetDlgItem(hwnd,NAV_BASE+i as i32),null(),0); } InvalidateRect(hwnd,null(),0); }",
        "    if let Some((control,_))=state.page_controls.first() { let hwnd=windows_sys::Win32::UI::WindowsAndMessaging::GetParent(*control); for i in 0..PAGE_COUNT { InvalidateRect(GetDlgItem(hwnd,NAV_BASE+i as i32),null(),0); } InvalidateRect(hwnd,null(),0); }",
        "Settings page switch");
    replace_once(&mut source,
        "unsafe fn heading(hwnd: HWND, state: &mut SettingsState, page: usize, text: &str, x: i32, y: i32) {\n    let h = create_static(hwnd, text, x, y, 650, 28); state.page_controls.push((h, page));\n}",
        "unsafe fn heading(hwnd: HWND, state: &mut SettingsState, page: usize, text: &str, x: i32, y: i32) {\n    let h = create_static(hwnd, text, x, y, 580, 24); crate::ui_theme::set_font(h,crate::ui_theme::FontRole::Heading); state.page_controls.push((h, page));\n}\nunsafe fn page_intro(hwnd:HWND,state:&mut SettingsState,page:usize,title:&str,subtitle:&str){heading(hwnd,state,page,title,174,14);info(hwnd,state,page,subtitle,174,40,580,20);}\nunsafe fn section(hwnd:HWND,state:&mut SettingsState,page:usize,text:&str,x:i32,y:i32,w:i32){let c=create_static(hwnd,text,x,y,w,18);crate::ui_theme::set_font(c,crate::ui_theme::FontRole::Secondary);state.page_controls.push((c,page));}\nunsafe fn inline_field(hwnd:HWND,state:&mut SettingsState,page:usize,text:&str,id:i32,x:i32,y:i32,label_w:i32,edit_w:i32){label(hwnd,state,page,text,x,y+4,label_w,20);edit(hwnd,state,page,id,x+label_w+6,y,edit_w,26,false);}",
        "Settings typography helpers");
    replace_once(&mut source,
        "unsafe fn info(hwnd: HWND, state: &mut SettingsState, page: usize, text: &str, x: i32, y: i32, w: i32, h: i32) {\n    let c = create_static(hwnd, text, x, y, w, h); state.page_controls.push((c, page));\n}",
        "unsafe fn info(hwnd: HWND, state: &mut SettingsState, page: usize, text: &str, x: i32, y: i32, w: i32, h: i32) {\n    let c = create_static(hwnd, text, x, y, w, h); crate::ui_theme::set_font(c,crate::ui_theme::FontRole::Secondary); state.page_controls.push((c, page));\n}",
        "Settings secondary text");
    replace_once(&mut source,
        "    let c = create_control(hwnd, \"COMBOBOX\", \"\", id, x, y, w, h, WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_VSCROLL | CBS_DROPDOWNLIST, 0); state.page_controls.push((c, page));",
        "    let c = create_control(hwnd, \"COMBOBOX\", \"\", id, x, y, w, h, WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_VSCROLL | CBS_DROPDOWNLIST | 0x0010 | 0x0200, 0); state.page_controls.push((c, page));",
        "Settings dark combos");
    replace_once(&mut source,
        "        let font = GetStockObject(DEFAULT_GUI_FONT);\n        SendMessageW(c, WM_SETFONT, font as usize, 1);\n        let theme = wide(\"DarkMode_Explorer\");\n        let _ = SetWindowTheme(c, theme.as_ptr(), null());",
        "        crate::ui_theme::theme_control(c);",
        "Settings control theme");
    replace_between(&mut source,
        "unsafe fn draw_settings_button(state: &SettingsState, item: *const windows_sys::Win32::UI::Controls::DRAWITEMSTRUCT) -> LRESULT {",
        "unsafe fn try_dark_titlebar(hwnd: HWND) {",
        "unsafe fn draw_settings_button(state: &SettingsState, item: *const windows_sys::Win32::UI::Controls::DRAWITEMSTRUCT) -> LRESULT {\n    if item.is_null(){return 0;} let id=(*item).CtlID as i32;\n    let selected=id>=NAV_BASE&&id<NAV_BASE+PAGE_COUNT as i32&&(id-NAV_BASE)as usize==state.current_page;\n    let primary=id==ID_APPLY; let danger=matches!(id,ID_TAB_DELETE|ID_CLEAR_BLOCKED); let nav=id>=NAV_BASE&&id<NAV_BASE+PAGE_COUNT as i32;\n    crate::ui_theme::draw_button(item,selected,primary,danger,nav)\n}\n\n",
        "Settings button design");
    fs::write(path,source).expect("write v1.17 Settings UI");
}

fn patch_event_tracker(out:&Path){
    let path=out.join("event_tracker_ui_v1160_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read generated Event Tracker UI");
    replace_once(&mut source,"const WIDTH: i32 = 760;\nconst HEIGHT: i32 = 600;","const WIDTH: i32 = crate::ui_theme::EVENT_W;\nconst HEIGHT: i32 = crate::ui_theme::EVENT_H;","Event Tracker size");
    replace_once(&mut source,"    form: crate::ui::ScrollForm,\n","","remove Event Tracker scroll form");
    replace_once(&mut source,
        "        brush: CreateSolidBrush(rgb(19, 24, 29)),\n        input_brush: CreateSolidBrush(rgb(34, 43, 52)),\n        form: Default::default(),",
        "        brush: CreateSolidBrush(crate::ui_theme::BG),\n        input_brush: CreateSolidBrush(crate::ui_theme::INPUT),",
        "Event Tracker design tokens");
    replace_once(&mut source,
        "        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | 0x00040000,",
        "        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,",
        "fixed Event Tracker window");
    replace_once(&mut source,"    try_dark_titlebar(hwnd);","    crate::ui_theme::dark_titlebar(hwnd);","Event Tracker title bar");
    replace_once(&mut source,"            if !ptr.is_null() { build_controls(hwnd, &mut *ptr); (*ptr).form.capture(hwnd, 740, 550); }","            if !ptr.is_null() { build_controls(hwnd, &mut *ptr); }","remove Event Tracker capture");
    replace_once(&mut source,
        "        0x0005 => { if !ptr.is_null() { (*ptr).form.layout(hwnd); } 0 }\n\n        crate::ui::WM_REVEAL_FOCUS => { if !ptr.is_null() { (*ptr).form.reveal_focus(hwnd); } 0 }",
        "        0x0005 => 0,\n\n        crate::ui::WM_REVEAL_FOCUS => 0,",
        "remove Event Tracker resize scrolling");
    replace_once(&mut source,"            SetTextColor(hdc,rgb(230,237,243));\n            SetBkColor(hdc,rgb(19,24,29));","            SetTextColor(hdc,crate::ui_theme::TEXT);\n            SetBkColor(hdc,crate::ui_theme::BG);","Event Tracker static colors");
    replace_once(&mut source,"            SetTextColor(hdc,rgb(235,240,244));\n            SetBkColor(hdc,rgb(34,43,52));","            SetTextColor(hdc,crate::ui_theme::TEXT);\n            SetBkColor(hdc,crate::ui_theme::INPUT);","Event Tracker input colors");
    replace_once(&mut source,
        "        0x002B => { if !ptr.is_null() { draw_tracker_button(lparam as *const windows_sys::Win32::UI::Controls::DRAWITEMSTRUCT) } else { 0 } }",
        "        0x002B => {let item=lparam as *const windows_sys::Win32::UI::Controls::DRAWITEMSTRUCT;if item.is_null(){0}else if (*item).CtlType==3{crate::ui_theme::draw_combo(item)}else if !ptr.is_null(){draw_tracker_button(item)}else{0}}",
        "Event Tracker owner draw routing");
    replace_between(&mut source,
        "unsafe fn build_controls(hwnd: HWND, state: &mut UiState) {",
        "unsafe fn handle_command(hwnd: HWND, state: &mut UiState, id: i32, code: u16) {",
        include_str!("ui_v1170/event_tracker_layout.txt"),
        "Event Tracker layout");
    replace_once(&mut source,"            crate::ui::SetFocus(GetDlgItem(hwnd,ID_EVENT_ID));state.form.reveal_focus(hwnd);","            crate::ui::SetFocus(GetDlgItem(hwnd,ID_EVENT_ID));","Event Tracker focus");
    replace_once(&mut source,
        "    create_control(hwnd, \"COMBOBOX\", \"\", id, x, y, w, h, WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_VSCROLL | CBS_DROPDOWNLIST, 0)",
        "    create_control(hwnd, \"COMBOBOX\", \"\", id, x, y, w, h, WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_VSCROLL | CBS_DROPDOWNLIST | 0x0010 | 0x0200, 0)",
        "Event Tracker dark combos");
    replace_once(&mut source,
        "        SendMessageW(c, WM_SETFONT, GetStockObject(DEFAULT_GUI_FONT) as usize, 1);\n        let theme = wide(\"DarkMode_Explorer\");\n        let _ = SetWindowTheme(c, theme.as_ptr(), null());",
        "        crate::ui_theme::theme_control(c);",
        "Event Tracker control theme");
    replace_between(&mut source,
        "unsafe fn draw_tracker_button(item: *const windows_sys::Win32::UI::Controls::DRAWITEMSTRUCT) -> LRESULT {",
        "unsafe fn message_error(hwnd: HWND, text: &str) {",
        "unsafe fn draw_tracker_button(item:*const windows_sys::Win32::UI::Controls::DRAWITEMSTRUCT)->LRESULT{if item.is_null(){return 0;}let id=(*item).CtlID as i32;crate::ui_theme::draw_button(item,false,id==ID_APPLY,id==ID_REMOVE,false)}\n\n",
        "Event Tracker buttons");
    fs::write(path,source).expect("write v1.17 Event Tracker UI");
}

fn patch_feature_settings(out:&Path){
    let path=out.join("feature_overlays_v170_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read generated feature overlays");
    replace_once(&mut source,
        "let ptr=Box::into_raw(Box::new(SettingsState{kind:state.kind,parent,paths:state.paths.clone(),features:state.features.clone(),form:Default::default(),background:CreateSolidBrush(rgb(19,24,29))}));",
        "let ptr=Box::into_raw(Box::new(SettingsState{kind:state.kind,parent,paths:state.paths.clone(),features:state.features.clone(),form:Default::default(),background:CreateSolidBrush(crate::ui_theme::BG)}));",
        "feature Settings background");
    replace_once(&mut source,
        "    let (w,h)=if state.kind==Kind::Dps{(600,625)}else{(770,580)};\n    let hwnd=CreateWindowExW(WS_EX_TOOLWINDOW|WS_EX_TOPMOST,class.as_ptr(),title.as_ptr(),WS_POPUP|WS_THICKFRAME|0x00c00000|0x00080000|0x02000000,0,0,w,h,parent,null_mut(),instance,ptr.cast::<c_void>());",
        "    let (w,h)=if state.kind==Kind::Dps{(crate::ui_theme::DPS_SETTINGS_W,crate::ui_theme::DPS_SETTINGS_H)}else{(crate::ui_theme::MECH_SETTINGS_W,crate::ui_theme::MECH_SETTINGS_H)};\n    let hwnd=CreateWindowExW(WS_EX_TOOLWINDOW|WS_EX_TOPMOST,class.as_ptr(),title.as_ptr(),WS_POPUP|0x00c00000|0x00080000|0x02000000,0,0,w,h,parent,null_mut(),instance,ptr.cast::<c_void>());",
        "fixed feature Settings windows");
    replace_once(&mut source,
        "    state.settings_hwnd=hwnd;crate::ui::fit_window(hwnd,parent,true);ShowWindow(hwnd,SW_SHOW);SetForegroundWindow(hwnd);",
        "    state.settings_hwnd=hwnd;crate::ui::fit_window(hwnd,parent,true);crate::ui_theme::dark_titlebar(hwnd);ShowWindow(hwnd,SW_SHOW);SetForegroundWindow(hwnd);",
        "feature Settings titlebar");
    replace_once(&mut source,"    SendMessageW(c,0x0030,GetStockObject(DEFAULT_GUI_FONT) as usize,1);c","    crate::ui_theme::theme_control(c);c","feature Settings control theme");
    replace_once(&mut source,"unsafe fn feature_label(hwnd:HWND,id:i32,text:&str,x:i32,y:i32,w:i32,h:i32) {feature_control(hwnd,\"STATIC\",id,text,x,y,w,h,0x80);}","unsafe fn feature_label(hwnd:HWND,id:i32,text:&str,x:i32,y:i32,w:i32,h:i32)->HWND {feature_control(hwnd,\"STATIC\",id,text,x,y,w,h,0x80)}","feature Settings heading handle");
    replace_once(&mut source,"unsafe fn feature_button(hwnd:HWND,id:i32,text:&str,x:i32,y:i32,w:i32) {feature_control(hwnd,\"BUTTON\",id,text,x,y,w,30,0x00010000);}","unsafe fn feature_button(hwnd:HWND,id:i32,text:&str,x:i32,y:i32,w:i32) {feature_control(hwnd,\"BUTTON\",id,text,x,y,w,30,0x00010000|0x0000000B);}","feature Settings buttons");
    replace_once(&mut source,"    let c=feature_control(hwnd,\"COMBOBOX\",id,\"\",x,y,155,180,0x00210003);","    let c=feature_control(hwnd,\"COMBOBOX\",id,\"\",x,y,155,180,0x00210213);","feature Settings dark combos");
    replace_between(&mut source,
        "unsafe fn build_feature_form(hwnd:HWND,state:&mut SettingsState) {",
        "unsafe fn refresh_feature_form(hwnd:HWND,state:&SettingsState){",
        include_str!("ui_v1170/feature_settings_layout.txt"),
        "feature Settings layout");
    replace_once(&mut source,
        "        WM_SIZE=>{state.form.layout(hwnd);0}\n\n        crate::ui::WM_REVEAL_FOCUS=>{state.form.reveal_focus(hwnd);0}",
        "        WM_SIZE=>0,\n\n        crate::ui::WM_REVEAL_FOCUS=>0,",
        "remove feature Settings resizing");
    replace_once(&mut source,
        "        0x0135|0x0138=>{let hdc=wparam as HDC;windows_sys::Win32::Graphics::Gdi::SetBkColor(hdc,rgb(19,24,29));SetTextColor(hdc,rgb(230,236,244));state.background as isize}",
        "        0x0135|0x0138=>{let hdc=wparam as HDC;windows_sys::Win32::Graphics::Gdi::SetBkColor(hdc,crate::ui_theme::BG);SetTextColor(hdc,crate::ui_theme::TEXT);state.background as isize}\n        0x002B=>{let item=lparam as *const windows_sys::Win32::UI::Controls::DRAWITEMSTRUCT;if item.is_null(){0}else if (*item).CtlType==3{crate::ui_theme::draw_combo(item)}else{crate::ui_theme::draw_button(item,false,false,false,false)}}",
        "feature Settings owner draw");
    replace_once(&mut source,
        "unsafe fn paint_popup_toolbar(hdc:HDC,rc:RECT,title:&str){fill(hdc,&RECT{left:0,top:0,right:rc.right,bottom:TOOLBAR_H},rgb(23,28,35));fill(hdc,&RECT{left:0,top:TOOLBAR_H-2,right:rc.right,bottom:TOOLBAR_H},rgb(66,211,190));SetTextColor(hdc,rgb(231,237,244));",
        "unsafe fn paint_popup_toolbar(hdc:HDC,rc:RECT,title:&str){fill(hdc,&RECT{left:0,top:0,right:rc.right,bottom:TOOLBAR_H},crate::ui_theme::SIDEBAR);fill(hdc,&RECT{left:0,top:TOOLBAR_H-2,right:rc.right,bottom:TOOLBAR_H},crate::ui_theme::ACCENT);SetTextColor(hdc,crate::ui_theme::TEXT);",
        "overlay toolbar tokens");
    fs::write(path,source).expect("write v1.17 feature UI");
}

fn main(){
    prior::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_settings(&out);
    patch_event_tracker(&out);
    patch_feature_settings(&out);
    println!("cargo:rerun-if-changed=build_v1170.rs");
    println!("cargo:rerun-if-changed=ui_v1170/settings_layout.txt");
    println!("cargo:rerun-if-changed=ui_v1170/event_tracker_layout.txt");
    println!("cargo:rerun-if-changed=ui_v1170/feature_settings_layout.txt");
    println!("cargo:rerun-if-changed=src/ui_theme.rs");
}
