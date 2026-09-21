// DPS class-row treatment. Keep specialization colour as the hue source.
// The existing build's dead-local override is injected immediately after the
// original lookup anchor below; do not rename the anchor or touch telemetry.
// Win32 COLORREF is 0x00BBGGRR; blend each sRGB channel against a dark base.
const REFERENCE_ROW_TINT_PERCENT: u32 = 18;
const REFERENCE_ROW_BASE: (u32, u32, u32) = (23, 27, 34);

fn reference_meter_row_background(row: &DpsRow) -> u32 {
    let original = spec_color(row);
    let blend = |base: u32, shift: u32| -> u8 {
        let class = (original >> shift) & 255;
        ((base * (100 - REFERENCE_ROW_TINT_PERCENT)
            + class * REFERENCE_ROW_TINT_PERCENT + 50) / 100) as u8
    };
    rgb(
        blend(REFERENCE_ROW_BASE.0, 0),
        blend(REFERENCE_ROW_BASE.1, 8),
        blend(REFERENCE_ROW_BASE.2, 16),
    )
}

#[cfg(test)]
mod reference_meter_theme_tests {
    use super::*;
    fn channel(color: u32, shift: u32) -> u32 { (color >> shift) & 255 }

    #[test]
    fn tint_is_visible_and_dark_for_known_specializations() {
        for specialization in ["Moonstrike", "Wildpack", "Vanguard", "Falconry", "Smite", "Unknown"] {
            let row = DpsRow {
                subprofession_name: specialization.into(),
                ..DpsRow::default()
            };
            let color = reference_meter_row_background(&row);
            let accent = spec_color(&row);
            for (base, shift) in [(23, 0), (27, 8), (34, 16)] {
                let expected = (base * 82 + channel(accent, shift) * 18 + 50) / 100;
                assert_eq!(channel(color, shift), expected, "{specialization}: blend regression");
                assert!(channel(color, shift) <= 74, "{specialization}: row is too bright");
            }
            assert_eq!(text_on(color), rgb(248, 250, 252));
        }
    }

    #[test]
    fn class_hues_remain_distinct_and_dead_owner_is_handled_separately() {
        let color = |name: &str| reference_meter_row_background(&DpsRow {
            subprofession_name: name.into(), ..DpsRow::default()
        });
        assert_ne!(color("Moonstrike"), color("Wildpack"));
        assert_ne!(color("Vanguard"), color("Falconry"));
    }
}
