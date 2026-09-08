use crate::{logging, paths::AppPaths};
use serde::{Deserialize, Serialize};
use std::{fs, io};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FeatureSettings {
    pub dps_overlay_enabled: bool,
    pub mechanics_overlay_enabled: bool,
    pub dps: OverlayLayout,
    pub mechanics: OverlayLayout,
}

impl Default for FeatureSettings {
    fn default() -> Self {
        Self {
            dps_overlay_enabled: true,
            mechanics_overlay_enabled: true,
            dps: OverlayLayout { width: 470, height: 330, collapse_side: "Right".into(), ..OverlayLayout::default() },
            mechanics: OverlayLayout { width: 500, height: 280, collapse_side: "Right".into(), ..OverlayLayout::default() },
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
        Self {
            x: i32::MIN,
            y: i32::MIN,
            width: 460,
            height: 300,
            opacity: 100,
            collapse_side: "Right".into(),
        }
    }
}

impl FeatureSettings {
    pub fn normalize(&mut self) {
        self.dps.normalize(360, 180);
        self.mechanics.normalize(380, 160);
    }
}

impl OverlayLayout {
    fn normalize(&mut self, min_w: i32, min_h: i32) {
        self.width = self.width.clamp(min_w, 1800);
        self.height = self.height.clamp(min_h, 1200);
        self.opacity = self.opacity.clamp(35, 100);
        if !matches!(self.collapse_side.to_ascii_lowercase().as_str(), "left" | "right" | "top" | "bottom") {
            self.collapse_side = "Right".into();
        }
    }
}

fn path(paths: &AppPaths) -> std::path::PathBuf { paths.root.join("features.json") }

pub fn load(paths: &AppPaths) -> FeatureSettings {
    let p = path(paths);
    if let Ok(text) = fs::read_to_string(&p) {
        match serde_json::from_str::<FeatureSettings>(&text) {
            Ok(mut value) => {
                value.normalize();
                return value;
            }
            Err(err) => logging::write(format!("feature settings: load failed: {err}")),
        }
    }
    let mut value = FeatureSettings::default();
    value.normalize();
    let _ = save(paths, &value);
    value
}

pub fn save(paths: &AppPaths, settings: &FeatureSettings) -> io::Result<()> {
    let mut value = settings.clone();
    value.normalize();
    let p = path(paths);
    let tmp = p.with_extension("json.new");
    let json = serde_json::to_string_pretty(&value).map_err(io::Error::other)?;
    fs::write(&tmp, json)?;
    if p.exists() { fs::remove_file(&p)?; }
    fs::rename(tmp, p)
}
