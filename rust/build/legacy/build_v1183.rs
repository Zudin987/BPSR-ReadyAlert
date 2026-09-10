use std::{env, fs, path::{Path, PathBuf}};

mod prior {
    include!("build_v1182.rs");
    pub fn run() { main(); }
}

fn replace_once(source:&mut String,from:&str,to:&str,label:&str){
    let count=source.matches(from).count();
    assert_eq!(count,1,"v1.18.3 patch {label} expected one match, found {count}");
    *source=source.replacen(from,to,1);
}

fn patch_feature_overlay(out:&Path){
    let path=out.join("feature_overlays_v170_fixed.rs");
    let mut source=fs::read_to_string(&path).expect("read v1.18.2 generated overlays").replace("\r\n","\n");

    replace_once(
        &mut source,
        "fn sticky_rank_indices(total:usize,local_index:Option<usize>,scroll:usize,regular_page:usize,physical:usize)->Vec<usize>{if total==0||physical==0||regular_page==0{return Vec::new();}let page=regular_page.min(total).max(1);let start=scroll.min(total.saturating_sub(page));let end=(start+page).min(total);let mut indices:Vec<usize>=(start..end).collect();if let Some(local)=local_index.filter(|index|*index<total){if !indices.contains(&local){if indices.len()<physical{indices.push(local);}else if let Some(last)=indices.last_mut(){*last=local;}}}indices}",
        "fn sticky_rank_indices(total:usize,local_index:Option<usize>,scroll:usize,regular_page:usize,physical:usize)->Vec<usize>{if total==0||physical==0||regular_page==0{return Vec::new();}let page=regular_page.min(total).max(1);let start=scroll.min(total.saturating_sub(page));let end=(start+page).min(total);let mut indices:Vec<usize>=(start..end).collect();if let Some(local)=local_index.filter(|index|*index<total){if local>=end{if indices.len()<physical{indices.push(local);}else if let Some(last)=indices.last_mut(){*last=local;}}}indices}",
        "pin self only below visible ranks",
    );

    replace_once(
        &mut source,
        "fn forced_self_row(total:usize,rank_index:usize,scroll:usize,regular_page:usize)->bool{if total==0||regular_page==0||rank_index>=total{return false;}let page=regular_page.min(total).max(1);let start=scroll.min(total.saturating_sub(page));let end=(start+page).min(total);rank_index<start||rank_index>=end}",
        "fn forced_self_row(total:usize,rank_index:usize,scroll:usize,regular_page:usize)->bool{if total==0||regular_page==0||rank_index>=total{return false;}let page=regular_page.min(total).max(1);let start=scroll.min(total.saturating_sub(page));let end=(start+page).min(total);rank_index>=end}",
        "forced self border only below visible ranks",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1183_sticky_self_tests {
    use super::*;

    #[test]
    fn self_below_visible_range_is_force_pinned() {
        // Visible ranks 1-5, self rank 14: replace the final visible slot.
        assert_eq!(sticky_rank_indices(20, Some(13), 0, 5, 5), vec![0, 1, 2, 3, 13]);
        assert!(forced_self_row(20, 13, 0, 5));

        // Visible ranks 10-14, self rank 20: force self at the bottom.
        assert_eq!(sticky_rank_indices(25, Some(19), 9, 5, 5), vec![9, 10, 11, 12, 19]);
        assert!(forced_self_row(25, 19, 9, 5));
    }

    #[test]
    fn self_above_visible_range_is_not_force_pinned() {
        // Visible ranks 6-10, self rank 1: scrolling past self hides it.
        assert_eq!(sticky_rank_indices(20, Some(0), 5, 5, 5), vec![5, 6, 7, 8, 9]);
        assert!(!forced_self_row(20, 0, 5, 5));

        // Visible ranks 10-14, self rank 5: do not pull self back into view.
        assert_eq!(sticky_rank_indices(25, Some(4), 9, 5, 5), vec![9, 10, 11, 12, 13]);
        assert!(!forced_self_row(25, 4, 9, 5));
    }

    #[test]
    fn self_inside_visible_range_is_natural_and_not_duplicated() {
        assert_eq!(sticky_rank_indices(20, Some(7), 5, 5, 5), vec![5, 6, 7, 8, 9]);
        assert!(!forced_self_row(20, 7, 5, 5));
    }

    #[test]
    fn spare_physical_row_keeps_full_visible_range_before_pinned_self() {
        assert_eq!(sticky_rank_indices(20, Some(13), 0, 5, 6), vec![0, 1, 2, 3, 4, 13]);
    }
}
"#);

    fs::write(path,source).expect("write v1.18.3 feature overlay");
}

fn main(){
    prior::run();
    let out=PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_feature_overlay(&out);
    println!("cargo:rerun-if-changed=build/legacy/build_v1183.rs");
}
