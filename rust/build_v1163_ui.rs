use std::{env, fs, path::PathBuf};

mod prior {
    include!("build_v1163.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.16.3 UI polish {label} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn main() {
    prior::run();

    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read generated v1.16.3 overlay for UI polish");

    // DEFAULT_GUI_FONT is narrower than the old conservative 7px ASCII estimate.
    // A 6px estimate keeps the specialization/score text compact and puts the
    // first Battle Imagine immediately after the displayed Illusion Break score.
    replace_once(
        &mut source,
        "fn identity_text_px(text:&str,bold:bool)->i32{text.chars().map(|ch|if ch.is_ascii(){if bold{8}else{7}}else{14}).sum::<i32>().max(0)}",
        "fn identity_text_px(text:&str,bold:bool)->i32{text.chars().map(|ch|if ch.is_ascii(){if bold{8}else{6}}else{14}).sum::<i32>().max(0)}",
        "compact secondary identity width",
    );
    replace_once(
        &mut source,
        "let badge_left=if badge_count>0{(spec_right+4).min(total_left-badge_span-2)}else{total_left};",
        "let badge_left=if badge_count>0{(spec_right+2).min(total_left-badge_span-2)}else{total_left};",
        "tight Imagine gap",
    );

    // The local-player marker is the rank itself now: bold, name-sized and red.
    // Remove the white frame and reclaim its two pixels for the normal progress bar.
    replace_once(
        &mut source,
        "let bar_bottom=if row.is_local{r.bottom-2}else{r.bottom};",
        "let bar_bottom=r.bottom;",
        "remove owner border reservation",
    );
    replace_once(
        &mut source,
        "SelectObject(hdc,GetStockObject(DEFAULT_GUI_FONT));SetTextColor(hdc,text_on(bg));draw(hdc,&rank.to_string(),RECT{left:r.left+3,top:r.top,right:r.left+21,bottom:content_bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);SelectObject(hdc,bold_font);",
        "if row.is_local{SelectObject(hdc,bold_font);SetTextColor(hdc,rgb(255,0,0));}else{SelectObject(hdc,GetStockObject(DEFAULT_GUI_FONT));SetTextColor(hdc,text_on(bg));}draw(hdc,&rank.to_string(),RECT{left:r.left+3,top:r.top,right:r.left+21,bottom:content_bottom},DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);SelectObject(hdc,bold_font);",
        "owner rank emphasis",
    );
    replace_once(
        &mut source,
        "if row.is_local{outline(hdc,r,rgb(255,255,255),2);}",
        "",
        "remove owner white frame",
    );

    fs::write(path, source).expect("write v1.16.3 owner/spacing polish");
    println!("cargo:rerun-if-changed=build_v1163_ui.rs");
}
