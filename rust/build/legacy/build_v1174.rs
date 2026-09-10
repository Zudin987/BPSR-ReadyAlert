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
        "unsafe fn on_wheel(hwnd:HWND,state:&mut State,wparam:WPARAM){state.hover_text=None;if state.collapsed{return;}let delta=(((wparam>>16)&0xffff)as u16 as i16)as i32;if delta==0{return;}let steps=((delta.abs()/120).max(1)as usize)*3;if delta>0{state.scroll=state.scroll.saturating_sub(steps);}else{state.scroll=state.scroll.saturating_add(steps);}clamp_scroll(hwnd,state);InvalidateRect(hwnd,null(),0);}",
        "fn wheel_row_steps(delta:i32)->usize{if delta==0{0}else{(delta.abs()/120).max(1)as usize}}\nunsafe fn on_wheel(hwnd:HWND,state:&mut State,wparam:WPARAM){state.hover_text=None;if state.collapsed{return;}let delta=(((wparam>>16)&0xffff)as u16 as i16)as i32;let steps=wheel_row_steps(delta);if steps==0{return;}if delta>0{state.scroll=state.scroll.saturating_sub(steps);}else{state.scroll=state.scroll.saturating_add(steps);}clamp_scroll(hwnd,state);InvalidateRect(hwnd,null(),0);}",
        "one row per wheel notch",
    );

    replace_once(
        &mut source,
        "fn meter_page_rows<'a>(state:&'a State,physical:usize)->Vec<(usize,&'a DpsRow)>{let rows=meter_rows(state);let regular=meter_regular_page_size(state,physical);let keep_self=state.features.read().map(|f|f.meter.always_show_self).unwrap_or(true);let local_index=keep_self.then(||rows.iter().position(|row|row.is_local)).flatten();sticky_rank_indices(rows.len(),local_index,state.scroll,regular,physical).into_iter().filter_map(|index|rows.get(index).map(|row|(index+1,*row))).collect()}",
        "fn meter_page_rows<'a>(state:&'a State,physical:usize)->Vec<(usize,&'a DpsRow)>{let rows=meter_rows(state);let regular=meter_regular_page_size(state,physical);let keep_self=state.features.read().map(|f|f.meter.always_show_self).unwrap_or(true);let local_index=keep_self.then(||rows.iter().position(|row|row.is_local)).flatten();sticky_rank_indices(rows.len(),local_index,state.scroll,regular,physical).into_iter().filter_map(|index|rows.get(index).map(|row|(index+1,*row))).collect()}\nfn forced_self_row(total:usize,rank_index:usize,scroll:usize,regular_page:usize)->bool{if total==0||regular_page==0||rank_index>=total{return false;}let page=regular_page.min(total).max(1);let start=scroll.min(total.saturating_sub(page));let end=(start+page).min(total);rank_index<start||rank_index>=end}\nunsafe fn paint_pinned_self_frame(hdc:HDC,r:&RECT){let white=rgb(255,255,255);fill(hdc,&RECT{left:r.left,top:r.top,right:r.right,bottom:r.top+1},white);fill(hdc,&RECT{left:r.left,top:r.bottom-1,right:r.right,bottom:r.bottom},white);fill(hdc,&RECT{left:r.left,top:r.top,right:r.left+1,bottom:r.bottom},white);fill(hdc,&RECT{left:r.right-1,top:r.top,right:r.right,bottom:r.bottom},white);}",
        "pinned self border helpers",
    );

    replace_once(
        &mut source,
        "let leader=rows.first().map(|row|mode_metric(row,state.sort_mode)).unwrap_or(0).max(1);let shown=meter_page_rows(state,visible);let bold_font=if state.font.is_null(){GetStockObject(DEFAULT_GUI_FONT)}else{state.font};",
        "let leader=rows.first().map(|row|mode_metric(row,state.sort_mode)).unwrap_or(0).max(1);let regular_page=meter_regular_page_size(state,visible);let shown=meter_page_rows(state,visible);let bold_font=if state.font.is_null(){GetStockObject(DEFAULT_GUI_FONT)}else{state.font};",
        "pinned self regular slice",
    );

    replace_once(
        &mut source,
        "fill(hdc,&RECT{left:r.left,top:bar_top,right:r.left+filled,bottom:bar_bottom},color);}}paint_scrollbar(hdc,rc,rows.len(),meter_regular_page_size(state,visible),state.scroll,top);}",
        "fill(hdc,&RECT{left:r.left,top:bar_top,right:r.left+filled,bottom:bar_bottom},color);}if row.is_local&&forced_self_row(rows.len(),rank.saturating_sub(1),state.scroll,regular_page){paint_pinned_self_frame(hdc,&r);}}paint_scrollbar(hdc,rc,rows.len(),regular_page,state.scroll,top);}",
        "paint forced self frame",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1174_overlay_tests {
    use super::*;

    #[test]
    fn mouse_wheel_moves_one_row_per_notch() {
        assert_eq!(wheel_row_steps(120), 1);
        assert_eq!(wheel_row_steps(-120), 1);
        assert_eq!(wheel_row_steps(240), 2);
        assert_eq!(wheel_row_steps(-240), 2);
        assert_eq!(wheel_row_steps(60), 1);
        assert_eq!(wheel_row_steps(0), 0);
    }

    #[test]
    fn white_frame_only_marks_forced_self_row() {
        assert!(forced_self_row(14, 13, 0, 5));
        assert!(forced_self_row(14, 13, 0, 4));
        assert!(!forced_self_row(14, 2, 0, 5));
        assert!(!forced_self_row(14, 13, 9, 5));
        assert!(!forced_self_row(4, 3, 0, 4));
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
