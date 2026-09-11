fn patch_chat(out:&Path){
    let path=out.join("overlay_v150_v1181.rs");
    let mut source=fs::read_to_string(&path).expect("read v1.20 chat overlay").replace("\r\n","\n");

    replace_once(&mut source,r###"pub unsafe fn refresh(hwnd: HWND) {
    if hwnd.is_null() || IsWindow(hwnd) == 0 { return; }
    if let Some(state) = state_mut(hwnd) {
        state.scroll_from_bottom = 0;
        let snapshot = state.settings.read().map(|s| s.clone()).unwrap_or_default();
        sync_hotkey(hwnd, state, &snapshot);
    }
    InvalidateRect(hwnd, null(), 0);
}"###,r###"pub unsafe fn refresh(hwnd: HWND) {
    if hwnd.is_null() || IsWindow(hwnd) == 0 { return; }
    if let Some(state) = state_mut(hwnd) {
        // Redraw/re-style only. Never destroy the user's reading position.
        let snapshot = state.settings.read().map(|s| s.clone()).unwrap_or_default();
        sync_hotkey(hwnd, state, &snapshot);
    }
    InvalidateRect(hwnd, null(), 0);
}"###,"refresh preserves scroll");

    replace_once(&mut source,r###"let len=snapshot.chat.tabs.len().min(MAX_MENU_TABS);if len>0 {let i=snapshot.chat.tabs.iter().position(|t|t.id==snapshot.chat.last_selected_tab_id).unwrap_or(0);let next=if crate::ui::GetKeyState(0x10)<0{(i+len-1)%len}else{(i+1)%len};PostMessageW(state.main_hwnd,WM_COMMAND,(CMD_TAB_BASE+next as u32) as usize,0);}return 0;"###,r###"let len=snapshot.chat.tabs.len().min(MAX_MENU_TABS);if len>0 {let i=snapshot.chat.tabs.iter().position(|t|t.id==snapshot.chat.last_selected_tab_id).unwrap_or(0);let next=if crate::ui::GetKeyState(0x10)<0{(i+len-1)%len}else{(i+1)%len};state.scroll_from_bottom=0;PostMessageW(state.main_hwnd,WM_COMMAND,(CMD_TAB_BASE+next as u32) as usize,0);}return 0;"###,"ctrl-tab follows selected tab");

    replace_once(&mut source,r###"if delta > 0 { state.scroll_from_bottom = (state.scroll_from_bottom + 3).min(max_scroll); }
                else if delta < 0 { state.scroll_from_bottom = state.scroll_from_bottom.saturating_sub(3); }"###,r###"if delta > 0 { state.scroll_from_bottom = (state.scroll_from_bottom + 1).min(max_scroll); }
                else if delta < 0 { state.scroll_from_bottom = state.scroll_from_bottom.saturating_sub(1); }"###,"one-message wheel step");

    replace_once(&mut source,r###"                    for (index, rect) in tab_rects(&snapshot, actions.add.left - 4).into_iter().enumerate() {
                        if index >= MAX_MENU_TABS { break; }
                        if hit(rect, x, y) {
                            PostMessageW(state.main_hwnd, WM_COMMAND, (CMD_TAB_BASE + index as u32) as usize, 0);
                            return 0;
                        }
                    }
                }
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)"###,r###"                    for (index, rect) in tab_rects(&snapshot, actions.add.left - 4).into_iter().enumerate() {
                        if index >= MAX_MENU_TABS { break; }
                        if hit(rect, x, y) {
                            state.scroll_from_bottom=0;
                            PostMessageW(state.main_hwnd, WM_COMMAND, (CMD_TAB_BASE + index as u32) as usize, 0);
                            return 0;
                        }
                    }
                    if let Some(rect)=tab_overflow_rect(&snapshot,actions.add.left-4){if hit(rect,x,y){show_tab_menu(hwnd,state);return 0;}}
                } else if state.scroll_from_bottom>0 {
                    let mut client:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut client);
                    if hit(new_message_rect(client.right),x,y){state.scroll_from_bottom=0;InvalidateRect(hwnd,null(),0);return 0;}
                }
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)"###,"tab and new-message click handling");

    replace_once(&mut source,r###"let rect = RECT { left: client.right.saturating_sub(170), top: TOOLBAR_HEIGHT + 5, right: client.right - 10, bottom: TOOLBAR_HEIGHT + 29 };"###,r###"let rect = new_message_rect(client.right);"###,"new-message rect helper");

    replace_between(&mut source,"unsafe fn draw_tabs(hdc:HDC,settings:&AppSettings,max_right:i32)","fn hit(rect: RECT, x: i32, y: i32) -> bool",r###"unsafe fn draw_tabs(hdc:HDC,settings:&AppSettings,max_right:i32){
    for(index,rect)in tab_rects(settings,max_right).into_iter().enumerate(){let Some(tab)=settings.chat.tabs.get(index)else{break;};let selected=tab.id==settings.chat.last_selected_tab_id;if selected{fill(hdc,&rect,crate::ui_theme::SURFACE_HOVER);fill(hdc,&RECT{left:rect.left+10,top:rect.bottom-3,right:rect.right-10,bottom:rect.bottom},crate::ui_theme::ACCENT);}let color=if selected{crate::ui_theme::TEXT}else{crate::ui_theme::TEXT_SECONDARY};draw_text(hdc,&tab.name,rect,color,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_END_ELLIPSIS|DT_NOPREFIX,false);}
    if let Some(rect)=tab_overflow_rect(settings,max_right){fill(hdc,&rect,crate::ui_theme::SURFACE);draw_text(hdc,"…",rect,crate::ui_theme::TEXT_SECONDARY,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX,false);}
}
fn tab_rects_with_limit(settings:&AppSettings,max_right:i32)->Vec<RECT>{
    let mut x=DRAG_WIDTH+4;let mut out=Vec::new();
    for tab in settings.chat.tabs.iter().take(MAX_MENU_TABS){let guessed=28+tab.name.chars().count()as i32*8;let w=guessed.clamp(68,150);if x+w>max_right&&!out.is_empty(){break;}if x>=max_right{break;}out.push(RECT{left:x,top:2,right:(x+w).min(max_right),bottom:TOOLBAR_HEIGHT});x+=w+3;}
    out
}
fn tab_rects(settings:&AppSettings,max_right:i32)->Vec<RECT>{
    let first=tab_rects_with_limit(settings,max_right);let total=settings.chat.tabs.len().min(MAX_MENU_TABS);
    if first.len()<total{tab_rects_with_limit(settings,(max_right-31).max(DRAG_WIDTH+36))}else{first}
}
fn tab_overflow_rect(settings:&AppSettings,max_right:i32)->Option<RECT>{
    let shown=tab_rects(settings,max_right).len();let total=settings.chat.tabs.len().min(MAX_MENU_TABS);(shown<total).then_some(RECT{left:(max_right-28).max(DRAG_WIDTH+4),top:2,right:max_right,bottom:TOOLBAR_HEIGHT})
}
fn new_message_rect(width:i32)->RECT{RECT{left:width.saturating_sub(170),top:TOOLBAR_HEIGHT+5,right:width-10,bottom:TOOLBAR_HEIGHT+29}}
"###,"tab overflow layout");

    replace_between(&mut source,"unsafe fn measure_row(hdc:HDC,settings:&AppSettings,item:&OverlayItem,width:i32)->i32{","unsafe fn draw_row(hdc:HDC,settings:&AppSettings,item:&OverlayItem,ordinal:usize,rect:RECT,bold_font:HFONT)",r###"unsafe fn measure_row(hdc:HDC,settings:&AppSettings,item:&OverlayItem,width:i32)->i32{
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
    let translation_h=item.translation.as_ref().map(|(source,text)|measure_text(hdc,&format!("{source} → EN: {text}"),content_width).max(one).min(one*2)+4).unwrap_or(0);
    if settings.chat.compact_mode{message_h+translation_h+14}else{line_height(settings)+message_h+translation_h+14}
}
"###,"two-line translation measurement");

    replace_between(&mut source,"unsafe fn draw_row(hdc:HDC,settings:&AppSettings,item:&OverlayItem,ordinal:usize,rect:RECT,bold_font:HFONT)","unsafe fn draw_message_text(hdc:HDC,settings:&AppSettings,bold_font:HFONT,text:&str,rect:RECT,color:u32,shadow:bool)",r###"unsafe fn draw_row(hdc:HDC,settings:&AppSettings,item:&OverlayItem,ordinal:usize,rect:RECT,bold_font:HFONT){
    let base=if settings.chat.show_zebra_stripes&&(ordinal&1)==1{(26,32,39)}else{(15,19,23)};let mut back=base;let hay=format!("{} {}",item.message.sender_name,item.message.text);
    if item.message.channel==5&&settings.chat.private_highlight_enabled{back=blend_tuple(parse_hex(&settings.chat.private_highlight_color).unwrap_or((86,53,93)),back,40);}else if !settings.chat.highlight_if_matches.trim().is_empty()&&chat::matches_expression(&hay,&settings.chat.highlight_if_matches){back=blend_tuple(parse_hex(&settings.chat.highlight_color).unwrap_or((107,90,58)),back,36);}
    let back_ref=rgb(back.0,back.1,back.2);fill(hdc,&rect,back_ref);let channel_rgb=settings.chat.channel_colors.get(&item.message.channel).and_then(|x|parse_hex(x)).unwrap_or((199,199,199));let channel=rgb(channel_rgb.0,channel_rgb.1,channel_rgb.2);
    if settings.chat.show_color_band{fill(hdc,&RECT{left:rect.left,top:rect.top+2,right:rect.left+3,bottom:rect.bottom-2},channel);}let channel_text=if settings.chat.show_color_band{blend_color(channel,back_ref,74)}else{channel};let left=rect.left+if settings.chat.show_color_band{10}else{7};let right=rect.right-9;let mut y=rect.top+5;
    let sender=if item.message.sender_name.trim().is_empty(){"?"}else{item.message.sender_name.trim()};let sender_label=if item.message.sender_level>0{format!("{} Lv{}",sender,item.message.sender_level)}else{sender.to_string()};let sender_color=sender_color(&item.message);let text_color=blend_color(crate::ui_theme::TEXT,back_ref,settings.chat.text_opacity);let meta_color=blend_color(crate::ui_theme::MUTED,back_ref,settings.chat.text_opacity);
    let shadow=settings.chat.text_shadow&&settings.chat.background_opacity<90;let lh=line_height(settings);let one=(lh-3).max(16);let translation_text=item.translation.as_ref().map(|(source,text)|format!("{source} → EN: {text}"));let translation_h=translation_text.as_ref().map(|text|measure_text(hdc,text,(right-left).max(20)).max(one).min(one*2)+4).unwrap_or(0);
    if settings.chat.compact_mode{let mut x=left;x=draw_inline(hdc,&format!("{} · ",channel_name(item.message.channel)),x,y,right,channel_text,shadow,lh);if settings.chat.show_time{x=draw_inline(hdc,&format!("{}  ",time_text(&item.message,settings.chat.show_time_as_ago)),x,y,right,meta_color,shadow,lh);}if !bold_font.is_null(){let old=SelectObject(hdc,bold_font);x=draw_inline(hdc,&format!("{}  ",sender_label),x,y,right,sender_color,shadow,lh);SelectObject(hdc,old);}else{x=draw_inline(hdc,&format!("{}  ",sender_label),x,y,right,sender_color,shadow,lh);}let message_rect=RECT{left:x.min(right-20),top:y,right,bottom:rect.bottom-5-translation_h};draw_message_text(hdc,settings,bold_font,item.message.text.trim(),message_rect,text_color,shadow);}else{let mut x=left;x=draw_inline(hdc,&format!("{}  ",channel_name(item.message.channel)),x,y,right,channel_text,shadow,lh);if !bold_font.is_null(){let old=SelectObject(hdc,bold_font);x=draw_inline(hdc,&sender_label,x,y,right,sender_color,shadow,lh);SelectObject(hdc,old);}else{x=draw_inline(hdc,&sender_label,x,y,right,sender_color,shadow,lh);}if settings.chat.show_time{let _=draw_inline(hdc,&format!("   {}",time_text(&item.message,settings.chat.show_time_as_ago)),x,y,right,meta_color,shadow,lh);}y+=lh;let message_rect=RECT{left,top:y,right,bottom:rect.bottom-5-translation_h};draw_message_text(hdc,settings,bold_font,item.message.text.trim(),message_rect,text_color,shadow);}
    if let Some(text)=translation_text{draw_text(hdc,&text,RECT{left,top:rect.bottom-translation_h+2,right,bottom:rect.bottom-4},blend_color(rgb(143,203,255),back_ref,settings.chat.text_opacity),DT_WORDBREAK|DT_END_ELLIPSIS|DT_NOPREFIX,shadow);}if settings.chat.show_separators{fill(hdc,&RECT{left:rect.left+9,top:rect.bottom-1,right:rect.right-2,bottom:rect.bottom},blend_color(crate::ui_theme::TEXT,back_ref,10));}
}
"###,"translation draw and adaptive shadow");

    replace_once(&mut source,"unsafe fn show_tab_menu(hwnd:HWND,state:&OverlayState){","unsafe fn show_tab_menu(hwnd:HWND,state:&mut OverlayState){","tab menu mutable state");
    replace_once(&mut source,"if command>0{PostMessageW(state.main_hwnd,WM_COMMAND,command as usize,0);}","if command>0{state.scroll_from_bottom=0;PostMessageW(state.main_hwnd,WM_COMMAND,command as usize,0);}","tab menu follows selected tab");

    fs::write(path,source).expect("write v1.21 chat overlay");
}
