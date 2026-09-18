use std::{env, fs, path::PathBuf};

mod previous {
    include!("build_v1361_hit_test_type_fix.rs");
    pub fn run() { main(); }
}

fn replace_once(source: &mut String, old: &str, new: &str, label: &str) {
    let matches = source.matches(old).count();
    assert_eq!(matches, 1, "enrage header {label}: expected one anchor, found {matches}");
    *source = source.replacen(old, new, 1);
}

fn main() {
    previous::run();
    let path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"))
        .join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("generated overlay source");

    // The telemetry module already tracks the exact Power Seal/Enrage buff on
    // the selected monster and exposes its remaining time in the DPS snapshot.
    // Only redraw the active meter when that field exists; repaint at 500 ms
    // both advances the countdown and alternates the last-30-second label.
    replace_once(&mut source,
        "const DPS_CONSUMABLE_TIMER_ID:usize=0x1212;",
        "const DPS_CONSUMABLE_TIMER_ID:usize=0x1212;\nconst DPS_ENRAGE_TIMER_ID:usize=0x1215;",
        "timer identifier");
    replace_once(&mut source,
        "if kind==Kind::Dps{let popup=create_consumable_popup(instance,hwnd);",
        "if kind==Kind::Dps{SetTimer(hwnd,DPS_ENRAGE_TIMER_ID,500,None);let popup=create_consumable_popup(instance,hwnd);",
        "start meter repaint timer");
    replace_once(&mut source,
        "WM_TIMER=>{if !ptr.is_null()&&(*ptr).kind==Kind::Mechanics&&wparam==MECH_CONSUMABLE_TIMER_ID{",
        "WM_TIMER=>{if !ptr.is_null()&&(*ptr).kind==Kind::Dps&&wparam==DPS_ENRAGE_TIMER_ID{let state=&*ptr;if !state.collapsed&&state.history_index.is_none()&&state.dps.target.as_ref().and_then(|target|target.enrage_remaining_ms).is_some(){InvalidateRect(hwnd,null(),0);}}if !ptr.is_null()&&(*ptr).kind==Kind::Mechanics&&wparam==MECH_CONSUMABLE_TIMER_ID{",
        "active enrage repaint");
    replace_once(&mut source,
        "WM_NCDESTROY=>{KillTimer(hwnd,MECH_CONSUMABLE_TIMER_ID);",
        "WM_NCDESTROY=>{KillTimer(hwnd,DPS_ENRAGE_TIMER_ID);KillTimer(hwnd,MECH_CONSUMABLE_TIMER_ID);",
        "timer cleanup");

    // Replace the header's redundant aggregate Total Damage / Total Healing /
    // Total Taken segment, not the per-player totals or meter calculations.
    // Preserve target name, HP, encounter time, spacing and color roles. The
    // Enrage slot occupies no width until a live target has a detected timer.
    let start_anchor = "unsafe fn paint_reference_encounter(";
    let end_anchor = "unsafe fn paint_mode_tab(";
    assert_eq!(source.matches(start_anchor).count(), 1, "one encounter painter");
    assert_eq!(source.matches(end_anchor).count(), 1, "one mode-tab painter");
    let start = source.find(start_anchor).expect("encounter painter");
    let end = source[start..].find(end_anchor).expect("mode-tab painter") + start;
    assert!(source[start..end].contains("Total Damage") == false || source[start..end].contains("reference_meter_total_label"), "unexpected encounter painter");
    source.replace_range(start..end, r#"unsafe fn paint_reference_encounter(hdc: HDC, rc: RECT, state: &State, settings: &FeatureSettings) {
    let snapshot = view_snapshot(state);
    let top = TOOLBAR_H;
    let bottom = reference_tabs_top();
    let now = now_ms();
    let remaining = live_enrage_remaining_ms(state, snapshot, now);
    let enrage = meter_enrage_header_label(remaining).unwrap_or_default();
    let hp = if settings.meter.show_target {
        format!("HP {}", target_hp_value(snapshot))
    } else {
        String::new()
    };
    let time = format_time(snapshot.encounter_ms);
    let title_min = if rc.right < 320 { 44 } else { 75 };
    let enrage_reserve = if enrage.is_empty() { 0 }
        else { dps_text_width(hdc, &enrage, identity_text_px(&enrage, true)) + 20 };
    let mut cursor = rc.right - 8;
    // Read from left to right as target | ENRAGE | encounter time | HP.
    // Reserve space for Enrage before optional HP/time, including its blink-off
    // frames, so the other fields never jump as the countdown flashes.
    for (text, color, is_enrage) in [
        (&hp, rgb(207, 91, 109), false),
        (&time, rgb(241, 243, 247), false),
        (&enrage, rgb(237, 121, 91), true),
    ] {
        if text.is_empty() { continue; }
        let width = dps_text_width(hdc, text, identity_text_px(text, true)) + 4;
        let reserve = if is_enrage { 0 } else { enrage_reserve };
        if !is_enrage && cursor - width - 16 - reserve < title_min + 8 {
            continue;
        }
        let left = (cursor - width).max(8);
        if !is_enrage || remaining.is_some_and(|ms| enrage_blink_visible(ms, now)) {
            SetTextColor(hdc, color);
            draw(hdc, text, RECT { left, top, right: cursor, bottom },
                DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        }
        cursor = left - 8;
        fill(hdc, &RECT { left: cursor, top: top + 11, right: cursor + 1, bottom: bottom - 11 },
            rgb(57, 63, 74));
        cursor -= 8;
    }
    let title = if settings.meter.show_target { target_title(snapshot) }
        else { "Encounter".into() };
    SetTextColor(hdc, rgb(241, 243, 247));
    draw(hdc, &title, RECT { left: 8, top, right: cursor.max(8), bottom },
        DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS);
}
"#);

    // Match hover information to the header even when the target-name display
    // option is disabled: Enrage itself depends only on detected live telemetry.
    replace_once(&mut source,
        "        if settings.meter.show_target {\n            if let Some(ms) = live_enrage_remaining_ms(state, snap, now_ms()) {\n                text.push_str(&format!(\" • Enrage {}\", format_enrage_countdown(ms)));\n            }\n        }\n        return Some(text);",
        "        if let Some(ms) = live_enrage_remaining_ms(state, snap, now_ms()) {\n            text.push_str(&format!(\" • Enrage {}\", format_enrage_countdown(ms)));\n        }\n        return Some(text);",
        "header hover information");

    source.push_str(r#"
fn meter_enrage_header_label(remaining: Option<i64>) -> Option<String> {
    remaining.filter(|ms| *ms > 0).map(|ms| format!("ENRAGE {}", format_enrage_countdown(ms)))
}
#[cfg(test)]
mod enrage_header_regressions {
    use super::*;
    #[test]
    fn only_detected_positive_enrage_time_has_header_text() {
        assert_eq!(meter_enrage_header_label(None), None);
        assert_eq!(meter_enrage_header_label(Some(0)), None);
        assert_eq!(meter_enrage_header_label(Some(-1)), None);
        assert_eq!(meter_enrage_header_label(Some(30_000)).as_deref(), Some("ENRAGE 0:30"));
        assert_eq!(meter_enrage_header_label(Some(61_001)).as_deref(), Some("ENRAGE 1:02"));
    }
    #[test]
    fn blink_starts_at_thirty_seconds_and_not_before() {
        assert!(enrage_blink_visible(30_001, 500));
        assert!(enrage_blink_visible(30_000, 0));
        assert!(!enrage_blink_visible(30_000, 500));
        assert!(enrage_blink_visible(500, 1_000));
        assert!(!enrage_blink_visible(500, 1_500));
    }
}
"#);
    fs::write(path, source).expect("write restored Enrage header");
    println!("cargo:rerun-if-changed=build/legacy/build_v1362_enrage_header.rs");
}
