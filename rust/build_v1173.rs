use std::{env, fs, path::{Path, PathBuf}};

mod prior {
    include!("build_v1172.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.17.3 patch {label} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn patch_overlay(out: &Path) {
    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("read v1.17.2 overlays").replace("\r\n", "\n");

    replace_once(
        &mut source,
        "fn sticky_rank_indices(total:usize,local_index:Option<usize>,scroll:usize,regular_page:usize,physical:usize)->Vec<usize>{if total==0||physical==0||regular_page==0{return Vec::new();}let page=regular_page.min(total).max(1);let start=scroll.min(total.saturating_sub(page));let end=(start+page).min(total);let mut indices:Vec<usize>=(start..end).collect();if indices.len()<physical{if let Some(local)=local_index.filter(|index|*index<total){if !indices.contains(&local){indices.push(local);}}}indices}",
        "fn sticky_rank_indices(total:usize,local_index:Option<usize>,scroll:usize,regular_page:usize,physical:usize)->Vec<usize>{if total==0||physical==0||regular_page==0{return Vec::new();}let page=regular_page.min(total).max(1);let start=scroll.min(total.saturating_sub(page));let end=(start+page).min(total);let mut indices:Vec<usize>=(start..end).collect();if let Some(local)=local_index.filter(|index|*index<total){if !indices.contains(&local){if indices.len()<physical{indices.push(local);}else if let Some(last)=indices.last_mut(){*last=local;}}}indices}",
        "always-show-self short window fallback",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1173_overlay_tests {
    use super::*;

    #[test]
    fn sticky_self_replaces_last_slot_when_no_spare_row_exists() {
        assert_eq!(sticky_rank_indices(14, Some(13), 0, 4, 4), vec![0, 1, 2, 13]);
        assert_eq!(sticky_rank_indices(10, Some(5), 0, 4, 4), vec![0, 1, 2, 5]);
    }

    #[test]
    fn sticky_self_still_uses_extra_row_when_space_exists() {
        assert_eq!(sticky_rank_indices(14, Some(13), 0, 5, 6), vec![0, 1, 2, 3, 4, 13]);
    }

    #[test]
    fn sticky_self_is_not_duplicated_when_actual_rank_is_visible() {
        assert_eq!(sticky_rank_indices(14, Some(13), 9, 5, 5), vec![9, 10, 11, 12, 13]);
        assert_eq!(sticky_rank_indices(14, Some(1), 0, 4, 4), vec![0, 1, 2, 3]);
    }
}
"#);

    fs::write(path, source).expect("write v1.17.3 overlays");
}

fn main() {
    prior::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    patch_overlay(&out);
    println!("cargo:rerun-if-changed=build_v1173.rs");
}
