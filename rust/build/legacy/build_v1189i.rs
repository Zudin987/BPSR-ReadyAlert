use std::{env,fs,path::{Path,PathBuf}};

mod prior { include!("build_v1189h.rs"); pub fn run(){main();} }

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){let count=source.matches(from).count();assert_eq!(count,1,"v1.18.9i patch {label} expected one match, found {count}");*source=source.replacen(from,to,1);}
fn insert_before_once(source:&mut String,anchor:&str,insertion:&str,label:&str){let count=source.matches(anchor).count();assert_eq!(count,1,"v1.18.9i patch {label} expected one anchor, found {count}");let at=source.find(anchor).expect("v1.18.9i insertion anchor");source.insert_str(at,insertion);}

fn patch_settings_validation(out:&Path){
    let path=out.join("settings_ui_v1160_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read v1.18.9h settings source").replace("\r\n","\n");

    replace_once(&mut source,
        r###"    if crate::hotkeys::parse(&hotkey).is_none() || crate::hotkeys::is_combat(&hotkey) {
        MessageBoxW(hwnd,wide("Use a valid recovery shortcut, such as Ctrl+Shift+F9. Ctrl+Shift+F10 is reserved for DPS + Mechanics.").as_ptr(),wide("Chat recovery shortcut").as_ptr(),MB_OK|MB_ICONERROR);
        return;
    }
"###,
        r###"    if crate::hotkeys::parse(&hotkey).is_none() || crate::hotkeys::is_combat(&hotkey) {
        MessageBoxW(hwnd,wide("Use a valid recovery shortcut, such as Ctrl+Shift+F9. Ctrl+Shift+F10 is reserved for DPS + Mechanics.").as_ptr(),wide("Chat recovery shortcut").as_ptr(),MB_OK|MB_ICONERROR);
        let c=GetDlgItem(hwnd,ID_CLICK_HOTKEY);if !c.is_null(){crate::ui::SetFocus(c);SendMessageW(c,0x00B1,0,-1isize);}
        return;
    }
"###,
        "focus invalid recovery hotkey");

    replace_once(&mut source,
        r###"    let Some(chat_sound_volume)=read_i32_range(hwnd,ID_CHAT_SOUND_VOLUME,"Chat sound volume",0,100)else{return;};
    let s = &mut state.working;
"###,
        r###"    let Some(chat_sound_volume)=read_i32_range(hwnd,ID_CHAT_SOUND_VOLUME,"Chat sound volume",0,100)else{return;};
    let color_channels=[1,2,3,4,5,6,7,8,9,99];
    let color_labels=["World color","Local color","Team color","Guild color","Private color","Group color","Notice color","Play color","Newbie color","System color"];
    let mut channel_colors=Vec::with_capacity(color_channels.len());
    for(i,label)in color_labels.iter().enumerate(){let Some(value)=read_hex_color(hwnd,ID_COLOR_BASE+i as i32,label)else{return;};channel_colors.push(value);}
    let Some(highlight_color)=read_hex_color(hwnd,ID_HIGHLIGHT_COLOR,"Keyword highlight color")else{return;};
    let Some(private_color)=read_hex_color(hwnd,ID_PRIVATE_COLOR,"Private highlight color")else{return;};
    let s = &mut state.working;
"###,
        "validate color fields before apply");

    replace_once(&mut source,
        r###"    let channels = [1,2,3,4,5,6,7,8,9,99];
    for (i, channel) in channels.iter().enumerate() { s.chat.channel_colors.insert(*channel, get_text(hwnd, ID_COLOR_BASE + i as i32)); }
    s.chat.highlight_if_matches = highlight_expr;
    s.chat.highlight_color = get_text(hwnd, ID_HIGHLIGHT_COLOR);
    s.chat.private_highlight_color = get_text(hwnd, ID_PRIVATE_COLOR);
"###,
        r###"    for (channel,color) in color_channels.into_iter().zip(channel_colors.into_iter()) { s.chat.channel_colors.insert(channel,color); }
    s.chat.highlight_if_matches = highlight_expr;
    s.chat.highlight_color = highlight_color;
    s.chat.private_highlight_color = private_color;
"###,
        "apply validated colors");

    insert_before_once(&mut source,
        "fn parse_i32_range(text:&str,min:i32,max:i32)->Option<i32>{",
        r###"fn valid_hex_color(text:&str)->bool{let value=text.trim().as_bytes();value.len()==7&&value[0]==b'#'&&value[1..].iter().all(u8::is_ascii_hexdigit)}
unsafe fn read_hex_color(hwnd:HWND,id:i32,label:&str)->Option<String>{let text=get_text(hwnd,id);if valid_hex_color(&text){return Some(text.trim().to_ascii_uppercase());}let message=format!("{label} must use #RRGGBB, for example #63C7FF.");MessageBoxW(hwnd,wide(&message).as_ptr(),wide("Invalid color").as_ptr(),MB_OK|MB_ICONERROR);let c=GetDlgItem(hwnd,id);if !c.is_null(){crate::ui::SetFocus(c);SendMessageW(c,0x00B1,0,-1isize);}None}
"###,
        "hex color validator");

    source.push_str(r###"

#[cfg(test)]
mod v1189_repeat_color_validation_tests{
    use super::*;
    #[test]fn chat_colors_require_exact_hex(){assert!(valid_hex_color("#63C7FF"));assert!(valid_hex_color("#abcdef"));for bad in ["63C7FF","#12345","#GG0000","#1234567",""]{assert!(!valid_hex_color(bad));}}
}
"###);
    fs::write(path,source).expect("write v1.18.9i settings source");
}

fn main(){prior::run();let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));patch_settings_validation(&out);println!("cargo:rerun-if-changed=build/legacy/build_v1189i.rs");}
