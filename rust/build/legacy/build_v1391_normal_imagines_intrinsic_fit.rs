// The normal meter used to hide both Imagines while class/name columns still
// had unused room: a second arbitrary 48px gate was added to their real bounds.
// Budget for existing badges before OPTIONAL share/death and reserve the exact
// measured class+ability text rather than an unrelated hard-coded cutoff.
use std::{env, fs, path::PathBuf};
mod previous { include!("build_v1390_resize_compact_reuse.rs"); pub fn run() { main(); } }
fn replace_once(part: &mut String, old: &str, new: &str, label: &str) {
    assert_eq!(part.matches(old).count(), 1, "normal Imagine fit: {label} expected one anchor");
    *part = part.replacen(old, new, 1);
}
fn section(src: &mut String, start: &str, end: &str, edits: &[(&str, &str, &str)]) {
    assert_eq!(src.matches(start).count(), 1, "normal Imagine fit: ambiguous {start}");
    let from = src.find(start).unwrap();
    let to = from + src[from..].find(end).expect("normal Imagine fit: section boundary");
    let mut part = src[from..to].to_owned();
    for (old, new, label) in edits { replace_once(&mut part, old, new, label); }
    src.replace_range(from..to, &part);
}
fn main() {
    previous::run();
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let file = out.join("feature_overlays_v170_fixed.rs");
    let mut src = fs::read_to_string(&file).expect("native meter");
    section(&mut src, "unsafe fn dps_row_layout_responsive(", "unsafe fn paint_dps_secondary_colored(", &[
        ("    let class_preferred = measured_class.max(dps_adaptive_logical_px(178,scale)).min(dps_adaptive_logical_px(290,scale));",
         r#"    let class_preferred = measured_class.max(dps_adaptive_logical_px(178,scale)).min(dps_adaptive_logical_px(290,scale));
    // Keep specialization and ability score before reserving two Imagine icons.
    // Strength (+number) is a lower-priority metadata tier at very small widths.
    let spec = dps_spec(_row);
    let scored = if _row.ability_score > 0 {
        if spec.is_empty() { score(_row.ability_score) }
        else { format!("{} · {}", spec, score(_row.ability_score)) }
    } else { spec };
    let old_secondary = SelectObject(hdc, _secondary_font);
    let score_min = dps_text_width(hdc, &scored, identity_text_px(&scored, false)) + 10;
    SelectObject(hdc, old_secondary);
    let class_badge_min = score_min.max(class_min).min(class_preferred);"#,
         "measured specialization and score minimum"),
        ("    // Optional columns only consume true surplus after name, class and rates.\n    let mut extra = available - active_span - total_span - name_min - class_preferred - gap;",
         r#"    // Badges outrank optional share/death. Reserve their *actual* widths if
    // the name, specialization+score and badges can all fit together.
    let requested_badges = if show_imagines { _row.imagines.len().min(2) as i32 } else { 0 };
    let badge_w = dps_badge_w(scale);
    let badge_gap = dps_badge_gap(scale);
    let badge_outer_gap = gap + 2;
    let requested_span = if requested_badges > 0 {
        requested_badges * badge_w + (requested_badges - 1) * badge_gap + badge_outer_gap
    } else { 0 };
    let badge_priority = if requested_badges > 0 &&
        available - active_span - total_span >= name_min + gap + class_badge_min + requested_span {
        requested_span
    } else { 0 };
    // No optional column may steal width already promised to visible Imagines.
    let mut extra = available - active_span - total_span - name_min - class_preferred - gap - badge_priority;"#,
         "optional metrics must budget visible badges"),
        ("    let badge_w = dps_badge_w(scale);\n    let badge_gap = dps_badge_gap(scale);\n    let badge_outer_gap = gap + 2;\n    let spare_after_class = identity_w - name_min\n        - if has_class { class_preferred + gap } else { 0 };\n    let badge_count = if show_imagines && has_class && spare_after_class >=\n        2 * badge_w + badge_gap + badge_outer_gap + dps_adaptive_logical_px(48, scale) { 2 }\n        else if show_imagines && _row.imagines.len()==1 && has_class && spare_after_class >= badge_w + badge_outer_gap + dps_adaptive_logical_px(48, scale) { 1 }\n        else { 0 };",
         r#"    // Use an intrinsic fit, not a 48px cliff. A two-badge loadout is atomic:
    // never show one of two equipped Imagines as though it were the only one.
    let pair_span = 2 * badge_w + badge_gap + badge_outer_gap;
    let single_span = badge_w + badge_outer_gap;
    let badge_count = if show_imagines && has_class && _row.imagines.len() >= 2 &&
        identity_w >= name_min + gap + class_badge_min + pair_span { 2 }
    else if show_imagines && has_class && _row.imagines.len() == 1 &&
        identity_w >= name_min + gap + class_badge_min + single_span { 1 }
    else { 0 };"#,
         "remove arbitrary 48px badge cliff"),
    ]);
    section(&mut src, "unsafe fn paint_reference_headers(", "unsafe fn paint_dps(", &[
        ("    let layout = dps_row_layout_responsive(\n        hdc,\n        content,",
         r#"    // Use an actual visible loadout instead of an empty DpsRow: header
    // visibility and geometry must follow the same Imagine fit as the rows.
    let header_row = meter_rows(state).into_iter().filter(|row| !row.is_dead)
        .max_by_key(|row| row.imagines.len().min(2)).cloned().unwrap_or_default();
    let layout = dps_row_layout_responsive(
        hdc,
        content,"#,
         "headers derive visibility from actual loadout"),
        ("        &DpsRow::default(),\n        dps_primary_font(state),",
         "        &header_row,\n        dps_primary_font(state),",
         "headers use visible metadata and Imagine count"),
    ]);
    src.push_str(r#"
#[cfg(all(test, windows))] mod normal_imagine_intrinsic_fit_tests {
    use super::*;
    #[test] fn a_tiny_resize_does_not_drop_two_imagines_while_class_has_spare_width() { unsafe {
        let hdc = GetDC(std::ptr::null_mut()); assert!(!hdc.is_null());
        let row = DpsRow {
            name: "MrHard".into(), subprofession_name: "Smite".into(),
            ability_score: 59_000, illusion_break: 3210,
            imagines: vec![ImagineBadge::default(),ImagineBadge::default()],
            ..DpsRow::default()
        };
        let font = dps_cached_font(100,true);
        let secondary = dps_cached_font(100,false);
        for client in [649,650,659,660] {
            let r = normal_meter_row_layout_rect(RECT{left:6,top:0,right:client-8,bottom:40},100,true);
            let l = dps_row_layout_responsive(hdc,r,100,true,true,false,false,&row,font,secondary);
            assert_eq!(l.badge_count,2,"{client}px still has room for both Imagine icons");
            assert!(l.name_right + 6 <= l.spec_left,"{client}px name and class must not overlap");
            assert!(l.spec_right + 6 <= l.badge_left,"{client}px metadata and badge must not overlap");
            assert!(l.badge_left+2*dps_badge_w(100)+dps_badge_gap(100)<=l.total_left,
                "{client}px badges and Total must not overlap");
            let previous = SelectObject(hdc,secondary);
            let text = dps_secondary_compact_measured(hdc,&row,l.spec_right-l.spec_left);
            assert!(text.contains("59k"),"{client}px must retain ability score: {text}");
            SelectObject(hdc,previous);
        }
        ReleaseDC(std::ptr::null_mut(),hdc);
    }}
    #[test] fn badge_fit_is_monotone_across_resize_directions_and_scales() { unsafe {
        let hdc=GetDC(std::ptr::null_mut()); assert!(!hdc.is_null());
        let row=DpsRow{name:"MrHard".into(),subprofession_name:"Smite".into(),ability_score:59000,
            imagines:vec![ImagineBadge::default(),ImagineBadge::default()],..DpsRow::default()};
        for scale in [60,100] {
            let mut prior=0;
            for client in 450..=950 {
                let r=normal_meter_row_layout_rect(RECT{left:6,top:0,right:client-8,bottom:40},scale,true);
                let l=dps_row_layout_responsive(hdc,r,scale,true,true,false,false,&row,
                    dps_cached_font(scale,true),dps_cached_font(scale,false));
                assert!(l.badge_count>=prior,"{scale}%: badges disappeared while widening at {client}px");
                prior=l.badge_count;
            }
            for client in (450..=950).rev() {
                let r=normal_meter_row_layout_rect(RECT{left:6,top:0,right:client-8,bottom:40},scale,true);
                let l=dps_row_layout_responsive(hdc,r,scale,true,true,false,false,&row,
                    dps_cached_font(scale,true),dps_cached_font(scale,false));
                let again=dps_row_layout_responsive(hdc,r,scale,true,true,false,false,&row,
                    dps_cached_font(scale,true),dps_cached_font(scale,false));
                assert_eq!(l.badge_count,again.badge_count,"{scale}% open and shrinking disagree at {client}px");
            }
        }
        ReleaseDC(std::ptr::null_mut(),hdc);
    }}
}
"#);
    fs::write(&file,src).expect("write intrinsic normal Imagine fit");
    let qa = out.join("ui_meter_qa_tests_raid_v1345.rs");
    let mut qa_source=fs::read_to_string(&qa).expect("meter native render QA");
    qa_source.push_str(r#"
#[cfg(windows)]
#[test]
fn normal_imagines_capture_near_the_reported_resize_breakpoint() {
    unsafe {
        let output=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/ui-qa-render");
        std::fs::create_dir_all(&output).unwrap();
        for scale in [60,100] {
            for width in [640,649,650,659,660,680] {
                render_meter_case(&output,scale,false,false,width);
            }
        }
    }
}
"#);
    fs::write(qa,qa_source).expect("write near-breakpoint native renders");
    println!("cargo:rerun-if-changed=build/legacy/build_v1391_normal_imagines_intrinsic_fit.rs");
}
