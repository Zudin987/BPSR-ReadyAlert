use super::*;

pub fn patch(out:&Path){
    let path=out.join("feature_overlays_v170_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read generated v1.16.6 overlay");
    replace_once(&mut source,"const SETTINGS_CLASS: &str = \"BPSRReadyAlertFeatureSettingsV180\";","const SETTINGS_CLASS: &str = \"BPSRReadyAlertFeatureSettingsV180\";\nconst CONSUMABLE_CLASS:&str=\"BPSRReadyAlertConsumablePopupV1166\";","add consumable popup class");
    replace_once(&mut source,"const DPS_CONSUMABLE_ICON:i32=17;\nconst DPS_CONSUMABLE_GAP:i32=2;\nconst DPS_CONSUMABLE_GUTTER:i32=40;","const DPS_CONSUMABLE_ICON:i32=21;\nconst DPS_CONSUMABLE_GAP:i32=3;\nconst DPS_CONSUMABLE_POPUP_W:i32=49;\nconst DPS_CONSUMABLE_POPUP_GAP:i32=3;\nconst DPS_CONSUMABLE_KEY:u32=0x00030201;","larger circular floating indicators");
    replace_once(&mut source,"settings_hwnd: HWND,\nfont: HFONT,","settings_hwnd: HWND,\nconsumable_hwnd: HWND,\nfont: HFONT,","store popup handle");
    replace_once(&mut source,"settings_hwnd:null_mut(),font:create_overlay_font","settings_hwnd:null_mut(),consumable_hwnd:null_mut(),font:create_overlay_font","initialize popup handle");
    // Restore the meter itself to full width; indicators now live in another HWND.
    replace_once(&mut source,"let hr=RECT{left:6+DPS_CONSUMABLE_GUTTER,top:head_top,right:rc.right-8,bottom:head_top+19};","let hr=RECT{left:6,top:head_top,right:rc.right-8,bottom:head_top+19};","restore header geometry");
    replace_once(&mut source,"let r=RECT{left:6+DPS_CONSUMABLE_GUTTER,top:y,right:rc.right-8,bottom:y+DPS_ROW_H-2};paint_player_consumables(hdc,row,y,now_ms());let bg=if row.is_dead","let r=RECT{left:6,top:y,right:rc.right-8,bottom:y+DPS_ROW_H-2};let bg=if row.is_dead","remove in-meter indicators");
    replace_once(&mut source,"let r=RECT{left:6+DPS_CONSUMABLE_GUTTER,top:top+screen_i as i32*DPS_ROW_H,right:rc.right-8,bottom:top+screen_i as i32*DPS_ROW_H+DPS_ROW_H-2};let layout=dps_row_layout(r,true,row);","let r=RECT{left:6,top:top+screen_i as i32*DPS_ROW_H,right:rc.right-8,bottom:top+screen_i as i32*DPS_ROW_H+DPS_ROW_H-2};let layout=dps_row_layout(r,true,row);","restore Imagine hover geometry");
    replace_once(&mut source,"let next=overlay_help(hwnd,state,x,y).or_else(||hover_consumable_at(hwnd,state,x,y)).or_else(||hover_badge_at(hwnd,state,x,y));","let next=overlay_help(hwnd,state,x,y).or_else(||hover_badge_at(hwnd,state,x,y));","move consumable hover to popup");
    replace_once(&mut source,"for child in[state.detail_hwnd,state.settings_hwnd]{","for child in[state.detail_hwnd,state.settings_hwnd,state.consumable_hwnd]{","destroy popup with meter");
    replace_once(&mut source,"WM_SIZE=>{if !ptr.is_null(){clamp_scroll(hwnd,&mut*ptr);}InvalidateRect(hwnd,null(),0);0}","0x0003=>{if !ptr.is_null(){sync_consumable_popup(hwnd,&mut*ptr);}0},WM_SIZE=>{if !ptr.is_null(){clamp_scroll(hwnd,&mut*ptr);sync_consumable_popup(hwnd,&mut*ptr);}InvalidateRect(hwnd,null(),0);0}","sync popup on move resize");
    replace_once(&mut source,"state.collapsed=true;SetWindowPos(hwnd,HWND_TOPMOST,x,y,new_width,new_height,SWP_NOACTIVATE);InvalidateRect(hwnd,null(),0);}","state.collapsed=true;SetWindowPos(hwnd,HWND_TOPMOST,x,y,new_width,new_height,SWP_NOACTIVATE);sync_consumable_popup(hwnd,state);InvalidateRect(hwnd,null(),0);}","hide popup on collapse");
    replace_once(&mut source,"state.collapsed=false;SetWindowPos(hwnd,HWND_TOPMOST,rect.left,rect.top,(rect.right-rect.left).max(100),(rect.bottom-rect.top).max(80),SWP_NOACTIVATE);InvalidateRect(hwnd,null(),0);}","state.collapsed=false;SetWindowPos(hwnd,HWND_TOPMOST,rect.left,rect.top,(rect.right-rect.left).max(100),(rect.bottom-rect.top).max(80),SWP_NOACTIVATE);sync_consumable_popup(hwnd,state);InvalidateRect(hwnd,null(),0);}","restore popup on expand");
    // Existing 8 Hz overlay paint keeps the circular countdown moving even idle.
    replace_once(&mut source,"EndPaint(hwnd,&ps);}","EndPaint(hwnd,&ps);if state.kind==Kind::Dps&&!state.consumable_hwnd.is_null()&&IsWindow(state.consumable_hwnd)!=0{InvalidateRect(state.consumable_hwnd,null(),0);}}","refresh popup countdown");
    let shared=include_str!("build_v1166_ring_helpers.txt");
    replace_between(&mut source,"fn active_consumable(status:Option<&ConsumableStatus>","unsafe fn hover_badge_at",shared,"replace old in-client consumable helpers");
    let popup=include_str!("build_v1166_popup.txt");
    replace_once(&mut source,"pub unsafe fn update_dps(hwnd:HWND,snapshot:DpsSnapshot)",&format!("{popup}\npub unsafe fn update_dps(hwnd:HWND,snapshot:DpsSnapshot)"),"insert floating popup implementation");
    replace_once(&mut source,"apply_opacity(hwnd,layout.opacity);SetWindowPos(hwnd,HWND_TOPMOST,0,0,0,0,SWP_NOACTIVATE|SWP_NOMOVE|SWP_NOSIZE);Ok(hwnd)}","apply_opacity(hwnd,layout.opacity);SetWindowPos(hwnd,HWND_TOPMOST,0,0,0,0,SWP_NOACTIVATE|SWP_NOMOVE|SWP_NOSIZE);if kind==Kind::Dps{let popup=create_consumable_popup(instance,hwnd);with_state(hwnd,|s|{s.consumable_hwnd=popup;sync_consumable_popup(hwnd,s);});}Ok(hwnd)}","create floating popup");
    source.push_str(r#"

#[cfg(test)]
mod v1166_floating_consumable_tests{
    use super::*;
    #[test]fn floating_badges_no_longer_consume_meter_width(){assert_eq!(DPS_CONSUMABLE_POPUP_W,49);assert_eq!(DPS_CONSUMABLE_ICON,21);}
    #[test]fn expired_popup_status_vanishes(){let status=ConsumableStatus{buff_id:1,name:"Serum".into(),expires_unix_ms:100,duration_ms:1_000};assert!(active_consumable(Some(&status),101).is_none());}
}
"#);
    fs::write(path,source).expect("write v1.16.6 overlay");
}
