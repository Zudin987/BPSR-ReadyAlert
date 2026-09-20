// Shared DPS class-row treatment. Keep the original specialization colour as
// the sole hue source; never assign new class colours or touch telemetry.
// COLORREF is 0x00BBGGRR. Blend per channel in sRGB so the rendered surface
// visibly communicates class identity while staying a dark overlay.
const REFERENCE_ROW_TINT_PERCENT: u32 = 18;
const REFERENCE_ROW_BASE: (u32, u32, u32) = (23, 27, 34);

fn reference_meter_row_background(row: &DpsRow) -> u32 {
    let accent = spec_color(row);
    let blend = |base: u32, shift: u32| -> u8 {
        let class = (accent >> shift) & 255;
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
    fn card_tint_is_visible_but_dark_for_every_known_specialization() {
        for specialization in ["Moonstrike", "Wildpack", "Vanguard", "Falconry", "Smite", "Unknown"] {
            let row = DpsRow {
                subprofession_name: specialization.into(),
                ..DpsRow::default()
            };
            let color = reference_meter_row_background(&row);
            let accent = spec_color(&row);
            for (base, shift) in [(23, 0), (27, 8), (34, 16)] {
                let expected = (base * 82 + channel(accent, shift) * 18 + 50) / 100;
                assert_eq!(channel(color, shift), expected, "{specialization}: tint channel differs from contract");
                assert!(channel(color, shift) <= 74, "{specialization}: class card is too bright");
            }
            assert_eq!(text_on(color), rgb(248, 250, 252));
        }
    }

    #[test]
    fn classes_keep_distinct_identity_without_a_global_pink_fill() {
        let color = |name: &str| reference_meter_row_background(&DpsRow {
            subprofession_name: name.into(), ..DpsRow::default()
        });
        assert_ne!(color("Moonstrike"), color("Wildpack"));
        assert_ne!(color("Vanguard"), color("Falconry"));
        assert_ne!(color("Smite"), color("Moonstrike"));
    }
}
