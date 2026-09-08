use crate::{logging, paths::AppPaths};
use serde::{Deserialize, Serialize};
use std::{fs, io};

pub const ATTR_HEALING_MASTERY: i32 = 11442;
pub const ATTR_LUCK: i32 = 11446;
pub const ATTR_ACCURACY: i32 = 11447;
pub const ATTR_HASTE: i32 = 11463;
pub const ATTR_MASTERY: i32 = 11464;
pub const ATTR_VERSATILITY: i32 = 11465;

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
            mechanics: OverlayLayout { width: 540, height: 330, collapse_side: "Right".into(), ..OverlayLayout::default() },
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
    /// Attr ids displayed in the mechanics header strip. Kept as ids so new
    /// attributes can be added without a settings-format migration.
    pub tracked: Vec<i32>,
}
impl Default for MechanicAttributeSettings {
    fn default() -> Self { Self { tracked: vec![ATTR_LUCK, ATTR_HASTE, ATTR_MASTERY] } }
}

impl FeatureSettings {
    pub fn normalize(&mut self) {
        self.dps.normalize(420, 220);
        self.mechanics.normalize(400, 190);
        const ALLOWED: [i32; 6] = [ATTR_HEALING_MASTERY, ATTR_LUCK, ATTR_ACCURACY, ATTR_HASTE, ATTR_MASTERY, ATTR_VERSATILITY];
        self.mechanic_attributes.tracked.retain(|x| ALLOWED.contains(x));
        self.mechanic_attributes.tracked.dedup();
        self.mechanic_attributes.tracked.truncate(6);
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
    match id {
        ATTR_HEALING_MASTERY => "Healing",
        ATTR_LUCK => "Luck",
        ATTR_ACCURACY => "Accuracy",
        ATTR_HASTE => "Haste",
        ATTR_MASTERY => "Mastery",
        ATTR_VERSATILITY => "Versatility",
        _ => "Attr",
    }
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
