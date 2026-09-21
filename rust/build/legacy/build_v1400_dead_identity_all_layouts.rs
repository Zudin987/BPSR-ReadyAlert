// All four native DPS layout routes must render every deceased player's rank
// and name in the same bright red, not just the local player. Keep background,
// class metadata, metrics, revive status, self pinning, and layout untouched.
use std::{env, fs, path::PathBuf};
mod previous { include!("build_v1399_short_options_34px_copy.rs"); pub fn run() { main(); } }

fn replace_once(source: &mut String, old: &str, new: &str, name: &str) {
    assert_eq!(source.matches(old).count(), 1, "v1400 {name}: source anchor changed");
    *source = source.replacen(old, new, 1);
}

fn main() {
    previous::run();
    let path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"))
        .join("feature_overlays_v170_fixed.rs");
    let mut source = fs::read_to_string(&path).expect("native meter generated source");

    // Rank coloring is shared by Normal, Compact, Raid and Compact Raid.
    replace_once(
        &mut source,
        "fn reference_owner_rank_color(row:&DpsRow)->u32{if row.is_local{rgb(92,200,255)}else{crate::ui_modern::BPSR_MUTED}}",
        "fn reference_meter_identity_color(row:&DpsRow, alive_color:u32)->u32{if row.is_dead{rgb(255,107,107)}else{alive_color}}\nfn reference_owner_rank_color(row:&DpsRow)->u32{reference_meter_identity_color(row,if row.is_local{rgb(92,200,255)}else{crate::ui_modern::BPSR_MUTED})}",
        "shared dead-first rank palette",
    );

    // Normal uses the class-tinted row's existing readable alive-name color.
    replace_once(
        &mut source,
        "SetTextColor(hdc, if row.is_local && row.is_dead { rgb(255,107,107) } else { base });",
        "SetTextColor(hdc, reference_meter_identity_color(row, base));",
        "Normal name",
    );

    // Compact Raid calls the exact same player renderer as Compact.
    replace_once(
        &mut source,
        "if row.is_local && row.is_dead {\n            rgb(255, 107, 107)\n        } else if row.is_dead {\n            rgb(255, 135, 135)\n        } else {\n            base\n        },",
        "reference_meter_identity_color(row, base),",
        "Compact and Compact Raid name",
    );

    // The Raid name had the same self-only predicate as Normal.
    replace_once(
        &mut source,
        "SetTextColor(hdc, if row.is_local && row.is_dead { rgb(255,107,107) } else { rgb(241,243,247) });",
        "SetTextColor(hdc, reference_meter_identity_color(row, rgb(241,243,247)));",
        "Raid name",
    );

    // Assert every painted mode uses shared rank and name policy. Four layouts
    // are routed via Normal / Raid / Compact (also used by Compact Raid).
    assert_eq!(source.matches("SetTextColor(hdc, reference_owner_rank_color(row));").count(), 3);
    assert_eq!(source.matches("SetTextColor(hdc, reference_meter_identity_color(row, base));").count(), 1);
    assert_eq!(source.matches("reference_meter_identity_color(row, base),").count(), 1);
    assert_eq!(source.matches("SetTextColor(hdc, reference_meter_identity_color(row, rgb(241,243,247)));").count(), 1);

    source.push_str(r#"
#[cfg(test)]
mod v1400_dead_identity_palette_tests {
    use super::*;

    #[test]
    fn death_takes_priority_over_self_rank_and_name_for_every_player() {
        let alive_local = DpsRow { is_local: true, is_dead: false, ..DpsRow::default() };
        let alive_other = DpsRow { is_local: false, is_dead: false, ..DpsRow::default() };
        let dead_local = DpsRow { is_local: true, is_dead: true, ..DpsRow::default() };
        let dead_other = DpsRow { is_local: false, is_dead: true, ..DpsRow::default() };
        let red = rgb(255, 107, 107);
        let readable_alive_name = rgb(241, 243, 247);
        assert_eq!(reference_owner_rank_color(&alive_local), rgb(92, 200, 255));
        assert_eq!(reference_owner_rank_color(&alive_other), crate::ui_modern::BPSR_MUTED);
        for row in [&dead_local, &dead_other] {
            assert_eq!(reference_owner_rank_color(row), red);
            assert_eq!(reference_meter_identity_color(row, readable_alive_name), red);
            assert_eq!(reference_meter_identity_color(row, rgb(255, 255, 255)), red);
        }
        for row in [&alive_local, &alive_other] {
            assert_eq!(reference_meter_identity_color(row, readable_alive_name), readable_alive_name);
        }
    }
}
"#);
    fs::write(&path, source).expect("write dead identity palette");
    println!("cargo:rerun-if-changed=build/legacy/build_v1400_dead_identity_all_layouts.rs");
}
