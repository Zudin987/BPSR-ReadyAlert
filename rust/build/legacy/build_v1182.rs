use std::{env, fs, path::{Path, PathBuf}};

mod prior {
    include!("build_v1181.rs");
    pub fn run() { main(); }
}

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){
    let count=source.matches(from).count();
    assert_eq!(count,1,"v1.18.2 patch {label} expected one match, found {count}");
    *source=source.replacen(from,to,1);
}

fn replace_between(source:&mut String,start:&str,end:&str,replacement:&str,label:&str){
    let count=source.matches(start).count();
    assert_eq!(count,1,"v1.18.2 patch {label} start expected one match, found {count}");
    let begin=source.find(start).expect("v1.18.2 start anchor");
    let rel_end=source[begin..].find(end).unwrap_or_else(||panic!("v1.18.2 patch {label} end anchor missing"));
    source.replace_range(begin..begin+rel_end,replacement);
}

fn patch_feature_overlay(out:&Path){
    let path=out.join("feature_overlays_v170_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read v1.18.1 generated overlays").replace("\r\n","\n");

    replace_once(
        &mut source,
        "let image=take(96,\"Copy as Image\");",
        "let image=take(116,\"Copy as Image\");",
        "widen Copy as Image action",
    );

    replace_between(
        &mut source,
        "unsafe fn paint_toolbar_action(hdc:HDC,r:RECT,label:&str,active:bool,hovered:bool){",
        "fn overlay_header(kind:Kind)->&'static str{",
        r###"unsafe fn paint_toolbar_action(hdc:HDC,r:RECT,label:&str,active:bool,hovered:bool){
    let back=if active{crate::ui_theme::ACCENT}else if hovered{crate::ui_theme::SURFACE_HOVER}else{crate::ui_theme::SURFACE};
    fill(hdc,&r,back);SetTextColor(hdc,if active{rgb(248,253,252)}else{crate::ui_theme::TEXT});
    draw(hdc,label,r,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);
}
"###,
        "toolbar actions without ellipsis",
    );

    replace_between(
        &mut source,
        "fn dps_row_layout(r:RECT,show_imagines:bool,row:&DpsRow)->DpsRowLayout{",
        "fn dps_identity(row:&DpsRow)->String{",
        r###"fn dps_row_layout(r:RECT,show_imagines:bool,row:&DpsRow)->DpsRowLayout{
    let width=(r.right-r.left).max(1);let death_w=if width>=760{24}else{20};let share_w=50;let active_w=74;let total_w=80;let gap=3;
    let death_right=r.right-4;let death_left=death_right-death_w;let share_right=death_left-gap;let share_left=share_right-share_w;let active_right=share_left-gap;let active_left=active_right-active_w;let total_right=active_left-gap;let total_left=total_right-total_w;
    let name_left=r.left+25;let badge_count=if show_imagines{row.imagines.len().min(2)}else{0};let reserved_badges:usize=if show_imagines{2}else{0};
    let badge_span=reserved_badges as i32*BADGE_W+(reserved_badges.saturating_sub(1)as i32)*BADGE_GAP;
    let badge_left=if reserved_badges>0{(total_left-badge_span-5).max(name_left+82)}else{total_left};let identity_right=if reserved_badges>0{badge_left-5}else{total_left-5};let identity_w=(identity_right-name_left).max(80);
    let name_needed=identity_text_px(&row.name,true).saturating_add(24);let spec_reserve=if identity_w>=190{62}else{36};let name_cap=(identity_right-spec_reserve).max(name_left+80);let name_right=(name_left+name_needed).min(name_cap).min(identity_right);let spec_left=(name_right+6).min(identity_right);let spec_right=identity_right;
    DpsRowLayout{name_left,name_right,spec_left,spec_right,badge_left,badge_count,total_left,total_right,active_left,active_right,share_left,share_right,death_left,death_right}
}
"###,
        "prioritize full player names",
    );

    replace_once(
        &mut source,
        "let time_r=RECT{left:(rc.right-92).max(target_r.left+80),top:target_r.top,right:rc.right-10,bottom:target_r.bottom};let hp_r=RECT{left:(time_r.left-100).max(target_r.left+80),top:target_r.top,right:time_r.left-6,bottom:target_r.bottom};let enrage_r=RECT{left:(hp_r.left-150).max(target_r.left+120),top:target_r.top,right:hp_r.left-5,bottom:target_r.bottom};let title_right=if enrage.is_some(){enrage_r.left-6}else{hp_r.left-8};",
        "let time_r=RECT{left:(rc.right-90).max(target_r.left+80),top:target_r.top,right:rc.right-10,bottom:target_r.bottom};let hp_r=RECT{left:(time_r.left-78).max(target_r.left+80),top:target_r.top,right:time_r.left-8,bottom:target_r.bottom};let enrage_r=RECT{left:(hp_r.left-155).max(target_r.left+120),top:target_r.top,right:hp_r.left-12,bottom:target_r.bottom};let title_right=if enrage.is_some(){enrage_r.left-8}else{hp_r.left-8};",
        "tighten HP and separate Enrage",
    );

    replace_once(
        &mut source,
        "let hp_label=RECT{left:hp_r.left,top:hp_r.top,right:hp_r.left+22,bottom:hp_r.bottom};let hp_value=RECT{left:hp_r.left+20,top:hp_r.top,right:hp_r.right,bottom:hp_r.bottom};",
        "let hp_label=RECT{left:hp_r.left,top:hp_r.top,right:hp_r.left+18,bottom:hp_r.bottom};let hp_value=RECT{left:hp_r.left+19,top:hp_r.top,right:hp_r.right,bottom:hp_r.bottom};",
        "tight HP label/value spacing",
    );

    replace_once(
        &mut source,
        "let class_bg=spec_color(row);let bg=if row.is_dead{dim_color(class_bg,74)}else{class_bg};",
        "let class_bg=spec_color(row);let bg=if row.is_dead{dim_color(crate::ui_theme::CRITICAL,48)}else{class_bg};",
        "restore red dead-row background",
    );

    replace_once(
        &mut source,
        "SetTextColor(hdc,crate::ui_theme::RANK_TEXT);draw(hdc,&rank.to_string(),",
        "SetTextColor(hdc,if row.is_local{crate::ui_theme::CRITICAL}else{crate::ui_theme::RANK_TEXT});draw(hdc,&rank.to_string(),",
        "restore red local rank number",
    );

    replace_once(
        &mut source,
        "draw(hdc,&row.name,RECT{left:layout.name_left,top:r.top,right:layout.name_right,bottom:content_bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);",
        "draw(hdc,&row.name,RECT{left:layout.name_left,top:r.top,right:layout.name_right,bottom:content_bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);",
        "player name without ellipsis",
    );

    if source.contains("const MECH_ATTR_H: i32 = 29;") {
        replace_once(&mut source,"const MECH_ATTR_H: i32 = 29;","const MECH_ATTR_H: i32 = 38;","Tracker attribute row height");
    } else {
        replace_once(&mut source,"const MECH_ATTR_H:i32=29;","const MECH_ATTR_H:i32=38;","Tracker attribute row height");
    }

    replace_once(
        &mut source,
        "draw(hdc,&format!(\"Track attributes ({selected}/6)\"),",
        "draw(hdc,&format!(\"Track attributes ({selected}/{})\",feature_settings::MAX_TRACKED_ATTRIBUTES),",
        "Tracker settings selected count",
    );
    replace_once(
        &mut source,
        "else if features.mechanic_attributes.tracked.len()<6{features.mechanic_attributes.tracked.push(id);}",
        "else if features.mechanic_attributes.tracked.len()<feature_settings::MAX_TRACKED_ATTRIBUTES{features.mechanic_attributes.tracked.push(id);}",
        "Tracker settings selection limit",
    );
    replace_once(
        &mut source,
        "change(&mut features);features.normalize();let snapshot=features.clone();",
        "change(&mut features);feature_settings::normalize_v1182(&mut features);let snapshot=features.clone();",
        "Tracker-aware feature normalization",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1182_ui_tests {
    use super::*;

    #[test]
    fn copy_image_button_has_room_for_full_label() {
        let actions=toolbar_action_rects(700);
        assert_eq!(actions[3].1,"Copy as Image");
        assert!(actions[3].0.right-actions[3].0.left>=116);
        assert_eq!(actions[3].0.left-actions[2].0.right,12);
    }

    #[test]
    fn tracker_attributes_use_two_roomier_rows_for_eight() {
        assert_eq!(mechanic_attr_rows(8),2);
        assert!(MECH_ATTR_H>=38);
        assert_eq!(crate::feature_settings::MAX_TRACKED_ATTRIBUTES,8);
    }
}
"#);

    fs::write(path,source).expect("write v1.18.2 feature overlay");
}

fn main(){
    prior::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_feature_overlay(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1182.rs");
}
