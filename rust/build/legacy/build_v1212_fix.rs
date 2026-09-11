use std::{env,fs,path::{Path,PathBuf}};

mod prior {
    include!("build_v1212.rs");
    pub fn run(){main();}
}

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){
    let count=source.matches(from).count();
    assert_eq!(count,1,"v1.21.2 fix {label} expected one match, found {count}");
    *source=source.replacen(from,to,1);
}

fn patch_feature_overlay(out:&Path){
    let path=out.join("feature_overlays_v170_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read v1.21.2 generated overlays").replace("\r\n","\n");

    replace_once(
        &mut source,
        "const DPS_CONSUMABLE_ORANGE:u32=rgb(238,139,47);",
        "const DPS_CONSUMABLE_ORANGE:u32=0x002F8BEE;",
        "const orange COLORREF",
    );

    replace_once(
        &mut source,
        "SetForegroundWindow, SetLayeredWindowAttributes, SetWindowLongPtrW, SetWindowPos, ShowWindow,",
        "SetForegroundWindow, SetLayeredWindowAttributes, SetWindowLongPtrW, SetWindowPos, ShowWindow, SetTimer, KillTimer,",
        "timer imports",
    );
    replace_once(
        &mut source,
        "WM_LBUTTONDOWN, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_NCCALCSIZE, WM_NCHITTEST, WM_NCCREATE, WM_NCDESTROY, WM_PAINT, WM_SIZE,",
        "WM_LBUTTONDOWN, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_NCCALCSIZE, WM_NCHITTEST, WM_NCCREATE, WM_NCDESTROY, WM_PAINT, WM_SIZE, WM_TIMER,",
        "WM_TIMER import",
    );
    replace_once(
        &mut source,
        "const DPS_CONSUMABLE_ORANGE:u32=0x002F8BEE;",
        "const DPS_CONSUMABLE_ORANGE:u32=0x002F8BEE;\nconst DPS_CONSUMABLE_TIMER_ID:usize=0x1212;",
        "consumable timer constant",
    );
    replace_once(
        &mut source,
        "SetLayeredWindowAttributes(hwnd,DPS_CONSUMABLE_KEY,0,1);ShowWindow(hwnd,SW_SHOW);hwnd",
        "SetLayeredWindowAttributes(hwnd,DPS_CONSUMABLE_KEY,0,1);SetTimer(hwnd,DPS_CONSUMABLE_TIMER_ID,200,None);ShowWindow(hwnd,SW_SHOW);hwnd",
        "popup repaint timer start",
    );
    replace_once(
        &mut source,
        "WM_PAINT=>{if !ptr.is_null(){paint_consumable_popup(hwnd,&*ptr);}0},",
        "WM_TIMER=>{if _wparam==DPS_CONSUMABLE_TIMER_ID{InvalidateRect(hwnd,null(),0);}0},\n        WM_PAINT=>{if !ptr.is_null(){paint_consumable_popup(hwnd,&*ptr);}0},",
        "popup timer repaint",
    );
    replace_once(
        &mut source,
        "WM_NCDESTROY=>{SetWindowLongPtrW(hwnd,GWLP_USERDATA,0);if !ptr.is_null(){drop(Box::from_raw(ptr));}DefWindowProcW(hwnd,msg,_wparam,lparam)},",
        "WM_NCDESTROY=>{KillTimer(hwnd,DPS_CONSUMABLE_TIMER_ID);SetWindowLongPtrW(hwnd,GWLP_USERDATA,0);if !ptr.is_null(){drop(Box::from_raw(ptr));}DefWindowProcW(hwnd,msg,_wparam,lparam)},",
        "popup timer cleanup",
    );

    // v1.21.1 lowered the 100% DPS minimum width from 500 to 300 pixels.
    // The old v1.18.8 screen-limit fixture used a 1280px work area because the
    // former 300% minimum was 1500px. At the new 900px minimum that fixture is
    // intentionally no longer screen-limited, so keep the test's original
    // purpose by using a genuinely narrower 800px work area.
    replace_once(
        &mut source,
        "let small=RECT{left:0,top:0,right:1280,bottom:680};",
        "let small=RECT{left:0,top:0,right:800,bottom:680};",
        "v1.18.8 screen-limit fixture after 300px minimum",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1212_timer_fix_tests{
    use super::*;

    #[test]
    fn orange_colorref_matches_requested_rgb(){
        assert_eq!(DPS_CONSUMABLE_ORANGE,0x002F8BEE);
    }

    #[test]
    fn urgent_blink_has_two_visual_phases(){
        let status=ConsumableStatus{buff_id:1,name:"Food".into(),expires_unix_ms:30_000,duration_ms:300_000};
        assert_ne!(consumable_bar_color(&status,0),consumable_bar_color(&status,400));
    }
}
"#);

    fs::write(path,source).expect("write v1.21.2 consumable timer fix");
}

fn main(){
    prior::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_feature_overlay(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1212_fix.rs");
}
