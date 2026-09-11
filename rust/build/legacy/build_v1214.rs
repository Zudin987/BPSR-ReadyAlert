use std::{env,fs,path::{Path,PathBuf}};

mod prior {
    include!("build_v1213.rs");
    pub fn run(){main();}
}

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){
    let count=source.matches(from).count();
    assert_eq!(count,1,"v1.21.4 patch {label} expected one match, found {count}");
    *source=source.replacen(from,to,1);
}

fn patch_feature_overlay(out:&Path){
    let path=out.join("feature_overlays_v170_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read v1.21.3 generated overlays").replace("\r\n","\n");

    replace_once(
        &mut source,
        "const DPS_CONSUMABLE_LABEL_NUDGE_Y:i32=-1;",
        "const DPS_CONSUMABLE_LABEL_NUDGE_Y:i32=-1;\nconst MECH_CONSUMABLE_TIMER_ID:usize=0x1214;",
        "mechanics consumable repaint timer constant",
    );

    replace_once(
        &mut source,
        "SetWindowPos(hwnd,HWND_TOPMOST,0,0,0,0,SWP_NOACTIVATE|SWP_NOMOVE|SWP_NOSIZE);if kind==Kind::Dps{let popup=create_consumable_popup(instance,hwnd);with_state(hwnd,|s|{s.consumable_hwnd=popup;sync_consumable_popup(hwnd,s);});}Ok(hwnd)",
        "SetWindowPos(hwnd,HWND_TOPMOST,0,0,0,0,SWP_NOACTIVATE|SWP_NOMOVE|SWP_NOSIZE);if kind==Kind::Dps{let popup=create_consumable_popup(instance,hwnd);with_state(hwnd,|s|{s.consumable_hwnd=popup;sync_consumable_popup(hwnd,s);});}else{SetTimer(hwnd,MECH_CONSUMABLE_TIMER_ID,200,None);}Ok(hwnd)",
        "mechanics consumable repaint timer start",
    );

    replace_once(
        &mut source,
        "WM_PAINT=>{if !ptr.is_null(){paint(hwnd,&mut*ptr);}0},",
        "WM_TIMER=>{if !ptr.is_null()&&(*ptr).kind==Kind::Mechanics&&wparam==MECH_CONSUMABLE_TIMER_ID{InvalidateRect(hwnd,null(),0);}0},\nWM_PAINT=>{if !ptr.is_null(){paint(hwnd,&mut*ptr);}0},",
        "mechanics consumable repaint timer handler",
    );

    replace_once(
        &mut source,
        "WM_NCDESTROY=>{SetWindowLongPtrW(hwnd,GWLP_USERDATA,0);if !ptr.is_null(){let state=&mut*ptr;for child in[state.detail_hwnd,state.settings_hwnd,state.consumable_hwnd]{if !child.is_null()&&IsWindow(child)!=0{DestroyWindow(child);}}if !state.font.is_null(){DeleteObject(state.font);}drop(Box::from_raw(ptr));}DefWindowProcW(hwnd,msg,wparam,lparam)},",
        "WM_NCDESTROY=>{KillTimer(hwnd,MECH_CONSUMABLE_TIMER_ID);SetWindowLongPtrW(hwnd,GWLP_USERDATA,0);if !ptr.is_null(){let state=&mut*ptr;for child in[state.detail_hwnd,state.settings_hwnd,state.consumable_hwnd]{if !child.is_null()&&IsWindow(child)!=0{DestroyWindow(child);}}if !state.font.is_null(){DeleteObject(state.font);}drop(Box::from_raw(ptr));}DefWindowProcW(hwnd,msg,wparam,lparam)},",
        "mechanics consumable repaint timer cleanup",
    );

    replace_once(
        &mut source,
        "fn paint_consumable_status(hdc:HDC,label:&str,status:Option<&ConsumableStatus>,r:RECT){unsafe{let now=now_ms();let active=status.filter(|s|s.expires_unix_ms<=0||s.expires_unix_ms>now);let name=active.map(|s|s.name.as_str()).unwrap_or(\"None\");SetTextColor(hdc,if active.is_some(){crate::ui_theme::TEXT_SECONDARY}else{crate::ui_theme::MUTED});draw(hdc,&format!(\"{label}: {name}\"),RECT{left:r.left,top:r.top,right:r.right-94,bottom:r.bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);if let Some(status)=active{if status.expires_unix_ms>0{let left=status.expires_unix_ms.saturating_sub(now).max(0);SetTextColor(hdc,timer_urgency_color(left));draw(hdc,&format_countdown(left as u64),RECT{left:r.right-90,top:r.top,right:r.right,bottom:r.bottom},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}}}}",
        "fn mechanics_consumable_row_color(status:Option<&ConsumableStatus>,now:i64)->u32{let Some(status)=status else{return crate::ui_theme::MUTED;};if status.expires_unix_ms<=0{return crate::ui_theme::TEXT_SECONDARY;}let left=status.expires_unix_ms.saturating_sub(now).max(0);if left>0&&left<=30_000{if consumable_blink_on(now){crate::ui_theme::CRITICAL}else{rgb(108,34,34)}}else{timer_urgency_color(left)}}\nfn paint_consumable_status(hdc:HDC,label:&str,status:Option<&ConsumableStatus>,r:RECT){unsafe{let now=now_ms();let active=status.filter(|s|s.expires_unix_ms<=0||s.expires_unix_ms>now);let name=active.map(|s|s.name.as_str()).unwrap_or(\"None\");let row_color=mechanics_consumable_row_color(active,now);SetTextColor(hdc,row_color);draw(hdc,&format!(\"{label}: {name}\"),RECT{left:r.left,top:r.top,right:r.right-94,bottom:r.bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);if let Some(status)=active{if status.expires_unix_ms>0{let left=status.expires_unix_ms.saturating_sub(now).max(0);SetTextColor(hdc,row_color);draw(hdc,&format_countdown(left as u64),RECT{left:r.right-90,top:r.top,right:r.right,bottom:r.bottom},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}}}}",
        "mechanics Food/Serum full-row urgency color",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1214_mechanics_consumable_alert_tests{
    use super::*;

    fn status(left:i64,now:i64)->ConsumableStatus{
        ConsumableStatus{buff_id:1,name:"Consumable".into(),expires_unix_ms:now+left,duration_ms:300_000}
    }

    #[test]
    fn mechanics_consumable_row_uses_existing_timer_urgency_colors(){
        let now=100_000;
        let warning=status(271_000,now);
        assert_eq!(mechanics_consumable_row_color(Some(&warning),now),timer_urgency_color(271_000));
        let critical=status(45_000,now);
        assert_eq!(mechanics_consumable_row_color(Some(&critical),now),crate::ui_theme::CRITICAL);
    }

    #[test]
    fn mechanics_consumable_entire_row_blinks_during_final_thirty_seconds(){
        let urgent=status(30_000,100_000);
        let on=mechanics_consumable_row_color(Some(&urgent),100_000);
        let off=mechanics_consumable_row_color(Some(&urgent),100_400);
        assert_eq!(on,crate::ui_theme::CRITICAL);
        assert_eq!(off,rgb(108,34,34));
        assert_ne!(on,off);
    }

    #[test]
    fn mechanics_consumable_blink_starts_at_thirty_seconds_not_before(){
        let now=100_000;
        let not_yet=status(30_001,now);
        assert_eq!(mechanics_consumable_row_color(Some(&not_yet),now),crate::ui_theme::CRITICAL);
        assert_eq!(mechanics_consumable_row_color(Some(&not_yet),now+400),crate::ui_theme::CRITICAL);
    }

    #[test]
    fn missing_and_timeless_consumables_keep_nonurgent_colors(){
        let now=100_000;
        assert_eq!(mechanics_consumable_row_color(None,now),crate::ui_theme::MUTED);
        let timeless=ConsumableStatus{buff_id:1,name:"Persistent".into(),expires_unix_ms:0,duration_ms:0};
        assert_eq!(mechanics_consumable_row_color(Some(&timeless),now),crate::ui_theme::TEXT_SECONDARY);
    }
}
"#);

    fs::write(path,source).expect("write v1.21.4 mechanics consumable alert patch");
}

fn main(){
    prior::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_feature_overlay(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1214.rs");
}
