use crate::core::paths::{app_config_dir, config_file_path};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;

/// The layout and colour theme last used with a skin.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SkinPrefs {
    #[serde(default)]
    pub layout: String,
    #[serde(default)]
    pub color_theme: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default = "default_active_skin")]
    pub active_skin: String,
    #[serde(default)]
    pub skin_prefs: BTreeMap<String, SkinPrefs>,
}

impl AppSettings {
    /// Remembered layout and colour theme for a skin (empty fields mean "use the skin's default").
    pub fn prefs_for(&self, skin_id: &str) -> SkinPrefs {
        self.skin_prefs.get(skin_id).cloned().unwrap_or_default()
    }

    pub fn set_active(&mut self, skin_id: &str, prefs: SkinPrefs) {
        self.active_skin = skin_id.to_string();
        self.skin_prefs.insert(skin_id.to_string(), prefs);
    }
}

fn default_active_skin() -> String {
    "".to_string()
}


impl Default for AppSettings {
    fn default() -> Self {
        Self {
            active_skin: default_active_skin(),
            skin_prefs: BTreeMap::new(),
        }
    }
}

pub fn load_settings() -> AppSettings {
    let Some(path) = config_file_path() else {
        return AppSettings::default();
    };
    let Ok(data) = fs::read_to_string(path) else {
        return AppSettings::default();
    };
    serde_json::from_str(&data).unwrap_or_default()
}

pub fn save_settings(settings: &AppSettings) {
    if let Ok(json) = serde_json::to_string_pretty(settings)
        && let Some(cfg_dir) = app_config_dir()
    {
        let _ = fs::write(cfg_dir.join("config.json"), json);
    }
}
