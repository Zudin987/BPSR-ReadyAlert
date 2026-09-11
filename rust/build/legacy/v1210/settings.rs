fn patch_settings(out:&Path){
    let path=out.join("settings_ui_v1160_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read v1.20 settings UI").replace("\r\n","\n");

    replace_once(&mut source,
        "const ID_APPLY: i32 = 3098;\nconst ID_CLOSE: i32 = 3099;",
        "const ID_VERSION: i32 = 3097;\nconst ID_APPLY: i32 = 3098;\nconst ID_CLOSE: i32 = 3099;",
        "version control id");
    replace_once(&mut source,
        "const ID_HIGHLIGHT_EXPR: i32 = 3422;",
        "const ID_HIGHLIGHT_EXPR: i32 = 3422;\nconst ID_COLOR_SWATCH_BASE:i32=3450;\nconst ID_HIGHLIGHT_SWATCH:i32=3460;\nconst ID_PRIVATE_SWATCH:i32=3461;",
        "color swatch ids");

    replace_once(&mut source,
        "#[link(name = \"user32\")]\nextern \"system\" {\n    fn EnableWindow(hwnd: HWND, enable: i32) -> i32;\n}",
        r###"#[link(name = "user32")]
extern "system" {
    fn EnableWindow(hwnd: HWND, enable: i32) -> i32;
}
#[repr(C)]
struct ChooseColorWData{l_struct_size:u32,hwnd_owner:HWND,h_instance:*mut c_void,rgb_result:u32,custom_colors:*mut u32,flags:u32,cust_data:isize,hook:*mut c_void,template:*const u16}
#[link(name="comdlg32")]
extern "system"{fn ChooseColorW(data:*mut ChooseColorWData)->i32;}"###,
        "native color picker");

    replace_once(&mut source,
        "    input_background: HBRUSH,\n}",
        "    input_background: HBRUSH,\n    sidebar_background:HBRUSH,\n    dirty:bool,\n}",
        "settings dirty state");
    replace_once(&mut source,
        "        input_background: CreateSolidBrush(crate::ui_theme::INPUT),\n    });",
        "        input_background: CreateSolidBrush(crate::ui_theme::INPUT),\n        sidebar_background:CreateSolidBrush(crate::ui_theme::SIDEBAR),\n        dirty:false,\n    });",
        "initialize dirty state");
    replace_once(&mut source,
        "        DeleteObject((*state_ptr).input_background);\n        drop(Box::from_raw(state_ptr));",
        "        DeleteObject((*state_ptr).input_background);\n        DeleteObject((*state_ptr).sidebar_background);\n        drop(Box::from_raw(state_ptr));",
        "cleanup failed create sidebar brush");
    replace_once(&mut source,
        "                DeleteObject((*state_ptr).input_background);\n                drop(Box::from_raw(state_ptr));",
        "                DeleteObject((*state_ptr).input_background);\n                DeleteObject((*state_ptr).sidebar_background);\n                drop(Box::from_raw(state_ptr));",
        "cleanup sidebar brush");

    replace_once(&mut source,
        "                load_all(hwnd, &mut *state_ptr);\n                let page = (*state_ptr).current_page;",
        "                load_all(hwnd, &mut *state_ptr);\n                set_dirty(hwnd,&mut *state_ptr,false);\n                let page = (*state_ptr).current_page;",
        "initial clean state");

    replace_once(&mut source,
        "        WM_COMMAND => {\n            if !state_ptr.is_null() { handle_command(hwnd, &mut *state_ptr, wparam); }\n            0\n        }",
        r###"        WM_COMMAND => {
            if !state_ptr.is_null() {
                let id=(wparam as u32&0xffff)as i32;let notify=((wparam as u32>>16)&0xffff)as u16;
                if let Some(edit)=swatch_edit_id(id){if choose_color(hwnd,edit){set_dirty(hwnd,&mut *state_ptr,true);}return 0;}
                if let Some(swatch)=edit_swatch_id(id){windows_sys::Win32::Graphics::Gdi::InvalidateRect(GetDlgItem(hwnd,swatch),null(),0);}
                if is_mutating_command(id,notify){set_dirty(hwnd,&mut *state_ptr,true);}
                handle_command(hwnd, &mut *state_ptr, wparam);
            }
            0
        }"###,
        "dirty tracking and swatches");

    replace_once(&mut source,
        "        WM_CTLCOLORSTATIC | WM_CTLCOLORBTN => {\n            if !state_ptr.is_null() {\n                let hdc = wparam as HDC;\n                SetTextColor(hdc, crate::ui_theme::TEXT);\n                SetBkColor(hdc, crate::ui_theme::BG);\n                return (*state_ptr).background as isize;\n            }\n            DefWindowProcW(hwnd, msg, wparam, lparam)\n        }",
        r###"        WM_CTLCOLORSTATIC | WM_CTLCOLORBTN => {
            if !state_ptr.is_null() {
                let hdc=wparam as HDC;let control=lparam as HWND;let id=windows_sys::Win32::UI::WindowsAndMessaging::GetDlgCtrlID(control);
                SetTextColor(hdc,crate::ui_theme::TEXT);
                if id==ID_VERSION{SetBkColor(hdc,crate::ui_theme::SIDEBAR);return (*state_ptr).sidebar_background as isize;}
                SetBkColor(hdc,crate::ui_theme::BG);return (*state_ptr).background as isize;
            }
            DefWindowProcW(hwnd,msg,wparam,lparam)
        }"###,
        "sidebar version background");

    replace_once(&mut source,
        "        WM_CLOSE => { DestroyWindow(hwnd); 0 }",
        "        WM_CLOSE => { close_settings(hwnd,&mut *state_ptr); 0 }",
        "unsaved close prompt");

    replace_once(&mut source,
        "    let version=create_static(hwnd,&format!(\"ReadyAlert  v{}\",env!(\"CARGO_PKG_VERSION\")),14,326,130,20);\n    crate::ui_theme::set_font(version,crate::ui_theme::FontRole::Secondary);\n    create_button(hwnd, ID_APPLY, \"Apply\", 610, 365, 80, 30);",
        "    let version=create_control(hwnd,\"STATIC\",&format!(\"ReadyAlert  v{}\",env!(\"CARGO_PKG_VERSION\")),ID_VERSION,14,326,130,20,WS_CHILD|WS_VISIBLE|0x80,0);\n    crate::ui_theme::set_font(version,crate::ui_theme::FontRole::Secondary);\n    let apply=create_button(hwnd,ID_APPLY,\"Apply\",610,365,80,30);EnableWindow(apply,0);",
        "version and apply state");

    replace_between(&mut source,"unsafe fn build_colors(hwnd: HWND, state: &mut SettingsState) {","unsafe fn build_speech(hwnd: HWND, state: &mut SettingsState) {",r###"unsafe fn build_colors(hwnd:HWND,state:&mut SettingsState){
    page_intro(hwnd,state,PAGE_COLORS,"Chat colors","Channel colors and message highlighting. Use #RRGGBB values or click a swatch.");
    section(hwnd,state,PAGE_COLORS,"CHANNEL COLORS",174,64,580);
    let entries=["World","Local","Team","Guild","Private","Group","Notice","Play","Newbie","System"];
    for(i,name)in entries.iter().enumerate(){let col=i/5;let row=i%5;let x=174+col as i32*286;let y=86+row as i32*32;inline_field(hwnd,state,PAGE_COLORS,name,ID_COLOR_BASE+i as i32,x,y,70,130);color_swatch(hwnd,state,PAGE_COLORS,ID_COLOR_SWATCH_BASE+i as i32,x+212,y+3);}
    section(hwnd,state,PAGE_COLORS,"HIGHLIGHTS",174,256,580);
    inline_field(hwnd,state,PAGE_COLORS,"Match rule",ID_HIGHLIGHT_EXPR,174,278,92,410);
    inline_field(hwnd,state,PAGE_COLORS,"Keyword color",ID_HIGHLIGHT_COLOR,174,310,92,130);color_swatch(hwnd,state,PAGE_COLORS,ID_HIGHLIGHT_SWATCH,410,313);
    inline_field(hwnd,state,PAGE_COLORS,"Private color",ID_PRIVATE_COLOR,460,310,92,130);color_swatch(hwnd,state,PAGE_COLORS,ID_PRIVATE_SWATCH,696,313);
    info(hwnd,state,PAGE_COLORS,"Filters support OR, AND and regular expressions.",174,340,580,20);
}

"###,"chat color swatches");

    replace_once(&mut source,
        "    button(hwnd,state,PAGE_SPEECH,ID_TEST_TTS,\"Test TTS\",458,270,112,30);",
        "    button(hwnd,state,PAGE_SPEECH,ID_TEST_TTS,\"Test TTS\",642,226,106,30);",
        "align TTS test action");

    replace_once(&mut source,
        "    PostMessageW(state.main_hwnd, WM_COMMAND, CMD_SETTINGS_APPLIED as usize, 0);\n    load_all(hwnd, state);",
        "    PostMessageW(state.main_hwnd, WM_COMMAND, CMD_SETTINGS_APPLIED as usize, 0);\n    load_all(hwnd,state);set_dirty(hwnd,state,false);",
        "clear dirty after apply");

    replace_once(&mut source,
        "        ID_CLOSE => { DestroyWindow(hwnd); }",
        "        ID_CLOSE => close_settings(hwnd,state),",
        "close button dirty prompt");

    replace_once(&mut source,
        "unsafe fn inline_field(hwnd:HWND,state:&mut SettingsState,page:usize,text:&str,id:i32,x:i32,y:i32,label_w:i32,edit_w:i32){label(hwnd,state,page,text,x,y+4,label_w,20);edit(hwnd,state,page,id,x+label_w+6,y,edit_w,26,false);}",
        r###"unsafe fn inline_field(hwnd:HWND,state:&mut SettingsState,page:usize,text:&str,id:i32,x:i32,y:i32,label_w:i32,edit_w:i32){label(hwnd,state,page,text,x,y+4,label_w,20);edit(hwnd,state,page,id,x+label_w+6,y,edit_w,26,false);}
unsafe fn color_swatch(hwnd:HWND,state:&mut SettingsState,page:usize,id:i32,x:i32,y:i32){let c=create_button(hwnd,id,"",x,y,20,20);state.page_controls.push((c,page));}"###,
        "color swatch control helper");

    replace_once(&mut source,
        "unsafe fn combo(hwnd: HWND, state: &mut SettingsState, page: usize, id: i32, x: i32, y: i32, w: i32, h: i32) {\n    let c = create_control(hwnd, \"COMBOBOX\", \"\", id, x, y, w, h, WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_VSCROLL | CBS_DROPDOWNLIST | 0x0010 | 0x0200, 0); state.page_controls.push((c, page));\n}",
        "unsafe fn combo(hwnd: HWND, state: &mut SettingsState, page: usize, id: i32, x: i32, y: i32, w: i32, h: i32) {\n    let c=create_control(hwnd,\"COMBOBOX\",\"\",id,x,y,w,h,WS_CHILD|WS_VISIBLE|WS_TABSTOP|WS_VSCROLL|CBS_DROPDOWNLIST|0x0010|0x0200,0);crate::ui_theme::theme_combo(c);state.page_controls.push((c,page));\n}",
        "dark main settings combos");

    replace_once(&mut source,
        "unsafe fn draw_settings_button(state: &SettingsState, item: *const windows_sys::Win32::UI::Controls::DRAWITEMSTRUCT) -> LRESULT {\n    if item.is_null(){return 0;} let id=(*item).CtlID as i32;\n    let selected=id>=NAV_BASE&&id<NAV_BASE+PAGE_COUNT as i32&&(id-NAV_BASE)as usize==state.current_page;\n    let primary=id==ID_APPLY; let danger=matches!(id,ID_TAB_DELETE|ID_CLEAR_BLOCKED); let nav=id>=NAV_BASE&&id<NAV_BASE+PAGE_COUNT as i32;\n    crate::ui_theme::draw_button(item,selected,primary,danger,nav)\n}",
        r###"unsafe fn draw_settings_button(state:&SettingsState,item:*const windows_sys::Win32::UI::Controls::DRAWITEMSTRUCT)->LRESULT{
    if item.is_null(){return 0;}let id=(*item).CtlID as i32;
    if swatch_edit_id(id).is_some(){return draw_color_swatch(item,id);}
    let selected=id>=NAV_BASE&&id<NAV_BASE+PAGE_COUNT as i32&&(id-NAV_BASE)as usize==state.current_page;
    let primary=id==ID_APPLY;let danger=matches!(id,ID_TAB_DELETE|ID_CLEAR_BLOCKED);let nav=id>=NAV_BASE&&id<NAV_BASE+PAGE_COUNT as i32;
    crate::ui_theme::draw_button(item,selected,primary,danger,nav)
}
unsafe fn draw_color_swatch(item:*const windows_sys::Win32::UI::Controls::DRAWITEMSTRUCT,id:i32)->LRESULT{
    let parent=windows_sys::Win32::UI::WindowsAndMessaging::GetParent((*item).hwndItem);let edit=swatch_edit_id(id).unwrap_or(ID_HIGHLIGHT_COLOR);let color=hex_colorref(&get_text(parent,edit)).unwrap_or(crate::ui_theme::SURFACE_HOVER);let r=(*item).rcItem;let br=CreateSolidBrush(color);FillRect((*item).hDC,&r,br);DeleteObject(br);let border=CreateSolidBrush(crate::ui_theme::BORDER_STRONG);let top=RECT{left:r.left,top:r.top,right:r.right,bottom:r.top+1};let bottom=RECT{left:r.left,top:r.bottom-1,right:r.right,bottom:r.bottom};let left=RECT{left:r.left,top:r.top,right:r.left+1,bottom:r.bottom};let right=RECT{left:r.right-1,top:r.top,right:r.right,bottom:r.bottom};for edge in [&top,&bottom,&left,&right]{FillRect((*item).hDC,edge,border);}DeleteObject(border);1
}
fn swatch_edit_id(id:i32)->Option<i32>{if id>=ID_COLOR_SWATCH_BASE&&id<ID_COLOR_SWATCH_BASE+10{Some(ID_COLOR_BASE+(id-ID_COLOR_SWATCH_BASE))}else if id==ID_HIGHLIGHT_SWATCH{Some(ID_HIGHLIGHT_COLOR)}else if id==ID_PRIVATE_SWATCH{Some(ID_PRIVATE_COLOR)}else{None}}
fn edit_swatch_id(id:i32)->Option<i32>{if id>=ID_COLOR_BASE&&id<ID_COLOR_BASE+10{Some(ID_COLOR_SWATCH_BASE+(id-ID_COLOR_BASE))}else if id==ID_HIGHLIGHT_COLOR{Some(ID_HIGHLIGHT_SWATCH)}else if id==ID_PRIVATE_COLOR{Some(ID_PRIVATE_SWATCH)}else{None}}
fn hex_colorref(text:&str)->Option<u32>{if !valid_hex_color(text){return None;}let v=u32::from_str_radix(&text.trim()[1..],16).ok()?;Some(rgb(((v>>16)&255)as u8,((v>>8)&255)as u8,(v&255)as u8))}
fn colorref_hex(c:u32)->String{format!("#{:02X}{:02X}{:02X}",c&255,(c>>8)&255,(c>>16)&255)}
unsafe fn choose_color(hwnd:HWND,edit:i32)->bool{let mut custom=[0u32;16];let mut data=ChooseColorWData{l_struct_size:std::mem::size_of::<ChooseColorWData>()as u32,hwnd_owner:hwnd,h_instance:null_mut(),rgb_result:hex_colorref(&get_text(hwnd,edit)).unwrap_or(0),custom_colors:custom.as_mut_ptr(),flags:0x00000001|0x00000002,cust_data:0,hook:null_mut(),template:null()};if ChooseColorW(&mut data)==0{return false;}set_text(hwnd,edit,&colorref_hex(data.rgb_result));if let Some(swatch)=edit_swatch_id(edit){windows_sys::Win32::Graphics::Gdi::InvalidateRect(GetDlgItem(hwnd,swatch),null(),0);}true}
fn is_mutating_command(id:i32,notify:u16)->bool{if matches!(id,ID_APPLY|ID_CLOSE|ID_UPDATE_CHECK_NOW|ID_TEST_TTS|3207|3208|3209|ID_OPEN_LOGS|ID_OPEN_JSON|ID_OPEN_FOLDER){return false;}if id>=NAV_BASE&&id<NAV_BASE+PAGE_COUNT as i32{return false;}let settings_range=(3200..=3422).contains(&id);settings_range||matches!(id,ID_TAB_ADD|ID_TAB_DELETE|ID_UNBLOCK|ID_CLEAR_BLOCKED)||notify!=0&&id>0}
unsafe fn set_dirty(hwnd:HWND,state:&mut SettingsState,dirty:bool){state.dirty=dirty;EnableWindow(GetDlgItem(hwnd,ID_APPLY),dirty as i32);windows_sys::Win32::Graphics::Gdi::InvalidateRect(GetDlgItem(hwnd,ID_APPLY),null(),0);}
unsafe fn close_settings(hwnd:HWND,state:&mut SettingsState){if state.dirty{let result=MessageBoxW(hwnd,wide("Discard unsaved settings changes?").as_ptr(),wide("ReadyAlert Settings").as_ptr(),0x00000004|0x00000030);if result!=6{return;}}DestroyWindow(hwnd);}
"###,
        "dirty state and color picker helpers");

    fs::write(path,source).expect("write v1.21 settings UI");
}
