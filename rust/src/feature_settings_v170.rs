use crate::{logging, paths::AppPaths};
use serde::{Deserialize, Serialize};
use std::{fs, io};

// Legacy/internal attributes still used outside the Mechanics stat picker.
pub const ATTR_DEFENSE_POWER: i32 = 0x0033;
pub const ATTR_BASE_STRENGTH: i32 = 0x0046;
pub const ATTR_ENDURANCE: i32 = 0x0067;
pub const ATTR_MOVEMENT_SPEED: i32 = 0x0074;
pub const ATTR_TOTAL_POWER: i32 = 0x0105;
pub const ATTR_PHYSICAL_ATTACK: i32 = 0x0106;
pub const ATTR_MAGIC_ATTACK: i32 = 0x0107;
pub const ATTR_LEVEL: i32 = 0x2710;
pub const ATTR_FIGHT_POINT: i32 = 0x272e;
pub const ATTR_RANK_LEVEL: i32 = 0x274c;

// Legacy raw ratings kept for compatibility with old history/settings data.
pub const ATTR_CRIT_RATING: i32 = 0x2b66;
pub const ATTR_HASTE_RATING: i32 = 0x2b70;
pub const ATTR_LUCK_RATING: i32 = 0x2b7a;
pub const ATTR_MASTERY_RATING: i32 = 0x2b84;
pub const ATTR_VERSATILITY_RATING: i32 = 0x2b8e;

pub const ATTR_CURRENT_HP: i32 = 0x2c2e;
pub const ATTR_MAX_HP: i32 = 0x2c38;
pub const ATTR_MAX_MP: i32 = 0x2c39;
pub const ATTR_STAMINA: i32 = 0x2c3c;
pub const ATTR_CURRENT_SHIELD: i32 = 0x2c3d;
pub const ATTR_MIN_ENERGY: i32 = 0x2c42;
pub const ATTR_MAX_ENERGY: i32 = 0x2c43;
pub const ATTR_ENERGY_REGEN: i32 = 0x2c46;
pub const ATTR_SEASON_STRENGTH: i32 = 0x2cb0;

// CN Resonance Logs panel attributes. Percent values are hundredths of one
// percent on the wire (e.g. 4_130 => 41.30%). Keep these ids aligned with
// `game.panelAttr.*` / DEFAULT_ATTRIBUTE_DISPLAYS in resonance-logs-cn.
pub const ATTR_PANEL_STRENGTH: i32 = 11_010;
pub const ATTR_PANEL_INTELLIGENCE: i32 = 11_020;
pub const ATTR_PANEL_AGILITY: i32 = 11_030;
pub const ATTR_PANEL_PHYSICAL_ATTACK: i32 = 11_330;
pub const ATTR_PANEL_MAGIC_ATTACK: i32 = 11_340;
pub const ATTR_PANEL_PHYSICAL_DEFENSE: i32 = 11_350;
pub const ATTR_CRIT: i32 = 11_710;
pub const ATTR_ATTACK_SPEED: i32 = 11_720;
pub const ATTR_CAST_SPEED: i32 = 11_730;
pub const ATTR_COOLDOWN_REDUCTION: i32 = 11_760;
pub const ATTR_LUCKY: i32 = 11_780;
pub const ATTR_SHIELD_STRENGTH: i32 = 11_810;
pub const ATTR_HASTE: i32 = 11_930;
pub const ATTR_MASTERY: i32 = 11_940;
pub const ATTR_VERSATILITY: i32 = 11_950;
pub const ATTR_CD_ACCELERATE_PCT: i32 = 11_960;
pub const ATTR_BLOCK: i32 = 11_970;
pub const ATTR_CRITICAL_DAMAGE: i32 = 12_510;
pub const ATTR_LUCKY_DAMAGE_MULTIPLIER: i32 = 12_530;
pub const ATTR_BLOCK_DAMAGE_REDUCTION: i32 = 12_540;

// Compatibility aliases for older internal names/settings. These ids were
// previously labelled as penetration/CD fields but are the CN panel attrs above.
pub const ATTR_SKILL_CD: i32 = 0x2de6;
pub const ATTR_SKILL_CD_PCT: i32 = ATTR_COOLDOWN_REDUCTION;
pub const ATTR_PHYSICAL_PENETRATION: i32 = ATTR_ATTACK_SPEED;
pub const ATTR_MAGIC_PENETRATION: i32 = ATTR_CAST_SPEED;
pub const ATTR_ELEMENTAL_RES_1: i32 = 0x3372;
pub const ATTR_ELEMENTAL_RES_2: i32 = 0x3373;
pub const ATTR_ELEMENTAL_RES_3: i32 = 0x3374;
pub const ATTR_LUCK: i32 = ATTR_LUCKY;
pub const ATTR_ILLUSION_BREAK: i32 = ATTR_SEASON_STRENGTH;
pub const ATTR_HEALING_MASTERY: i32 = 11_442;
pub const ATTR_ACCURACY: i32 = 11_447;

/// Exact Dungeon Mechanics stat picker requested for v1.16.4. Users may select
/// at most six for the compact overlay header.
pub const ATTRIBUTE_CATALOG: &[(i32, &str)] = &[
    (ATTR_VERSATILITY, "Versatility"),
    (ATTR_CAST_SPEED, "Cast Speed"),
    (ATTR_LUCKY, "Luck"),
    (ATTR_LUCKY_DAMAGE_MULTIPLIER, "Lucky Damage Multiplier"),
    (ATTR_CRITICAL_DAMAGE, "Critical Damage"),
    (ATTR_CRIT, "Crit Rate"),
    (ATTR_ATTACK_SPEED, "Attack Speed"),
    (ATTR_HASTE, "Haste"),
    (ATTR_MASTERY, "Mastery"),
    (ATTR_PANEL_STRENGTH, "Strength"),
    (ATTR_PANEL_INTELLIGENCE, "Intelligence"),
    (ATTR_PANEL_AGILITY, "Agility"),
    (ATTR_PANEL_PHYSICAL_ATTACK, "Physical Attack"),
    (ATTR_PANEL_MAGIC_ATTACK, "Magic Attack"),
    (ATTR_BLOCK, "Block"),
    (ATTR_PANEL_PHYSICAL_DEFENSE, "Physical Defense"),
    (ATTR_SHIELD_STRENGTH, "Shield Strength"),
    (ATTR_BLOCK_DAMAGE_REDUCTION, "Block Damage Reduction"),
    (ATTR_COOLDOWN_REDUCTION, "Cooldown Reduction"),
    (ATTR_CD_ACCELERATE_PCT, "Cooldown Acceleration"),
];

pub fn tracked_attr_ids() -> impl Iterator<Item = i32> {
    ATTRIBUTE_CATALOG.iter().map(|(id, _)| *id)
}

pub fn is_trackable_attr(id: i32) -> bool {
    ATTRIBUTE_CATALOG.iter().any(|(candidate, _)| *candidate == id)
}

pub const fn attr_is_percent(id: i32) -> bool {
    matches!(
        id,
        ATTR_VERSATILITY
            | ATTR_CAST_SPEED
            | ATTR_LUCKY
            | ATTR_LUCKY_DAMAGE_MULTIPLIER
            | ATTR_CRITICAL_DAMAGE
            | ATTR_CRIT
            | ATTR_ATTACK_SPEED
            | ATTR_HASTE
            | ATTR_MASTERY
            | ATTR_BLOCK
            | ATTR_SHIELD_STRENGTH
            | ATTR_BLOCK_DAMAGE_REDUCTION
            | ATTR_COOLDOWN_REDUCTION
            | ATTR_CD_ACCELERATE_PCT
    )
}

pub const fn attr_is_panel_integer(id: i32) -> bool {
    matches!(
        id,
        ATTR_PANEL_STRENGTH
            | ATTR_PANEL_INTELLIGENCE
            | ATTR_PANEL_AGILITY
            | ATTR_PANEL_PHYSICAL_ATTACK
            | ATTR_PANEL_MAGIC_ATTACK
            | ATTR_PANEL_PHYSICAL_DEFENSE
    )
}

fn integer_with_commas(value: i64) -> String {
    let negative = value < 0;
    let digits = value.unsigned_abs().to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3 + (if negative { 1 } else { 0 }));
    if negative { out.push('-'); }
    let first = digits.len() % 3;
    if first != 0 {
        out.push_str(&digits[..first]);
        if digits.len() > first { out.push(','); }
    }
    for (index, chunk) in digits[first..].as_bytes().chunks(3).enumerate() {
        if index > 0 { out.push(','); }
        out.push_str(std::str::from_utf8(chunk).unwrap_or(""));
    }
    out
}

pub fn format_attr_value(id: i32, value: i64) -> String {
    if attr_is_percent(id) {
        return format!("{:.2}%", value as f64 / 100.0);
    }
    if attr_is_panel_integer(id) {
        return integer_with_commas(value);
    }
    // Preserve compact legacy formatting for non-picker attributes used by the
    // entity inspector/history paths.
    let abs = value.unsigned_abs();
    if abs >= 1_000_000 {
        format!("{:.2}M", value as f64 / 1_000_000.0)
    } else if abs >= 100_000 {
        format!("{:.1}K", value as f64 / 1000.0)
    } else {
        value.to_string()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FeatureSettings {
    pub dps_overlay_enabled: bool,
    pub mechanics_overlay_enabled: bool,
    pub dps: OverlayLayout,
    pub mechanics: OverlayLayout,
    pub meter: MeterSettings,
    pub mechanic_attributes: MechanicAttributeSettings,
}

impl Default for FeatureSettings {
    fn default() -> Self {
        Self {
            dps_overlay_enabled: true,
            mechanics_overlay_enabled: true,
            dps: OverlayLayout { width: 650, height: 420, collapse_side: "Right".into(), ..OverlayLayout::default() },
            mechanics: OverlayLayout { width: 600, height: 410, collapse_side: "Right".into(), ..OverlayLayout::default() },
            meter: MeterSettings::default(),
            mechanic_attributes: MechanicAttributeSettings::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OverlayLayout {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub opacity: i32,
    pub collapse_side: String,
}

impl Default for OverlayLayout {
    fn default() -> Self {
        Self { x: i32::MIN, y: i32::MIN, width: 460, height: 300, opacity: 100, collapse_side: "Right".into() }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MeterSettings {
    pub show_damage_share: bool,
    pub show_healing_share: bool,
    pub show_tank_share: bool,
    pub show_deaths: bool,
    pub show_imagines: bool,
    pub show_target: bool,
    pub remember_scroll: bool,
    /// Show active rate first and encounter rate second (A/E) in live/history rows.
    pub show_active_rates: bool,
    /// Hide zero-contribution rows for the active Damage/Heal/Tank tab.
    pub only_contributors: bool,
    /// Limit rows to known members of the local party.
    pub party_only: bool,
    /// Keep the local player visible even when outside a configured row limit.
    pub always_show_self: bool,
    /// 0 = fit the window; otherwise cap rows to this count.
    pub visible_rows: usize,
    /// Maximum compressed encounter files retained locally.
    pub history_limit: usize,
}
impl Default for MeterSettings {
    fn default() -> Self {
        Self {
            show_damage_share: true,
            show_healing_share: true,
            show_tank_share: true,
            show_deaths: true,
            show_imagines: true,
            show_target: true,
            remember_scroll: false,
            show_active_rates: true,
            only_contributors: false,
            party_only: false,
            always_show_self: true,
            visible_rows: 0,
            history_limit: 50,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MechanicAttributeSettings {
    /// Protocol attr ids shown in the header strip. Maximum six.
    pub tracked: Vec<i32>,
}
impl Default for MechanicAttributeSettings {
    fn default() -> Self { Self { tracked: vec![ATTR_CRIT, ATTR_LUCKY, ATTR_HASTE, ATTR_MASTERY] } }
}

impl FeatureSettings {
    pub fn normalize(&mut self) {
        self.dps.normalize(600, 220);
        self.mechanics.normalize(400, 220);
        self.meter.visible_rows = match self.meter.visible_rows {
            0 | 5 | 10 | 20 | 30 | 50 => self.meter.visible_rows,
            value if value < 8 => 5,
            value if value < 15 => 10,
            value if value < 25 => 20,
            value if value < 40 => 30,
            _ => 50,
        };
        self.meter.history_limit = self.meter.history_limit.clamp(10, 200);

        // Migrate v1.7 and v1.8.0/1.8.1 selections by their UI meaning.
        const OLD_V17_LUCK: i32 = 11_446;
        const OLD_V17_HASTE: i32 = 11_463;
        const OLD_V17_MASTERY: i32 = 11_464;
        const OLD_V18_CRIT: i32 = 11_110;
        const OLD_V18_LUCK: i32 = 11_130;
        const OLD_V18_HASTE: i32 = 11_140;
        const OLD_V18_MASTERY: i32 = 11_150;
        for id in &mut self.mechanic_attributes.tracked {
            *id = match *id {
                OLD_V17_LUCK | OLD_V18_LUCK => ATTR_LUCKY,
                OLD_V17_HASTE | OLD_V18_HASTE => ATTR_HASTE,
                OLD_V17_MASTERY | OLD_V18_MASTERY => ATTR_MASTERY,
                OLD_V18_CRIT => ATTR_CRIT,
                other => other,
            };
        }
        self.mechanic_attributes.tracked.retain(|id| is_trackable_attr(*id));
        let mut dedup = Vec::with_capacity(6);
        for id in self.mechanic_attributes.tracked.drain(..) {
            if !dedup.contains(&id) { dedup.push(id); }
            if dedup.len() == 6 { break; }
        }
        self.mechanic_attributes.tracked = dedup;
    }
}

impl OverlayLayout {
    fn normalize(&mut self, min_w: i32, min_h: i32) {
        self.width = self.width.clamp(min_w, 2200);
        self.height = self.height.clamp(min_h, 1600);
        self.opacity = self.opacity.clamp(25, 100);
        if !matches!(self.collapse_side.to_ascii_lowercase().as_str(), "left" | "right" | "top" | "bottom") { self.collapse_side = "Right".into(); }
    }
}

pub fn attr_label(id: i32) -> &'static str {
    ATTRIBUTE_CATALOG.iter().find_map(|(candidate, label)| (*candidate == id).then_some(*label)).unwrap_or("Attr")
}

fn path(paths: &AppPaths) -> std::path::PathBuf { paths.root.join("features.json") }

pub fn load(paths: &AppPaths) -> FeatureSettings {
    let p = path(paths);
    if let Ok(text) = fs::read_to_string(&p) {
        match serde_json::from_str::<FeatureSettings>(&text) {
            Ok(mut value) => { value.normalize(); return value; }
            Err(err) => logging::write(format!("feature settings: load failed: {err}")),
        }
    }
    let mut value = FeatureSettings::default(); value.normalize(); let _ = save(paths, &value); value
}

pub fn save(paths: &AppPaths, settings: &FeatureSettings) -> io::Result<()> {
    let mut value = settings.clone(); value.normalize();
    let p = path(paths); let tmp = p.with_extension("json.new");
    let json = serde_json::to_string_pretty(&value).map_err(io::Error::other)?;
    fs::write(&tmp, json)?; if p.exists() { fs::remove_file(&p)?; } fs::rename(tmp, p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cn_panel_catalog_matches_requested_v1164_stats() {
        let ids: Vec<i32> = ATTRIBUTE_CATALOG.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, vec![
            11_950, 11_730, 11_780, 12_530, 12_510, 11_710, 11_720, 11_930, 11_940,
            11_010, 11_020, 11_030, 11_330, 11_340, 11_970, 11_350, 11_810, 12_540,
            11_760, 11_960,
        ]);
    }

    #[test]
    fn panel_percent_attributes_use_hundredths_and_keep_two_decimals() {
        assert_eq!(format_attr_value(ATTR_VERSATILITY, 2_226), "22.26%");
        assert_eq!(format_attr_value(ATTR_CAST_SPEED, 4_130), "41.30%");
        assert_eq!(format_attr_value(ATTR_CRITICAL_DAMAGE, 5_000), "50.00%");
        assert_eq!(format_attr_value(ATTR_LUCKY_DAMAGE_MULTIPLIER, 17_243), "172.43%");
        assert_eq!(format_attr_value(ATTR_BLOCK, 0), "0.00%");
    }

    #[test]
    fn panel_integer_attributes_are_raw_with_grouping() {
        assert_eq!(format_attr_value(ATTR_PANEL_STRENGTH, 622), "622");
        assert_eq!(format_attr_value(ATTR_PANEL_INTELLIGENCE, 5_804), "5,804");
        assert_eq!(format_attr_value(ATTR_PANEL_MAGIC_ATTACK, 4_102), "4,102");
        assert_eq!(format_attr_value(ATTR_PANEL_PHYSICAL_DEFENSE, 2_866), "2,866");
    }

    #[test]
    fn v181_tracked_ids_migrate_by_ui_meaning() {
        let mut value = FeatureSettings::default();
        value.mechanic_attributes.tracked = vec![11_110, 11_130, 11_140, 11_150];
        value.normalize();
        assert_eq!(value.mechanic_attributes.tracked, vec![ATTR_CRIT, ATTR_LUCKY, ATTR_HASTE, ATTR_MASTERY]);
    }

    #[test]
    fn grouped_dps_layout_keeps_safe_minimum_width() {
        let mut value = FeatureSettings::default();
        value.dps.width = 420;
        value.normalize();
        assert_eq!(value.dps.width, 600);
    }

    #[test]
    fn history_and_row_settings_are_bounded() {
        let mut value = FeatureSettings::default();
        value.meter.visible_rows = 13;
        value.meter.history_limit = 999;
        value.normalize();
        assert_eq!(value.meter.visible_rows, 10);
        assert_eq!(value.meter.history_limit, 200);
    }

    #[test]
    fn legacy_non_picker_attributes_keep_compact_numeric_format() {
        assert_eq!(format_attr_value(ATTR_CURRENT_HP, 247_900), "247.9K");
        assert_eq!(format_attr_value(ATTR_FIGHT_POINT, 58_404), "58404");
    }
}
