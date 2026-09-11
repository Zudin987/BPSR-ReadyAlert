use std::{env,fs,path::{Path,PathBuf}};

mod prior { include!("build_v1189g.rs"); pub fn run(){main();} }

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){let count=source.matches(from).count();assert_eq!(count,1,"v1.18.9h patch {label} expected one match, found {count}");*source=source.replacen(from,to,1);}

fn patch_overlay_gdi_churn(out:&Path){
    let path=out.join("feature_overlays_v170_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read v1.18.9g feature overlay source").replace("\r\n","\n");
    replace_once(&mut source,
        "sync::{Arc, RwLock},",
        "sync::{Arc, OnceLock, RwLock},",
        "OnceLock import");
    replace_once(&mut source,
        r###"unsafe fn draw_toolbar_symbol(hdc:HDC,text:&str,rect:RECT,pixel_height:i32){
    let face=wide("Segoe UI Symbol");
    let font=windows_sys::Win32::Graphics::Gdi::CreateFontW(-pixel_height,0,0,0,400,0,0,0,1,0,0,5,0,face.as_ptr());
    if font.is_null(){draw(hdc,text,rect,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);return;}
    let old=SelectObject(hdc,font);
    draw(hdc,text,rect,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);
    SelectObject(hdc,old);
    DeleteObject(font);
}
"###,
        r###"unsafe fn toolbar_symbol_font(pixel_height:i32)->HFONT{
    static NORMAL:OnceLock<usize>=OnceLock::new();
    static HIDE:OnceLock<usize>=OnceLock::new();
    let slot=if pixel_height==HIDE_ICON_PX{&HIDE}else{&NORMAL};
    *slot.get_or_init(||{let face=wide("Segoe UI Symbol");windows_sys::Win32::Graphics::Gdi::CreateFontW(-pixel_height,0,0,0,400,0,0,0,1,0,0,5,0,face.as_ptr())as usize})as HFONT
}
unsafe fn draw_toolbar_symbol(hdc:HDC,text:&str,rect:RECT,pixel_height:i32){
    let font=toolbar_symbol_font(pixel_height);
    if font.is_null(){draw(hdc,text,rect,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);return;}
    let old=SelectObject(hdc,font);
    draw(hdc,text,rect,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);
    SelectObject(hdc,old);
}
"###,
        "cache toolbar symbol fonts");
    source.push_str(r###"

#[cfg(test)]
mod v1189_repeat_gdi_tests{
    use super::{COMBAT_ICON_PX,HIDE_ICON_PX};
    #[test]fn toolbar_uses_only_two_symbol_font_sizes(){assert_ne!(COMBAT_ICON_PX,HIDE_ICON_PX);}
}
"###);
    fs::write(path,source).expect("write v1.18.9h feature overlay source");
}

fn main(){
    prior::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_overlay_gdi_churn(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1189h.rs");
}
