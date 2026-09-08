use crate::{logging, paths::AppPaths};
use serde::{Deserialize, Serialize};
use std::{fs, io};

// EntityAttr identifiers. The character sheet exposes separate rating and
// percentage attributes; do not format the raw ratings as percentages.
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

// Raw character-sheet ratings (ZDPS EnumEAttrType: 11110..11150).
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

// Actual character-panel percentages. ZDPS reads these values and displays
// value / 100, e.g. 4816 => 48.16%.
pub const ATTR_CRIT: i32 = 0x2dbe;                 // AttrCrit = 11710
pub const ATTR_SKILL_CD: i32 = 0x2de6;
pub const ATTR_SKILL_CD_PCT: i32 = 0x2df0;         // AttrSkillCDPCT = 11760
pub const ATTR_LUCKY: i32 = 0x2e04;                // AttrLuckyStrikeProb = 11780
pub const ATTR_HASTE: i32 = 0x2e9a;                // AttrHastePct = 11930
pub const ATTR_MASTERY: i32 = 0x2ea4;              // AttrMasteryPct = 11940
pub const ATTR_VERSATILITY: i32 = 0x2eae;           // AttrVersatilityPct = 11950
pub const ATTR_CD_ACCELERATE_PCT: i32 = 0x2eb8;     // AttrCdAcceleratePct = 11960

pub const ATTR_PHYSICAL_PENETRATION: i32 = 0x2dc8;
pub const ATTR_MAGIC_PENETRATION: i32 = 0x2dd2;
pub const ATTR_ELEMENTAL_RES_1: i32 = 0x3372;
pub const ATTR_ELEMENTAL_RES_2: i32 = 0x3373;
pub const ATTR_ELEMENTAL_RES_3: i32 = 0x3374;

// Compatibility aliases for v1.7 settings and old internal names.
pub const ATTR_LUCK: i32 = ATTR_LUCKY;
pub const ATTR_ILLUSION_BREAK: i32 = ATTR_SEASON_STRENGTH;
pub const ATTR_HEALING_MASTERY: i32 = 11_442;
pub const ATTR_ACCURACY: i32 = 11_447;

/// Full compact catalog exposed by Dungeon Mechanics. Users may select at most six.
pub const ATTRIBUTE_CATALOG: &[(i32, &str)] = &[
    (ATTR_CRIT, "Crit %"),
    (ATTR_LUCKY, "Lucky %"),
    (ATTR_HASTE, "Haste %"),
    (ATTR_MASTERY, "Mastery %"),
    (ATTR_VERSATILITY, "Versatility %"),
    (ATTR_CRIT_RATING, "Crit Rating"),
    (ATTR_LUCK_RATING, "Luck Rating"),
    (ATTR_HASTE_RATING, "Haste Rating"),
    (ATTR_MASTERY_RATING, "Mastery Rating"),
    (ATTR_VERSATILITY_RATING, "Versatility Rating"),
    (ATTR_FIGHT_POINT, "Ability Score"),
    (ATTR_SEASON_STRENGTH, "Illusion Break"),
    (ATTR_CURRENT_HP, "HP"),
    (ATTR_MAX_HP, "Max HP"),
    (ATTR_LEVEL, "Level"),
    (ATTR_RANK_LEVEL, "Rank"),
    (ATTR_TOTAL_POWER, "Total Power"),
    (ATTR_PHYSICAL_ATTACK, "Physical ATK"),
    (ATTR_MAGIC_ATTACK, "Magic ATK"),
    (ATTR_DEFENSE_POWER, "Defense"),
    (ATTR_BASE_STRENGTH, "Base Strength"),
    (ATTR_ENDURANCE, "Endurance"),
    (ATTR_MAX_MP, "Max MP"),
    (ATTR_STAMINA, "Stamina"),
    (ATTR_CURRENT_SHIELD, "Shield"),
    (ATTR_MIN_ENERGY, "Min Energy"),
    (ATTR_MAX_ENERGY, "Max Energy"),
    (ATTR_ENERGY_REGEN, "Energy Regen"),
    (ATTR_PHYSICAL_PENETRATION, "Physical Pen"),
    (ATTR_MAGIC_PENETRATION, "Magic Pen"),
    (ATTR_SKILL_CD, "Skill CD"),
    (ATTR_SKILL_CD_PCT, "Skill CD %"),
    (ATTR_CD_ACCELERATE_PCT, "CD Accel %"),
    (ATTR_MOVEMENT_SPEED, "Move Speed"),
    (ATTR_ELEMENTAL_RES_1, "Element Res 1"),
    (ATTR_ELEMENTAL_RES_2, "Element Res 2"),
    (ATTR_ELEMENTAL_RES_3, "Element Res 3"),
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
        ATTR_CRIT
            | ATTR_LUCKY
            | ATTR_HASTE
            | ATTR_MASTERY
            | ATTR_VERSATILITY
            | ATTR_SKILL_CD_PCT
            | ATTR_CD_ACCELERATE_PCT
    )
}

pub fn format_attr_value(id: i32, value: i64) -> String {
    if attr_is_percent(id) {
        let percent = value as f64 / 100.0;
        let mut text = format!("{percent:.2}");
        while text.contains('.') && text.ends_with('0') { text.pop(); }
        if text.ends_with('.') { text.pop(); }
        return format!("{text}%");
    }
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
}
impl Default for MeterSettings {
    fn default() -> Self {
        Self { show_damage_share: true, show_healing_share: true, show_tank_share: true, show_deaths: true, show_imagines: true, show_target: true, remember_scroll: false }
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
        // The v1.8.3 grouped row reserves independent left identity, middle
        // throughput and right share regions. Below 600 px those regions can
        // physically overlap, so prevent resizing into an unreadable state.
        self.dps.normalize(600, 220);
        self.mechanics.normalize(400, 220);

        // Migrate v1.7 and v1.8.0/1.8.1 selections by their UI meaning. Those
        // releases used raw rating ids (and two shifted ids) but labelled them as
        // percentages. Preserve the user's chosen labels while moving to the
        // real character-panel percentage attributes.
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
    fn panel_rate_attributes_use_hundredths_of_a_percent() {
        assert_eq!(format_attr_value(ATTR_LUCKY, 4_816), "48.16%");
        assert_eq!(format_attr_value(ATTR_HASTE, 4_167), "41.67%");
        assert_eq!(format_attr_value(ATTR_MASTERY, 564), "5.64%");
    }

    #[test]
    fn raw_ratings_are_not_mislabelled_as_percentages() {
        assert_eq!(format_attr_value(ATTR_CRIT_RATING, 4_816), "4816");
        assert_eq!(format_attr_value(ATTR_HASTE_RATING, 3_250), "3250");
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
    fn non_rate_attributes_keep_compact_numeric_format() {
        assert_eq!(format_attr_value(ATTR_CURRENT_HP, 247_900), "247.9K");
        assert_eq!(format_attr_value(ATTR_FIGHT_POINT, 58_404), "58404");
    }
}
