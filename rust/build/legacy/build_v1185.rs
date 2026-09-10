use std::{env, fs, path::{Path, PathBuf}};

mod prior {
    // Intentionally branch from v1.18.3: v1.18.4 only added the class icons
    // plus the grouped build-info layout. Reapply the layout without icon code/assets.
    include!("build_v1183.rs");
    pub fn run() { main(); }
}

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){
    let count=source.matches(from).count();
    assert_eq!(count,1,"v1.18.5 patch {label} expected one match, found {count}");
    *source=source.replacen(from,to,1);
}

fn replace_between(source:&mut String,start:&str,end:&str,replacement:&str,label:&str){
    let count=source.matches(start).count();
    assert_eq!(count,1,"v1.18.5 patch {label} start expected one match, found {count}");
    let begin=source.find(start).expect("v1.18.5 start anchor");
    let rel_end=source[begin..].find(end).unwrap_or_else(||panic!("v1.18.5 patch {label} end anchor missing"));
    source.replace_range(begin..begin+rel_end,replacement);
}

fn patch_feature_overlay(out:&Path){
    let path=out.join("feature_overlays_v170_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read v1.18.3 generated overlays").replace("\r\n","\n");

    replace_once(
        &mut source,
        "const BADGE_W: i32 = 25;\nconst BADGE_GAP: i32 = 3;",
        "const BADGE_W: i32 = 25;\nconst BADGE_GAP: i32 = 3;\nconst BUILD_BADGE_GAP:i32=5;",
        "grouped build spacing",
    );

    replace_between(
        &mut source,
        "#[derive(Clone,Copy)]struct DpsRowLayout{",
        "fn dps_identity(row:&DpsRow)->String{",
        r###"#[derive(Clone,Copy)]struct DpsRowLayout{name_left:i32,name_right:i32,spec_left:i32,spec_right:i32,badge_left:i32,badge_count:usize,total_left:i32,total_right:i32,active_left:i32,active_right:i32,share_left:i32,share_right:i32,death_left:i32,death_right:i32}
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
    let name_left=r.left+25;let badge_count=if show_imagines{row.imagines.len().min(2)}else{0};let reserved_badges:usize=if show_imagines{2}else{0};
    let badge_span=reserved_badges as i32*BADGE_W+(reserved_badges.saturating_sub(1)as i32)*BADGE_GAP;
    let badge_left=if reserved_badges>0{(total_left-badge_span-BUILD_BADGE_GAP).max(name_left+82)}else{total_left};let identity_right=if reserved_badges>0{badge_left-BUILD_BADGE_GAP}else{total_left-BUILD_BADGE_GAP};let identity_w=(identity_right-name_left).max(80);
    let name_needed=identity_text_px(&row.name,true).saturating_add(24);let spec_reserve=if identity_w>=190{72}else{48};let name_cap=(identity_right-spec_reserve).max(name_left+80);let name_right=(name_left+name_needed).min(name_cap).min(identity_right);let spec_left=(name_right+8).min(identity_right);let spec_right=identity_right;
    DpsRowLayout{name_left,name_right,spec_left,spec_right,badge_left,badge_count,total_left,total_right,active_left,active_right,share_left,share_right,death_left,death_right}
}
"###,
        "group build metadata with Imagines without class icon",
    );

    replace_once(
        &mut source,
        "let base=text_on(bg);SelectObject(hdc,name_font);SetTextColor(hdc,base);draw(hdc,&row.name,RECT{left:layout.name_left,top:r.top,right:layout.name_right,bottom:content_bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);SelectObject(hdc,GetStockObject(DEFAULT_GUI_FONT));let secondary=if row.is_dead{let revive=revive_status_text(row,now).0;let spec=dps_spec(row);if spec.is_empty(){format!(\"DEAD · {revive}\")}else{format!(\"{spec} · DEAD · {revive}\")}}else{dps_secondary(row)};SetTextColor(hdc,crate::ui_theme::TEXT_SECONDARY);draw(hdc,&secondary,RECT{left:layout.spec_left,top:r.top,right:layout.spec_right,bottom:content_bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);",
        "let base=text_on(bg);SelectObject(hdc,name_font);SetTextColor(hdc,base);draw(hdc,&row.name,RECT{left:layout.name_left,top:r.top,right:layout.name_right,bottom:content_bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);SelectObject(hdc,GetStockObject(DEFAULT_GUI_FONT));let secondary=if row.is_dead{let revive=revive_status_text(row,now).0;let spec=dps_spec(row);if spec.is_empty(){format!(\"DEAD · {revive}\")}else{format!(\"{spec} · DEAD · {revive}\")}}else{dps_secondary_compact(row,(layout.spec_right-layout.spec_left).max(0))};SetTextColor(hdc,crate::ui_theme::TEXT_SECONDARY);draw(hdc,&secondary,RECT{left:layout.spec_left,top:r.top,right:layout.spec_right,bottom:content_bottom},DT_RIGHT|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX|DT_END_ELLIPSIS);",
        "right-anchor build text to Imagine cluster",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1185_dps_identity_tests{
    use super::*;
    fn row(spec:&str)->DpsRow{DpsRow{name:"MrHard".into(),subprofession_name:spec.into(),ability_score:58_000,illusion_break:3_210,imagines:vec![ImagineBadge::default(),ImagineBadge::default()],..DpsRow::default()}}

    #[test]
    fn username_returns_to_original_rank_spacing_without_class_icon(){
        let r=RECT{left:6,top:0,right:612,bottom:31};let layout=dps_row_layout(r,true,&row("Smite"));
        assert_eq!(layout.name_left,r.left+25);
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

    fs::write(path,source).expect("write v1.18.5 feature overlay");
}

fn main(){
    prior::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_feature_overlay(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1185.rs");
}
