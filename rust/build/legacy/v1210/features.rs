fn patch_features(out:&Path){
    let path=out.join("feature_overlays_v170_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read v1.20 feature overlays").replace("\r\n","\n");

    replace_once(&mut source,
        "unsafe fn feature_combo(hwnd:HWND,id:i32,x:i32,y:i32,items:&[&str]) {\n    let c=feature_control(hwnd,\"COMBOBOX\",id,\"\",x,y,155,180,0x00210213);\n    for text in items {SendMessageW(c,0x0143,0,wide(text).as_ptr() as isize);}\n}",
        "unsafe fn feature_combo(hwnd:HWND,id:i32,x:i32,y:i32,items:&[&str]) {\n    let c=feature_control(hwnd,\"COMBOBOX\",id,\"\",x,y,155,180,0x00210213);crate::ui_theme::theme_combo(c);\n    for text in items {SendMessageW(c,0x0143,0,wide(text).as_ptr() as isize);}\n}",
        "dark feature combo");
    replace_once(&mut source,
        "unsafe fn feature_combo_sized(hwnd:HWND,id:i32,x:i32,y:i32,w:i32,items:&[&str]) {\n    let c=feature_control(hwnd,\"COMBOBOX\",id,\"\",x,y,w,180,0x00210213);\n    for text in items {SendMessageW(c,0x0143,0,wide(text).as_ptr() as isize);}\n}",
        "unsafe fn feature_combo_sized(hwnd:HWND,id:i32,x:i32,y:i32,w:i32,items:&[&str]) {\n    let c=feature_control(hwnd,\"COMBOBOX\",id,\"\",x,y,w,180,0x00210213);crate::ui_theme::theme_combo(c);\n    for text in items {SendMessageW(c,0x0143,0,wide(text).as_ptr() as isize);}\n}",
        "dark sized feature combo");

    replace_between(&mut source,"unsafe fn build_feature_form(hwnd:HWND,state:&mut SettingsState) {","unsafe fn refresh_feature_form(hwnd:HWND,state:&SettingsState){",r###"unsafe fn build_feature_form(hwnd:HWND,state:&mut SettingsState){
    InitCommonControls();
    let title=if state.kind==Kind::Dps{"DPS Meter"}else{"Dungeon Mechanics"};let title_hwnd=feature_label(hwnd,0,title,18,14,320,24);crate::ui_theme::set_font(title_hwnd,crate::ui_theme::FontRole::Heading);
    feature_label(hwnd,7000,"",18,50,84,22);feature_button(hwnd,7001,"−",108,44,30);feature_button(hwnd,7002,"+",142,44,30);
    feature_label(hwnd,7004,"",190,50,90,22);feature_button(hwnd,7005,"−",286,44,30);feature_button(hwnd,7006,"+",320,44,30);feature_button(hwnd,7008,"Reset 100%",354,44,88);
    feature_label(hwnd,0,"Collapse",458,50,60,22);feature_combo_sized(hwnd,7003,520,44,84,&["Right","Bottom","Left","Top"]);
    feature_scale_slider(hwnd,7007,190,74,252);SendMessageW(fc(hwnd,7007),0x0407,1,overlay_scale_min(state.kind)as isize);feature_label(hwnd,7009,"",458,74,146,22);
    if state.kind==Kind::Dps{
        let section_hwnd=feature_label(hwnd,0,"DISPLAY",18,100,180,18);crate::ui_theme::set_font(section_hwnd,crate::ui_theme::FontRole::Secondary);
        let display=["Damage share %","Healing share %","Tank share %","Death count","Battle Imagine badges","Target / HP summary","Active rate"];
        for(i,label)in display.iter().enumerate(){let col=(i/4)as i32;let row=(i%4)as i32;feature_check(hwnd,7100+i as i32,label,18+col*286,120+row*27,270);}
        feature_label(hwnd,0,"ROSTER & HISTORY",18,236,220,18);
        let roster=["Only contributors","Party only","Always show self","Remember scroll between encounters"];
        for(i,label)in roster.iter().enumerate(){let col=(i/2)as i32;let row=(i%2)as i32;feature_check(hwnd,7107+i as i32,label,18+col*286,258+row*27,270);}
        feature_label(hwnd,0,"Visible rows",18,322,90,22);feature_combo_sized(hwnd,7010,108,316,96,&["Auto","5","10","20","30","50"]);
        feature_label(hwnd,7011,"",286,322,150,22);feature_button(hwnd,7012,"−",438,316,36);feature_button(hwnd,7013,"+",480,316,36);
        feature_label(hwnd,0,"Changes save immediately.",18,362,220,20);feature_button(hwnd,2,"Close",494,356,92);
    }else{
        feature_label(hwnd,7020,"",18,100,260,22);feature_button(hwnd,7021,"Event Tracker",534,94,150);
        let section=feature_label(hwnd,0,"COMBAT ATTRIBUTES",18,134,300,18);crate::ui_theme::set_font(section,crate::ui_theme::FontRole::Secondary);
        let rows=(ATTRIBUTE_CATALOG.len()+2)/3;
        for(i,(_,label))in ATTRIBUTE_CATALOG.iter().enumerate(){feature_check(hwnd,7200+i as i32,label,18+(i/rows)as i32*224,156+(i%rows)as i32*26,216);}
        let bottom=156+rows as i32*26;
        feature_label(hwnd,0,"Choose up to 8 stats. Food, Serum and Event Tracker rows appear automatically.",18,bottom+10,520,34);
        feature_button(hwnd,2,"Close",598,bottom+12,92);
    }
    refresh_feature_form(hwnd,state);
}
"###,"combat settings spacing");

    replace_once(&mut source,
        "windows_sys::Win32::UI::WindowsAndMessaging::SetWindowTextW(fc(hwnd,7020),wide(&format!(\"Track attributes ({}/{})\",f.mechanic_attributes.tracked.len(),feature_settings::MAX_TRACKED_ATTRIBUTES)).as_ptr());",
        "windows_sys::Win32::UI::WindowsAndMessaging::SetWindowTextW(fc(hwnd,7020),wide(&format!(\"Tracked attributes  {} / {}\",f.mechanic_attributes.tracked.len(),feature_settings::MAX_TRACKED_ATTRIBUTES)).as_ptr());",
        "mechanics counter wording");

    replace_once(&mut source,
        "unsafe fn detail_card(hdc:HDC,r:RECT,title:&str,lines:&[String]){fill(hdc,&r,rgb(22,29,35));fill(hdc,&RECT{left:r.left,top:r.top,right:r.left+3,bottom:r.bottom},rgb(66,211,190));SetTextColor(hdc,rgb(112,205,190));draw(hdc,title,RECT{left:r.left+10,top:r.top+3,right:r.right-7,bottom:r.top+20},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);SetTextColor(hdc,rgb(228,235,239));for(i,text)in lines.iter().take(4).enumerate(){draw(hdc,text,RECT{left:r.left+10,top:r.top+20+i as i32*14,right:r.right-7,bottom:r.top+35+i as i32*14},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}}",
        "unsafe fn detail_card(hdc:HDC,r:RECT,title:&str,lines:&[String]){fill(hdc,&r,crate::ui_theme::SURFACE);fill(hdc,&RECT{left:r.left,top:r.top,right:r.left+3,bottom:r.bottom},crate::ui_theme::ACCENT);SetTextColor(hdc,crate::ui_theme::ACCENT_HOVER);draw(hdc,title,RECT{left:r.left+10,top:r.top+3,right:r.right-7,bottom:r.top+20},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);SetTextColor(hdc,crate::ui_theme::TEXT_SECONDARY);for(i,text)in lines.iter().take(4).enumerate(){draw(hdc,text,RECT{left:r.left+10,top:r.top+20+i as i32*14,right:r.right-7,bottom:r.top+35+i as i32*14},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);}}",
        "entity card hierarchy");

    replace_once(&mut source,
        "let(amount_head,aux_head,pct_head,rate_head)=match state.mode{DetailMode::Damage=>(\"Damage\",\"Boss\",\"Boss %\",\"DPS\"),DetailMode::Heal=>(\"Healing\",\"Effective\",\"OH %\",\"HPS\"),_=>(\"Value\",\"Aux\",\"%\",\"/s\")};",
        "let(amount_head,aux_head,pct_head,rate_head)=match state.mode{DetailMode::Damage=>(\"Damage\",\"Boss Dmg\",\"Boss %\",\"DPS\"),DetailMode::Heal=>(\"Healing\",\"Effective\",\"OH %\",\"HPS\"),_=>(\"Value\",\"Aux\",\"%\",\"/s\")};",
        "entity boss damage header");

    let old="fill(hdc,&RECT{left:3,top:head,right:rc.right-3,bottom:head+24},rgb(49,49,49));SetTextColor(hdc,rgb(225,225,225));";
    let count=source.matches(old).count();assert_eq!(count,3,"v1.21 entity table header count");
    source=source.replace(old,"fill(hdc,&RECT{left:3,top:head,right:rc.right-3,bottom:head+24},crate::ui_theme::RAISED);SetTextColor(hdc,crate::ui_theme::TEXT_SECONDARY);");

    replace_once(&mut source,
        "    ShowWindow(hwnd,SW_SHOW);\n    SetForegroundWindow(hwnd);",
        "    crate::ui_theme::dark_titlebar(hwnd);\n    ShowWindow(hwnd,SW_SHOW);\n    SetForegroundWindow(hwnd);",
        "dark entity window chrome");

    fs::write(path,source).expect("write v1.21 feature overlays");
}
