use crate::{logging, paths::AppPaths};
use serde::{Deserialize, Serialize};
use std::{fs, io};

// Current protocol ids mirrored from CN Resonance Logs `live/protocol/attrs.rs`.
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
pub const ATTR_CRIT: i32 = 0x2b66;
pub const ATTR_LUCKY: i32 = 0x2b7a;
pub const ATTR_HASTE: i32 = 0x2b84;
pub const ATTR_MASTERY: i32 = 0x2b8e;
pub const ATTR_CURRENT_HP: i32 = 0x2c2e;
pub const ATTR_MAX_HP: i32 = 0x2c38;
pub const ATTR_MAX_MP: i32 = 0x2c39;
pub const ATTR_STAMINA: i32 = 0x2c3c;
pub const ATTR_CURRENT_SHIELD: i32 = 0x2c3d;
pub const ATTR_MIN_ENERGY: i32 = 0x2c42;
pub const ATTR_MAX_ENERGY: i32 = 0x2c43;
pub const ATTR_ENERGY_REGEN: i32 = 0x2c46;
pub const ATTR_SEASON_STRENGTH: i32 = 0x2cb0;
pub const ATTR_PHYSICAL_PENETRATION: i32 = 0x2dc8;
pub const ATTR_MAGIC_PENETRATION: i32 = 0x2dd2;
pub const ATTR_SKILL_CD: i32 = 0x2de6;
pub const ATTR_SKILL_CD_PCT: i32 = 0x2df0;
pub const ATTR_CD_ACCELERATE_PCT: i32 = 0x2eb8;
pub const ATTR_ELEMENTAL_RES_1: i32 = 0x3372;
pub const ATTR_ELEMENTAL_RES_2: i32 = 0x3373;
pub const ATTR_ELEMENTAL_RES_3: i32 = 0x3374;

// Compatibility aliases for v1.7 settings and old internal names.
pub const ATTR_LUCK: i32 = ATTR_LUCKY;
pub const ATTR_ILLUSION_BREAK: i32 = ATTR_SEASON_STRENGTH;
pub const ATTR_HEALING_MASTERY: i32 = 11_442;
pub const ATTR_ACCURACY: i32 = 11_447;
pub const ATTR_VERSATILITY: i32 = 11_465;

/// Full compact catalog exposed by Dungeon Mechanics. Users may select at most six.
pub const ATTRIBUTE_CATALOG: &[(i32, &str)] = &[
    (ATTR_FIGHT_POINT, "Ability Score"),
    (ATTR_SEASON_STRENGTH, "Illusion Break"),
    (ATTR_LEVEL, "Level"),
    (ATTR_RANK_LEVEL, "Rank"),
    (ATTR_TOTAL_POWER, "Total Power"),
    (ATTR_PHYSICAL_ATTACK, "Physical ATK"),
    (ATTR_MAGIC_ATTACK, "Magic ATK"),
    (ATTR_DEFENSE_POWER, "Defense"),
    (ATTR_BASE_STRENGTH, "Base Strength"),
    (ATTR_ENDURANCE, "Endurance"),
    (ATTR_CRIT, "Crit"),
    (ATTR_LUCKY, "Lucky"),
    (ATTR_HASTE, "Haste"),
    (ATTR_MASTERY, "Mastery"),
    (ATTR_CURRENT_HP, "HP"),
    (ATTR_MAX_HP, "Max HP"),
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
    fn default() -> Self { Self { tracked: vec![ATTR_LUCKY, ATTR_HASTE, ATTR_MASTERY] } }
}

impl FeatureSettings {
    pub fn normalize(&mut self) {
        self.dps.normalize(420, 220);
        self.mechanics.normalize(400, 220);
        // Migrate v1.7's three defaults to their current protocol ids.
        const OLD_LUCK: i32 = 11_446;
        const OLD_HASTE: i32 = 11_463;
        const OLD_MASTERY: i32 = 11_464;
        for id in &mut self.mechanic_attributes.tracked {
            *id = match *id {
                OLD_LUCK => ATTR_LUCKY,
                OLD_HASTE => ATTR_HASTE,
                OLD_MASTERY => ATTR_MASTERY,
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
    ATTRIBUTE_CATALOG
        .iter()
        .find_map(|(candidate, label)| (*candidate == id).then_some(*label))
        .unwrap_or("Attr")
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
