use std::{env, fs, path::{Path, PathBuf}};

mod previous {
    include!("build_v1310_true_pixel.rs");
    pub fn run() { main(); }
}

fn replace_if_present(source: &mut String, from: &str, to: &str) {
    if source.contains(from) { *source = source.replace(from, to); }
}

fn patch_native_window_finish(source: &mut String) {
    replace_if_present(
        source,
        "crate::telemetry::ui_audit_v1302::dark_titlebar(hwnd);",
        "crate::telemetry::ui_audit_v1302::dark_titlebar(hwnd);crate::ui_modern::finish_native_window(hwnd);",
    );
    replace_if_present(
        source,
        "crate::telemetry::ui_audit_v1302::mist_titlebar(hwnd);",
        "crate::telemetry::ui_audit_v1302::mist_titlebar(hwnd);crate::ui_modern::finish_native_window(hwnd);",
    );
}

fn patch_feature_surfaces(source: &mut String) {
    // Keep class identity while moving the meter from saturated full-row fills to
    // neutral Pixel-style tonal surfaces. The existing class strip/progress semantics
    // remain unchanged.
    replace_if_present(
        source,
        "let class_bg=spec_color(row);let bg=if row.is_dead{dim_color(crate::ui_modern::BPSR_DANGER,48)}else{class_bg};fill(hdc,&r,bg);",
        "let class_bg=spec_color(row);let bg=if row.is_dead{crate::ui_modern::tonal_danger_surface()}else{crate::ui_modern::tonal_class_surface(class_bg)};fill(hdc,&r,bg);",
    );

    replace_if_present(
        source,
        "let bg=if row.is_dead{rgb(105,28,34)}else{spec_color(row)};fill(hdc,&r,bg);if row.is_local{outline(hdc,r,rgb(212,175,55),2);}let base=text_on(bg);",
        "let class_bg=spec_color(row);let bg=if row.is_dead{crate::ui_modern::tonal_danger_surface()}else{crate::ui_modern::tonal_class_surface(class_bg)};fill(hdc,&r,bg);fill(hdc,&RECT{left:r.left,top:r.top,right:r.left+3,bottom:r.bottom},if row.is_dead{crate::ui_modern::BPSR_DANGER}else{class_bg});if row.is_local{crate::ui_modern::stroke_round_rect(hdc,r,crate::ui_modern::BPSR_ACCENT,crate::ui_modern::RADIUS_SMALL,1);}let base=text_on(bg);",
    );

    replace_if_present(
        source,
        "if row.is_local{outline(hdc,r,rgb(212,175,55),1);}",
        "if row.is_local{crate::ui_modern::stroke_round_rect(hdc,r,crate::ui_modern::BPSR_ACCENT,crate::ui_modern::RADIUS_SMALL,1);}",
    );

    replace_if_present(
        source,
        "unsafe fn paint_pinned_self_frame(hdc:HDC,r:&RECT){let white=rgb(255,255,255);fill(hdc,&RECT{left:r.left,top:r.top,right:r.right,bottom:r.top+1},white);fill(hdc,&RECT{left:r.left,top:r.bottom-1,right:r.right,bottom:r.bottom},white);fill(hdc,&RECT{left:r.left,top:r.top,right:r.left+1,bottom:r.bottom},white);fill(hdc,&RECT{left:r.right-1,top:r.top,right:r.right,bottom:r.bottom},white);}",
        "unsafe fn paint_pinned_self_frame(hdc:HDC,r:&RECT){crate::ui_modern::stroke_round_rect(hdc,*r,crate::ui_modern::BPSR_ACCENT,crate::ui_modern::RADIUS_SMALL,1);}",
    );

    // Pixel-style custom scrollbar: quiet track, rounded neutral thumb. Interaction
    // and scroll math remain exactly the same.
    replace_if_present(
        source,
        "unsafe fn paint_scrollbar(hdc:HDC,rc:RECT,total:usize,visible:usize,scroll:usize,top:i32){if total<=visible||visible==0{return;}let track_top=top+2;let track_bottom=rc.bottom-5;let track_h=(track_bottom-track_top).max(20);fill(hdc,&RECT{left:rc.right-5,top:track_top,right:rc.right-2,bottom:track_bottom},crate::ui_modern::DARK_BORDER);let thumb_h=((track_h as f64*visible as f64/total as f64)as i32).clamp(16,track_h);let max_scroll=total.saturating_sub(visible).max(1);let pos=((track_h-thumb_h)as f64*scroll.min(max_scroll)as f64/max_scroll as f64)as i32;fill(hdc,&RECT{left:rc.right-6,top:track_top+pos,right:rc.right-1,bottom:track_top+pos+thumb_h},crate::ui_modern::BPSR_ACCENT);}",
        "unsafe fn paint_scrollbar(hdc:HDC,rc:RECT,total:usize,visible:usize,scroll:usize,top:i32){if total<=visible||visible==0{return;}let track_top=top+2;let track_bottom=rc.bottom-5;let track_h=(track_bottom-track_top).max(20);crate::ui_modern::fill_round_rect(hdc,RECT{left:rc.right-4,top:track_top,right:rc.right-2,bottom:track_bottom},crate::ui_modern::DARK_BORDER,2);let thumb_h=((track_h as f64*visible as f64/total as f64)as i32).clamp(16,track_h);let max_scroll=total.saturating_sub(visible).max(1);let pos=((track_h-thumb_h)as f64*scroll.min(max_scroll)as f64/max_scroll as f64)as i32;crate::ui_modern::fill_round_rect(hdc,RECT{left:rc.right-6,top:track_top+pos,right:rc.right-1,bottom:track_top+pos+thumb_h},crate::ui_modern::BPSR_MUTED,3);}",
    );

    // Entity Inspector still contained a handful of pre-design-system teal/raw dark
    // literals. Route only those unique presentation colors into shared Pixel tokens.
    for (from, to) in [
        ("rgb(15,19,24)", "crate::ui_modern::DARK_BG"),
        ("rgb(24,32,38)", "crate::ui_modern::DARK_SURFACE"),
        ("rgb(19,25,30)", "crate::ui_modern::DARK_SURFACE"),
        ("rgb(66,211,190)", "crate::ui_modern::BPSR_ACCENT"),
        ("rgb(240,245,247)", "crate::ui_modern::BPSR_TEXT"),
        ("rgb(163,180,190)", "crate::ui_modern::BPSR_TEXT_SECONDARY"),
        ("rgb(199,226,220)", "crate::ui_modern::BPSR_SUCCESS"),
        ("rgb(224,232,236)", "crate::ui_modern::BPSR_TEXT"),
        ("rgb(112,205,190)", "crate::ui_modern::BPSR_ACCENT_HOVER"),
    ] {
        replace_if_present(source, from, to);
    }
}

fn patch_file(out: &Path, name: &str, feature: bool) {
    let path = out.join(name);
    if !path.exists() { return; }
    let mut source = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {name} for Pixel phase 2: {e}"))
        .replace("\r\n", "\n");
    patch_native_window_finish(&mut source);
    if feature { patch_feature_surfaces(&mut source); }
    fs::write(path, source).unwrap_or_else(|e| panic!("write {name} for Pixel phase 2: {e}"));
}

fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_file(&out, "settings_ui_v1160_fixed.rs", false);
    patch_file(&out, "event_tracker_ui_v1160_fixed.rs", false);
    patch_file(&out, "feature_overlays_v170_fixed.rs", true);
    patch_file(&out, "overlay_v150_v1181.rs", false);
    patch_file(&out, "benchmark_ui_v1302.rs", false);
    patch_file(&out, "win_v182_fixed.rs", false);
    patch_file(&out, "updater_v1241.rs", false);
    println!("cargo:rerun-if-changed=build/legacy/build_pixel_phase2.rs");
    println!("cargo:rerun-if-changed=src/ui_pixel_phase2.rs");
}
