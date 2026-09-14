use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1322_regression_fix.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, from: &str, to: &str, label: &str) {
    let count = source.matches(from).count();
    assert_eq!(count, 1, "v1.33.1 {label} expected one match, found {count}");
    *source = source.replacen(from, to, 1);
}

fn main() {
    previous::run();

    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let path = out.join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path)
        .expect("read generated feature overlay for v1.33.1 normal-meter consumables")
        .replace("\r\n", "\n");

    replace_once(
        &mut source,
        r#"unsafe fn sync_consumable_popup(parent:HWND,state:&mut State){
    if state.kind==Kind::Dps&&(raid_active(state)||state.compact_mode){if !state.consumable_hwnd.is_null(){ShowWindow(state.consumable_hwnd,0);}return;}"#,
        r#"unsafe fn sync_consumable_popup(parent:HWND,state:&mut State){
    // v1.33.1: normal meter consumables are rendered inside each player row;
    // Raid Mode keeps its existing gutter indicators. The legacy floating popup
    // stays hidden so normal mode never shows duplicate F/S state outside the row.
    if state.kind==Kind::Dps{if !state.consumable_hwnd.is_null(){ShowWindow(state.consumable_hwnd,0);}return;}"#,
        "floating consumable popup suppression",
    );

    replace_once(
        &mut source,
        r#"    let half=(w/2).max(1);if food!=crate::ui_modern::BPSR_MUTED{SetTextColor(hdc,food);draw(hdc,"F",RECT{left:x,top:y,right:x+half,bottom:y+h},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}if serum!=crate::ui_modern::BPSR_MUTED{SetTextColor(hdc,serum);draw(hdc,"S",RECT{left:x+half,top:y,right:x+w,bottom:y+h},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}SelectObject(hdc,old);
}

unsafe fn paint_raid_player"#,
        r#"    let half=(w/2).max(1);if food!=crate::ui_modern::BPSR_MUTED{SetTextColor(hdc,food);draw(hdc,"F",RECT{left:x,top:y,right:x+half,bottom:y+h},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}if serum!=crate::ui_modern::BPSR_MUTED{SetTextColor(hdc,serum);draw(hdc,"S",RECT{left:x+half,top:y,right:x+w,bottom:y+h},DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_NOPREFIX);}SelectObject(hdc,old);
}

fn normal_meter_fs_width(scale:i32)->i32{dps_adaptive_logical_px(38,scale).max(30)}
fn normal_meter_content_rect(r:RECT,scale:i32)->RECT{let reserve=normal_meter_fs_width(scale);RECT{right:(r.right-reserve).max(r.left+1),..r}}
unsafe fn paint_normal_meter_fs(hdc:HDC,state:&State,row:&DpsRow,r:RECT){
    let reserve=normal_meter_fs_width(dps_layout_scale(state));let x=(r.right-reserve).max(r.left);
    paint_raid_fs(hdc,state,Some(&row),x,r.top,(r.right-x).max(1),(r.bottom-r.top).max(1));
}

unsafe fn paint_raid_player"#,
        "normal meter F/S renderer",
    );

    replace_once(
        &mut source,
        r#"let layout=dps_row_layout_responsive(hdc,hr,scale,false,settings.meter.show_active_rates,show_share,settings.meter.show_deaths,&dummy,primary_font,secondary_font);"#,
        r#"let header_layout_rect=if raid_active(state){hr}else{normal_meter_content_rect(hr,scale)};let layout=dps_row_layout_responsive(hdc,header_layout_rect,scale,false,settings.meter.show_active_rates,show_share,settings.meter.show_deaths,&dummy,primary_font,secondary_font);"#,
        "normal meter header reservation",
    );

    replace_once(
        &mut source,
        r#"let layout=dps_row_layout_responsive(hdc,r,scale,settings.meter.show_imagines,settings.meter.show_active_rates,show_share,settings.meter.show_deaths,row,primary_font,secondary_font);"#,
        r#"let row_layout_rect=if raid_active(state){r}else{normal_meter_content_rect(r,scale)};let layout=dps_row_layout_responsive(hdc,row_layout_rect,scale,settings.meter.show_imagines,settings.meter.show_active_rates,show_share,settings.meter.show_deaths,row,primary_font,secondary_font);if !raid_active(state){paint_normal_meter_fs(hdc,state,row,r);}"#,
        "normal meter row reservation and F/S paint",
    );

    source.push_str(r#"

#[cfg(test)]
mod v1331_normal_meter_consumable_tests {
    use super::*;

    #[test]
    fn normal_meter_reserves_a_stable_right_hand_fs_slot() {
        for scale in [50, 75, 100, 125, 150, 200] {
            let row = RECT { left: 6, top: 0, right: 620, bottom: 31 };
            let content = normal_meter_content_rect(row, scale);
            assert!(content.right < row.right);
            assert_eq!(row.right - content.right, normal_meter_fs_width(scale));
            assert!(normal_meter_fs_width(scale) >= 30);
        }
    }

    #[test]
    fn normal_meter_reuses_raid_food_and_serum_colors() {
        let now = 100_000;
        let food = ConsumableStatus { buff_id: 1, name: "Food".into(), expires_unix_ms: now + 180_001, duration_ms: 300_000 };
        let serum = ConsumableStatus { buff_id: 2, name: "Serum".into(), expires_unix_ms: now + 20_000, duration_ms: 300_000 };
        assert_eq!(raid_consumable_color(Some(&food), now), rgb(72,226,116));
        assert_eq!(raid_consumable_color(Some(&serum), now), crate::ui_modern::BPSR_DANGER);
        assert_eq!(raid_consumable_color(None, now), crate::ui_modern::BPSR_MUTED);
    }

    #[test]
    fn consumable_indicator_boundary_semantics_are_stable() {
        let color = |left: i64, now: i64| {
            let status = ConsumableStatus {
                buff_id: 3,
                name: "Boundary".into(),
                expires_unix_ms: now + left,
                duration_ms: 300_000,
            };
            raid_consumable_color(Some(&status), now)
        };
        let even_blink = 400_000;
        let odd_blink = even_blink + 400;

        assert_eq!(color(180_001, even_blink), rgb(72,226,116));
        assert_eq!(color(180_000, even_blink), rgb(245,190,55));
        assert_eq!(color(179_999, even_blink), rgb(245,190,55));
        assert_eq!(color(60_001, even_blink), rgb(245,190,55));
        assert_eq!(color(60_000, even_blink), crate::ui_modern::BPSR_DANGER);
        assert_eq!(color(59_999, even_blink), crate::ui_modern::BPSR_DANGER);
        assert_eq!(color(30_001, even_blink), crate::ui_modern::BPSR_DANGER);
        assert_eq!(color(30_000, even_blink), crate::ui_modern::BPSR_DANGER);
        assert_eq!(color(29_999, even_blink), crate::ui_modern::BPSR_DANGER);
        assert_eq!(color(30_000, odd_blink), rgb(108,34,34));
        assert_eq!(color(29_999, odd_blink), rgb(108,34,34));
    }
}
"#);

    fs::write(path, source).expect("write v1.33.1 normal-meter consumable overlay");
    println!("cargo:rerun-if-changed=build/legacy/build_v1331_normal_meter_consumables.rs");
}