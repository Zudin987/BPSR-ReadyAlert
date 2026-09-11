use std::{env,fs,path::{Path,PathBuf}};

mod prior { include!("build_v1189j.rs"); pub fn run(){main();} }

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){let count=source.matches(from).count();assert_eq!(count,1,"build_v1190 patch {label} expected one match, found {count}");*source=source.replacen(from,to,1);}
fn insert_before_once(source:&mut String,anchor:&str,insertion:&str,label:&str){let count=source.matches(anchor).count();assert_eq!(count,1,"build_v1190 patch {label} expected one anchor, found {count}");let at=source.find(anchor).expect("build_v1190 insertion anchor");source.insert_str(at,insertion);}

fn patch_feature_overlay(out:&Path){let path=out.join("feature_overlays_v170_fixed.rs");let mut source=fs::read_to_string(&path).expect("read generated overlays").replace("\r\n","\n");
replace_once(&mut source,r###"Foundation::{GetLastError, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM},"###,r###"Foundation::{GetLastError, HINSTANCE, HWND, LPARAM, LRESULT, RECT, SIZE, WPARAM},"###,"SIZE import");
replace_once(&mut source,r###"InvalidateRect, SelectObject, SetArcDirection, SetBkMode, SetTextColor, StretchDIBits, AD_CLOCKWISE, BITMAPINFO, DEFAULT_GUI_FONT, DIB_RGB_COLORS, HDC, HFONT, NULL_BRUSH, PAINTSTRUCT, PS_SOLID, SRCCOPY,"###,r###"GetTextExtentPoint32W, InvalidateRect, SelectObject, SetArcDirection, SetBkMode, SetTextColor, StretchDIBits, AD_CLOCKWISE, BITMAPINFO, DEFAULT_GUI_FONT, DIB_RGB_COLORS, HDC, HFONT, NULL_BRUSH, PAINTSTRUCT, PS_SOLID, SRCCOPY,"###,"text measurement import");
replace_once(&mut source,r###"const OVERLAY_SCALE_MIN:i32=30;"###,r###"const OVERLAY_SCALE_MIN:i32=50;"###,"minimum scale");
replace_once(&mut source,r###"unsafe fn toolbar_symbol_font(pixel_height:i32)->HFONT{
    static NORMAL:OnceLock<usize>=OnceLock::new();
    static HIDE:OnceLock<usize>=OnceLock::new();
    let slot=if pixel_height==HIDE_ICON_PX{&HIDE}else{&NORMAL};
    *slot.get_or_init(||{let face=wide("Segoe UI Symbol");windows_sys::Win32::Graphics::Gdi::CreateFontW(-pixel_height,0,0,0,400,0,0,0,1,0,0,5,0,face.as_ptr())as usize})as HFONT
}
"###,r###"unsafe fn toolbar_symbol_font(pixel_height:i32)->HFONT{
    static FONTS:OnceLock<std::sync::Mutex<std::collections::HashMap<i32,usize>>>=OnceLock::new();
    let fonts=FONTS.get_or_init(||std::sync::Mutex::new(std::collections::HashMap::new()));
    let mut fonts=match fonts.lock(){Ok(value)=>value,Err(poisoned)=>poisoned.into_inner()};
    if let Some(font)=fonts.get(&pixel_height){return *font as HFONT;}
    let face=wide("Segoe UI Symbol");let font=windows_sys::Win32::Graphics::Gdi::CreateFontW(-pixel_height.max(1),0,0,0,400,0,0,0,1,0,0,5,0,face.as_ptr());
    if !font.is_null(){fonts.insert(pixel_height,font as usize);}font
}
"###,"dynamic toolbar font cache");
insert_before_once(&mut source,r###"fn dps_identity(row:&DpsRow)->String"###,r###"#[derive(Clone,Copy,Debug,Eq,PartialEq)]
enum DpsLayoutTier{Comfortable,Compact,Dense,Minimum}
fn dps_readability_percent(scale:i32)->i32{let s=clamp_overlay_scale(scale);if s<100{(s+100)/2}else{s}}
fn dps_adaptive_logical_px(value:i32,scale:i32)->i32{let s=clamp_overlay_scale(scale).max(1)as i64;let r=dps_readability_percent(scale)as i64;(((value.max(1)as i64)*r+s/2)/s).clamp(1,i32::MAX as i64)as i32}
fn dps_effective_width(logical:i32,scale:i32)->i32{let s=clamp_overlay_scale(scale).max(1)as i64;let r=dps_readability_percent(scale).max(1)as i64;(((logical.max(1)as i64)*s+r/2)/r).clamp(1,i32::MAX as i64)as i32}
fn dps_layout_scale(state:&State)->i32{if state.image_render_all{100}else{state.scale_percent}}
fn dps_layout_tier(width:i32,scale:i32)->DpsLayoutTier{let effective=dps_effective_width(width,scale);if effective>=600{DpsLayoutTier::Comfortable}else if effective>=480{DpsLayoutTier::Compact}else if effective>=340{DpsLayoutTier::Dense}else{DpsLayoutTier::Minimum}}
fn dps_row_h(scale:i32)->i32{dps_adaptive_logical_px(DPS_ROW_H,scale)}
fn dps_badge_w(scale:i32)->i32{dps_adaptive_logical_px(BADGE_W,scale)}
fn dps_badge_h(scale:i32)->i32{dps_adaptive_logical_px(23,scale)}
fn dps_badge_gap(scale:i32)->i32{dps_adaptive_logical_px(BADGE_GAP,scale)}
fn dps_build_badge_gap(scale:i32)->i32{dps_adaptive_logical_px(BUILD_BADGE_GAP,scale)}
fn dps_consumable_icon(scale:i32)->i32{dps_adaptive_logical_px(DPS_CONSUMABLE_ICON,scale)}
fn dps_consumable_gap(scale:i32)->i32{dps_adaptive_logical_px(DPS_CONSUMABLE_GAP,scale)}
fn dps_mode_share_enabled(settings:&FeatureSettings,mode:SortMode)->bool{match mode{SortMode::Damage=>settings.meter.show_damage_share,SortMode::Heal=>settings.meter.show_healing_share,SortMode::Tank=>settings.meter.show_tank_share}}
unsafe fn dps_cached_font(scale:i32,bold:bool)->HFONT{
    static FONTS:OnceLock<std::sync::Mutex<std::collections::HashMap<(i32,bool),usize>>>=OnceLock::new();
    let key=(clamp_overlay_scale(scale),bold);let fonts=FONTS.get_or_init(||std::sync::Mutex::new(std::collections::HashMap::new()));let mut fonts=match fonts.lock(){Ok(value)=>value,Err(poisoned)=>poisoned.into_inner()};if let Some(font)=fonts.get(&key){return *font as HFONT;}
    let base=if bold{13}else{12};let height=dps_adaptive_logical_px(base,scale);let face=wide("Segoe UI Variable Text");let font=CreateFontW(-height,0,0,0,if bold{600}else{400},0,0,0,1,0,0,5,0,face.as_ptr());if !font.is_null(){fonts.insert(key,font as usize);}font
}
unsafe fn dps_primary_font(state:&State)->HFONT{let scale=dps_layout_scale(state);if scale>=100{if state.font.is_null(){GetStockObject(DEFAULT_GUI_FONT)}else{state.font}}else{let font=dps_cached_font(scale,true);if font.is_null(){if state.font.is_null(){GetStockObject(DEFAULT_GUI_FONT)}else{state.font}}else{font}}}
unsafe fn dps_secondary_font(state:&State)->HFONT{let scale=dps_layout_scale(state);if scale>=100{GetStockObject(DEFAULT_GUI_FONT)}else{let font=dps_cached_font(scale,false);if font.is_null(){GetStockObject(DEFAULT_GUI_FONT)}else{font}}}
unsafe fn dps_text_width(hdc:HDC,text:&str,fallback:i32)->i32{if text.is_empty(){return 0;}let value=wide(text);let count=value.len().saturating_sub(1).min(i32::MAX as usize)as i32;let mut size:SIZE=std::mem::zeroed();if count>0&&GetTextExtentPoint32W(hdc,value.as_ptr(),count,&mut size)!=0{size.cx.max(0)}else{fallback.max(0)}}
unsafe fn dps_secondary_compact_measured(hdc:HDC,row:&DpsRow,max_px:i32)->String{if max_px<=0{return String::new();}let spec=dps_spec(row);let full=dps_secondary(row);if dps_text_width(hdc,&full,identity_text_px(&full,false))<=max_px{return full;}let primary=if row.ability_score>0{score(row.ability_score)}else{String::new()};let medium=if spec.is_empty(){primary.clone()}else if primary.is_empty(){spec.clone()}else{format!("{} · {}",spec,primary)};if dps_text_width(hdc,&medium,identity_text_px(&medium,false))<=max_px{return medium;}if !spec.is_empty()&&dps_text_width(hdc,&spec,identity_text_px(&spec,false))<=max_px{return spec;}if spec.is_empty()&&dps_text_width(hdc,&primary,identity_text_px(&primary,false))<=max_px{return primary;}String::new()}
fn dps_take_column(cursor:&mut i32,width:i32,gap:i32)->(i32,i32){if width<=0{return(*cursor,*cursor);}let right=*cursor;let left=right.saturating_sub(width);*cursor=left.saturating_sub(gap);(left,right)}
unsafe fn dps_row_layout_responsive(hdc:HDC,r:RECT,scale:i32,show_imagines:bool,show_active:bool,show_share:bool,show_deaths:bool,row:&DpsRow,name_font:HFONT,secondary_font:HFONT)->DpsRowLayout{
    let tier=dps_layout_tier((r.right-r.left).max(1),scale);let(gap,total_w,active_w,share_w,death_w)=match tier{DpsLayoutTier::Comfortable=>(3,80,74,50,if r.right-r.left>=760{24}else{20}),DpsLayoutTier::Compact=>(2,70,62,46,20),DpsLayoutTier::Dense=>(2,60,54,46,18),DpsLayoutTier::Minimum=>(1,56,50,44,0)};
    let mut cursor=r.right-4;let(death_left,death_right)=dps_take_column(&mut cursor,if show_deaths{death_w}else{0},gap);let(share_left,share_right)=dps_take_column(&mut cursor,if show_share{share_w}else{0},gap);let(active_left,active_right)=dps_take_column(&mut cursor,if show_active{active_w}else{0},gap);let(total_left,total_right)=dps_take_column(&mut cursor,total_w,gap);
    let name_left=r.left+25;let badge_w=dps_badge_w(scale);let badge_gap=dps_badge_gap(scale);let build_gap=dps_build_badge_gap(scale);let min_identity=match tier{DpsLayoutTier::Comfortable=>90,DpsLayoutTier::Compact=>76,DpsLayoutTier::Dense=>62,DpsLayoutTier::Minimum=>48};let mut badge_count=if show_imagines{row.imagines.len().min(2)}else{0};
    while badge_count>0{let span=badge_count as i32*badge_w+(badge_count.saturating_sub(1)as i32)*badge_gap;if cursor-span-build_gap>=name_left+min_identity{break;}badge_count-=1;}
    let badge_span=badge_count as i32*badge_w+(badge_count.saturating_sub(1)as i32)*badge_gap;let badge_left=if badge_count>0{cursor-badge_span}else{cursor};let identity_right=if badge_count>0{badge_left-build_gap}else{cursor};
    let old=SelectObject(hdc,name_font);let measured_name=dps_text_width(hdc,&row.name,identity_text_px(&row.name,true));SelectObject(hdc,old);let name_gap=match tier{DpsLayoutTier::Comfortable=>8,DpsLayoutTier::Compact=>6,DpsLayoutTier::Dense=>4,DpsLayoutTier::Minimum=>3};let name_right=(name_left+measured_name+2).min(identity_right).max(name_left);let spec_left=(name_right+name_gap).min(identity_right);let spec_right=identity_right;
    let _=secondary_font;DpsRowLayout{name_left,name_right,spec_left,spec_right,badge_left,badge_count,total_left,total_right,active_left,active_right,share_left,share_right,death_left,death_right}
}
fn dps_header_labels(tier:DpsLayoutTier)->(&'static str,&'static str,&'static str){match tier{DpsLayoutTier::Comfortable=>("TOTAL","ACTIVE/s","%"),DpsLayoutTier::Compact=>("TOTAL","ACTIVE","%"),DpsLayoutTier::Dense=>("TOT","ACT/s","%"),DpsLayoutTier::Minimum=>("T","A/s","%")}}
fn dps_tab_damage_label(tier:DpsLayoutTier)->&'static str{if matches!(tier,DpsLayoutTier::Dense|DpsLayoutTier::Minimum){"Dmg"}else{"Damage"}}
fn dps_toolbar_button_w(right:i32,scale:i32)->i32{let tier=dps_layout_tier(right.max(1),scale);match tier{DpsLayoutTier::Comfortable=>BUTTON_W,DpsLayoutTier::Compact=>44,DpsLayoutTier::Dense=>36,DpsLayoutTier::Minimum=>30}}
fn toolbar_action_rects_responsive(right:i32,scale:i32)->[(RECT,&'static str);5]{let tier=dps_layout_tier(right.max(1),scale);let button=dps_toolbar_button_w(right,scale);let(arrow,live,image,reset,gap,label_image,label_reset)=match tier{DpsLayoutTier::Comfortable=>(30,46,116,52,12,"Copy as Image","Reset"),DpsLayoutTier::Compact=>(24,40,82,44,8,"Copy Image","Reset"),DpsLayoutTier::Dense=>(20,34,54,36,4,"Copy","Reset"),DpsLayoutTier::Minimum=>(18,30,38,28,2,"Img","R")};let mut x=right-button*3;let mut take=|w:i32,label:&'static str|{let r=RECT{left:x-w,top:5,right:x,bottom:29};x-=w;(r,label)};let reset_r=take(reset,label_reset);let image_r=take(image,label_image);let _=take(gap,"");let newer=take(arrow,">");let live_r=take(live,"LIVE");let older=take(arrow,"<");[older,live_r,newer,image_r,reset_r]}
"###,"responsive DPS helpers");
fs::write(path,source).expect("write patched overlays");}

fn main(){prior::run();let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));patch_feature_overlay(&out);println!("cargo:rerun-if-changed=build/legacy/build_v1190.rs");}
