use std::{env,fs,path::{Path,PathBuf}};

mod prior {
    include!("build_v1212_fix.rs");
    pub fn run(){main();}
}

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){
    let count=source.matches(from).count();
    assert_eq!(count,1,"v1.21.3 patch {label} expected one match, found {count}");
    *source=source.replacen(from,to,1);
}

fn patch_feature_overlay(out:&Path){
    let path=out.join("feature_overlays_v170_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read v1.21.2 generated overlays").replace("\r\n","\n");

    // GDI's default font cell makes F/S look optically lower than the square
    // segments even when DT_VCENTER is mathematically centered. Shift the
    // label by one logical pixel upward; the overlay transform scales this
    // naturally with the selected DPS scale.
    replace_once(
        &mut source,
        "const DPS_CONSUMABLE_TIMER_ID:usize=0x1212;",
        "const DPS_CONSUMABLE_TIMER_ID:usize=0x1212;\nconst DPS_CONSUMABLE_LABEL_NUDGE_Y:i32=-1;",
        "consumable label optical-alignment constant",
    );

    replace_once(
        &mut source,
        "draw(hdc,label,RECT{left:x,top:y,right:x+label_w,bottom:y+line_h},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);",
        "let label_y=y+DPS_CONSUMABLE_LABEL_NUDGE_Y;draw(hdc,label,RECT{left:x,top:label_y,right:x+label_w,bottom:label_y+line_h},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);",
        "F/S label center alignment",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1213_consumable_alignment_tests{
    use super::*;

    #[test]
    fn consumable_labels_receive_one_logical_pixel_upward_optical_nudge(){
        assert_eq!(DPS_CONSUMABLE_LABEL_NUDGE_Y,-1);
    }
}
"#);

    fs::write(path,source).expect("write v1.21.3 consumable alignment patch");
}

fn main(){
    prior::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_feature_overlay(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1213.rs");
}
