// Screenshot-inspired DPS-meter surface treatment. The existing class hue is
// still the source of truth; we lower its luminance rather than assigning a new
// color to a specialization. This preserves class recognition while keeping the
// player names and numbers readable on dark translucent overlays.
//
// This file is appended to the generated feature overlay by the final build
// stage. It intentionally changes rendering only, never telemetry or sorting.
fn reference_meter_row_background(row: &DpsRow) -> u32 {
    let original = spec_color(row);
    // Win32 COLORREF packs RGB as 0x00BBGGRR.
    let red = (original & 0xff) as u16;
    let green = ((original >> 8) & 0xff) as u16;
    let blue = ((original >> 16) & 0xff) as u16;
    rgb(
        (23 + red * 9 / 100) as u8,
        (24 + green * 9 / 100) as u8,
        (29 + blue * 9 / 100) as u8,
    )
}

#[cfg(test)]
mod reference_meter_theme_tests {
    use super::*;

    #[test]
    fn cards_keep_a_dark_readable_surface_across_classes() {
        for specialization in ["Moonstrike", "Wildpack", "Vanguard", "Falconry", "Smite", "Unknown"] {
            let row = DpsRow {
                subprofession_name: specialization.into(),
                ..DpsRow::default()
            };
            let color = reference_meter_row_background(&row);
            for component in [color & 0xff, (color >> 8) & 0xff, (color >> 16) & 0xff] {
                assert!((23..=52).contains(&component), "{specialization} produced an overly bright card channel: {component}");
            }
            assert_eq!(text_on(color), rgb(248, 250, 252));
        }
    }

    #[test]
    fn class_hues_remain_distinguishable() {
        let color = |name: &str| {
            reference_meter_row_background(&DpsRow {
                subprofession_name: name.into(),
                ..DpsRow::default()
            })
        };
        assert_ne!(color("Moonstrike"), color("Wildpack"));
        assert_ne!(color("Vanguard"), color("Falconry"));
    }
}
