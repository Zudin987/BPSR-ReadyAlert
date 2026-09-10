use std::{env, fs, path::{Path, PathBuf}};

mod prior {
    include!("build_v1183.rs");
    pub fn run() { main(); }
}

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){
    let count=source.matches(from).count();
    assert_eq!(count,1,"v1.18.4 patch {label} expected one match, found {count}");
    *source=source.replacen(from,to,1);
}

fn replace_between(source:&mut String,start:&str,end:&str,replacement:&str,label:&str){
    let count=source.matches(start).count();
    assert_eq!(count,1,"v1.18.4 patch {label} start expected one match, found {count}");
    let begin=source.find(start).expect("v1.18.4 start anchor");
    let rel_end=source[begin..].find(end).unwrap_or_else(||panic!("v1.18.4 patch {label} end anchor missing"));
    source.replace_range(begin..begin+rel_end,replacement);
}

fn decode_b64(input:&str)->Vec<u8>{
    let mut out=Vec::with_capacity(input.len()*3/4);let mut acc=0u32;let mut bits=0u8;
    for b in input.bytes(){let value=match b{b'A'..=b'Z'=>(b-b'A')as u32,b'a'..=b'z'=>(b-b'a'+26)as u32,b'0'..=b'9'=>(b-b'0'+52)as u32,b'+'=>62,b'/'=>63,b'='=>break,b' '|b'\t'|b'\r'|b'\n'=>continue,_=>panic!("invalid class icon base64")};acc=(acc<<6)|value;bits+=6;if bits>=8{bits-=8;out.push(((acc>>bits)&0xff)as u8);if bits==0{acc=0}else{acc&=(1u32<<bits)-1;}}}
    out
}

fn materialize_class_icons(out:&Path){
    let source=PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR")).join("../assets/source");let mut text=String::new();for index in 1..=6{let path=source.join(format!("ClassIcons.v1184.b64.{index:03}"));text.push_str(&fs::read_to_string(path).expect("read v1.18.4 class icon bundle"));text.push('\n');}
    let dir=out.join("class-icons");fs::create_dir_all(&dir).expect("create v1.18.4 class icon dir");let mut count=0usize;
    for line in text.lines().filter(|line|!line.trim().is_empty()){let(name,data)=line.split_once('\t').expect("class icon manifest name/data");let bytes=decode_b64(data);assert!(bytes.len()>54&&&bytes[0..2]==b"BM","invalid v1.18.4 class icon {name}");fs::write(dir.join(format!("{name}.bmp")),bytes).expect("write v1.18.4 class icon");count+=1;}
    assert_eq!(count,18,"v1.18.4 class icon bundle must contain all 18 specs");
}

fn patch_feature_overlay(out:&Path){
    let path=out.join("feature_overlays_v170_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read v1.18.3 generated overlays").replace("\r\n","\n");

    replace_once(&mut source,"sync::{Arc, RwLock},","sync::{Arc, OnceLock, RwLock},","class icon cache import");
    replace_once(&mut source,"const BADGE_W: i32 = 25;\nconst BADGE_GAP: i32 = 3;","const BADGE_W: i32 = 25;\nconst BADGE_GAP: i32 = 3;\nconst CLASS_ICON_W:i32=18;\nconst CLASS_ICON_GAP:i32=4;\nconst BUILD_BADGE_GAP:i32=5;","DPS identity spacing constants");

    replace_between(
        &mut source,
        "#[derive(Clone,Copy)]struct DpsRowLayout{",
        "fn dps_identity(row:&DpsRow)->String{",
        r###"#[derive(Clone,Copy)]struct DpsRowLayout{class_icon_left:i32,name_left:i32,name_right:i32,spec_left:i32,spec_right:i32,badge_left:i32,badge_count:usize,total_left:i32,total_right:i32,active_left:i32,active_right:i32,share_left:i32,share_right:i32,death_left:i32,death_right:i32}
fn identity_text_px(text:&str,bold:bool)->i32{text.chars().map(|ch|if ch.is_ascii(){if bold{8}else{6}}else{14}).sum::<i32>().max(0)}
fn dps_secondary(row:&DpsRow)->String{let spec=dps_spec(row);let score=dps_score_pair(row);if score.is_empty(){spec}else if spec.is_empty(){score}else{format!("{} · {}",spec,score)}}
fn dps_secondary_compact(row:&DpsRow,max_px:i32)->String{
    if max_px<=0{return String::new();}
    let spec=dps_spec(row);let full=dps_secondary(row);if identity_text_px(&full,false)<=max_px{return full;}
    let primary_score=if row.ability_score>0{score(row.ability_score)}else{String::new()};
    let medium=if spec.is_empty(){primary_score.clone()}else if primary_score.is_empty(){spec.clone()}else{format!("{} · {}",spec,primary_score)};
    if identity_text_px(&medium,false)<=max_px{return medium;}
    if !spec.is_empty()&&identity_text_px(&spec,false)<=max_px{return spec;}
    if spec.is_empty()&&identity_text_px(&primary_score,false)<=max_px{return primary_score;}
    String::new()
}
fn dps_row_layout(r:RECT,show_imagines:bool,row:&DpsRow)->DpsRowLayout{
    let width=(r.right-r.left).max(1);let death_w=if width>=760{24}else{20};let share_w=50;let active_w=74;let total_w=80;let gap=3;
    let death_right=r.right-4;let death_left=death_right-death_w;let share_right=death_left-gap;let share_left=share_right-share_w;let active_right=share_left-gap;let active_left=active_right-active_w;let total_right=active_left-gap;let total_left=total_right-total_w;
    let class_icon_left=r.left+23;let name_left=class_icon_left+CLASS_ICON_W+CLASS_ICON_GAP;let badge_count=if show_imagines{row.imagines.len().min(2)}else{0};let reserved_badges:usize=if show_imagines{2}else{0};
    let badge_span=reserved_badges as i32*BADGE_W+(reserved_badges.saturating_sub(1)as i32)*BADGE_GAP;
    let badge_left=if reserved_badges>0{(total_left-badge_span-BUILD_BADGE_GAP).max(name_left+82)}else{total_left};let identity_right=if reserved_badges>0{badge_left-BUILD_BADGE_GAP}else{total_left-BUILD_BADGE_GAP};let identity_w=(identity_right-name_left).max(80);
    let name_needed=identity_text_px(&row.name,true).saturating_add(10);let spec_reserve=if identity_w>=190{72}else{48};let name_cap=(identity_right-spec_reserve).max(name_left+70);let name_right=(name_left+name_needed).min(name_cap).min(identity_right);let spec_left=(name_right+8).min(identity_right);let spec_right=identity_right;
    DpsRowLayout{class_icon_left,name_left,name_right,spec_left,spec_right,badge_left,badge_count,total_left,total_right,active_left,active_right,share_left,share_right,death_left,death_right}
}
"###,
        "DPS identity layout and compact build text",
    );

    replace_once(&mut source,"draw(hdc,\"PLAYER\",RECT{left:hr.left+25,top:hr.top,right:layout.total_left-5,bottom:hr.bottom}","draw(hdc,\"PLAYER\",RECT{left:layout.name_left,top:hr.top,right:layout.total_left-5,bottom:hr.bottom}","align PLAYER header with username");

    replace_once(
        &mut source,
        "fill(hdc,&RECT{left:r.left,top:r.top,right:r.left+2,bottom:bar_top},if row.is_dead{crate::ui_theme::CRITICAL}else{class_bg});SelectObject(hdc,GetStockObject(DEFAULT_GUI_FONT));SetTextColor(hdc,if row.is_local{crate::ui_theme::CRITICAL}else{crate::ui_theme::RANK_TEXT});draw(hdc,&rank.to_string(),RECT{left:r.left+3,top:r.top,right:r.left+21,bottom:content_bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);",
        "fill(hdc,&RECT{left:r.left,top:r.top,right:r.left+2,bottom:bar_top},if row.is_dead{crate::ui_theme::CRITICAL}else{class_bg});SelectObject(hdc,GetStockObject(DEFAULT_GUI_FONT));SetTextColor(hdc,if row.is_local{crate::ui_theme::CRITICAL}else{crate::ui_theme::RANK_TEXT});draw(hdc,&rank.to_string(),RECT{left:r.left+3,top:r.top,right:r.left+21,bottom:content_bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);let class_icon=RECT{left:layout.class_icon_left,top:r.top+5,right:layout.class_icon_left+CLASS_ICON_W,bottom:r.top+5+CLASS_ICON_W};draw_class_icon_asset(hdc,class_icon,row);",
        "draw class icon between rank and username",
    );

    replace_once(
        &mut source,
        "let base=text_on(bg);SelectObject(hdc,name_font);SetTextColor(hdc,base);draw(hdc,&row.name,RECT{left:layout.name_left,top:r.top,right:layout.name_right,bottom:content_bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);SelectObject(hdc,GetStockObject(DEFAULT_GUI_FONT));let secondary=if row.is_dead{let revive=revive_status_text(row,now).0;let spec=dps_spec(row);if spec.is_empty(){format!(\"DEAD · {revive}\")}else{format!(\"{spec} · DEAD · {revive}\")}}else{dps_secondary(row)};SetTextColor(hdc,crate::ui_theme::TEXT_SECONDARY);draw(hdc,&secondary,RECT{left:layout.spec_left,top:r.top,right:layout.spec_right,bottom:content_bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);",
        "let base=text_on(bg);SelectObject(hdc,name_font);SetTextColor(hdc,base);draw(hdc,&row.name,RECT{left:layout.name_left,top:r.top,right:layout.name_right,bottom:content_bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);SelectObject(hdc,GetStockObject(DEFAULT_GUI_FONT));let secondary=if row.is_dead{let revive=revive_status_text(row,now).0;let spec=dps_spec(row);if spec.is_empty(){format!(\"DEAD · {revive}\")}else{format!(\"{spec} · DEAD · {revive}\")}}else{dps_secondary_compact(row,(layout.spec_right-layout.spec_left).max(0))};SetTextColor(hdc,crate::ui_theme::TEXT_SECONDARY);draw(hdc,&secondary,RECT{left:layout.spec_left,top:r.top,right:layout.spec_right,bottom:content_bottom},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);",
        "right-anchor build text to Imagine cluster",
    );

    replace_once(
        &mut source,
        "fn attr_color(id:i32)->u32{",
        r###"#[repr(C)]
#[derive(Clone,Copy)]
struct ClassBlendFunction{blend_op:u8,blend_flags:u8,source_constant_alpha:u8,alpha_format:u8}
#[derive(Clone,Copy,Default)]
struct ClassIconSurface{dc:usize,bitmap:usize,width:i32,height:i32}
static CLASS_ICON_CACHE:[OnceLock<ClassIconSurface>;18]=[const{OnceLock::new()};18];
#[link(name="gdi32")]
extern "system"{fn CreateDIBSection(hdc:HDC,pbmi:*const BITMAPINFO,usage:u32,ppv_bits:*mut *mut c_void,hsection:*mut c_void,offset:u32)->*mut c_void;}
#[link(name="msimg32")]
extern "system"{fn AlphaBlend(hdc_dest:HDC,x_dest:i32,y_dest:i32,w_dest:i32,h_dest:i32,hdc_src:HDC,x_src:i32,y_src:i32,w_src:i32,h_src:i32,blend:ClassBlendFunction)->i32;}
fn class_icon_asset(row:&DpsRow)->Option<(usize,&'static[u8])>{
    let spec=row.subprofession_name.trim().to_ascii_lowercase();
    match spec.as_str(){
        "iaido"|"iai"=>Some((0,include_bytes!(concat!(env!("OUT_DIR"),"/class-icons/Iaido.bmp")))),
        "moonstrike"=>Some((1,include_bytes!(concat!(env!("OUT_DIR"),"/class-icons/Moonstrike.bmp")))),
        "icicle"|"ice spear"=>Some((2,include_bytes!(concat!(env!("OUT_DIR"),"/class-icons/Icicle.bmp")))),
        "frostbeam"|"crystal"=>Some((3,include_bytes!(concat!(env!("OUT_DIR"),"/class-icons/Frostbeam.bmp")))),
        "formless"|"voidflame"=>Some((4,include_bytes!(concat!(env!("OUT_DIR"),"/class-icons/Formless.bmp")))),
        "crimson"|"blazecrimson"=>Some((5,include_bytes!(concat!(env!("OUT_DIR"),"/class-icons/Crimson.bmp")))),
        "vanguard"=>Some((6,include_bytes!(concat!(env!("OUT_DIR"),"/class-icons/Vanguard.bmp")))),
        "skyward"=>Some((7,include_bytes!(concat!(env!("OUT_DIR"),"/class-icons/Skyward.bmp")))),
        "smite"=>Some((8,include_bytes!(concat!(env!("OUT_DIR"),"/class-icons/Smite.bmp")))),
        "lifebind"=>Some((9,include_bytes!(concat!(env!("OUT_DIR"),"/class-icons/Lifebind.bmp")))),
        "earthfort"=>Some((10,include_bytes!(concat!(env!("OUT_DIR"),"/class-icons/Earthfort.bmp")))),
        "block"=>Some((11,include_bytes!(concat!(env!("OUT_DIR"),"/class-icons/Block.bmp")))),
        "wildpack"|"taming"=>Some((12,include_bytes!(concat!(env!("OUT_DIR"),"/class-icons/Wildpack.bmp")))),
        "falconry"=>Some((13,include_bytes!(concat!(env!("OUT_DIR"),"/class-icons/Falconry.bmp")))),
        "recovery"=>Some((14,include_bytes!(concat!(env!("OUT_DIR"),"/class-icons/Recovery.bmp")))),
        "shield"=>Some((15,include_bytes!(concat!(env!("OUT_DIR"),"/class-icons/Shield.bmp")))),
        "dissonance"=>Some((16,include_bytes!(concat!(env!("OUT_DIR"),"/class-icons/Dissonance.bmp")))),
        "concerto"=>Some((17,include_bytes!(concat!(env!("OUT_DIR"),"/class-icons/Concerto.bmp")))),
        _=>None,
    }
}
unsafe fn create_class_icon_surface(hdc:HDC,bytes:&'static[u8])->ClassIconSurface{
    if bytes.len()<54||&bytes[0..2]!=b"BM"{return ClassIconSurface::default();}
    let offset=u32::from_le_bytes([bytes[10],bytes[11],bytes[12],bytes[13]])as usize;let width=i32::from_le_bytes([bytes[18],bytes[19],bytes[20],bytes[21]]);let raw_height=i32::from_le_bytes([bytes[22],bytes[23],bytes[24],bytes[25]]);let bpp=u16::from_le_bytes([bytes[28],bytes[29]]);
    if width<=0||raw_height==0||bpp!=32{return ClassIconSurface::default();}let height=raw_height.abs();let Some(pixel_len)=(width as usize).checked_mul(height as usize).and_then(|px|px.checked_mul(4))else{return ClassIconSurface::default();};let Some(end)=offset.checked_add(pixel_len)else{return ClassIconSurface::default();};if end>bytes.len(){return ClassIconSurface::default();}
    let mem=CreateCompatibleDC(hdc);if mem.is_null(){return ClassIconSurface::default();}let mut info:BITMAPINFO=std::mem::zeroed();info.bmiHeader.biSize=std::mem::size_of_val(&info.bmiHeader)as u32;info.bmiHeader.biWidth=width;info.bmiHeader.biHeight=raw_height;info.bmiHeader.biPlanes=1;info.bmiHeader.biBitCount=32;let mut bits:*mut c_void=null_mut();let bitmap=CreateDIBSection(hdc,&info,DIB_RGB_COLORS,&mut bits,null_mut(),0);if bitmap.is_null()||bits.is_null(){if !bitmap.is_null(){DeleteObject(bitmap);}DeleteDC(mem);return ClassIconSurface::default();}
    std::ptr::copy_nonoverlapping(bytes[offset..end].as_ptr(),bits.cast::<u8>(),pixel_len);let old=SelectObject(mem,bitmap);if old.is_null(){DeleteObject(bitmap);DeleteDC(mem);return ClassIconSurface::default();}ClassIconSurface{dc:mem as usize,bitmap:bitmap as usize,width,height}
}
unsafe fn draw_class_icon_asset(hdc:HDC,r:RECT,row:&DpsRow)->bool{
    let Some((slot,bytes))=class_icon_asset(row)else{return false;};let surface=*CLASS_ICON_CACHE[slot].get_or_init(||create_class_icon_surface(hdc,bytes));if surface.dc==0||surface.bitmap==0{return false;}let blend=ClassBlendFunction{blend_op:0,blend_flags:0,source_constant_alpha:255,alpha_format:1};AlphaBlend(hdc,r.left,r.top,r.right-r.left,r.bottom-r.top,surface.dc as HDC,0,0,surface.width,surface.height,blend)!=0
}
fn attr_color(id:i32)->u32{"###,
        "class icon assets and alpha renderer",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1184_dps_identity_tests{
    use super::*;
    fn row(spec:&str)->DpsRow{DpsRow{name:"MrHard".into(),subprofession_name:spec.into(),ability_score:58_000,illusion_break:3_210,imagines:vec![ImagineBadge::default(),ImagineBadge::default()],..DpsRow::default()}}

    #[test]
    fn shipped_class_icons_cover_every_detected_spec(){
        let specs=["Iaido","Moonstrike","Icicle","Frostbeam","Formless","Crimson","Vanguard","Skyward","Smite","Lifebind","Earthfort","Block","Wildpack","Falconry","Recovery","Shield","Dissonance","Concerto"];
        for(index,spec)in specs.iter().enumerate(){let(actual,bytes)=class_icon_asset(&row(spec)).unwrap_or_else(||panic!("missing {spec}"));assert_eq!(actual,index,"slot for {spec}");assert!(bytes.len()>54&&&bytes[0..2]==b"BM","asset for {spec}");}
        for alias in ["Iai","Ice Spear","Crystal","Voidflame","BlazeCrimson","Taming"]{assert!(class_icon_asset(&row(alias)).is_some(),"alias {alias}");}
        assert!(class_icon_asset(&row("Unknown")).is_none());
    }

    #[test]
    fn class_icon_sits_between_rank_and_username(){
        let r=RECT{left:6,top:0,right:612,bottom:31};let layout=dps_row_layout(r,true,&row("Smite"));
        assert!(layout.class_icon_left>r.left+21);
        assert_eq!(layout.name_left,layout.class_icon_left+CLASS_ICON_W+CLASS_ICON_GAP);
    }

    #[test]
    fn build_metadata_stays_attached_to_imagines_when_meter_widens(){
        let smite=row("Smite");let narrow=dps_row_layout(RECT{left:6,top:0,right:612,bottom:31},true,&smite);let wide=dps_row_layout(RECT{left:6,top:0,right:900,bottom:31},true,&smite);
        assert_eq!(narrow.badge_left-narrow.spec_right,BUILD_BADGE_GAP);
        assert_eq!(wide.badge_left-wide.spec_right,BUILD_BADGE_GAP);
        assert_eq!(dps_secondary_compact(&smite,narrow.spec_right-narrow.spec_left),"Smite · 58k +3210");
    }

    #[test]
    fn narrow_layout_drops_break_then_score_before_spec(){
        let smite=row("Smite");let full="Smite · 58k +3210";let medium="Smite · 58k";let spec="Smite";
        assert_eq!(dps_secondary_compact(&smite,identity_text_px(full,false)),full);
        assert_eq!(dps_secondary_compact(&smite,identity_text_px(medium,false)),medium);
        assert_eq!(dps_secondary_compact(&smite,identity_text_px(spec,false)),spec);
    }
}
"#);

    fs::write(path,source).expect("write v1.18.4 feature overlay");
}

fn main(){
    prior::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    materialize_class_icons(&out);
    patch_feature_overlay(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1184.rs");
    println!("cargo:rerun-if-changed=../assets/source/ClassIcons.v1184.b64.001");
    println!("cargo:rerun-if-changed=../assets/source/ClassIcons.v1184.b64.002");
    println!("cargo:rerun-if-changed=../assets/source/ClassIcons.v1184.b64.003");
    println!("cargo:rerun-if-changed=../assets/source/ClassIcons.v1184.b64.004");
    println!("cargo:rerun-if-changed=../assets/source/ClassIcons.v1184.b64.005");
    println!("cargo:rerun-if-changed=../assets/source/ClassIcons.v1184.b64.006");
}
