use std::{env, fs, path::{Path, PathBuf}};

mod prior {
    include!("build_v1173.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.17.4 patch {label} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn patch_overlay(out: &Path) {
    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read v1.17.3 overlays").replace("\r\n", "\n");

    replace_once(
        &mut source,
        "fn meter_page_rows<'a>(state:&'a State,physical:usize)->Vec<(usize,&'a DpsRow)>{",
        "fn rank_is_forced_pin(total:usize,scroll:usize,regular_page:usize,rank:usize)->bool{if total==0||regular_page==0||rank==0||rank>total{return false;}let page=regular_page.min(total).max(1);let start=scroll.min(total.saturating_sub(page));let end=(start+page).min(total);let index=rank-1;index<start||index>=end}\nfn meter_page_rows<'a>(state:&'a State,physical:usize)->Vec<(usize,&'a DpsRow)>{",
        "forced-pin detection helper",
    );

    replace_once(
        &mut source,
        "unsafe fn on_wheel(hwnd:HWND,state:&mut State,wparam:WPARAM){state.hover_text=None;if state.collapsed{return;}let delta=(((wparam>>16)&0xffff)as u16 as i16)as i32;if delta==0{return;}let steps=((delta.abs()/120).max(1)as usize)*3;",
        "fn wheel_row_steps(delta:i32)->usize{if delta==0{0}else{(delta.abs()/120).max(1)as usize}}\nunsafe fn on_wheel(hwnd:HWND,state:&mut State,wparam:WPARAM){state.hover_text=None;if state.collapsed{return;}let delta=(((wparam>>16)&0xffff)as u16 as i16)as i32;if delta==0{return;}let steps=wheel_row_steps(delta);",
        "one-row mouse wheel scrolling",
    );

    replace_once(
        &mut source,
        "for(screen_i,(rank,row))in shown.iter().enumerate(){let rank=*rank;let row=*row;let y=top+screen_i as i32*DPS_ROW_H;",
        "for(screen_i,(rank,row))in shown.iter().enumerate(){let rank=*rank;let row=*row;let forced_self=row.is_local&&rank_is_forced_pin(rows.len(),state.scroll,meter_regular_page_size(state,visible),rank);let y=top+screen_i as i32*DPS_ROW_H;",
        "tag force-pinned owner row",
    );

    replace_once(
        &mut source,
        "fill(hdc,&RECT{left:r.left,top:bar_top,right:r.left+filled,bottom:bar_bottom},color);}}paint_scrollbar(hdc,rc,rows.len(),meter_regular_page_size(state,visible),state.scroll,top);",
        "fill(hdc,&RECT{left:r.left,top:bar_top,right:r.left+filled,bottom:bar_bottom},color);}if forced_self{let white=rgb(255,255,255);fill(hdc,&RECT{left:r.left,top:r.top,right:r.right,bottom:r.top+1},white);fill(hdc,&RECT{left:r.left,top:r.bottom-1,right:r.right,bottom:r.bottom},white);fill(hdc,&RECT{left:r.left,top:r.top,right:r.left+1,bottom:r.bottom},white);fill(hdc,&RECT{left:r.right-1,top:r.top,right:r.right,bottom:r.bottom},white);}}paint_scrollbar(hdc,rc,rows.len(),meter_regular_page_size(state,visible),state.scroll,top);",
        "thin white force-pinned owner border",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1174_overlay_tests {
    use super::*;

    #[test]
    fn forced_pin_marker_only_applies_outside_ranked_slice() {
        assert!(rank_is_forced_pin(14, 0, 5, 14));
        assert!(rank_is_forced_pin(14, 0, 4, 14));
        assert!(!rank_is_forced_pin(14, 0, 5, 3));
        assert!(!rank_is_forced_pin(14, 9, 5, 14));
    }

    #[test]
    fn mouse_wheel_moves_one_row_per_notch() {
        assert_eq!(wheel_row_steps(120), 1);
        assert_eq!(wheel_row_steps(-120), 1);
        assert_eq!(wheel_row_steps(240), 2);
        assert_eq!(wheel_row_steps(-360), 3);
        assert_eq!(wheel_row_steps(0), 0);
    }
}
"#);

    fs::write(path, source).expect("write v1.17.4 overlays");
}

fn main() {
    prior::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_overlay(&out);
    println!("cargo:rerun-if-changed=build_v1174.rs");
}
