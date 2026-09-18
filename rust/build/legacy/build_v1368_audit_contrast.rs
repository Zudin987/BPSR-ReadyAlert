// Final UI audit: selectively improve meaningful muted copy without altering
// approved DPS bars, class colours, overall brightness or disabled controls.
use std::{env, fs, path::PathBuf};
mod previous {
    include!("build_v1367_final_ui_audit.rs");
    pub fn run() { main(); }
}
fn replace_once(src: &mut String, from: &str, to: &str, issue: &str) {
    assert_eq!(src.matches(from).count(), 1, "audit contrast {issue}: expected unique generated anchor");
    *src = src.replacen(from, to, 1);
}
fn patch_qa(out: &PathBuf) {
    let path = out.join("ui_pixel_qa_v1316.rs");
    let mut src = fs::read_to_string(&path).expect("read themed QA drawing");
    replace_once(&mut src, "unsafe fn qa_empty_copy(kind: usize)",
        "pub const AUDIT_MEANINGFUL_MUTED: u32 = crate::ui_theme::rgb(150,155,170);\n\nunsafe fn qa_empty_copy(kind: usize)",
        "local secondary text token");
    let start = src.find("pub unsafe fn qa_draw_empty_state(").expect("empty state start");
    let end = start + src[start..].find("unsafe fn qa_paint_empty_list(").expect("empty state end");
    let mut body = src[start..end].to_owned();
    replace_once(&mut body,"SetTextColor(hdc, BPSR_MUTED);","SetTextColor(hdc, AUDIT_MEANINGFUL_MUTED);","meaningful empty explanation");
    src.replace_range(start..end,&body);
    // WCAG 2.1 text contrast calculation using actual RGB endpoints and alpha
    // compositing over both white spell effects and a black game background.
    src.push_str(r#"
#[cfg(test)] mod audit_composited_contrast_tests {
    use super::*;
    fn luminance(color: u32) -> f64 {
        let channel = |shift| {
            let value = ((color >> shift) & 255) as f64 / 255.0;
            if value <= 0.04045 {value / 12.92} else {((value + 0.055) / 1.055).powf(2.4)}
        };
        0.2126*channel(0)+0.7152*channel(8)+0.0722*channel(16)
    }
    fn contrast(first:u32,second:u32)->f64{
        let a=luminance(first);let b=luminance(second);
        (a.max(b)+0.05)/(a.min(b)+0.05)
    }
    fn composite(foreground:u32,background:u32,alpha:f64)->u32{
        let blend=|shift|{
            let fg=((foreground>>shift)&255) as f64;
            let bg=((background>>shift)&255) as f64;
            (fg*alpha+bg*(1.0-alpha)).round() as u32
        };
        blend(0)|(blend(8)<<8)|(blend(16)<<16)
    }
    #[test] fn meaningful_tracker_and_empty_text_is_readable_at_85_percent_on_light_and_dark_gameplay(){
        for gameplay in [crate::ui_theme::rgb(255,255,255),crate::ui_theme::rgb(0,0,0)] {
            let text=composite(AUDIT_MEANINGFUL_MUTED,gameplay,0.85);
            let panel=composite(DARK_SURFACE,gameplay,0.85);
            assert!(contrast(text,panel)>=4.5, "meaningful muted text contrast failed at 85% opacity");
        }
    }
    #[test] fn ordinary_enabled_field_outline_reaches_three_to_one(){
        assert!(contrast(crate::ui_theme::rgb(118,124,140),MIST_INPUT)>=3.0);
    }
}
"#);
    fs::write(path,src).expect("write audited QA contrast");
}
fn patch_tracker(out:&PathBuf){
    let path=out.join("feature_overlays_v170_fixed.rs");
    let mut src=fs::read_to_string(&path).expect("read generated mechanics overlay");
    replace_once(&mut src,
        "SetTextColor(hdc,crate::ui_modern::BPSR_MUTED);draw(hdc,mechanic_attr_overlay_label(",
        "SetTextColor(hdc,crate::ui_modern::AUDIT_MEANINGFUL_MUTED);draw(hdc,mechanic_attr_overlay_label(",
        "tracker attribute label only");
    replace_once(&mut src,
        "SetTextColor(hdc,row_color);draw(hdc,&format!(\"{label}: {name}\")",
        "SetTextColor(hdc,if active.is_none(){crate::ui_modern::AUDIT_MEANINGFUL_MUTED}else{row_color});draw(hdc,&format!(\"{label}: {name}\")",
        "Food and Serum absent copy only");
    fs::write(path,src).expect("write targeted mechanic labels");
}
fn main(){
    previous::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_qa(&out);
    patch_tracker(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1368_audit_contrast.rs");
}
