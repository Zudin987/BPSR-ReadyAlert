use std::{env, fs, path::{Path, PathBuf}};

mod prior {
    include!("build_v1180.rs");
    pub fn run() { main(); }
}

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){
    let count=source.matches(from).count();
    assert_eq!(count,1,"v1.18.1 patch {label} expected one match, found {count}");
    *source=source.replacen(from,to,1);
}

fn patch_feature_overlay(out:&Path){
    let path=out.join("feature_overlays_v170_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read v1.18.0 generated overlays").replace("\r\n","\n");

    replace_once(
        &mut source,
        "pub unsafe fn create_mechanics(instance:HINSTANCE,main_hwnd:HWND,features:Arc<RwLock<FeatureSettings>>,paths:AppPaths)->Result<HWND,String>{create(instance,main_hwnd,features,paths,Kind::Mechanics)}\nunsafe fn create(instance:HINSTANCE,main_hwnd:HWND,features:Arc<RwLock<FeatureSettings>>,paths:AppPaths,kind:Kind)->Result<HWND,String>{",
        "pub unsafe fn create_mechanics(instance:HINSTANCE,main_hwnd:HWND,features:Arc<RwLock<FeatureSettings>>,paths:AppPaths)->Result<HWND,String>{create(instance,main_hwnd,features,paths,Kind::Mechanics)}\nfn restored_overlay_extent(saved:i32,minimum:i32,work:i32)->i32{let work=work.max(1);saved.max(minimum.min(work)).min(work).max(1)}\nunsafe fn create(instance:HINSTANCE,main_hwnd:HWND,features:Arc<RwLock<FeatureSettings>>,paths:AppPaths,kind:Kind)->Result<HWND,String>{",
        "restored overlay extent helper",
    );

    replace_once(
        &mut source,
        "let mut bounds:RECT=std::mem::zeroed();GetWindowRect(hwnd,&mut bounds);let work=crate::ui::work_area(hwnd);let w=(bounds.right-bounds.left).max(if kind==Kind::Dps{620}else{340}).min(work.right-work.left);SetWindowPos(hwnd,null_mut(),bounds.left,bounds.top,w,(bounds.bottom-bounds.top).min(work.bottom-work.top),SWP_NOACTIVATE|windows_sys::Win32::UI::WindowsAndMessaging::SWP_NOZORDER);",
        "let mut bounds:RECT=std::mem::zeroed();GetWindowRect(hwnd,&mut bounds);let work=crate::ui::work_area(hwnd);let work_w=(work.right-work.left).max(1);let work_h=(work.bottom-work.top).max(1);let w=restored_overlay_extent(bounds.right-bounds.left,if kind==Kind::Dps{620}else{340},work_w);let h=restored_overlay_extent(bounds.bottom-bounds.top,260,work_h);SetWindowPos(hwnd,null_mut(),bounds.left,bounds.top,w,h,SWP_NOACTIVATE|windows_sys::Win32::UI::WindowsAndMessaging::SWP_NOZORDER);",
        "enforce current minimum height on restored overlays",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1181_restored_overlay_tests {
    use super::*;

    #[test]
    fn legacy_short_height_is_recovered_to_current_minimum() {
        assert_eq!(restored_overlay_extent(180, 260, 900), 260);
        assert_eq!(restored_overlay_extent(420, 260, 900), 420);
    }

    #[test]
    fn small_work_area_still_wins_over_minimum() {
        assert_eq!(restored_overlay_extent(180, 260, 220), 220);
        assert_eq!(restored_overlay_extent(0, 620, 500), 500);
    }
}
"#);

    fs::write(path,source).expect("write v1.18.1 feature overlay");
}

fn patch_chat_overlay(out:&Path){
    let input=out.join("overlay_v150_v1180.rs");
    let mut source=fs::read_to_string(&input).expect("read v1.18.0 chat renderer").replace("\r\n","\n");

    replace_once(
        &mut source,
        "unsafe fn measure_row(hdc:HDC,settings:&AppSettings,item:&OverlayItem,width:i32)->i32{let message_width=(width-16).max(50);let one=(line_height(settings)-3).max(16);let message_h=measure_text(hdc,item.message.text.trim(),message_width).max(one).min(one*2);let translation_h=item.translation.as_ref().map(|_|one+4).unwrap_or(0);if settings.chat.compact_mode{message_h+translation_h+14}else{line_height(settings)+message_h+translation_h+14}}",
        r#"fn compact_message_width(content_width:i32,prefix_width:i32)->i32{content_width.saturating_sub(prefix_width).max(20)}
unsafe fn inline_measure_width(hdc:HDC,text:&str)->i32{if text.is_empty(){return 0;}let w=wide(text);let mut r=RECT{left:0,top:0,right:4096,bottom:0};DrawTextW(hdc,w.as_ptr(),-1,&mut r,DT_CALCRECT|DT_SINGLELINE|DT_NOPREFIX);(r.right-r.left).max(0)}
unsafe fn measure_row(hdc:HDC,settings:&AppSettings,item:&OverlayItem,width:i32)->i32{
    let content_width=(width-16).max(20);
    let message_width=if settings.chat.compact_mode{
        let sender=if item.message.sender_name.trim().is_empty(){"?"}else{item.message.sender_name.trim()};
        let sender_label=if item.message.sender_level>0{format!("{} Lv{}",sender,item.message.sender_level)}else{sender.to_string()};
        let mut prefix=inline_measure_width(hdc,&format!("{} · ",channel_name(item.message.channel)));
        if settings.chat.show_time{prefix=prefix.saturating_add(inline_measure_width(hdc,&format!("{}  ",time_text(&item.message,settings.chat.show_time_as_ago))));}
        prefix=prefix.saturating_add(inline_measure_width(hdc,&format!("{}  ",sender_label)));
        compact_message_width(content_width,prefix)
    }else{content_width.max(50)};
    let one=(line_height(settings)-3).max(16);
    let message_h=measure_text(hdc,item.message.text.trim(),message_width).max(one).min(one*2);
    let translation_h=item.translation.as_ref().map(|_|one+4).unwrap_or(0);
    if settings.chat.compact_mode{message_h+translation_h+14}else{line_height(settings)+message_h+translation_h+14}
}"#,
        "measure compact chat using actual remaining message width",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1181_chat_layout_tests {
    use super::*;

    #[test]
    fn compact_message_width_accounts_for_prefix() {
        assert_eq!(compact_message_width(380, 140), 240);
        assert_eq!(compact_message_width(380, 370), 20);
        assert_eq!(compact_message_width(40, 200), 20);
    }
}
"#);

    fs::write(out.join("overlay_v150_v1181.rs"),source).expect("write v1.18.1 chat renderer");
}

fn main(){
    prior::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_feature_overlay(&out);
    patch_chat_overlay(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1181.rs");
}
