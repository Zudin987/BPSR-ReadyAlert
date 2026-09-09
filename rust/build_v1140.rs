use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1130.rs");
    pub fn run() { main(); }
}

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){let count=source.matches(from).count();assert_eq!(count,1,"v1.14.0 patch `{label}` expected one match, found {count}");*source=source.replacen(from,to,1);}
fn replace_between(source:&mut String,start:&str,end:&str,replacement:&str,label:&str){let count=source.matches(start).count();assert_eq!(count,1,"v1.14.0 patch `{label}` start expected one match, found {count}");let begin=source.find(start).expect("v1.14 start checked");let rel_end=source[begin..].find(end).unwrap_or_else(||panic!("v1.14.0 patch `{label}` end anchor missing"));source.replace_range(begin..begin+rel_end,replacement);}

fn main(){
    previous::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));

    let overlay_path=out.join("feature_overlays_v170_fixed.rs");
    let mut overlay=fs::read_to_string(&overlay_path).expect("read generated v1.13 overlay");

    // Make the compact toolbar setting affordance understandable without relying
    // on a tooltip or remembering what a one-letter S means.
    replace_once(
        &mut overlay,
        "draw(hdc,\"S\",RECT{left:rc.right-BUTTON_W*3,top:0,right:rc.right-BUTTON_W*2,bottom:TOOLBAR_H},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);",
        "draw(hdc,\"Set\",RECT{left:rc.right-BUTTON_W*3,top:0,right:rc.right-BUTTON_W*2,bottom:TOOLBAR_H},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);",
        "clear settings toolbar label",
    );

    replace_between(
        &mut overlay,
        "unsafe fn paint_mechanics(hdc:HDC,rc:RECT,state:&State){",
        "fn mechanic_timer(",
        include_str!("overlay_v1140_mechanics_patch.txt"),
        "custom tracker mechanics renderer",
    );

    // v1.8.4 made attributes two physical rows when >3, and v1.8.3 can add a
    // fixed tracked-buff section. Account for both when clamping the scroll list,
    // then include custom tracker rows in the same scrollable virtual list.
    replace_between(
        &mut overlay,
        "unsafe fn clamp_scroll(hwnd:HWND,state:&mut State){",
        "unsafe fn open_detail",
        r#"fn mechanics_scroll_metrics(state:&State,bottom:i32)->(usize,usize,i32){let tracked=state.features.read().map(|f|f.mechanic_attributes.tracked.len()).unwrap_or(0);let attr_rows=if tracked==0{0}else if tracked<=3{1}else{2};let mut top=TOOLBAR_H+5+attr_rows as i32*MECH_ATTR_H+MECH_CONSUMABLE_H;let now=now_ms();let special=state.mechanics.rows.iter().filter(|row|row.key.starts_with("trackedbuff:")&&(row.persistent||row.expires_unix_ms<=0||row.expires_unix_ms>now)).count();if special>0{top+=18+special as i32*29;}let mechanics=state.mechanics.rows.iter().filter(|row|!row.key.starts_with("trackedbuff:")&&(row.persistent||row.expires_unix_ms<=0||row.expires_unix_ms>now)).count();let total=crate::event_tracker::rows().len().saturating_add(mechanics);let visible=((bottom-top)/MECH_ROW_H).max(0)as usize;(total,visible,top)}
unsafe fn clamp_scroll(hwnd:HWND,state:&mut State){let mut rc:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rc);let(total,visible)=if state.kind==Kind::Dps{(meter_rows(state).len(),visible_dps_rows(rc.bottom))}else{let(total,visible,_)=mechanics_scroll_metrics(state,rc.bottom);(total,visible)};state.scroll=state.scroll.min(total.saturating_sub(visible.max(1)));}
"#,
        "mechanics scroll geometry",
    );

    replace_between(
        &mut overlay,
        "unsafe fn paint_feature_settings(hwnd:HWND,state:&SettingsState){",
        "fn mutate_features",
        include_str!("overlay_v1140_settings_patch.txt"),
        "whole combat settings UX",
    );

    // Clamp the two custom-drawn combat settings windows to the current screen
    // and keep their three-column mechanics page usable on 720p displays.
    replace_between(
        &mut overlay,
        "unsafe fn open_feature_settings(parent:HWND,state:&mut State){",
        "unsafe extern \"system\" fn settings_wnd_proc",
        r#"unsafe fn open_feature_settings(parent:HWND,state:&mut State){if !state.settings_hwnd.is_null()&&IsWindow(state.settings_hwnd)!=0{SetForegroundWindow(state.settings_hwnd);return;}let instance=GetModuleHandleW(null());let class=wide(SETTINGS_CLASS);let wc=WNDCLASSW{lpfnWndProc:Some(settings_wnd_proc),hInstance:instance,hCursor:LoadCursorW(null_mut(),IDC_ARROW),lpszClassName:class.as_ptr(),..std::mem::zeroed()};if RegisterClassW(&wc)==0&&GetLastError()!=1410{return;}let mut pr:RECT=std::mem::zeroed();GetWindowRect(parent,&mut pr);let(request_w,request_h)=if state.kind==Kind::Dps{(500,610)}else{(720,540)};let screen_w=windows_sys::Win32::UI::WindowsAndMessaging::GetSystemMetrics(0).max(800);let screen_h=windows_sys::Win32::UI::WindowsAndMessaging::GetSystemMetrics(1).max(600);let width=request_w.min(screen_w-20);let height=request_h.min(screen_h-40);let x=(pr.left+35).clamp(10,(screen_w-width-10).max(10));let y=(pr.top+45).clamp(10,(screen_h-height-30).max(10));let ptr=Box::into_raw(Box::new(SettingsState{kind:state.kind,parent,paths:state.paths.clone(),features:state.features.clone()}));let title=wide("ReadyAlert Overlay Settings");let hwnd=CreateWindowExW(WS_EX_TOOLWINDOW|WS_EX_TOPMOST,class.as_ptr(),title.as_ptr(),WS_POPUP|WS_THICKFRAME,x,y,width,height,parent,null_mut(),instance,ptr.cast::<c_void>());if hwnd.is_null(){drop(Box::from_raw(ptr));return;}state.settings_hwnd=hwnd;ShowWindow(hwnd,SW_SHOW);SetForegroundWindow(hwnd);}
"#,
        "screen-safe combat settings",
    );

    fs::write(&overlay_path,overlay).expect("write v1.14 generated overlay");

    let win_path=out.join("win_v182_fixed.rs");
    let mut win=fs::read_to_string(&win_path).expect("read generated v1.13 win source");
    replace_once(
        &mut win,
        "TrayAction::OpenSettings=>",
        "TrayAction::OpenEventTracker=>{crate::event_tracker_ui::show(hwnd);},TrayAction::OpenSettings=>",
        "tray event tracker action",
    );
    fs::write(&win_path,win).expect("write v1.14 generated win source");

    println!("cargo:rerun-if-changed=build_v1140.rs");
    println!("cargo:rerun-if-changed=overlay_v1140_mechanics_patch.txt");
    println!("cargo:rerun-if-changed=overlay_v1140_settings_patch.txt");
    println!("cargo:rerun-if-changed=src/event_tracker.rs");
    println!("cargo:rerun-if-changed=src/event_tracker_ui.rs");
    println!("cargo:rerun-if-changed=src/telemetry_v1140.rs");
    println!("cargo:rerun-if-changed=src/settings_ui_v181.rs");
}
