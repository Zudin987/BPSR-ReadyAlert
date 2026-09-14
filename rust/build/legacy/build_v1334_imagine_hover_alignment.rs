use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1333_consumable_controls.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.33.4 {label} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn main() {
    previous::run();

    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .expect("read generated feature overlay for v1.33.4 Imagine hover alignment")
        .replace("\r\n", "\n");

    // Keep the normal-meter content rectangle used for Imagine layout in one
    // place. v1.33.3 correctly moved the painted badges left when the F/S
    // reservation is visible, but hover_badge_at still measured against the
    // old full-width row, shifting both invisible hitboxes to the right.
    replace_once(
        &mut source,
        r#"fn normal_meter_content_rect(r:RECT,scale:i32)->RECT{let reserve=normal_meter_fs_width(scale);RECT{right:(r.right-reserve).max(r.left+1),..r}}"#,
        r#"fn normal_meter_content_rect(r:RECT,scale:i32)->RECT{let reserve=normal_meter_fs_width(scale);RECT{right:(r.right-reserve).max(r.left+1),..r}}
fn normal_meter_row_layout_rect(r:RECT,scale:i32,show_consumables:bool)->RECT{if show_consumables{normal_meter_content_rect(r,scale)}else{r}}"#,
        "shared normal-meter row layout rectangle",
    );

    replace_once(
        &mut source,
        r#"let row_layout_rect=if raid_active(state)||!settings.meter.show_consumables{r}else{normal_meter_content_rect(r,scale)};let layout=dps_row_layout_responsive(hdc,row_layout_rect,scale,settings.meter.show_imagines,settings.meter.show_active_rates,show_share,settings.meter.show_deaths,row,primary_font,secondary_font);"#,
        r#"let row_layout_rect=if raid_active(state){r}else{normal_meter_row_layout_rect(r,scale,settings.meter.show_consumables)};let layout=dps_row_layout_responsive(hdc,row_layout_rect,scale,settings.meter.show_imagines,settings.meter.show_active_rates,show_share,settings.meter.show_deaths,row,primary_font,secondary_font);"#,
        "painted Imagine row layout",
    );

    replace_once(
        &mut source,
        r#"let hdc=GetDC(hwnd);if hdc.is_null(){return None;}configure_overlay_dc(hdc,state.scale_percent);let layout=dps_row_layout_responsive(hdc,r,scale,true,settings.meter.show_active_rates,show_share,settings.meter.show_deaths,row,name_font,secondary_font);reset_overlay_dc(hdc);"#,
        r#"let hdc=GetDC(hwnd);if hdc.is_null(){return None;}configure_overlay_dc(hdc,state.scale_percent);let hover_layout_rect=normal_meter_row_layout_rect(r,scale,settings.meter.show_consumables);let layout=dps_row_layout_responsive(hdc,hover_layout_rect,scale,true,settings.meter.show_active_rates,show_share,settings.meter.show_deaths,row,name_font,secondary_font);reset_overlay_dc(hdc);"#,
        "Imagine hover row layout",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1334_imagine_hover_alignment_tests {
    use super::*;

    #[test]
    fn imagine_hover_and_paint_share_the_same_normal_meter_layout_rect() {
        let row=RECT{left:6,top:117,right:620,bottom:148};
        for scale in [50,75,100,125,150,200] {
            let with_consumables=normal_meter_row_layout_rect(row,scale,true);
            let without_consumables=normal_meter_row_layout_rect(row,scale,false);
            assert_eq!(without_consumables.left,row.left);
            assert_eq!(without_consumables.right,row.right);
            assert_eq!(with_consumables.left,row.left);
            assert_eq!(row.right-with_consumables.right,normal_meter_fs_width(scale));
            assert_eq!(with_consumables.right,normal_meter_content_rect(row,scale).right);
        }
    }
}
"#);

    fs::write(path, source).expect("write v1.33.4 Imagine hover alignment overlay");
    println!("cargo:rerun-if-changed=build/legacy/build_v1334_imagine_hover_alignment.rs");
}
