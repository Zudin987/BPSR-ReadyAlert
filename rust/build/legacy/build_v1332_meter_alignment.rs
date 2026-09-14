use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1331_normal_meter_consumables.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.33.2 {label} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn main() {
    previous::run();

    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .expect("read generated feature overlay for v1.33.2 meter alignment")
        .replace("\r\n", "\n");

    replace_once(
        &mut source,
        r#"fn normal_meter_fs_width(scale:i32)->i32{dps_adaptive_logical_px(38,scale).max(30)}
fn normal_meter_content_rect(r:RECT,scale:i32)->RECT{let reserve=normal_meter_fs_width(scale);RECT{right:(r.right-reserve).max(r.left+1),..r}}
unsafe fn paint_normal_meter_fs(hdc:HDC,state:&State,row:&DpsRow,r:RECT){
    let reserve=normal_meter_fs_width(dps_layout_scale(state));let x=(r.right-reserve).max(r.left);
    paint_raid_fs(hdc,state,Some(&row),x,r.top,(r.right-x).max(1),(r.bottom-r.top).max(1));
}"#,
        r#"fn normal_meter_fs_width(scale:i32)->i32{dps_adaptive_logical_px(38,scale).max(30)}
fn normal_meter_content_rect(r:RECT,scale:i32)->RECT{let reserve=normal_meter_fs_width(scale);RECT{right:(r.right-reserve).max(r.left+1),..r}}
fn normal_meter_fs_rect(r:RECT,scale:i32)->RECT{let reserve=normal_meter_fs_width(scale);let x=(r.right-reserve).max(r.left);let content_bottom=(r.bottom-4).max(r.top+1);RECT{left:x,top:r.top,right:r.right,bottom:content_bottom}}
unsafe fn paint_normal_meter_fs(hdc:HDC,state:&State,row:&DpsRow,r:RECT){
    // Match the same vertical content box used by the player name/metrics. The
    // final four pixels are reserved for the row's bottom damage bar, so using
    // the full row height makes F/S look visibly lower than the other columns.
    let fs=normal_meter_fs_rect(r,dps_layout_scale(state));
    paint_raid_fs(hdc,state,Some(&row),fs.left,fs.top,(fs.right-fs.left).max(1),(fs.bottom-fs.top).max(1));
}"#,
        "normal meter F/S vertical centering",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1332_meter_alignment_tests {
    use super::*;

    #[test]
    fn normal_meter_fs_uses_same_vertical_content_box_as_row_text() {
        for scale in [50, 75, 100, 125, 150, 200] {
            let row = RECT { left: 6, top: 117, right: 620, bottom: 148 };
            let fs = normal_meter_fs_rect(row, scale);
            assert_eq!(fs.top, row.top);
            assert_eq!(fs.bottom, row.bottom - 4);
            assert_eq!(fs.right, row.right);
            assert_eq!(row.right - fs.left, normal_meter_fs_width(scale));
        }
    }
}
"#);

    fs::write(path, source).expect("write v1.33.2 meter alignment overlay");
    println!("cargo:rerun-if-changed=build/legacy/build_v1332_meter_alignment.rs");
}
